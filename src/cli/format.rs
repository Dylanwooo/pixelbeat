use crate::daemon::player::PlayerState;

/// Anthropic orange palette
const ORANGE_PRIMARY: &str = "\x1b[38;2;227;137;62m"; // #E3893E - Anthropic orange
const ORANGE_BRIGHT: &str = "\x1b[38;2;255;170;80m"; // Brighter orange for highlights
const ORANGE_DIM: &str = "\x1b[38;2;140;85;40m"; // Dim orange for inactive
const ORANGE_BG: &str = "\x1b[38;2;60;35;15m"; // Very dim for background elements
const WARM_WHITE: &str = "\x1b[38;2;200;180;160m"; // Warm white for box drawing
const RESET: &str = "\x1b[0m";

const SPECTRUM_BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Truncate a string to at most `max_chars` characters, appending "…" if truncated.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max_chars.saturating_sub(1)).collect();
    if chars.next().is_some() {
        // String was longer than max_chars
        format!("{}…", truncated)
    } else {
        // String fit within max_chars
        s.to_string()
    }
}

/// Format a duration in seconds to M:SS
fn format_time(secs: f64) -> String {
    let mins = secs as u64 / 60;
    let secs = secs as u64 % 60;
    format!("{}:{:02}", mins, secs)
}

/// Render the cassette tape reel visualization
///
/// Shows two reels with tape running between them:
///   ◎ ━━━━━━━●─────────── ◎
///   Left reel = supply (shrinks), Right reel = takeup (grows)
///   ● = playhead position
fn render_tape(progress: f64, width: usize) -> String {
    let progress = progress.clamp(0.0, 1.0);

    // Reel characters based on how much tape each side has
    // Left reel: starts full (supply), gets empty
    // Right reel: starts empty (takeup), gets full
    let left_reel = match ((1.0 - progress) * 4.0) as usize {
        4 => "◉",
        3 => "◉",
        2 => "◎",
        1 => "◎",
        _ => "○",
    };
    let right_reel = match (progress * 4.0) as usize {
        4 => "◉",
        3 => "◉",
        2 => "◎",
        1 => "◎",
        _ => "○",
    };

    let tape_width = width.saturating_sub(6); // space for reels + gaps
    let head_pos = (progress * tape_width as f64).round() as usize;
    let head_pos = head_pos.min(tape_width.saturating_sub(1));

    let played = head_pos;
    let remaining = tape_width.saturating_sub(head_pos + 1);

    format!(
        "{}{}{} {}{}{}●{}{}{}{}{}",
        ORANGE_DIM,
        left_reel,
        RESET,
        ORANGE_BG,
        "─".repeat(played),
        ORANGE_BRIGHT,
        ORANGE_DIM,
        "━".repeat(remaining),
        RESET,
        " ",
        format!("{}{}{}", ORANGE_DIM, right_reel, RESET),
    )
}

/// Render the spectrum bars with Anthropic orange gradient
fn render_spectrum(spectrum: &[f32], width: usize) -> String {
    let bars: usize = width.min(spectrum.len());
    let mut result = String::new();

    for i in 0..bars {
        let val = spectrum.get(i).copied().unwrap_or(0.0);
        let idx = (val * (SPECTRUM_BARS.len() - 1) as f32).round() as usize;
        let idx = idx.min(SPECTRUM_BARS.len() - 1);

        if val > 0.7 {
            result.push_str(ORANGE_BRIGHT);
        } else if val > 0.3 {
            result.push_str(ORANGE_PRIMARY);
        } else {
            result.push_str(ORANGE_DIM);
        }
        result.push(SPECTRUM_BARS[idx]);
    }
    result.push_str(RESET);
    result
}

/// Render a progress bar
fn render_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;

    format!(
        "{}{}{}{}{}",
        ORANGE_BRIGHT,
        "█".repeat(filled),
        ORANGE_BG,
        "░".repeat(empty),
        RESET,
    )
}

/// Render the play/pause icon
fn render_icon(playing: bool) -> &'static str {
    if playing {
        "▶"
    } else {
        "⏸"
    }
}

