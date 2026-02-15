use std::path::Path;

use anyhow::Result;
use id3::{Tag, TagLike, Version};

use crate::models::TrackInfo;

/// MP3 파일에서 ID3 태그를 읽어 TrackInfo로 변환한다.
/// 태그가 없거나 제목/아티스트/앨범이 모두 비어있으면 None을 반환한다.
pub fn read_tags(path: &Path) -> Result<Option<TrackInfo>> {
    let tag = match Tag::read_from_path(path) {
        Ok(tag) => tag,
        Err(id3::Error {
            kind: id3::ErrorKind::NoTag,
            ..
        }) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let has_any = tag.title().is_some()
        || tag.artist().is_some()
        || tag.album().is_some();

    if !has_any {
        return Ok(None);
    }

    let album_art = tag
        .pictures()
        .next()
        .map(|pic| pic.data.clone());

    let info = TrackInfo {
        title: tag.title().map(|s| s.to_string()),
        artist: tag.artist().map(|s| s.to_string()),
        album: tag.album().map(|s| s.to_string()),
        album_artist: tag.album_artist().map(|s| s.to_string()),
        track_number: tag.track(),
        year: tag.year(),
        genre: tag.genre_parsed().map(|s| s.to_string()),
        album_art,
        album_art_url: None,
        source: "id3".to_string(),
    };

    Ok(Some(info))
}

/// TrackInfo를 MP3 파일에 ID3v2.4 태그로 기록한다.
/// 기존 태그가 있으면 지정된 필드만 덮어쓴다.
pub fn write_tags(path: &Path, info: &TrackInfo) -> Result<()> {
    let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());

    if let Some(ref title) = info.title {
        tag.set_title(title);
    }
    if let Some(ref artist) = info.artist {
        tag.set_artist(artist);
    }
    if let Some(ref album) = info.album {
        tag.set_album(album);
    }
    if let Some(ref album_artist) = info.album_artist {
        tag.set_album_artist(album_artist);
    }
    if let Some(track) = info.track_number {
        tag.set_track(track);
    }
    if let Some(year) = info.year {
        tag.set_year(year);
    }
    if let Some(ref genre) = info.genre {
        tag.set_genre(genre);
    }
    if let Some(ref art_data) = info.album_art {
        tag.remove_all_pictures();
        tag.add_frame(id3::frame::Picture {
            mime_type: detect_mime_type(art_data),
            picture_type: id3::frame::PictureType::CoverFront,
            description: String::new(),
            data: art_data.clone(),
        });
    }

    tag.write_to_path(path, Version::Id3v24)?;
    Ok(())
}

/// 기존 태그와 새 태그를 병합한다. 새 값이 있으면 우선 적용된다.
pub fn merge_tags(existing: &Option<TrackInfo>, new_info: &TrackInfo) -> TrackInfo {
    match existing {
        Some(existing) => TrackInfo {
            title: new_info.title.clone().or_else(|| existing.title.clone()),
            artist: new_info.artist.clone().or_else(|| existing.artist.clone()),
            album: new_info.album.clone().or_else(|| existing.album.clone()),
            album_artist: new_info
                .album_artist
                .clone()
                .or_else(|| existing.album_artist.clone()),
            track_number: new_info.track_number.or(existing.track_number),
            year: new_info.year.or(existing.year),
            genre: new_info.genre.clone().or_else(|| existing.genre.clone()),
            album_art: new_info
                .album_art
                .clone()
                .or_else(|| existing.album_art.clone()),
            album_art_url: new_info
                .album_art_url
                .clone()
                .or_else(|| existing.album_art_url.clone()),
            source: new_info.source.clone(),
        },
        None => new_info.clone(),
    }
}

/// 이미지 바이너리의 매직 바이트로 MIME 타입을 판별한다.
fn detect_mime_type(data: &[u8]) -> String {
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png".to_string()
    } else {
        "image/jpeg".to_string()
    }
}
