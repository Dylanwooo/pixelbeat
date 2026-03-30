use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::player::{Player, PlayerState};

const SOCKET_NAME: &str = "pixelbeat.sock";

pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    runtime_dir.join(SOCKET_NAME)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd")]
pub enum Command {
    #[serde(rename = "play")]
    Play { path: Option<String> },
    #[serde(rename = "pause")]
    Pause,
    #[serde(rename = "toggle")]
    Toggle,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "next")]
    Next,
    #[serde(rename = "prev")]
    Prev,
    #[serde(rename = "volume")]
    Volume { level: f32 },
    #[serde(rename = "shuffle")]
    Shuffle { enabled: bool },
    #[serde(rename = "repeat")]
    Repeat { enabled: bool },
    #[serde(rename = "radio")]
    Radio { station: String },
    #[serde(rename = "youtube")]
    YouTube { url: String },
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "quit")]
    Quit,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<PlayerState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn ok(state: Option<PlayerState>) -> Self {
        Self {
            ok: true,
            state,
            error: None,
        }
    }

    pub fn err(msg: &str) -> Self {
        Self {
            ok: false,
            state: None,
            error: Some(msg.to_string()),
        }
    }
}

/// Handle a single client connection
fn handle_client(stream: UnixStream, player: &Arc<Mutex<Player>>) -> Result<bool> {
    let reader = BufReader::new(&stream);
    let mut writer = stream.try_clone()?;

    for line in reader.lines() {
        let line = line.context("Failed to read from client")?;
        if line.is_empty() {
            continue;
        }

        let cmd: Command = match serde_json::from_str(&line) {
            Ok(cmd) => cmd,
            Err(e) => {
                let resp = Response::err(&format!("Invalid command: {}", e));
                let _ = writeln!(writer, "{}", serde_json::to_string(&resp)?);
                continue;
            }
        };

        // Handle radio separately — blocking download + play, return result to client
        if let Command::Radio { ref station } = cmd {
            if crate::daemon::radio::find_station(station).is_none() {
                let stations = crate::daemon::radio::list_stations().join(", ");
                let resp = Response::err(&format!(
                    "Unknown station '{}'. Available: {}",
                    station, stations
                ));
                writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
                continue;
            }
            // Set loading state
            {
                let p = player.lock().unwrap();
                let mut state = p.state.lock().unwrap();
                state.title = format!("loading {}...", station);
            }
            // Download and play (blocking), then return result to client
            let p = player.lock().unwrap();
            let resp = match p.play_radio(station) {
                Ok(_) => Response::ok(Some(p.get_state())),
                Err(e) => {
                    eprintln!("pixelbeat: radio error: {}", e);
                    Response::err(&format!("Radio error: {}", e))
                }
            };
            writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
            continue;
        }

        // Handle YouTube separately — fetching playlist + resolving audio is slow
        if let Command::YouTube { ref url } = cmd {
            // Set loading state
            {
                let p = player.lock().unwrap();
                let mut state = p.state.lock().unwrap();
                state.title = "loading YouTube playlist...".to_string();
            }
            // Fetch and play (blocking), then return result to client
            let p = player.lock().unwrap();
            let resp = match p.play_youtube(url) {
                Ok(_) => Response::ok(Some(p.get_state())),
                Err(e) => {
                    eprintln!("pixelbeat: youtube error: {}", e);
                    Response::err(&format!("YouTube error: {}", e))
                }
            };
            writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
            continue;
        }

        let player = player.lock().unwrap();
        let response = match cmd {
            Command::Play { path } => {
                if let Some(p) = path {
                    let path_buf = PathBuf::from(&p);
                    if let Err(e) = player.load_path(&path_buf) {
                        Response::err(&format!("Failed to load: {}", e))
                    } else {
                        match player.play() {
                            Ok(_) => Response::ok(Some(player.get_state())),
                            Err(e) => Response::err(&format!("Failed to play: {}", e)),
                        }
                    }
                } else {
                    match player.play() {
                        Ok(_) => Response::ok(Some(player.get_state())),
                        Err(e) => Response::err(&format!("Failed to play: {}", e)),
                    }
                }
            }
            Command::Pause => {
                player.pause();
                Response::ok(Some(player.get_state()))
            }
            Command::Toggle => match player.toggle() {
                Ok(_) => Response::ok(Some(player.get_state())),
                Err(e) => Response::err(&format!("{}", e)),
            },
            Command::Stop => {
                player.stop();
                Response::ok(Some(player.get_state()))
            }
            Command::Next => match player.next() {
                Ok(_) => Response::ok(Some(player.get_state())),
                Err(e) => Response::err(&format!("{}", e)),
            },
            Command::Prev => match player.prev() {
                Ok(_) => Response::ok(Some(player.get_state())),
                Err(e) => Response::err(&format!("{}", e)),
            },
            Command::Volume { level } => {
                player.set_volume(level);
                Response::ok(Some(player.get_state()))
            }
            Command::Shuffle { enabled } => {
                player.set_shuffle(enabled);
                Response::ok(Some(player.get_state()))
            }
            Command::Repeat { enabled } => {
                player.set_repeat(enabled);
                Response::ok(Some(player.get_state()))
            }
            Command::Radio { .. } => unreachable!(), // handled above
            Command::YouTube { .. } => unreachable!(), // handled above
            Command::Status => Response::ok(Some(player.get_state())),
            Command::Quit => {
                let resp = Response::ok(None);
                let _ = writeln!(writer, "{}", serde_json::to_string(&resp)?);
                return Ok(true); // Signal to quit
            }
        };

        writeln!(writer, "{}", serde_json::to_string(&response)?)?;
    }

    Ok(false)
}

