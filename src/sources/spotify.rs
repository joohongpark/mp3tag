use anyhow::{Context, Result};
use base64::Engine;
use serde::Deserialize;

use crate::config::SpotifyConfig;
use crate::models::TrackInfo;
use crate::sources::MusicSource;

/// Spotify Web API 클라이언트.
/// Client Credentials Flow로 인증하여 검색 및 앨범 아트 다운로드를 수행한다.
pub struct SpotifyClient {
    client: reqwest::blocking::Client,
    access_token: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    tracks: TracksResult,
}

#[derive(Deserialize)]
struct TracksResult {
    items: Vec<SpotifyTrack>,
}

#[derive(Deserialize)]
struct SpotifyTrack {
    name: String,
    artists: Vec<SpotifyArtist>,
    album: SpotifyAlbum,
    track_number: u32,
}

#[derive(Deserialize)]
struct SpotifyArtist {
    name: String,
}

#[derive(Deserialize)]
struct SpotifyAlbum {
    name: String,
    release_date: Option<String>,
    images: Vec<SpotifyImage>,
}

#[derive(Deserialize)]
struct SpotifyImage {
    url: String,
    width: Option<u32>,
}

impl SpotifyClient {
    /// 설정에서 자격증명을 읽어 인증 후 클라이언트를 생성한다.
    pub fn new(config: &SpotifyConfig) -> Result<Self> {
        let client_id = config
            .client_id
            .as_ref()
            .context("Spotify client_id가 설정되지 않았습니다")?;
        let client_secret = config
            .client_secret
            .as_ref()
            .context("Spotify client_secret가 설정되지 않았습니다")?;

        let client = reqwest::blocking::Client::new();
        let access_token = Self::authenticate(&client, client_id, client_secret)?;

        Ok(Self {
            client,
            access_token,
        })
    }

    /// Client Credentials Flow로 access token을 발급받는다.
    fn authenticate(
        client: &reqwest::blocking::Client,
        client_id: &str,
        client_secret: &str,
    ) -> Result<String> {
        let credentials = format!("{}:{}", client_id, client_secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);

        let resp: TokenResponse = client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", encoded))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .context("Spotify 연결에 실패했습니다")?
            .error_for_status()
            .context("Spotify 인증에 실패했습니다. client_id와 client_secret를 확인하세요.")?
            .json()
            .context("Spotify 토큰 응답 파싱에 실패했습니다")?;

        Ok(resp.access_token)
    }

    /// 발매일 문자열에서 연도를 추출한다 (예: "2019-11-18" -> 2019).
    fn parse_year(release_date: &Option<String>) -> Option<i32> {
        release_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok())
    }

    /// Spotify API의 트랙 응답을 TrackInfo로 변환한다.
    fn convert_track(track: &SpotifyTrack) -> TrackInfo {
        let artist = track
            .artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let album_art_url = track
            .album
            .images
            .iter()
            .max_by_key(|img| img.width.unwrap_or(0))
            .map(|img| img.url.clone());

        TrackInfo {
            title: Some(track.name.clone()),
            artist: Some(artist),
            album: Some(track.album.name.clone()),
            album_artist: track.artists.first().map(|a| a.name.clone()),
            track_number: Some(track.track_number),
            year: Self::parse_year(&track.album.release_date),
            genre: None,
            album_art: None,
            album_art_url,
            source: "spotify".to_string(),
        }
    }
}

impl MusicSource for SpotifyClient {
    fn search(&self, query: &str) -> Result<Vec<TrackInfo>> {
        let resp: SearchResponse = self
            .client
            .get("https://api.spotify.com/v1/search")
            .bearer_auth(&self.access_token)
            .query(&[("q", query), ("type", "track"), ("limit", "10")])
            .send()
            .context("Spotify 검색에 실패했습니다")?
            .error_for_status()
            .context("Spotify 검색 요청이 실패했습니다")?
            .json()
            .context("Spotify 검색 응답 파싱에 실패했습니다")?;

        let results = resp
            .tracks
            .items
            .iter()
            .map(Self::convert_track)
            .collect();

        Ok(results)
    }

    fn fetch_album_art(&self, track: &TrackInfo) -> Result<Vec<u8>> {
        let url = track
            .album_art_url
            .as_ref()
            .context("앨범 아트 URL이 없습니다")?;

        let data = self
            .client
            .get(url)
            .send()
            .context("앨범 아트 다운로드에 실패했습니다")?
            .error_for_status()?
            .bytes()?
            .to_vec();

        Ok(data)
    }
}
