use std::path::Path;

use crate::models::TrackInfo;

/// 파일명을 파싱하여 아티스트와 제목이 포함된 TrackInfo를 반환한다.
///
/// 지원 패턴:
/// - "아티스트 - 제목.mp3"
/// - "01. 제목.mp3"
/// - "01 아티스트 - 제목.mp3"
/// - "제목.mp3" (폴백)
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

    // "01 아티스트 - 제목" 또는 "01. 아티스트 - 제목" 패턴 시도
    if let Some(info) = try_numbered_artist_title(&stem) {
        return info;
    }

    // "아티스트 - 제목" 패턴 시도
    if let Some(info) = try_artist_title(&stem) {
        return info;
    }

    // "01. 제목" 또는 "01 제목" 패턴 시도
    if let Some(info) = try_numbered_title(&stem) {
        return info;
    }

    // 폴백: 전체 파일명을 제목으로 사용
    TrackInfo {
        title: Some(stem),
        source: "filename".to_string(),
        ..Default::default()
    }
}

/// TrackInfo에서 검색 쿼리를 생성한다 (Spotify 검색용).
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

/// "01 아티스트 - 제목" 또는 "01. 아티스트 - 제목" 패턴을 시도한다.
fn try_numbered_artist_title(stem: &str) -> Option<TrackInfo> {
    // 패턴: "01. 아티스트 - 제목" 또는 "01 아티스트 - 제목"
    let rest = strip_track_number(stem)?;
    try_artist_title(rest)
}

/// "아티스트 - 제목" 패턴을 시도한다. " - "로 분리.
fn try_artist_title(stem: &str) -> Option<TrackInfo> {
    // " - "로 분리
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

/// "01. 제목" 또는 "01 제목" 패턴을 시도한다.
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

/// 문자열 앞의 트랙 번호를 제거하고 나머지를 반환한다.
fn strip_track_number(stem: &str) -> Option<&str> {
    let chars: Vec<char> = stem.chars().collect();
    if chars.len() < 2 {
        return None;
    }

    // 숫자로 시작해야 함
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }

    let rest = &stem[i..];

    // 선택적 "."과 공백 건너뛰기
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
