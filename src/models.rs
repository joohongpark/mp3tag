use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct TrackInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub album_art: Option<Vec<u8>>,
    pub album_art_url: Option<String>,
    pub source: String,
}

impl TrackInfo {
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("알 수 없음")
    }

    pub fn display_artist(&self) -> &str {
        self.artist.as_deref().unwrap_or("알 수 없음")
    }

    pub fn display_album(&self) -> &str {
        self.album.as_deref().unwrap_or("알 수 없음")
    }

    pub fn summary(&self) -> String {
        format!(
            "{} - {} [{}]",
            self.display_artist(),
            self.display_title(),
            self.display_album()
        )
    }
}

#[derive(Debug, Clone)]
pub struct Mp3File {
    pub path: PathBuf,
    pub current_tags: Option<TrackInfo>,
    pub has_tags: bool,
}

impl Mp3File {
    pub fn filename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("알 수 없음")
    }
}