/// Render the full cassette deck status line (multi-line)
/// Returns a complete cassette deck widget for the status line
pub fn render_cassette(state: &PlayerState, width: usize) -> String {
    let progress = if state.duration > 0.0 {
        (state.position / state.duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let icon = render_icon(state.playing);
    let elapsed = format_time(state.position);
    let duration = format_time(state.duration);

    // Truncate title
    let max_title = width.saturating_sub(20);
    let title = truncate_chars(&state.title, max_title);

    // Mode indicators
    let mut modes = String::new();
    if state.repeat {
        modes.push_str(&format!("{}🔁{}", ORANGE_BRIGHT, RESET));
    }
    if state.shuffle {
        modes.push_str(&format!("{}🔀{}", ORANGE_BRIGHT, RESET));
    }

    // Build the inner width for the box
    let inner_w = width.saturating_sub(4); // │ + space + space + │

    // Line 1: Top border with label
    let label = "PIXELBEAT";
    let border_after = inner_w.saturating_sub(label.len() + 2);
    let line_top = format!(
        "{}┌ {}{}{}{}{}┐{}",
        ORANGE_DIM,
        ORANGE_PRIMARY,
        label,
        ORANGE_DIM,
        " ─".repeat(border_after / 2),
        if border_after % 2 == 1 { "─" } else { "" },
        RESET,
    );

    // Line 2: Tape reels + icon + time
    let tape = render_tape(progress, inner_w.saturating_sub(14));
    let line_tape = format!(
        "{}│{} {} {}{} {}{}/{}{}{}│{}",
        ORANGE_DIM,
        RESET,
        tape,
        ORANGE_BRIGHT,
        icon,
        RESET,
        format!("{}{}{}", ORANGE_DIM, elapsed, RESET),
        format!("{}{}{}", ORANGE_BG, duration, RESET),
        " ".repeat(0),
        ORANGE_DIM,
        RESET,
    );

    // Line 3: Title + spectrum + modes
    let spectrum = render_spectrum(&state.spectrum, 12);
    let line_info = format!(
        "{}│{} {}{}{} {} {}{}│{}",
        ORANGE_DIM, RESET, ORANGE_BRIGHT, title, RESET, spectrum, modes, ORANGE_DIM, RESET,
    );

    // Line 4: Clickable control buttons with OSC 8 hyperlinks
    let osc_link = |url: &str, icon: &str, color: &str| -> String {
        format!("{}\x1b]8;;{}\x07{}\x1b]8;;\x07{}", color, url, icon, RESET)
    };
    let toggle_icon = if state.playing {
        "\u{23f8}"
    } else {
        "\u{25b6}"
    };
    let repeat_color = if state.repeat {
        ORANGE_BRIGHT
    } else {
        ORANGE_DIM
    };
    let shuffle_color = if state.shuffle {
        ORANGE_BRIGHT
    } else {
        ORANGE_DIM
    };
    let btn_prev = osc_link("pixelbeat://prev", "\u{23ee}", ORANGE_PRIMARY);
    let btn_toggle = osc_link("pixelbeat://toggle", toggle_icon, ORANGE_BRIGHT);
    let btn_next = osc_link("pixelbeat://next", "\u{23ed}", ORANGE_PRIMARY);
    let btn_repeat = osc_link("pixelbeat://repeat", "\u{1f501}", repeat_color);
    let btn_shuffle = osc_link("pixelbeat://shuffle", "\u{1f500}", shuffle_color);
    let line_buttons = format!(
        "{}│{}  {}   {}   {}       {}  {}       {}│{}",
        ORANGE_DIM,
        RESET,
        btn_prev,
        btn_toggle,
        btn_next,
        btn_repeat,
        btn_shuffle,
        ORANGE_DIM,
        RESET,
    );

    // Line 5: Bottom border
    let line_bottom = format!("{}└{}┘{}", ORANGE_DIM, "─".repeat(inner_w + 2), RESET,);

    format!(
        "{}\n{}\n{}\n{}\n{}",
        line_top, line_tape, line_info, line_buttons, line_bottom
    )
}

/// Parse and render a format string with player state data
pub fn render_format(fmt: &str, state: &PlayerState) -> String {
    let mut result = String::new();
    let mut chars = fmt.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut token = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                token.push(c);
            }
            result.push_str(&expand_token(&token, state));
        } else {
            result.push(ch);
        }
    }

    result
}

