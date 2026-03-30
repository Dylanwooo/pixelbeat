use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io::stdout;
use std::time::Duration;

use super::theme::Theme;
use crate::daemon::ipc::{self, Command};
use crate::daemon::player::PlayerState;

const SPECTRUM_BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::anthropic();
    let result = run_loop(&mut terminal, &theme);

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &Theme,
) -> Result<()> {
    let mut daemon_connected;

    loop {
        // Get current state from daemon
        let state = match ipc::send_command(&Command::Status) {
            Ok(resp) => {
                daemon_connected = true;
                resp.state.unwrap_or_default()
            }
            Err(_) => {
                daemon_connected = false;
                PlayerState::default()
            }
        };

        terminal.draw(|frame| {
            if daemon_connected {
                render(frame, &state, theme);
            } else {
                render_disconnected(frame, theme);
            }
        })?;

        // Handle input with timeout for animation
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char(' ') => {
                            ipc::send_command(&Command::Toggle).ok();
                        }
                        KeyCode::Char('n') | KeyCode::Right => {
                            ipc::send_command(&Command::Next).ok();
                        }
                        KeyCode::Char('p') | KeyCode::Left => {
                            ipc::send_command(&Command::Prev).ok();
                        }
                        KeyCode::Char('+') | KeyCode::Up => {
                            if let Ok(resp) = ipc::send_command(&Command::Status) {
                                if let Some(s) = resp.state {
                                    let new_vol = (s.volume + 0.05).min(1.0);
                                    ipc::send_command(&Command::Volume { level: new_vol }).ok();
                                }
                            }
                        }
                        KeyCode::Char('-') | KeyCode::Down => {
                            if let Ok(resp) = ipc::send_command(&Command::Status) {
                                if let Some(s) = resp.state {
                                    let new_vol = (s.volume - 0.05).max(0.0);
                                    ipc::send_command(&Command::Volume { level: new_vol }).ok();
                                }
                            }
                        }
                        KeyCode::Char('s') => {
                            ipc::send_command(&Command::Shuffle {
                                enabled: !state.shuffle,
                            })
                            .ok();
                        }
                        KeyCode::Char('r') => {
                            ipc::send_command(&Command::Repeat {
                                enabled: !state.repeat,
                            })
                            .ok();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn render(frame: &mut Frame, state: &PlayerState, theme: &Theme) {
    let area = frame.area();

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(theme.bg));
    frame.render_widget(bg_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Now Playing
            Constraint::Length(3), // Progress
            Constraint::Min(5),    // Spectrum
            Constraint::Length(3), // Controls help
        ])
        .split(area);

    render_header(frame, chunks[0], theme);
    render_now_playing(frame, chunks[1], state, theme);
    render_progress(frame, chunks[2], state, theme);
    render_spectrum(frame, chunks[3], state, theme);
    render_controls(frame, chunks[4], state, theme);
}

fn render_header(frame: &mut Frame, area: Rect, theme: &Theme) {
    let title = vec![
        Span::styled("pixel", Style::default().fg(theme.bright).bold()),
        Span::styled("beat", Style::default().fg(theme.primary)),
        Span::styled(" ♪", Style::default().fg(theme.dim)),
    ];

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.dim))
        .style(Style::default().bg(theme.bg));

    let header = Paragraph::new(Line::from(title)).block(block).centered();

    frame.render_widget(header, area);
}

fn render_disconnected(frame: &mut Frame, theme: &Theme) {
    let area = frame.area();
    let bg_block = Block::default().style(Style::default().bg(theme.bg));
    frame.render_widget(bg_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, chunks[0], theme);

    let msg = vec![
        Line::from(Span::styled(
            "Connecting to daemon...",
            Style::default().fg(theme.text_dim),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "If this persists, start manually: px daemon",
            Style::default().fg(theme.dim),
        )),
    ];
    let para = Paragraph::new(msg)
        .style(Style::default().bg(theme.bg))
        .centered();
    frame.render_widget(para, chunks[1]);
}

fn render_now_playing(frame: &mut Frame, area: Rect, state: &PlayerState, theme: &Theme) {
    // Show error if present
    if let Some(ref err) = state.last_error {
        let line = vec![
            Span::styled(" ⚠ ", Style::default().fg(ratatui::style::Color::Red)),
            Span::styled(
                err.as_str(),
                Style::default().fg(ratatui::style::Color::Red),
            ),
        ];
        let para = Paragraph::new(Line::from(line)).style(Style::default().bg(theme.bg));
        frame.render_widget(para, area);
        return;
    }

    let title = if state.title.is_empty() {
        "No track loaded".to_string()
    } else {
        state.title.clone()
    };

    let track_info = format!("[{}/{}]", state.track_index + 1, state.track_count);

    let line = vec![
        Span::styled(
            if state.playing { " ▶ " } else { " ⏸ " },
            Style::default().fg(theme.bright),
        ),
        Span::styled(&title, Style::default().fg(theme.text).bold()),
        Span::styled("  ", Style::default()),
        Span::styled(track_info, Style::default().fg(theme.text_dim)),
    ];

    let para = Paragraph::new(Line::from(line)).style(Style::default().bg(theme.bg));

    frame.render_widget(para, area);
}

