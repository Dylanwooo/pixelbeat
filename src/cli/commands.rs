use anyhow::Result;

use crate::cli::format;
use crate::daemon::ipc::{self, Command, Response};

pub fn handle_play(path: Option<String>) -> Result<()> {
    let resp = ipc::send_command(&Command::Play { path })?;
    print_response(&resp);
    Ok(())
}

pub fn handle_pause() -> Result<()> {
    let resp = ipc::send_command(&Command::Pause)?;
    print_response(&resp);
    Ok(())
}

pub fn handle_toggle() -> Result<()> {
    let resp = ipc::send_command(&Command::Toggle)?;
    print_response(&resp);
    Ok(())
}

pub fn handle_stop() -> Result<()> {
    let resp = ipc::send_command(&Command::Stop)?;
    print_response(&resp);
    Ok(())
}

pub fn handle_next() -> Result<()> {
    let resp = ipc::send_command(&Command::Next)?;
    print_response(&resp);
    Ok(())
}

pub fn handle_prev() -> Result<()> {
    let resp = ipc::send_command(&Command::Prev)?;
    print_response(&resp);
    Ok(())
}

pub fn handle_volume(level: f32) -> Result<()> {
    let resp = ipc::send_command(&Command::Volume { level })?;
    print_response(&resp);
    Ok(())
}

pub fn handle_shuffle(enabled: bool) -> Result<()> {
    let resp = ipc::send_command(&Command::Shuffle { enabled })?;
    print_response(&resp);
    Ok(())
}

pub fn handle_repeat(enabled: bool) -> Result<()> {
    let resp = ipc::send_command(&Command::Repeat { enabled })?;
    print_response(&resp);
    Ok(())
}

pub fn handle_status(fmt: Option<String>) -> Result<()> {
    let resp = ipc::send_command(&Command::Status)?;
    if let Some(state) = &resp.state {
        let fmt_str = fmt
            .as_deref()
            .unwrap_or(format::default_statusline_format());
        println!("{}", format::render_format(fmt_str, state));
    } else if !resp.ok {
        if let Some(err) = &resp.error {
            eprintln!("Error: {}", err);
        }
    }
    Ok(())
}

pub fn handle_radio(station: &str) -> Result<()> {
    eprintln!("Connecting to {} radio...", station);
    let resp = ipc::send_command(&Command::Radio {
        station: station.to_string(),
    })?;
    if resp.ok {
        if let Some(state) = &resp.state {
            eprintln!("📻 {} ▶", state.title);
        }
    } else if let Some(err) = &resp.error {
        eprintln!("Error: {}", err);
    }
    Ok(())
}

pub fn handle_youtube(url: &str) -> Result<()> {
    eprintln!("Fetching YouTube playlist...");
    let resp = ipc::send_command(&Command::YouTube {
        url: url.to_string(),
    })?;
    if resp.ok {
        eprintln!("YouTube playlist queued for playback");
    } else if let Some(err) = &resp.error {
        eprintln!("Error: {}", err);
    }
    Ok(())
}

pub fn handle_quit() -> Result<()> {
    let resp = ipc::send_command(&Command::Quit)?;
    if resp.ok {
        eprintln!("pixelbeat daemon stopped");
    }
    Ok(())
}

fn print_response(resp: &Response) {
    if !resp.ok {
        if let Some(err) = &resp.error {
            eprintln!("Error: {}", err);
        }
    } else if let Some(state) = &resp.state {
        let icon = if state.playing { "▶" } else { "⏸" };
        if !state.title.is_empty() {
            eprintln!(
                "{} {} [{}/{}]",
                icon,
                state.title,
                state.track_index + 1,
                state.track_count
            );
        }
    }
}
