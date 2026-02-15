pub mod spotify;

use anyhow::Result;

use crate::models::TrackInfo;

/// 음악 메타데이터 소스 트레이트.
/// Spotify, Bugs, Melon 등 다양한 소스를 이 트레이트로 추상화한다.
pub trait MusicSource {
    /// 소스 이름을 반환한다.
    fn name(&self) -> &str;
    /// 쿼리 문자열로 트랙을 검색한다.
    fn search(&self, query: &str) -> Result<Vec<TrackInfo>>;
    /// 트랙의 앨범 아트 이미지를 다운로드한다.
    fn fetch_album_art(&self, track: &TrackInfo) -> Result<Vec<u8>>;
}
