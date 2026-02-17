use std::path::Path;

use anyhow::Result;

use crate::core::tagger;
use crate::models::Mp3File;

/// 디렉토리를 재귀 탐색하여 모든 MP3 파일을 스캔한다.
/// 각 파일의 ID3 태그를 읽어 Mp3File 목록을 반환한다.
pub fn scan_directory(dir: &Path) -> Result<Vec<Mp3File>> {
    let mut files = Vec::new();
    collect_mp3_files(dir, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// 디렉토리를 재귀 순회하며 MP3 파일을 수집한다.
fn collect_mp3_files(dir: &Path, files: &mut Vec<Mp3File>) -> Result<()> {
    if !dir.is_dir() {
        anyhow::bail!("{}은(는) 디렉토리가 아닙니다", dir.display());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_mp3_files(&path, files)?;
        } else if is_mp3(&path) {
            let mp3 = load_mp3_file(&path);
            files.push(mp3);
        }
    }

    Ok(())
}

/// 확장자가 .mp3인지 확인한다 (대소문자 무시).
fn is_mp3(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("mp3"))
        .unwrap_or(false)
}

/// MP3 파일 하나를 로드하여 태그 정보를 포함한 Mp3File을 반환한다.
fn load_mp3_file(path: &Path) -> Mp3File {
    match tagger::read_tags(path) {
        Ok(Some(tags)) => Mp3File {
            path: path.to_path_buf(),
            has_tags: true,
            current_tags: Some(tags),
        },
        _ => Mp3File {
            path: path.to_path_buf(),
            has_tags: false,
            current_tags: None,
        },
    }
}

/// 단일 MP3 파일을 로드한다. 파일이 없거나 MP3가 아니면 에러.
pub fn load_single_file(path: &Path) -> Result<Mp3File> {
    if !path.exists() {
        anyhow::bail!("파일을 찾을 수 없습니다: {}", path.display());
    }
    if !is_mp3(path) {
        anyhow::bail!("MP3 파일이 아닙니다: {}", path.display());
    }
    Ok(load_mp3_file(path))
}

/// 경로가 디렉토리면 재귀 스캔, 파일이면 단일 로드한다.
pub fn scan_path(path: &Path) -> Result<Vec<Mp3File>> {
    if path.is_dir() {
        scan_directory(path)
    } else {
        Ok(vec![load_single_file(path)?])
    }
}