fn expand_token(token: &str, state: &PlayerState) -> String {
    let parts: Vec<&str> = token.split(':').collect();

    match parts[0] {
        "title" => {
            let title = &state.title;
            if parts.len() > 1 {
                let max_str = parts[1].trim_start_matches('.');
                if let Ok(max) = max_str.parse::<usize>() {
                    format!("{}{}{}", ORANGE_BRIGHT, truncate_chars(title, max), RESET)
                } else {
                    format!("{}{}{}", ORANGE_BRIGHT, title, RESET)
                }
            } else {
                format!("{}{}{}", ORANGE_BRIGHT, title, RESET)
            }
        }
        "icon" => format!("{}{}{}", ORANGE_PRIMARY, render_icon(state.playing), RESET),
        "bar" => {
            let width = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
            let progress = if state.duration > 0.0 {
                state.position / state.duration
            } else {
                0.0
            };
            render_bar(progress, width)
        }
        "tape" => {
            let width = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
            let progress = if state.duration > 0.0 {
                state.position / state.duration
            } else {
                0.0
            };
            render_tape(progress, width)
        }
        "cassette" => {
            let width = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(48);
            render_cassette(state, width)
        }
        "elapsed" => format!("{}{}{}", ORANGE_DIM, format_time(state.position), RESET),
        "duration" => format!("{}{}{}", ORANGE_BG, format_time(state.duration), RESET),
        "spectrum" => {
            let width = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(16);
            render_spectrum(&state.spectrum, width)
        }
        "vol" => {
            if parts.len() > 1 && parts[1] == "bar" {
                let width = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
                render_bar(state.volume as f64, width)
            } else {
                format!("{}{}%{}", ORANGE_DIM, (state.volume * 100.0) as u8, RESET)
            }
        }
        "index" => format!("{}{}{}", ORANGE_DIM, state.track_index + 1, RESET),
        "count" => format!("{}{}{}", ORANGE_DIM, state.track_count, RESET),
        "shuffle" => {
            if state.shuffle {
                format!("{}🔀{}", ORANGE_BRIGHT, RESET)
            } else {
                format!("{}🔀{}", ORANGE_BG, RESET)
            }
        }
        "repeat" => {
            if state.repeat {
                format!("{}🔁{}", ORANGE_BRIGHT, RESET)
            } else {
                format!("{}🔁{}", ORANGE_BG, RESET)
            }
        }
        "modes" => {
            let repeat_color = if state.repeat {
                ORANGE_BRIGHT
            } else {
                ORANGE_BG
            };
            let shuffle_color = if state.shuffle {
                ORANGE_BRIGHT
            } else {
                ORANGE_BG
            };
            format!("{}🔁{} {}🔀{}", repeat_color, RESET, shuffle_color, RESET)
        }
        "controls" => {
            format!(
                "{}⏮{} prev {}⏯{} toggle {}⏭{} next {}🔁{} loop",
                ORANGE_DIM, RESET, ORANGE_DIM, RESET, ORANGE_DIM, RESET, ORANGE_DIM, RESET,
            )
        }
        "buttons" => {
            let link = |url: &str, icon: &str, color: &str| -> String {
                format!("{}\x1b]8;;{}\x07{}\x1b]8;;\x07{}", color, url, icon, RESET)
            };
            let toggle_icon = if state.playing { "⏸" } else { "▶" };
            let repeat_color = if state.repeat {
                ORANGE_BRIGHT
            } else {
                ORANGE_DIM
            };
            let shuffle_color = if state.shuffle {
                ORANGE_BRIGHT
            } else {
                ORANGE_DIM
            };
            format!(
                "{}  {}  {}  {}  {}",
                link("pixelbeat://prev", "⏮", ORANGE_PRIMARY),
                link("pixelbeat://toggle", toggle_icon, ORANGE_BRIGHT),
                link("pixelbeat://next", "⏭", ORANGE_PRIMARY),
                link("pixelbeat://repeat", "🔁", repeat_color),
                link("pixelbeat://shuffle", "🔀", shuffle_color),
            )
        }
        _ => format!("{{{}}}", token),
    }
}

/// Default format for Claude Code status line
pub fn default_statusline_format() -> &'static str {
    "{cassette:50}"
}

/// Compact format
#[allow(dead_code)]
pub fn compact_format() -> &'static str {
    "{icon} {title:.20} {bar:8} {elapsed}"
}
