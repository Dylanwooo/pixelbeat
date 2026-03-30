use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};

/// A single track from a YouTube playlist
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YtTrack {
    pub video_id: String,
    pub title: String,
    pub duration: f64,
}

const MPV_SOCKET: &str = "/tmp/pixelbeat-mpv.sock";

/// Check if yt-dlp is installed
pub fn is_ytdlp_available() -> bool {
    Command::new("which")
        .arg("yt-dlp")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if mpv is installed
pub fn is_mpv_available() -> bool {
    Command::new("which")
        .arg("mpv")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Fetch all tracks from a YouTube playlist/video URL using yt-dlp
pub fn fetch_playlist(url: &str) -> Result<Vec<YtTrack>> {
    if !is_ytdlp_available() {
        if cfg!(target_os = "macos") {
            anyhow::bail!("yt-dlp is not installed. Install with: brew install yt-dlp");
        } else {
            anyhow::bail!(
                "yt-dlp is not installed. Install with: pip install yt-dlp / apt install yt-dlp"
            );
        }
    }

    eprintln!("pixelbeat: fetching YouTube playlist info...");

    let output = Command::new("yt-dlp")
        .args([
            "--flat-playlist",
            "--dump-json",
            "--no-warnings",
            "--quiet",
            url,
        ])
        .output()
        .context("Failed to run yt-dlp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tracks = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: serde_json::Value =
            serde_json::from_str(line).with_context(|| "Failed to parse yt-dlp JSON")?;

        let video_id = entry
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if video_id.is_empty() {
            continue;
        }

        let title = entry
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let duration = entry
            .get("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        tracks.push(YtTrack {
            video_id,
            title,
            duration,
        });
    }

    eprintln!("pixelbeat: found {} tracks in playlist", tracks.len());
    Ok(tracks)
}

/// mpv process handle for streaming YouTube audio
pub struct MpvPlayer {
    process: Option<Child>,
    /// Persistent IPC connection (reused across queries, ~100x fewer syscalls)
    conn: Option<UnixStream>,
}

impl MpvPlayer {
    pub fn new() -> Self {
        Self {
            process: None,
            conn: None,
        }
    }

    /// Start streaming a YouTube URL via mpv (instant playback, no download)
    pub fn play_url(
        &mut self,
        url: &str,
        volume: f32,
        cookies_browser: Option<&str>,
    ) -> Result<()> {
        if !is_mpv_available() {
            if cfg!(target_os = "macos") {
                anyhow::bail!("mpv is not installed. Install with: brew install mpv");
            } else {
                anyhow::bail!("mpv is not installed. Install with: apt install mpv / pacman -S mpv / dnf install mpv");
            }
        }

        self.stop();
        std::fs::remove_file(MPV_SOCKET).ok();

        let vol = (volume * 100.0) as u32;
        eprintln!("pixelbeat: streaming via mpv...");

        let mut args = vec![
            "--no-video".to_string(),
            "--ytdl-format=bestaudio".to_string(),
            format!("--volume={}", vol),
            "--really-quiet".to_string(),
            format!("--input-ipc-server={}", MPV_SOCKET),
        ];

        if let Some(browser) = cookies_browser {
            args.push(format!(
                "--ytdl-raw-options=cookies-from-browser={}",
                browser
            ));
        }

        args.push(url.to_string());

        let child = Command::new("mpv")
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn mpv")?;

        self.process = Some(child);

        // Wait for socket, then establish persistent connection
        for _ in 0..30 {
            if std::path::Path::new(MPV_SOCKET).exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(MPV_SOCKET, std::fs::Permissions::from_mode(0o700))
                        .ok();
                }
                self.connect();
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Ok(())
    }

    /// Establish persistent IPC connection to mpv
    fn connect(&mut self) {
        if let Ok(stream) = UnixStream::connect(MPV_SOCKET) {
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .ok();
            stream
                .set_write_timeout(Some(std::time::Duration::from_secs(1)))
                .ok();
            self.conn = Some(stream);
        }
    }

    /// Send a JSON command via persistent connection (reconnects once on failure)
    fn send_command(&mut self, cmd: &serde_json::Value) -> Result<serde_json::Value> {
        for attempt in 0..2 {
            if self.conn.is_none() {
                self.connect();
            }

            let conn = match self.conn.as_ref() {
                Some(s) => s,
                None => {
                    if attempt == 0 {
                        continue;
                    }
                    anyhow::bail!("No mpv IPC connection");
                }
            };

            let read_stream = match conn.try_clone() {
                Ok(s) => s,
                Err(_) => {
                    self.conn = None;
                    continue;
                }
            };

            let mut cmd_str = serde_json::to_string(cmd)?;
            cmd_str.push('\n');

            if let Err(_) = conn
                .try_clone()
                .and_then(|mut s| s.write_all(cmd_str.as_bytes()))
            {
                self.conn = None;
                if attempt == 0 {
                    continue;
                }
                anyhow::bail!("Failed to write to mpv IPC");
            }

            let reader = BufReader::new(read_stream);
            for line in reader.lines() {
                match line {
                    Ok(l) if !l.is_empty() => {
                        if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&l) {
                            if resp.get("error").is_some() {
                                return Ok(resp);
                            }
                        }
                    }
                    Err(_) => {
                        self.conn = None;
                        break;
                    }
                    _ => continue,
                }
            }

            if attempt == 0 {
                self.conn = None;
                continue;
            }
        }

        anyhow::bail!("No response from mpv")
    }

    pub fn get_property(&mut self, prop: &str) -> Option<serde_json::Value> {
        let cmd = serde_json::json!({"command": ["get_property", prop]});
        self.send_command(&cmd)
            .ok()
            .and_then(|r| r.get("data").cloned())
    }

    pub fn set_property(&mut self, prop: &str, value: serde_json::Value) -> Result<()> {
        let cmd = serde_json::json!({"command": ["set_property", prop, value]});
        self.send_command(&cmd)?;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        self.set_property("pause", serde_json::json!(true))
    }

    pub fn resume(&mut self) -> Result<()> {
        self.set_property("pause", serde_json::json!(false))
    }

    pub fn toggle_pause(&mut self) -> Result<()> {
        let paused = self
            .get_property("pause")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.set_property("pause", serde_json::json!(!paused))
    }

    pub fn set_volume(&mut self, vol: f32) -> Result<()> {
        self.set_property("volume", serde_json::json!((vol * 100.0) as u32))
    }

    pub fn get_position(&mut self) -> f64 {
        self.get_property("time-pos")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    pub fn get_duration(&mut self) -> f64 {
        self.get_property("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    pub fn get_title(&mut self) -> String {
        self.get_property("media-title")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default()
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.process = None;
                    self.conn = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn is_eof(&mut self) -> bool {
        self.get_property("eof-reached")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn is_paused(&mut self) -> bool {
        self.get_property("pause")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn stop(&mut self) {
        self.conn = None;
        if let Some(ref mut child) = self.process {
            child.kill().ok();
            child.wait().ok();
        }
        self.process = None;
        std::fs::remove_file(MPV_SOCKET).ok();
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
