use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    /// "local", "chillhop", "lofi", "youtube" — default source on startup
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default = "default_music_dir")]
    pub music_dir: String,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub repeat: bool,
    #[serde(default)]
    pub shuffle: bool,
    /// YouTube playlist URL (used when source = "youtube")
    #[serde(default)]
    pub youtube_url: Option<String>,
}

fn default_music_dir() -> String {
    "~/Music/pixelbeat".to_string()
}

fn default_volume() -> f32 {
    0.8
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source: None,
            music_dir: default_music_dir(),
            volume: default_volume(),
            repeat: false,
            shuffle: false,
            youtube_url: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = config_path();
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(cfg) => return cfg,
                    Err(e) => eprintln!("pixelbeat: config parse error: {}", e),
                },
                Err(e) => eprintln!("pixelbeat: config read error: {}", e),
            }
        }
        Self::default()
    }

    /// Expand ~ in music_dir and return as PathBuf
    pub fn music_dir_expanded(&self) -> Option<PathBuf> {
        let expanded = if self.music_dir.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(&self.music_dir[2..])
            } else {
                PathBuf::from(&self.music_dir)
            }
        } else {
            PathBuf::from(&self.music_dir)
        };
        Some(expanded)
    }
}

fn config_path() -> PathBuf {
    // Check ~/.config/pixelbeat/config.toml first (XDG), then platform default
    let xdg_path = dirs::home_dir()
        .map(|h| h.join(".config/pixelbeat/config.toml"));
    if let Some(ref p) = xdg_path {
        if p.exists() {
            return p.clone();
        }
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("pixelbeat")
        .join("config.toml")
}
