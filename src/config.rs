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
    /// Browser to extract cookies from for YouTube (e.g., "chrome", "firefox").
    /// Leave empty/None to disable cookie extraction.
    #[serde(default)]
    pub youtube_cookies_browser: Option<String>,
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
            youtube_cookies_browser: None,
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
        } else {
            // Auto-generate a commented template config on first run
            Self::generate_default_config(&config_path);
        }
        Self::default()
    }

    /// Generate a default config file with all options commented out
    fn generate_default_config(path: &PathBuf) {
        let template = r#"# pixelbeat configuration
# https://github.com/Dylanwooo/pixelbeat#configuration
#
# All fields are optional — sensible defaults are used when omitted.

# Default source on daemon startup: "local", "chillhop", "lofi", "youtube"
# source = "local"

# Local music directory (tilde expansion supported)
# music_dir = "~/Music/pixelbeat"

# YouTube playlist URL (used when source = "youtube")
# youtube_url = "https://www.youtube.com/watch?v=jfKfPfyJRdk"

# Default volume (0.0 - 1.0)
# volume = 0.8

# Auto-repeat / loop
# repeat = false

# Shuffle mode
# shuffle = false

# Browser to extract cookies from for YouTube (e.g., "chrome", "firefox")
# Useful for YouTube Music or age-restricted content
# youtube_cookies_browser = "chrome"
"#;

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("pixelbeat: failed to create config directory: {}", e);
                    return;
                }
            }
        }

        match std::fs::write(path, template) {
            Ok(_) => eprintln!("pixelbeat: created default config at {}", path.display()),
            Err(e) => eprintln!("pixelbeat: failed to write config: {}", e),
        }
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
    let xdg_path = dirs::home_dir().map(|h| h.join(".config/pixelbeat/config.toml"));
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