/// Start the IPC server
/// `autoplay_radio`: if Some, start radio after socket is bound
pub fn start_server(player: Arc<Mutex<Player>>, autoplay_radio: Option<String>) -> Result<()> {
    let path = socket_path();

    // Clean up stale socket
    if path.exists() {
        std::fs::remove_file(&path).ok();
    }

    let listener = UnixListener::bind(&path).context("Failed to bind Unix socket")?;

    // Restrict socket permissions to owner-only
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).ok();
    }
    listener
        .set_nonblocking(true)
        .context("Failed to set non-blocking")?;

    eprintln!("pixelbeat daemon listening on {}", path.display());

    // Start radio/youtube after socket is ready
    if let Some(ref source) = autoplay_radio {
        let p = player.lock().unwrap();
        if let Some(yt_url) = source.strip_prefix("youtube:") {
            if let Err(e) = p.play_youtube(yt_url) {
                eprintln!("pixelbeat: youtube error: {}", e);
            }
        } else {
            if let Err(e) = p.play_radio(source) {
                eprintln!("pixelbeat: radio '{}' error: {}", source, e);
            }
        }
    }

    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).ok();
                match handle_client(stream, &player) {
                    Ok(true) => {
                        // Quit command received
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        eprintln!("Client error: {}", e);
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No pending connection, tick the player
                {
                    let player = player.lock().unwrap();
                    player.tick().ok();
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }

    // Cleanup
    std::fs::remove_file(&path).ok();
    Ok(())
}

/// Check if the daemon is running by attempting to connect to the socket
pub fn is_daemon_running() -> bool {
    let path = socket_path();
    UnixStream::connect(&path).is_ok()
}

/// Send a command to the daemon and get a response
pub fn send_command(cmd: &Command) -> Result<Response> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .context("Cannot connect to pixelbeat daemon. Is it running? Start with: px daemon")?;

    let cmd_json = serde_json::to_string(cmd)?;
    writeln!(stream, "{}", cmd_json)?;

    let reader = BufReader::new(&stream);
    for line in reader.lines() {
        let line = line?;
        if !line.is_empty() {
            let resp: Response = serde_json::from_str(&line)?;
            return Ok(resp);
        }
    }

    anyhow::bail!("No response from daemon")
}
