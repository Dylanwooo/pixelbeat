use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::playlist::Playlist;
use super::radio::{self, RadioStation};
use super::spectrum::SpectrumAnalyzer;
use super::youtube;

/// Core player state shared across threads
#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct PlayerState {
    pub playing: bool,
    pub title: String,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub spectrum: Vec<f32>,
    pub track_index: usize,
    pub track_count: usize,
    pub shuffle: bool,
    pub repeat: bool,
    /// Last error message, displayed in TUI and cleared after timeout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            playing: false,
            title: String::new(),
            position: 0.0,
            duration: 0.0,
            volume: 0.8,
            spectrum: vec![0.0; 16],
            track_index: 0,
            track_count: 0,
            shuffle: false,
            repeat: false,
            last_error: None,
        }
    }
}

pub struct Player {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Arc<Mutex<Option<Sink>>>,
    pub playlist: Arc<Mutex<Playlist>>,
    pub state: Arc<Mutex<PlayerState>>,
    pub spectrum: Arc<Mutex<SpectrumAnalyzer>>,
    track_start: Arc<Mutex<Option<Instant>>>,
    pause_elapsed: Arc<Mutex<Duration>>,
    is_paused: Arc<AtomicBool>,
    radio_station: Arc<Mutex<Option<RadioStation>>>,
    yt_track_index: Arc<Mutex<usize>>,
    /// mpv subprocess for YouTube streaming (no download needed)
    mpv: Arc<Mutex<youtube::MpvPlayer>>,
    /// Browser for YouTube cookie extraction (from config, None = disabled)
    pub cookies_browser: Option<String>,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().context("Failed to open audio output")?;

