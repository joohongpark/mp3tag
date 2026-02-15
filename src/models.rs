use std::path::PathBuf;

/// 트랙의 메타데이터를 담는 구조체.
/// ID3 태그, Spotify 검색 결과, 파일명 파싱 결과 등 다양한 소스에서 생성된다.
#[derive(Debug, Clone, Default)]
pub struct TrackInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    /// 앨범 아트 바이너리 (JPEG/PNG)
    pub album_art: Option<Vec<u8>>,
    /// 앨범 아트 다운로드 URL (Spotify 등 외부 소스용)
    pub album_art_url: Option<String>,
    /// 데이터 출처 ("id3", "spotify", "filename", "manual")
    pub source: String,
}

impl TrackInfo {
    /// 제목을 표시용 문자열로 반환한다. 없으면 "알 수 없음".
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("알 수 없음")
    }

    /// 아티스트를 표시용 문자열로 반환한다. 없으면 "알 수 없음".
    pub fn display_artist(&self) -> &str {
        self.artist.as_deref().unwrap_or("알 수 없음")
    }

    /// 앨범을 표시용 문자열로 반환한다. 없으면 "알 수 없음".
    pub fn display_album(&self) -> &str {
        self.album.as_deref().unwrap_or("알 수 없음")
    }

    /// "아티스트 - 제목 [앨범]" 형식의 요약 문자열을 반환한다.
    pub fn summary(&self) -> String {
        format!(
            "{} - {} [{}]",
            self.display_artist(),
            self.display_title(),
            self.display_album()
        )
    }
}

/// 스캔된 MP3 파일 하나를 나타내는 구조체.
#[derive(Debug, Clone)]
pub struct Mp3File {
    pub path: PathBuf,
    pub current_tags: Option<TrackInfo>,
    pub has_tags: bool,
}

impl Mp3File {
    /// 파일명만 추출하여 반환한다.
    pub fn filename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("알 수 없음")
    }
}
