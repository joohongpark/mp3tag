use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::models::TrackInfo;
use crate::sources::MusicSource;

/// Melon 웹사이트 스크래핑 클라이언트.
/// 인증 없이 검색 페이지 HTML을 파싱하여 곡 정보를 가져온다.
pub struct MelonClient {
    client: reqwest::blocking::Client,
}

impl MelonClient {
    /// 새 MelonClient를 생성한다. User-Agent 헤더를 설정한다.
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .context("Melon HTTP 클라이언트 생성에 실패했습니다")?;

        Ok(Self { client })
    }

    /// 이미지 URL에서 `/melon/resize/...` 서픽스를 제거하여 원본 URL을 반환한다.
    fn strip_resize_suffix(url: &str) -> String {
        if let Some(pos) = url.find("/melon/resize/") {
            url[..pos].to_string()
        } else {
            url.to_string()
        }
    }
}

impl MusicSource for MelonClient {
    fn name(&self) -> &str {
        "Melon"
    }

    fn search(&self, query: &str) -> Result<Vec<TrackInfo>> {
        let url = format!(
            "https://www.melon.com/search/song/index.htm?q={}&section=&searchGnbYn=Y&kkoSpl=N&kkoDpType=",
            urlencoding(query)
        );

        let html = self
            .client
            .get(&url)
            .send()
            .context("Melon 검색에 실패했습니다")?
            .error_for_status()
            .context("Melon 검색 요청이 실패했습니다")?
            .text()
            .context("Melon 검색 응답 읽기에 실패했습니다")?;

        let document = Html::parse_document(&html);

        let tr_sel = Selector::parse("tr").unwrap();
        let input_sel = Selector::parse("input.input_check").unwrap();
        let title_sel = Selector::parse("a.fc_gray").unwrap();
        let artist_div_sel = Selector::parse("div#artistName a.fc_mgray").unwrap();
        let album_sel = Selector::parse("a.fc_mgray").unwrap();

        let mut results = Vec::new();

        for row in document.select(&tr_sel) {
            // 곡 ID 추출
            let song_id = match row.select(&input_sel).next() {
                Some(el) => match el.value().attr("value") {
                    Some(v) => v.to_string(),
                    None => continue,
                },
                None => continue,
            };

            // 제목 추출 (a.fc_gray의 title 속성)
            let title = match row.select(&title_sel).next() {
                Some(el) => el.value().attr("title").unwrap_or_default().to_string(),
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            // 아티스트 추출 (div#artistName 안 a.fc_mgray 텍스트)
            let artist = row
                .select(&artist_div_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // 앨범 추출 (앨범 열의 a.fc_mgray - 아티스트가 아닌 마지막 a.fc_mgray)
            let album = row
                .select(&album_sel)
                .filter(|el| {
                    // 아티스트 div 내부의 링크는 제외
                    el.value()
                        .attr("href")
                        .map(|h| h.contains("album"))
                        .unwrap_or(false)
                })
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let detail_url = format!("https://www.melon.com/song/detail.htm?songId={}", song_id);

            results.push(TrackInfo {
                title: Some(title),
                artist: if artist.is_empty() {
                    None
                } else {
                    Some(artist)
                },
                album: if album.is_empty() { None } else { Some(album) },
                album_art_url: Some(detail_url),
                source: "melon".to_string(),
                ..Default::default()
            });
        }

        Ok(results)
    }

    fn fetch_album_art(&self, track: &TrackInfo) -> Result<Vec<u8>> {
        let detail = self.fetch_detail(track)?;
        detail.album_art.context("앨범 아트를 찾을 수 없습니다")
    }

    fn fetch_detail(&self, track: &TrackInfo) -> Result<TrackInfo> {
        let url = track
            .album_art_url
            .as_ref()
            .context("상세 페이지 URL이 없습니다")?;

        let html = self
            .client
            .get(url)
            .send()
            .context("Melon 상세 페이지 로딩에 실패했습니다")?
            .error_for_status()
            .context("Melon 상세 페이지 요청이 실패했습니다")?
            .text()
            .context("Melon 상세 페이지 응답 읽기에 실패했습니다")?;

        let document = Html::parse_document(&html);
        let mut detailed = track.clone();

        // 메타데이터 파싱 (div.meta > dl.list 내의 dt/dd 쌍)
        let dt_sel = Selector::parse("div.meta dl.list dt").unwrap();
        let dd_sel = Selector::parse("div.meta dl.list dd").unwrap();

        let normalize = |s: String| s.replace('\u{a0}', " ").trim().to_string();

        let dts: Vec<String> = document
            .select(&dt_sel)
            .map(|el| normalize(el.text().collect::<String>()))
            .collect();
        let dds: Vec<String> = document
            .select(&dd_sel)
            .map(|el| normalize(el.text().collect::<String>()))
            .collect();

        for (label, value) in dts.iter().zip(dds.iter()) {
            match label.as_str() {
                "발매일" => {
                    // "2007.05.07" → 연도 2007
                    if let Some(year_str) = value.split('.').next() {
                        if let Ok(year) = year_str.parse::<i32>() {
                            detailed.year = Some(year);
                        }
                    }
                }
                "장르" => {
                    if !value.is_empty() {
                        detailed.genre = Some(value.clone());
                    }
                }
                "앨범" => {
                    if !value.is_empty() {
                        detailed.album = Some(value.clone());
                    }
                }
                _ => {}
            }
        }

        // 앨범 아트 이미지 다운로드
        let img_sel = Selector::parse("div#d_song_org img").unwrap();
        if let Some(img_url) = document
            .select(&img_sel)
            .next()
            .and_then(|el| el.value().attr("src"))
        {
            let original_url = Self::strip_resize_suffix(img_url);
            if let Ok(resp) = self.client.get(&original_url).send() {
                if let Ok(bytes) = resp.bytes() {
                    detailed.album_art = Some(bytes.to_vec());
                }
            }
        }

        Ok(detailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Melon 상세 페이지에서 메타데이터와 앨범 아트를 파싱하는 통합 테스트.
    /// 네트워크 접근이 필요하므로 기본 테스트에서는 제외한다.
    /// 실행: cargo test --features gui melon -- --ignored
    #[test]
    #[ignore]
    fn test_fetch_detail_from_melon() {
        let client = MelonClient::new().expect("MelonClient 생성 실패");

        let track = TrackInfo {
            title: Some("사랑아".to_string()),
            artist: Some("The One (더원)".to_string()),
            album_art_url: Some("https://www.melon.com/song/detail.htm?songId=1631371".to_string()),
            source: "melon".to_string(),
            ..Default::default()
        };

        let detail = client.fetch_detail(&track).expect("fetch_detail 실패");

        // 제목/아티스트는 입력값 유지
        assert_eq!(detail.title.as_deref(), Some("사랑아"));
        assert_eq!(detail.artist.as_deref(), Some("The One (더원)"));

        // 상세 페이지에서 파싱된 메타데이터
        assert_eq!(detail.album.as_deref(), Some("내 남자의 여자 OST"));
        assert_eq!(detail.year, Some(2007));
        assert_eq!(detail.genre.as_deref(), Some("국내드라마"));

        // 앨범 아트 다운로드 확인
        assert!(
            detail.album_art.is_some(),
            "앨범 아트가 다운로드되지 않았음"
        );
        let art = detail.album_art.unwrap();
        assert!(
            art.len() > 1000,
            "앨범 아트 크기가 너무 작음: {} bytes",
            art.len()
        );
        println!("앨범 아트 크기: {} bytes", art.len());
    }

    /// Melon 검색 후 첫 번째 결과의 상세 정보를 가져오는 통합 테스트.
    #[test]
    #[ignore]
    fn test_search_and_fetch_detail() {
        let client = MelonClient::new().expect("MelonClient 생성 실패");

        let results = client.search("사랑아 더원").expect("검색 실패");
        assert!(!results.is_empty(), "검색 결과가 없음");

        let first = &results[0];
        println!(
            "검색 결과: {} - {}",
            first.display_title(),
            first.display_artist()
        );

        let detail = client.fetch_detail(first).expect("fetch_detail 실패");

        println!("연도: {:?}", detail.year);
        println!("장르: {:?}", detail.genre);
        println!("앨범: {:?}", detail.album);
        println!(
            "앨범 아트: {} bytes",
            detail.album_art.as_ref().map(|a| a.len()).unwrap_or(0)
        );

        // 상세 정보가 채워졌는지 확인
        assert!(detail.year.is_some(), "연도가 없음");
        assert!(detail.genre.is_some(), "장르가 없음");
        assert!(detail.album.is_some(), "앨범이 없음");

        let genre = detail.genre.as_deref().unwrap();
        assert!(!genre.is_empty(), "장르가 빈 문자열");
        let album = detail.album.as_deref().unwrap();
        assert!(!album.is_empty(), "앨범이 빈 문자열");
    }
}

/// URL 인코딩 함수 (쿼리 문자열용).
fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }
    result
}
