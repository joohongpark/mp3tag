pub mod spotify;

use anyhow::Result;

use crate::models::TrackInfo;

pub trait MusicSource {
    fn name(&self) -> &str;
    fn search(&self, query: &str) -> Result<Vec<TrackInfo>>;
    fn fetch_album_art(&self, track: &TrackInfo) -> Result<Vec<u8>>;
}
