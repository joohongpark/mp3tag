use std::path::Path;

use crate::models::TrackInfo;

/// Parse a filename into a TrackInfo with artist and title.
///
/// Supported patterns:
/// - "Artist - Title.mp3"
/// - "01. Title.mp3"
/// - "01 Artist - Title.mp3"
/// - "Title.mp3" (fallback)
pub fn parse_filename(path: &Path) -> TrackInfo {
    let stem = match path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => {
            return TrackInfo {
                source: "filename".to_string(),
                ..Default::default()
            }
        }
    };

    let stem = stem.trim().to_string();

    // Try "01 Artist - Title" or "01. Artist - Title"
    if let Some(info) = try_numbered_artist_title(&stem) {
        return info;
    }

    // Try "Artist - Title"
    if let Some(info) = try_artist_title(&stem) {
        return info;
    }

    // Try "01. Title" or "01 Title"
    if let Some(info) = try_numbered_title(&stem) {
        return info;
    }

    // Fallback: entire stem is the title
    TrackInfo {
        title: Some(stem),
        source: "filename".to_string(),
        ..Default::default()
    }
}

/// Build a search query from a TrackInfo (for Spotify search).
pub fn build_search_query(info: &TrackInfo) -> String {
    let mut parts = Vec::new();
    if let Some(ref artist) = info.artist {
        parts.push(artist.clone());
    }
    if let Some(ref title) = info.title {
        parts.push(title.clone());
    }
    if parts.is_empty() {
        return String::new();
    }
    parts.join(" ")
}

fn try_numbered_artist_title(stem: &str) -> Option<TrackInfo> {
    // Pattern: "01. Artist - Title" or "01 Artist - Title"
    let rest = strip_track_number(stem)?;
    try_artist_title(rest)
}

fn try_artist_title(stem: &str) -> Option<TrackInfo> {
    // Split on " - "
    let parts: Vec<&str> = stem.splitn(2, " - ").collect();
    if parts.len() != 2 {
        return None;
    }

    let artist = parts[0].trim();
    let title = parts[1].trim();

    if artist.is_empty() || title.is_empty() {
        return None;
    }

    Some(TrackInfo {
        title: Some(title.to_string()),
        artist: Some(artist.to_string()),
        source: "filename".to_string(),
        ..Default::default()
    })
}

fn try_numbered_title(stem: &str) -> Option<TrackInfo> {
    let rest = strip_track_number(stem)?;
    let title = rest.trim();
    if title.is_empty() {
        return None;
    }
    Some(TrackInfo {
        title: Some(title.to_string()),
        source: "filename".to_string(),
        ..Default::default()
    })
}

fn strip_track_number(stem: &str) -> Option<&str> {
    let chars: Vec<char> = stem.chars().collect();
    if chars.len() < 2 {
        return None;
    }

    // Must start with digits
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }

    let rest = &stem[i..];

    // Skip optional "." and/or spaces
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    let rest = rest.trim_start();

    if rest.is_empty() {
        return None;
    }

    Some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_artist_title() {
        let info = parse_filename(&PathBuf::from("IU - Blueming.mp3"));
        assert_eq!(info.artist.as_deref(), Some("IU"));
        assert_eq!(info.title.as_deref(), Some("Blueming"));
    }

    #[test]
    fn test_numbered_title() {
        let info = parse_filename(&PathBuf::from("01. Blueming.mp3"));
        assert_eq!(info.title.as_deref(), Some("Blueming"));
        assert!(info.artist.is_none());
    }

    #[test]
    fn test_numbered_artist_title() {
        let info = parse_filename(&PathBuf::from("01 IU - Blueming.mp3"));
        assert_eq!(info.artist.as_deref(), Some("IU"));
        assert_eq!(info.title.as_deref(), Some("Blueming"));
    }

    #[test]
    fn test_fallback() {
        let info = parse_filename(&PathBuf::from("SomeSong.mp3"));
        assert_eq!(info.title.as_deref(), Some("SomeSong"));
        assert!(info.artist.is_none());
    }

    #[test]
    fn test_search_query() {
        let info = TrackInfo {
            title: Some("Blueming".to_string()),
            artist: Some("IU".to_string()),
            ..Default::default()
        };
        assert_eq!(build_search_query(&info), "IU Blueming");
    }
}
