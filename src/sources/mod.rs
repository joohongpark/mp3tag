pub mod melon;
pub mod spotify;

use anyhow::Result;

use crate::models::TrackInfo;

/// 음악 메타데이터 소스 트레이트.
/// Spotify, Bugs, Melon 등 다양한 소스를 이 트레이트로 추상화한다.
pub trait MusicSource {
    /// 쿼리 문자열로 트랙을 검색한다.
    fn search(&self, query: &str) -> Result<Vec<TrackInfo>>;
    /// 트랙의 앨범 아트 이미지를 다운로드한다.
    fn fetch_album_art(&self, track: &TrackInfo) -> Result<Vec<u8>>;
    /// 트랙의 상세 정보(메타데이터 + 앨범 아트)를 가져온다.
    /// 기본 구현은 앨범 아트만 추가하여 반환한다.
    fn fetch_detail(&self, track: &TrackInfo) -> Result<TrackInfo> {
        let art = self.fetch_album_art(track)?;
        let mut detailed = track.clone();
        detailed.album_art = Some(art);
        Ok(detailed)
    }
}
