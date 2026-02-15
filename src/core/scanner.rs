use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::tagger;
use crate::models::Mp3File;

pub fn scan_directory(dir: &Path) -> Result<Vec<Mp3File>> {
    let mut files = Vec::new();
    collect_mp3_files(dir, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

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

fn is_mp3(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("mp3"))
        .unwrap_or(false)
}

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

pub fn load_single_file(path: &Path) -> Result<Mp3File> {
    if !path.exists() {
        anyhow::bail!("파일을 찾을 수 없습니다: {}", path.display());
    }
    if !is_mp3(path) {
        anyhow::bail!("MP3 파일이 아닙니다: {}", path.display());
    }
    Ok(load_mp3_file(path))
}

pub fn scan_path(path: &Path) -> Result<Vec<Mp3File>> {
    if path.is_dir() {
        scan_directory(path)
    } else {
        Ok(vec![load_single_file(path)?])
    }
}

pub fn find_mp3_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_paths(dir, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_paths(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        anyhow::bail!("{}은(는) 디렉토리가 아닙니다", dir.display());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_paths(&path, paths)?;
        } else if is_mp3(&path) {
            paths.push(path);
        }
    }
    Ok(())
}
