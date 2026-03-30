use anyhow::{Context, Result};
use std::io::Cursor;

use super::youtube::YtTrack;

/// Built-in radio sources
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RadioStation {
    pub name: String,
    pub source: RadioSource,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum RadioSource {
    /// Individual tracks downloaded from a URL list
    TrackList {
        base_url: String,
        tracks: Vec<String>,
    },
    /// YouTube playlist — tracks resolved on-demand via yt-dlp
    YouTube {
        playlist_url: String,
        tracks: Vec<YtTrack>,
    },
}

/// Built-in stations
pub fn builtin_stations() -> Vec<RadioStation> {
    vec![
        RadioStation {
            name: "chillhop".to_string(),
            source: RadioSource::TrackList {
                base_url: "https://stream.chillhop.com/mp3/".to_string(),
                tracks: CHILLHOP_TRACKS.iter().map(|s| s.to_string()).collect(),
            },
        },
        RadioStation {
            name: "lofi".to_string(),
            source: RadioSource::TrackList {
                base_url: "https://ia601004.us.archive.org/31/items/lofigirl/".to_string(),
                tracks: ARCHIVE_TRACKS.iter().map(|s| s.to_string()).collect(),
            },
        },
    ]
}

pub fn find_station(name: &str) -> Option<RadioStation> {
    let name = name.to_lowercase();
    builtin_stations()
        .into_iter()
        .find(|s| s.name == name || s.name.contains(&name))
}

pub fn list_stations() -> Vec<String> {
    builtin_stations().iter().map(|s| s.name.clone()).collect()
}

/// Download a track from URL into memory bytes
pub fn download_track(url: &str) -> Result<Vec<u8>> {
    eprintln!("pixelbeat: GET {}", url);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) pixelbeat/0.1.0")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .context(format!("Failed to fetch: {}", url))?;

    eprintln!("pixelbeat: HTTP {} ({})", response.status(), url);

    if !response.status().is_success() {
        anyhow::bail!("HTTP {}: {}", response.status(), url);
    }

    let bytes = response.bytes().context("Failed to read response body")?;
    eprintln!("pixelbeat: downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

/// Download and decode a track, returning a rodio source
pub fn download_and_decode(url: &str) -> Result<rodio::Decoder<Cursor<Vec<u8>>>> {
    let bytes = download_track(url)?;
    let cursor = Cursor::new(bytes);
    rodio::Decoder::new(cursor).context("Failed to decode audio stream")
}

/// Get a random track URL from a station (for TrackList sources only)
pub fn random_track_url(station: &RadioStation) -> Option<String> {
    match &station.source {
        RadioSource::TrackList { base_url, tracks } => {
            if tracks.is_empty() {
                return None;
            }
            use rand::Rng;
            let idx = rand::thread_rng().gen_range(0..tracks.len());
            let track = &tracks[idx];

            // Chillhop format: "id!name" or just a path
            let path = if track.contains('!') {
                track.split('!').next().unwrap_or(track)
            } else {
                track
            };

            Some(format!("{}{}", base_url, path))
        }
        RadioSource::YouTube { .. } => {
            // YouTube tracks are resolved differently — use random_yt_track() instead
            None
        }
    }
}

/// Pick a random YtTrack from a YouTube station
pub fn random_yt_track(station: &RadioStation) -> Option<YtTrack> {
    match &station.source {
        RadioSource::YouTube { tracks, .. } => {
            if tracks.is_empty() {
                return None;
            }
            use rand::Rng;
            let idx = rand::thread_rng().gen_range(0..tracks.len());
            Some(tracks[idx].clone())
        }
        _ => None,
    }
}

/// Pick the next sequential YtTrack (for non-shuffle mode)
pub fn next_yt_track(station: &RadioStation, current_index: usize) -> Option<(YtTrack, usize)> {
    match &station.source {
        RadioSource::YouTube { tracks, .. } => {
            if tracks.is_empty() {
                return None;
            }
            let next_idx = (current_index + 1) % tracks.len();
            Some((tracks[next_idx].clone(), next_idx))
        }
        _ => None,
    }
}

/// Get the display name for a track entry
pub fn track_display_name(track: &str) -> String {
    if track.contains('!') {
        // Chillhop format: "id!Artist - Title"
        track.split('!').nth(1).unwrap_or(track).to_string()
    } else {
        // Archive format: "album/filename.mp3"
        track
            .rsplit('/')
            .next()
            .unwrap_or(track)
            .trim_end_matches(".mp3")
            .replace("%20", " ")
            .replace("%28", "(")
            .replace("%29", ")")
            .to_string()
    }
}

// ── Built-in track lists ──────────────────────────────────
// Chillhop tracks (id!display_name format)
const CHILLHOP_TRACKS: &[&str] = &[
    "9476!Guustavv - Apple Juice",
    "9272!Sleepy Fish - A Rainy Night in Kyoto",
    "9309!Psalm Trees - Still Awake",
    "10476!Aso - Seasons",
    "9950!Idealism - Lonely",
    "9124!L'indecis - Staying",
    "8888!Kupla - Owls of the Night",
    "9807!Birocratic - Belly Full of Turkey",
    "9670!jinsang - Solitude",
    "10181!quickly quickly - Cold Stares",
    "9421!Philanthrope - Maple Leaf",
    "9506!SwuM - Falling",
    "10098!Hoogway - Clockwork",
    "9362!Arbour Season - In Bloom",
    "9215!Saib - Sakura",
    "10269!Pandrezz - It's Okay",
    "9743!Bonus Points - Home",
    "9181!Tomppabeats - Monday Loop",
    "9090!Ian Ewing - Nostalgia",
    "10342!Middle School - Coastal",
    "9553!In Love With A Ghost - Healing",
    "9617!j'san - Moonlight",
    "10420!Oatmello - Warm",
    "9834!Yung Kartz - Cloudy",
    "10012!Vanilla - Twilight",
    "9889!Nymano - June",
    "10143!Kainbeats - Daisies",
    "9451!DLJ - Cruisin",
    "10225!Psalm Trees - Evergreen",
    "9305!Bonsaye - Sunday",
];

// Internet Archive lofi girl tracks
const ARCHIVE_TRACKS: &[&str] = &[
    "2-AM-Study-Session/01%20hoogway%20-%20Missing%20Earth%20%28Kupla%20Master%29.mp3",
    "2-AM-Study-Session/02%20foolk%20-%20coffee%20with%20cream.mp3",
    "2-AM-Study-Session/03%20Flovry%20-%20Aura.mp3",
    "2-AM-Study-Session/04%20brillion%20-%20like%20a%20dream.mp3",
    "2-AM-Study-Session/05%20Mondo%20Loops%20x%20Coa%20-%20Pastime.mp3",
    "2-AM-Study-Session/06%20Leavv%20-%20Shades%20of%20spring.mp3",
    "2-AM-Study-Session/07%20WYS%20-%20snowdrift.mp3",
    "2-AM-Study-Session/08%20Sleepy%20Fish%20-%20Lulling.mp3",
    "2-AM-Study-Session/09%20Dontcry%20-%20Birdsong.mp3",
    "2-AM-Study-Session/10%20Mondo%20Loops%20-%20Mirage.mp3",
    "2-AM-Study-Session/11%20Purrple%20Cat%20-%20Wish.mp3",
    "2-AM-Study-Session/12%20Kupla%20-%20April%20Showers.mp3",
    "2-AM-Study-Session/13%20eevee%20-%20Promise.mp3",
    "2-AM-Study-Session/14%20Philanthrope%20-%20Seed.mp3",
    "2-AM-Study-Session/15%20Luv%20Bird%20-%20Sleep%20On%20It.mp3",
];