fn render_progress(frame: &mut Frame, area: Rect, state: &PlayerState, theme: &Theme) {
    let progress = if state.duration > 0.0 {
        (state.position / state.duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let bar_width = area.width.saturating_sub(16) as usize;
    let filled = (progress * bar_width as f64).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    let elapsed = format_time(state.position);
    let duration = format_time(state.duration);

    let line = vec![
        Span::styled(
            format!(" {} ", elapsed),
            Style::default().fg(theme.text_dim),
        ),
        Span::styled("█".repeat(filled), Style::default().fg(theme.bright)),
        Span::styled("░".repeat(empty), Style::default().fg(theme.dim)),
        Span::styled(
            format!(" {} ", duration),
            Style::default().fg(theme.text_dim),
        ),
    ];

    let para = Paragraph::new(Line::from(line)).style(Style::default().bg(theme.bg));

    frame.render_widget(para, area);
}

fn render_spectrum(frame: &mut Frame, area: Rect, state: &PlayerState, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(theme.surface))
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let bar_count = inner.width as usize;
    let height = inner.height as usize;

    // Build the spectrum display from bottom to top
    let mut lines: Vec<Line> = Vec::new();

    for row in 0..height {
        let y_threshold = 1.0 - (row as f32 / height as f32);
        let mut spans = Vec::new();

        for col in 0..bar_count {
            // Map column to spectrum data with interpolation
            let spectrum_idx =
                (col as f32 / bar_count as f32 * state.spectrum.len() as f32) as usize;
            let val = state.spectrum.get(spectrum_idx).copied().unwrap_or(0.0);

            if val >= y_threshold {
                let color = theme.spectrum_color(val);
                let bar_char = if val - y_threshold < (1.0 / height as f32) / 2.0 {
                    // Top of this bar - use partial block
                    let frac = ((val - y_threshold) * height as f32 * 8.0) as usize;
                    SPECTRUM_BARS[frac.min(SPECTRUM_BARS.len() - 1)]
                } else {
                    '█'
                };
                spans.push(Span::styled(
                    bar_char.to_string(),
                    Style::default().fg(color).bg(theme.bg),
                ));
            } else {
                spans.push(Span::styled(" ", Style::default().bg(theme.bg)));
            }
        }

        lines.push(Line::from(spans));
    }

    let spectrum_widget = Paragraph::new(lines);
    frame.render_widget(spectrum_widget, inner);
}

fn render_controls(frame: &mut Frame, area: Rect, state: &PlayerState, theme: &Theme) {
    let vol_pct = (state.volume * 100.0) as u8;

    let shuffle_style = if state.shuffle {
        Style::default().fg(theme.bright)
    } else {
        Style::default().fg(theme.text_dim)
    };

    let repeat_style = if state.repeat {
        Style::default().fg(theme.bright)
    } else {
        Style::default().fg(theme.text_dim)
    };

    let controls = vec![
        Span::styled(" [space]", Style::default().fg(theme.dim)),
        Span::styled(" play/pause ", Style::default().fg(theme.text_dim)),
        Span::styled("[←/→]", Style::default().fg(theme.dim)),
        Span::styled(" prev/next ", Style::default().fg(theme.text_dim)),
        Span::styled("[↑/↓]", Style::default().fg(theme.dim)),
        Span::styled(
            format!(" vol:{}% ", vol_pct),
            Style::default().fg(theme.text_dim),
        ),
        Span::styled("[s]", Style::default().fg(theme.dim)),
        Span::styled(" shuffle ", shuffle_style),
        Span::styled("[r]", Style::default().fg(theme.dim)),
        Span::styled(" repeat ", repeat_style),
        Span::styled("[q]", Style::default().fg(theme.dim)),
        Span::styled(" quit", Style::default().fg(theme.text_dim)),
    ];

    let para = Paragraph::new(Line::from(controls)).style(Style::default().bg(theme.surface));

    frame.render_widget(para, area);
}

fn format_time(secs: f64) -> String {
    let mins = secs as u64 / 60;
    let secs = secs as u64 % 60;
    format!("{:02}:{:02}", mins, secs)
}
