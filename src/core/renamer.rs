use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::models::TrackInfo;

/// 파일명에 사용할 수 없는 문자를 `_`로 치환한다.
pub fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '/' || c == '\0' {
                return '_';
            }
            if cfg!(target_os = "windows") {
                if matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                    return '_';
                }
                if c.is_ascii_control() {
                    return '_';
                }
            }
            if cfg!(target_os = "macos") && c == ':' {
                return '_';
            }
            c
        })
        .collect()
}

/// TrackInfo에서 `"{artist} - {title}.mp3"` 형식의 파일명을 생성한다.
/// artist와 title이 모두 있어야 Some을 반환한다.
pub fn build_filename(info: &TrackInfo) -> Option<String> {
    let artist = info.artist.as_deref()?.trim();
    let title = info.title.as_deref()?.trim();
    if artist.is_empty() || title.is_empty() {
        return None;
    }
    Some(format!(
        "{} - {}.mp3",
        sanitize_filename(artist),
        sanitize_filename(title)
    ))
}

/// 파일명을 `"{artist} - {title}.mp3"` 형식으로 변경한다.
/// 이미 같은 이름이면 현재 경로를 그대로 반환한다.
/// 동일 디렉토리에 같은 이름의 파일이 이미 존재하면 에러를 반환한다.
pub fn rename_file(old_path: &Path, info: &TrackInfo) -> Result<PathBuf> {
    let new_name = match build_filename(info) {
        Some(name) => name,
        None => bail!("아티스트와 제목이 모두 필요합니다"),
    };

    let dir = old_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let new_path = dir.join(&new_name);

    // 이미 같은 이름이면 그대로 반환
    if old_path == new_path {
        return Ok(new_path);
    }

    // 이름 충돌 검사
    if new_path.exists() {
        bail!("파일이 이미 존재합니다: {}", new_name);
    }

    std::fs::rename(old_path, &new_path)?;
    Ok(new_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_removes_slash_and_null() {
        assert_eq!(sanitize_filename("a/b\0c"), "a_b_c");
    }

    #[test]
    fn test_sanitize_filename_normal_text() {
        assert_eq!(sanitize_filename("Hello World"), "Hello World");
    }

    #[test]
    fn test_sanitize_filename_korean() {
        assert_eq!(sanitize_filename("아이유 - 좋은날"), "아이유 - 좋은날");
    }

    #[test]
    fn test_build_filename_both_present() {
        let info = TrackInfo {
            artist: Some("IU".to_string()),
            title: Some("Good Day".to_string()),
            ..Default::default()
        };
        assert_eq!(build_filename(&info), Some("IU - Good Day.mp3".to_string()));
    }

    #[test]
    fn test_build_filename_missing_artist() {
        let info = TrackInfo {
            title: Some("Good Day".to_string()),
            ..Default::default()
        };
        assert_eq!(build_filename(&info), None);
    }

    #[test]
    fn test_build_filename_missing_title() {
        let info = TrackInfo {
            artist: Some("IU".to_string()),
            ..Default::default()
        };
        assert_eq!(build_filename(&info), None);
    }

    #[test]
    fn test_build_filename_empty_strings() {
        let info = TrackInfo {
            artist: Some("".to_string()),
            title: Some("Good Day".to_string()),
            ..Default::default()
        };
        assert_eq!(build_filename(&info), None);
    }

    #[test]
    fn test_build_filename_sanitizes() {
        let info = TrackInfo {
            artist: Some("AC/DC".to_string()),
            title: Some("Back\0Slash".to_string()),
            ..Default::default()
        };
        assert_eq!(
            build_filename(&info),
            Some("AC_DC - Back_Slash.mp3".to_string())
        );
    }
}
