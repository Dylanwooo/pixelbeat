use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// A single track from a YouTube playlist
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YtTrack {
    pub video_id: String,
    pub title: String,
    pub duration: f64,
}

/// Check if yt-dlp is installed and available on PATH
pub fn is_ytdlp_available() -> bool {
    Command::new("which")
        .arg("yt-dlp")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Fetch all tracks from a YouTube playlist using --flat-playlist.
/// Each line of JSON output contains a video entry with id, title, duration.
pub fn fetch_playlist(url: &str) -> Result<Vec<YtTrack>> {
    if !is_ytdlp_available() {
        anyhow::bail!(
            "yt-dlp is not installed. Install it with: brew install yt-dlp"
        );
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
        // Parse each JSON line — yt-dlp --flat-playlist outputs one JSON object per line
        let entry: serde_json::Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse yt-dlp JSON line"))?;

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

/// Resolve the direct audio URL for a video using yt-dlp --get-url.
/// The returned URL is typically a googlevideo.com URL valid for ~6 hours.
pub fn resolve_audio_url(video_id: &str) -> Result<String> {
    let video_url = format!("https://www.youtube.com/watch?v={}", video_id);

    eprintln!("pixelbeat: resolving audio URL for {}...", video_id);

    let output = Command::new("yt-dlp")
        .args([
            "-f",
            "bestaudio",
            "--get-url",
            "--no-warnings",
            "--quiet",
            &video_url,
        ])
        .output()
        .context("Failed to run yt-dlp --get-url")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp --get-url failed: {}", stderr.trim());
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if url.is_empty() {
        anyhow::bail!("yt-dlp returned empty URL for {}", video_id);
    }

    Ok(url)
}

/// Download audio bytes for a video using yt-dlp piped to stdout.
/// This avoids the 403 issue with --get-url since yt-dlp handles auth internally.
/// Returns (audio_bytes, title, duration_seconds).
pub fn download_audio(video_id: &str, title: &str, duration: f64) -> Result<(Vec<u8>, String, f64)> {
    let video_url = format!("https://www.youtube.com/watch?v={}", video_id);

    eprintln!("pixelbeat: downloading audio for '{}' via yt-dlp...", title);

    // yt-dlp outputs Opus/WebM which rodio may not decode directly.
    // Pipe through ffmpeg to convert to WAV (PCM) for guaranteed compatibility.
    use std::process::Stdio;

    let ytdlp = Command::new("yt-dlp")
        .args([
            "-f", "bestaudio",
            "-o", "-",
            "--cookies-from-browser", "chrome",
            "--no-warnings",
            "--quiet",
            "--no-playlist",
            &video_url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn yt-dlp")?;

    let output = Command::new("ffmpeg")
        .args([
            "-i", "pipe:0",        // read from stdin
            "-f", "wav",           // output WAV format
            "-acodec", "pcm_s16le",
            "-ar", "44100",        // 44.1kHz
            "-ac", "2",            // stereo
            "pipe:1",              // write to stdout
        ])
        .stdin(ytdlp.stdout.unwrap())
        .stderr(Stdio::null())
        .output()
        .context("Failed to run ffmpeg conversion (is ffmpeg installed?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp download failed for {}: {}", video_id, stderr.trim());
    }

    let bytes = output.stdout;
    if bytes.is_empty() {
        anyhow::bail!("yt-dlp returned empty audio for {}", video_id);
    }

    eprintln!("pixelbeat: downloaded {} bytes for '{}'", bytes.len(), title);
    Ok((bytes, title.to_string(), duration))
}