        Ok(Self {
            _stream: stream,
            stream_handle,
            sink: Arc::new(Mutex::new(None)),
            playlist: Arc::new(Mutex::new(Playlist::new())),
            state: Arc::new(Mutex::new(PlayerState::default())),
            spectrum: Arc::new(Mutex::new(SpectrumAnalyzer::new())),
            track_start: Arc::new(Mutex::new(None)),
            pause_elapsed: Arc::new(Mutex::new(Duration::ZERO)),
            is_paused: Arc::new(AtomicBool::new(false)),
            radio_station: Arc::new(Mutex::new(None)),
            yt_track_index: Arc::new(Mutex::new(0)),
            mpv: Arc::new(Mutex::new(youtube::MpvPlayer::new())),
            cookies_browser: None,
        })
    }

    /// Load files from a path (file or directory)
    pub fn load_path(&self, path: &PathBuf) -> Result<()> {
        let mut playlist = self.playlist.lock().unwrap();
        if path.is_dir() {
            playlist.load_directory(path)?;
        } else {
            playlist.add_file(path.clone())?;
        }
        let mut state = self.state.lock().unwrap();
        state.track_count = playlist.tracks.len();
        Ok(())
    }

    /// Play the current track
    pub fn play(&self) -> Result<()> {
        // If paused, just resume
        if self.is_paused.load(Ordering::Relaxed) {
            if let Some(ref sink) = *self.sink.lock().unwrap() {
                sink.play();
                self.is_paused.store(false, Ordering::Relaxed);
                let mut state = self.state.lock().unwrap();
                state.playing = true;
                // Resume timing
                *self.track_start.lock().unwrap() = Some(Instant::now());
                return Ok(());
            }
        }

        let track = {
            let playlist = self.playlist.lock().unwrap();
            playlist.current_track().cloned()
        };

        if let Some(track_path) = track {
            self.play_file(&track_path)?;
        }
        Ok(())
    }

    fn play_file(&self, path: &PathBuf) -> Result<()> {
        // Stop current playback
        if let Some(ref sink) = *self.sink.lock().unwrap() {
            sink.stop();
        }

        let file = File::open(path).context("Failed to open audio file")?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).context("Failed to decode audio file")?;

        // Get duration estimate from file metadata
        let duration = Self::estimate_duration(path);

        let sink = Sink::try_new(&self.stream_handle).context("Failed to create audio sink")?;

        let volume = self.state.lock().unwrap().volume;
        sink.set_volume(volume);
        sink.append(source);

        *self.sink.lock().unwrap() = Some(sink);
        *self.track_start.lock().unwrap() = Some(Instant::now());
        *self.pause_elapsed.lock().unwrap() = Duration::ZERO;
        self.is_paused.store(false, Ordering::Relaxed);

        // Update state
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let playlist = self.playlist.lock().unwrap();
        let mut state = self.state.lock().unwrap();
        state.playing = true;
        state.title = title;
        state.duration = duration;
        state.position = 0.0;
        state.track_index = playlist.index;
        state.track_count = playlist.tracks.len();

        Ok(())
    }

    fn estimate_duration(path: &PathBuf) -> f64 {
        // Try to get duration from file size and format
        // This is a rough estimate; rodio doesn't expose duration directly for all formats
        if let Ok(metadata) = std::fs::metadata(path) {
            let size = metadata.len() as f64;
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            match ext.as_str() {
                "mp3" => size / 16000.0,  // ~128kbps average
                "flac" => size / 88200.0, // ~706kbps average
                "wav" => size / 176400.0, // 44.1kHz 16-bit stereo
                "ogg" => size / 16000.0,
                _ => size / 16000.0,
            }
        } else {
            0.0
        }
    }

    pub fn pause(&self) {
        // If mpv is active, control mpv
        if self.is_mpv_active() {
            let mut mpv = self.mpv.lock().unwrap();
            mpv.pause().ok();
            let mut state = self.state.lock().unwrap();
            state.playing = false;
            return;
        }
        if let Some(ref sink) = *self.sink.lock().unwrap() {
            if !self.is_paused.load(Ordering::Relaxed) {
                if let Some(start) = *self.track_start.lock().unwrap() {
                    let mut elapsed = self.pause_elapsed.lock().unwrap();
                    *elapsed += start.elapsed();
                }
                sink.pause();
                self.is_paused.store(true, Ordering::Relaxed);
                let mut state = self.state.lock().unwrap();
                state.playing = false;
            }
        }
    }

    pub fn toggle(&self) -> Result<()> {
        if self.is_mpv_active() {
            let mut mpv = self.mpv.lock().unwrap();
            mpv.toggle_pause()?;
            let paused = mpv.is_paused();
            let mut state = self.state.lock().unwrap();
            state.playing = !paused;
            return Ok(());
        }
        if self.is_paused.load(Ordering::Relaxed) {
            self.play()
        } else if self.state.lock().unwrap().playing {
            self.pause();
            Ok(())
        } else {
            self.play()
        }
    }

    /// Check if mpv is the active player
    fn is_mpv_active(&self) -> bool {
        let mut mpv = self.mpv.lock().unwrap();
        mpv.is_running()
    }

    pub fn stop(&self) {
        // Stop mpv if running
        let mut mpv = self.mpv.lock().unwrap();
        mpv.stop();
        drop(mpv);

        if let Some(ref sink) = *self.sink.lock().unwrap() {
            sink.stop();
        }
        *self.track_start.lock().unwrap() = None;
        *self.pause_elapsed.lock().unwrap() = Duration::ZERO;
        self.is_paused.store(false, Ordering::Relaxed);
        let mut state = self.state.lock().unwrap();
        state.playing = false;
        state.position = 0.0;
    }

    pub fn next(&self) -> Result<()> {
        if self.is_radio_mode() {
            return self.play_next_radio_track();
        }
        {
            let mut playlist = self.playlist.lock().unwrap();
            playlist.next();
        }
        let track = {
            let playlist = self.playlist.lock().unwrap();
            playlist.current_track().cloned()
        };
        if let Some(track_path) = track {
            self.play_file(&track_path)?;
        }
        Ok(())
    }

    pub fn prev(&self) -> Result<()> {
        if self.is_radio_mode() {
            // In radio mode, prev just plays another random track
            return self.play_next_radio_track();
        }
        {
            let mut playlist = self.playlist.lock().unwrap();
            playlist.prev();
        }
        let track = {
            let playlist = self.playlist.lock().unwrap();
            playlist.current_track().cloned()
        };
        if let Some(track_path) = track {
            self.play_file(&track_path)?;
        }
        Ok(())
    }

    pub fn set_volume(&self, vol: f32) {
        let vol = vol.clamp(0.0, 1.0);
        if self.is_mpv_active() {
            let mut mpv = self.mpv.lock().unwrap();
            mpv.set_volume(vol).ok();
        }
        if let Some(ref sink) = *self.sink.lock().unwrap() {
            sink.set_volume(vol);
        }
        let mut state = self.state.lock().unwrap();
        state.volume = vol;
    }

    pub fn set_shuffle(&self, shuffle: bool) {
        let mut playlist = self.playlist.lock().unwrap();
        playlist.shuffle = shuffle;
        let mut state = self.state.lock().unwrap();
        state.shuffle = shuffle;
    }

    pub fn set_repeat(&self, repeat: bool) {
        let mut playlist = self.playlist.lock().unwrap();
        playlist.repeat = repeat;
        let mut state = self.state.lock().unwrap();
        state.repeat = repeat;
    }

    /// Update position and check if track ended
    pub fn tick(&self) -> Result<()> {
        // === mpv mode: sync state from mpv ===
        if self.is_mpv_active() {
            let mut mpv = self.mpv.lock().unwrap();
            let pos = mpv.get_position();
            let dur = mpv.get_duration();

            let mut state = self.state.lock().unwrap();
            if pos > 0.0 {
                state.position = pos;
            }
            if dur > 0.0 {
                state.duration = dur;
            }
            state.playing = !mpv.is_paused();

            // Check if track ended
            drop(state);
            drop(mpv);

            let eof = {
                let mut mpv = self.mpv.lock().unwrap();
                mpv.is_eof() || !mpv.is_running()
            };

            if eof && self.is_radio_mode() {
                self.play_next_radio_track().ok();
            }

            self.update_spectrum();
            return Ok(());
        }

        // === rodio mode ===
        let empty = {
            let sink = self.sink.lock().unwrap();
            sink.as_ref().map(|s| s.empty()).unwrap_or(true)
        };

        if empty && self.state.lock().unwrap().playing {
            if self.is_radio_mode() {
                if let Err(e) = self.play_next_radio_track() {
                    eprintln!("pixelbeat: radio error: {}, retrying...", e);
                    std::thread::sleep(Duration::from_secs(1));
                    self.play_next_radio_track().ok();
                }
            } else {
                let has_next = {
                    let mut playlist = self.playlist.lock().unwrap();
                    playlist.next();
                    playlist.current_track().is_some()
                };
                if has_next {
                    let track = {
                        let playlist = self.playlist.lock().unwrap();
                        playlist.current_track().cloned()
                    };
                    if let Some(track_path) = track {
                        self.play_file(&track_path)?;
                    }
                } else {
                    self.stop();
                }
            }
        }

        // Update position from timer
        if self.state.lock().unwrap().playing {
            if let Some(start) = *self.track_start.lock().unwrap() {
                let pause_elapsed = *self.pause_elapsed.lock().unwrap();
                let position = (start.elapsed() + pause_elapsed).as_secs_f64();
                let mut state = self.state.lock().unwrap();
                state.position = position;
            }
        }

        self.update_spectrum();
        Ok(())
    }

    fn update_spectrum(&self) {
        let mut spectrum = self.spectrum.lock().unwrap();
        let state_playing = self.state.lock().unwrap().playing;
        let data = spectrum.generate(state_playing);
        let mut state = self.state.lock().unwrap();
        state.spectrum = data;
    }

    /// Start radio mode: stream random tracks from an online station
    pub fn play_radio(&self, station_name: &str) -> Result<()> {
        let station = radio::find_station(station_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown station '{}'. Available: {}",
                station_name,
                radio::list_stations().join(", ")
            )
        })?;

        *self.radio_station.lock().unwrap() = Some(station.clone());
        *self.yt_track_index.lock().unwrap() = 0;
        self.set_repeat(true); // Radio always loops

        // Download and play first track
        self.play_next_radio_track()
    }

    /// Start playing a YouTube playlist by URL.
    /// Fetches the playlist metadata, creates a YouTube RadioStation, and starts playback.
    pub fn play_youtube(&self, playlist_url: &str) -> Result<()> {
        let tracks = youtube::fetch_playlist(playlist_url)?;
        if tracks.is_empty() {
            anyhow::bail!("YouTube playlist is empty or could not be fetched");
        }

        let station = RadioStation {
            name: "youtube".to_string(),
            source: radio::RadioSource::YouTube {
                playlist_url: playlist_url.to_string(),
                tracks,
            },
        };

        *self.radio_station.lock().unwrap() = Some(station);
        *self.yt_track_index.lock().unwrap() = 0;
        self.set_repeat(true);

        // Play first track
        self.play_next_radio_track()
    }

    /// Download and play the next track from the current radio station.
    /// Dispatches to the appropriate handler based on the station source type.
    fn play_next_radio_track(&self) -> Result<()> {
        let station = {
            let guard = self.radio_station.lock().unwrap();
            guard.clone()
        };

        let station = match station {
            Some(s) => s,
            None => return Ok(()),
        };

        match &station.source {
            radio::RadioSource::TrackList { .. } => self.play_next_tracklist_track(&station),
            radio::RadioSource::YouTube { .. } => self.play_next_youtube_track(&station),
        }
    }

    /// Play next track from a TrackList radio station
    fn play_next_tracklist_track(&self, station: &RadioStation) -> Result<()> {
        let max_retries = 5;
        let mut last_err = anyhow::anyhow!("No tracks available");

        for attempt in 0..max_retries {
            let url = radio::random_track_url(station)
                .ok_or_else(|| anyhow::anyhow!("No tracks available"))?;

            let title = match &station.source {
                radio::RadioSource::TrackList { tracks, .. } => {
                    let track_entry = tracks.iter().find(|t| {
                        let path = if t.contains('!') {
                            t.split('!').next().unwrap_or(t)
                        } else {
                            t
                        };
                        url.ends_with(path)
                    });
                    track_entry
                        .map(|t| radio::track_display_name(t))
                        .unwrap_or_else(|| station.name.clone())
                }
                _ => station.name.clone(),
            };

            eprintln!(
                "pixelbeat: downloading {} (attempt {}/{})...",
                title,
                attempt + 1,
                max_retries
            );

            match radio::download_track(&url) {
                Ok(bytes) => {
                    return self.play_radio_bytes(bytes, &title, None);
                }
                Err(e) => {
                    eprintln!("pixelbeat: track failed: {}, trying another...", e);
                    last_err = e;
                    continue;
                }
            }
        }

        Err(last_err.context("Failed to download radio track after retries"))
    }

    /// Play next track from a YouTube playlist station.
    /// Resolves the audio URL just-in-time (YouTube URLs expire after ~6 hours).
    fn play_next_youtube_track(&self, station: &RadioStation) -> Result<()> {
        let shuffle = self.state.lock().unwrap().shuffle;

        let yt_track = if shuffle {
            radio::random_yt_track(station)
        } else {
            let current_idx = *self.yt_track_index.lock().unwrap();
            if let Some((track, next_idx)) = radio::next_yt_track(station, current_idx) {
                *self.yt_track_index.lock().unwrap() = next_idx;
                Some(track)
            } else {
                None
            }
        };

        let yt_track = yt_track.ok_or_else(|| anyhow::anyhow!("No YouTube tracks available"))?;

        let track_count = match &station.source {
            radio::RadioSource::YouTube { tracks, .. } => tracks.len(),
            _ => 0,
        };

        // Stop rodio playback (we're switching to mpv)
        if let Some(ref sink) = *self.sink.lock().unwrap() {
            sink.stop();
        }

        // Stream via mpv — instant playback, no download
        let volume = self.state.lock().unwrap().volume;
        let url = format!("https://www.youtube.com/watch?v={}", yt_track.video_id);

        let mut mpv = self.mpv.lock().unwrap();
        mpv.play_url(&url, volume, self.cookies_browser.as_deref())?;

        // Update state
        let mut state = self.state.lock().unwrap();
        state.playing = true;
        state.title = format!("📻 {}", yt_track.title);
        state.duration = yt_track.duration;
        state.position = 0.0;
        state.track_count = track_count;

        *self.track_start.lock().unwrap() = Some(Instant::now());
        self.is_paused.store(false, Ordering::Relaxed);

        eprintln!(
            "pixelbeat: streaming: {} ({:.0}s)",
            yt_track.title, yt_track.duration
        );
        Ok(())
    }

    /// Play audio from in-memory bytes (used by both TrackList and YouTube radio).
    ///
    /// `yt_info`: if Some, provides (known_duration, track_count) from YouTube metadata
    ///            instead of estimating from byte length.
    fn play_radio_bytes(
        &self,
        bytes: Vec<u8>,
        title: &str,
        yt_info: Option<(f64, usize)>,
    ) -> Result<()> {
        let byte_len = bytes.len();

        if let Some(ref sink) = *self.sink.lock().unwrap() {
            sink.stop();
        }

        let cursor = Cursor::new(bytes);
        let source = Decoder::new(cursor).context("Failed to decode radio track")?;

        let sink = Sink::try_new(&self.stream_handle)?;
        let volume = self.state.lock().unwrap().volume;
        sink.set_volume(volume);
        sink.append(source);

        *self.sink.lock().unwrap() = Some(sink);
        *self.track_start.lock().unwrap() = Some(Instant::now());
        *self.pause_elapsed.lock().unwrap() = Duration::ZERO;
        self.is_paused.store(false, Ordering::Relaxed);

        let (duration, track_count) = match yt_info {
            Some((dur, count)) => (dur, Some(count)),
            None => (byte_len as f64 / 16000.0, None),
        };

        let mut state = self.state.lock().unwrap();
        state.playing = true;
        state.title = format!("📻 {}", title);
        state.duration = duration;
        state.position = 0.0;
        if let Some(count) = track_count {
            state.track_count = count;
        }

        eprintln!("pixelbeat: now playing: {} ({:.0}s)", title, duration);
        Ok(())
    }

    pub fn is_radio_mode(&self) -> bool {
        self.radio_station.lock().unwrap().is_some()
    }

    pub fn get_state(&self) -> PlayerState {
        self.state.lock().unwrap().clone()
    }
}
