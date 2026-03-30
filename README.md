# pixelbeat

> A pixel-art terminal music player daemon built for long coding sessions.

[English](README.md) | [中文](README_CN.md)

[![Rust](https://img.shields.io/badge/Rust-2021_Edition-E3893E?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![CI](https://github.com/Dylanwooo/pixelbeat/actions/workflows/ci.yml/badge.svg)](https://github.com/Dylanwooo/pixelbeat/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-E3893E.svg)](LICENSE)
[![macOS](https://img.shields.io/badge/macOS-supported-E3893E?logo=apple&logoColor=white)]()

![pixelbeat demo](assets/demo.gif)

```
┌ PIXELBEAT ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ──┐
│ ◉ ──────────────●━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ○  ▶ 2:47/4:12   │
│ 🎵 Sleepy Fish - A Rainy Night in Kyoto  ▅▂█▄▇▁▃▆▂▅▃▇              │
│  ⏮   ⏸   ⏭                                                         │
└────────────────────────────────────────────────────────────────────────┘
```

pixelbeat runs as a background daemon and exposes a tiny CLI (`px`) for playback control. It streams YouTube playlists via mpv, plays local audio files through rodio, and ships with built-in chillhop and lofi radio stations. The animated cassette tape UI and spectrum visualizer plug directly into Claude Code's status line, tmux, or starship.

## Features

- **Daemon architecture** -- Start once, control from anywhere via Unix socket IPC
- **YouTube streaming** -- Play any YouTube video or playlist instantly through mpv (no download step)
- **Local file playback** -- MP3, FLAC, WAV, OGG, M4A, AAC, Opus, AIFF via rodio/symphonia
- **Built-in radio** -- Chillhop and lofi stations with curated track lists, ready out of the box
- **Cassette tape UI** -- Animated reel-to-reel visualization with playhead tracking
- **Spectrum visualizer** -- 32-bar beat-synced spectrum analyzer with Anthropic orange gradient
- **TUI mode** -- Full-screen terminal interface built with ratatui
- **Status bar integration** -- Plug into Claude Code, tmux, or starship with one command
- **Format template engine** -- Compose your own status line with tokens like `{tape:30}`, `{spectrum:16}`, `{cassette:70}`
- **Shuffle and repeat** -- Persistent mode toggles across sessions via config file
- **Clickable controls** -- OSC 8 hyperlink buttons (prev/toggle/next/repeat/shuffle) in supported terminals

## Quick Start

**Prerequisites**: [Rust](https://rustup.rs/) toolchain, [mpv](https://mpv.io/) and [yt-dlp](https://github.com/yt-dlp/yt-dlp) (for YouTube).

```bash
# Install dependencies (macOS)
brew install mpv yt-dlp

# Clone and build
git clone https://github.com/Dylanwooo/pixelbeat.git
cd pixelbeat
cargo build --release

# Add to PATH
cp target/release/px ~/.local/bin/  # or anywhere on your PATH

# Play a YouTube playlist (daemon starts automatically)
px yt "https://www.youtube.com/watch?v=jfKfPfyJRdk"

# Or start with built-in radio
px radio lofi
```

You should hear music within a few seconds. Run `px tui` to open the full-screen player, or `px status` to see the cassette deck in your terminal.

## Installation

### From source (recommended)

```bash
git clone https://github.com/Dylanwooo/pixelbeat.git
cd pixelbeat
cargo install --path .
```

This installs the `px` binary to `~/.cargo/bin/`. Make sure that directory is in your `PATH`.

### Dependencies

| Dependency | Required | Purpose | Install |
|-----------|----------|---------|---------|
| **Rust** (2021 edition) | Yes | Build toolchain | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **mpv** | For YouTube | Audio streaming backend | macOS: `brew install mpv` / Linux: `apt install mpv` |
| **yt-dlp** | For YouTube | Playlist resolution and audio extraction | macOS: `brew install yt-dlp` / Linux: `pip install yt-dlp` |

Local file playback and built-in radio work without mpv or yt-dlp. The daemon warns at startup if these optional dependencies are missing.

## Usage

### Start the daemon

The daemon starts automatically when you run any playback command (e.g., `px play`, `px radio`). You can also start it explicitly:

```bash
# Start the daemon (blocks the terminal)
px daemon

# Start and immediately play a directory
px daemon --play ~/Music

# Start in the background
px daemon &
```

### Play local files

```bash
# Play a single file
px play ~/Music/song.mp3

# Play all supported files in a directory
px play ~/Music/chillhop/

# Resume playback (if paused)
px play
```

Supported formats: MP3, FLAC, WAV, OGG, M4A, AAC, Opus, AIFF.

### Play YouTube

Stream any YouTube video or playlist. Audio plays instantly via mpv -- nothing is downloaded to disk.

```bash
# Play a single video
px yt "https://www.youtube.com/watch?v=jfKfPfyJRdk"

# Play a playlist
px yt "https://www.youtube.com/playlist?list=PLOzDu-MXXLliO9fBNZOQTBDddoA3FzZUo"
```

### Built-in radio

```bash
# List available stations
px radio

# Play a station
px radio chillhop
px radio lofi
```

Stations: **chillhop** (30 curated tracks from Chillhop Music), **lofi** (15 tracks from the Lofi Girl archive).

### Playback controls

```bash
px toggle          # Play/pause toggle
px pause           # Pause
px next            # Next track
px prev            # Previous track
px stop            # Stop playback
px vol 0.5         # Set volume (0.0 to 1.0)
px shuffle         # Toggle shuffle mode
px repeat          # Toggle repeat mode
```

### TUI mode

Open a full-screen terminal UI with spectrum visualizer, progress bar, and keyboard controls.

```bash
px tui
```

**TUI keybindings**:

| Key | Action |
|-----|--------|
| `Space` | Play / pause |
| `n` or `Right` | Next track |
| `p` or `Left` | Previous track |
| `+` or `Up` | Volume up |
| `-` or `Down` | Volume down |
| `s` | Toggle shuffle |
| `r` | Toggle repeat |
| `q` or `Esc` | Quit TUI |

### Status bar output

Query the current player state, formatted however you want.

```bash
# Default: renders the full cassette deck widget
px status

# Custom format string
px status --format "{icon} {title:.25} {bar:12} {elapsed}/{duration}"

# Spectrum only
px status --format "{spectrum:32}"

# Cassette tape with custom width
px status --format "{tape:40}"
```

### Stop the daemon

```bash
px quit
```

## Status Bar Integration

### Claude Code

![pixelbeat in Claude Code](assets/demo.png)

Run the setup wizard:

```bash
px setup claude-code
```

Or manually add this to `~/.claude/statusline.sh`:

```bash
# pixelbeat music player status
if command -v px &>/dev/null; then
    PX_STATUS=$(px status --format "♪ {title:.25} {icon} {bar:12} {elapsed}/{duration}" 2>/dev/null)
    if [ -n "$PX_STATUS" ]; then
        echo "$PX_STATUS"
        echo "$(px status --format "  {spectrum:32}" 2>/dev/null)"
    fi
fi
```

A ready-to-source script is also available at [`integrations/claude-code.sh`](integrations/claude-code.sh).

### tmux

Run `px setup tmux`, or add to `~/.tmux.conf`:

```tmux
set -g status-right '#(px status --format "{icon} {title:.20} {bar:8} {elapsed}" 2>/dev/null)'
set -g status-interval 1
```

Then reload: `tmux source-file ~/.tmux.conf`

### Starship

Run `px setup starship`, or add to `~/.config/starship.toml`:

```toml
[custom.music]
command = "px status --format '{icon} {title:.15} {elapsed}'"
when = "px status"
format = "[$output]($style) "
style = "bold #E3893E"
```

## Format Template Tokens

Use these tokens in `px status --format "..."` to build custom status lines.

| Token | Description | Example Output |
|-------|-------------|----------------|
| `{title}` | Track title (full) | `Sleepy Fish - A Rainy Night in Kyoto` |
| `{title:.N}` | Track title, truncated to N chars | `Sleepy Fish - A Rain…` |
| `{icon}` | Play/pause icon | `▶` or `⏸` |
| `{bar:N}` | Progress bar, N chars wide | `████░░░░░░` |
| `{tape:N}` | Cassette tape reel visualization, N chars wide | `◉ ──────●━━━━━━━ ◎` |
| `{spectrum:N}` | Animated spectrum bars, N bars wide | `▅▂█▄▇▁▃▆▂▅▃▇▁▄▆▂` |
| `{cassette:N}` | Full cassette deck widget (multi-line), N chars wide | *(see above)* |
| `{elapsed}` | Elapsed time | `2:47` |
| `{duration}` | Total duration | `4:12` |
| `{vol}` | Volume percentage | `80%` |
| `{vol:bar:N}` | Volume as a bar, N chars wide | `████░` |
| `{index}` | Current track number (1-based) | `3` |
| `{count}` | Total track count | `12` |
| `{shuffle}` | Shuffle indicator (bright when on) | `🔀` |
| `{repeat}` | Repeat indicator (bright when on) | `🔁` |
| `{modes}` | Combined repeat + shuffle indicators | `🔁 🔀` |
| `{controls}` | Text-based control legend | `⏮ prev ⏯ toggle ⏭ next 🔁 loop` |
| `{buttons}` | Clickable OSC 8 hyperlink buttons | `⏮  ⏸  ⏭  🔁  🔀` |

All tokens render with Anthropic orange ANSI color codes (`#E3893E` primary, with bright/dim/background variants).

## Configuration

pixelbeat reads its config from `~/.config/pixelbeat/config.toml`. Every field is optional -- sensible defaults are used when omitted.

```toml
# Default source on daemon startup: "local", "chillhop", "lofi", "youtube"
source = "local"

# YouTube playlist URL (used when source = "youtube")
youtube_url = "https://www.youtube.com/watch?v=jfKfPfyJRdk"

# Local music directory (used when source = "local")
music_dir = "~/Music/pixelbeat"

# Default volume (0.0 - 1.0)
volume = 0.8

# Auto-repeat / loop
repeat = false

# Shuffle mode
shuffle = false
```

### Config reference

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `source` | `string` | *(none)* | Auto-play source on startup. One of `"local"`, `"chillhop"`, `"lofi"`, `"youtube"`. If omitted, the daemon loads `music_dir` when it exists. |
| `youtube_url` | `string` | *(none)* | YouTube video or playlist URL. Only used when `source = "youtube"`. |
| `music_dir` | `string` | `"~/Music/pixelbeat"` | Directory to scan for local audio files. Tilde expansion is supported. |
| `volume` | `float` | `0.8` | Initial volume level, from `0.0` (mute) to `1.0` (max). |
| `repeat` | `bool` | `false` | Loop the playlist when it reaches the end. Radio mode forces this on. |
| `shuffle` | `bool` | `false` | Randomize track order. Re-shuffles on each loop when repeat is also enabled. |

## Architecture

```
┌─────────┐    Unix socket IPC    ┌────────────────────┐
│  px CLI  │ ──────────────────── │  px daemon         │
│          │   JSON commands      │                    │
│  px tui  │ ◄────────────────── │  ┌──────────────┐  │
└─────────┘   JSON responses      │  │ Player       │  │
                                  │  │  - rodio     │  │  Local files
                                  │  │  - mpv IPC   │──│──────────────
                                  │  └──────────────┘  │
                                  │  ┌──────────────┐  │  YouTube
                                  │  │ mpv process  │──│──────────────
                                  │  └──────────────┘  │  (yt-dlp)
                                  │  ┌──────────────┐  │
                                  │  │ Spectrum      │  │  Radio streams
                                  │  │ Analyzer     │──│──────────────
                                  │  └──────────────┘  │  (HTTP)
                                  └────────────────────┘
```

**Daemon** (`px daemon`) -- Long-running process that owns the audio output. Listens on a Unix socket at `$XDG_RUNTIME_DIR/pixelbeat.sock` (falls back to `/tmp/pixelbeat.sock`). Ticks at 50ms intervals to update playback position, detect track endings, and generate spectrum data.

**CLI** (`px <command>`) -- Thin client that serializes commands as JSON, sends them over the Unix socket, and prints the response. Each invocation connects, sends one command, reads one response, and exits.

**TUI** (`px tui`) -- Full-screen ratatui interface that polls the daemon for state every 100ms and renders a live spectrum visualizer. All input is translated to the same IPC commands the CLI uses.

**Player engine** -- Local files are decoded by rodio (via symphonia). YouTube audio is streamed through an mpv subprocess controlled via mpv's JSON IPC protocol. The player transparently switches between rodio and mpv depending on the source.

**Spectrum analyzer** -- Generates 32-bar beat-synced animation data at ~20 FPS. Uses deterministic pseudo-random waves with contrast boosting and spike injection for a punchy, reactive look. An FFT-based real PCM analysis path is implemented for future use.

## Troubleshooting

### No sound after starting

- Check your system audio output: `px vol 0.8` to set volume.
- Ensure no other process has exclusive access to the audio device.
- Try restarting the daemon: `px quit && px daemon`.

### Cannot connect to daemon

- The daemon auto-starts with most commands. If it fails, start manually: `px daemon`.
- Check if a stale socket exists: `rm ${XDG_RUNTIME_DIR:-/tmp}/pixelbeat.sock` and retry.
- Check if the daemon is already running: `pgrep -f "px daemon"`.

### YouTube not working

- Ensure both **mpv** and **yt-dlp** are installed. The daemon warns at startup if they are missing.
- Test directly: `yt-dlp --flat-playlist --dump-json <URL>` to verify yt-dlp works.
- For age-restricted or YouTube Music content, set `youtube_cookies_browser` in your config:
  ```toml
  youtube_cookies_browser = "chrome"  # or "firefox", "brave", etc.
  ```

### Radio station silent or failing

- Radio tracks are streamed over HTTP. Check your network connection.
- Some tracks may be temporarily unavailable; pixelbeat retries up to 5 times per track automatically.
- Check daemon output for errors: run `px daemon` in the foreground (without `&`).

### Config file location

pixelbeat reads from `~/.config/pixelbeat/config.toml`. A commented template is auto-generated on first run. If the file becomes corrupted, delete it and restart the daemon to regenerate defaults.

## Contributing

Contributions are welcome. Here is how to get started:

1. Fork the repository and clone your fork.
2. Create a feature branch: `git checkout -b my-feature`.
3. Make your changes. Run `cargo fmt` and `cargo clippy` before committing.
4. Test locally: start the daemon with `cargo run -- daemon`, then exercise your changes with the CLI.
5. Open a pull request against `main`.

If you find a bug or have a feature idea, please open an issue first so we can discuss it.

## License

MIT -- see [LICENSE](LICENSE) for details.

Copyright (c) 2026 Dylan Woo
