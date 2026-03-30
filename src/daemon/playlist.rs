use anyhow::Result;
use std::path::PathBuf;

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "aiff"];

pub struct Playlist {
    pub tracks: Vec<PathBuf>,
    pub index: usize,
    pub shuffle: bool,
    pub repeat: bool,
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            index: 0,
            shuffle: false,
            repeat: false,
        }
    }

    pub fn load_directory(&mut self, dir: &PathBuf) -> Result<()> {
        self.tracks.clear();
        self.index = 0;

        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|ext| {
                        SUPPORTED_EXTENSIONS
                            .contains(&ext.to_string_lossy().to_lowercase().as_str())
                    })
                    .unwrap_or(false)
            })
            .collect();

        entries.sort();
        self.tracks = entries;

        if self.shuffle {
            self.shuffle_tracks();
        }

        Ok(())
    }

    pub fn add_file(&mut self, path: PathBuf) -> Result<()> {
        if path.exists() {
            self.tracks.push(path);
        }
        Ok(())
    }

    pub fn current_track(&self) -> Option<&PathBuf> {
        self.tracks.get(self.index)
    }

    pub fn next(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        if self.index + 1 < self.tracks.len() {
            self.index += 1;
        } else if self.repeat {
            self.index = 0;
            if self.shuffle {
                self.shuffle_tracks();
            }
        }
    }

    pub fn prev(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        if self.index > 0 {
            self.index -= 1;
        } else if self.repeat {
            self.index = self.tracks.len() - 1;
        }
    }

    fn shuffle_tracks(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.tracks.shuffle(&mut rng);
    }
}
