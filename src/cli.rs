use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Table};
use dialoguer::{Input, Select};

use crate::config::{self, SpotifyConfig};
use crate::core::{parser, scanner, tagger};
use crate::models::TrackInfo;
use crate::sources::spotify::SpotifyClient;
use crate::sources::MusicSource;

#[derive(Parser)]
#[command(name = "mp3tag", about = "Spotify 연동 MP3 ID3 태그 편집기")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// GUI 모드로 실행
    #[arg(long)]
    pub gui: bool,

    /// GUI 모드에서 열 디렉토리
    #[arg(value_name = "DIRECTORY")]
    pub directory: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 디렉토리를 스캔하여 태그 현황 표시
    Scan {
        /// 스캔할 디렉토리
        directory: PathBuf,
    },
    /// 파일의 태그 편집
    Edit {
        /// 편집할 MP3 파일
        file: PathBuf,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        artist: Option<String>,
        #[arg(long)]
        album: Option<String>,
        #[arg(long, name = "album-artist")]
        album_artist: Option<String>,
        #[arg(long)]
        track: Option<u32>,
        #[arg(long)]
        year: Option<i32>,
        #[arg(long)]
        genre: Option<String>,
        #[arg(long, name = "album-art")]
        album_art: Option<PathBuf>,
    },
    /// Spotify에서 태그 가져오기
    Fetch {
        /// MP3 파일 또는 디렉토리
        path: PathBuf,
    },
    /// Spotify 자격증명 설정
    Config,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Scan { directory }) => cmd_scan(&directory),
        Some(Commands::Edit {
            file,
            title,
            artist,
            album,
            album_artist,
            track,
            year,
            genre,
            album_art,
        }) => cmd_edit(
            &file,
            title,
            artist,
            album,
            album_artist,
            track,
            year,
            genre,
            album_art,
        ),
        Some(Commands::Fetch { path }) => cmd_fetch(&path),
        Some(Commands::Config) => cmd_config(),
        None => {
            if cli.gui {
                #[cfg(feature = "gui")]
                {
                    crate::gui::launch(cli.directory);
                    Ok(())
                }
                #[cfg(not(feature = "gui"))]
                {
                    anyhow::bail!(
                        "GUI 기능이 활성화되지 않았습니다. 다시 빌드하세요: cargo build --features gui"
                    );
                }
            } else {
                println!("사용법: mp3tag <명령어> 또는 mp3tag --gui");
                println!("자세한 정보는 mp3tag --help를 실행하세요.");
                Ok(())
            }
        }
    }
}

fn cmd_scan(directory: &PathBuf) -> Result<()> {
    let files = scanner::scan_directory(directory)?;

    if files.is_empty() {
        println!("{}에서 MP3 파일을 찾을 수 없습니다", directory.display());
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["파일", "제목", "아티스트", "앨범", "태그"]);

    for file in &files {
        let tags_status = if file.has_tags { "있음" } else { "없음" };
        let (title, artist, album) = match &file.current_tags {
            Some(t) => (
                t.display_title().to_string(),
                t.display_artist().to_string(),
                t.display_album().to_string(),
            ),
            None => ("-".to_string(), "-".to_string(), "-".to_string()),
        };

        table.add_row(vec![
            Cell::new(file.filename()),
            Cell::new(&title),
            Cell::new(&artist),
            Cell::new(&album),
            Cell::new(tags_status),
        ]);
    }

    println!("{table}");
    println!(
        "\n총 {} 파일 (태그 있음: {}, 태그 없음: {})",
        files.len(),
        files.iter().filter(|f| f.has_tags).count(),
        files.iter().filter(|f| !f.has_tags).count(),
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_edit(
    file: &PathBuf,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    track: Option<u32>,
    year: Option<i32>,
    genre: Option<String>,
    album_art_path: Option<PathBuf>,
) -> Result<()> {
    let mp3 = scanner::load_single_file(file)?;

    let album_art = if let Some(ref art_path) = album_art_path {
        Some(std::fs::read(art_path).context("앨범 아트 이미지를 읽을 수 없습니다")?)
    } else {
        None
    };

    let new_info = TrackInfo {
        title,
        artist,
        album,
        album_artist,
        track_number: track,
        year,
        genre,
        album_art,
        album_art_url: None,
        source: "manual".to_string(),
    };

    let merged = tagger::merge_tags(&mp3.current_tags, &new_info);
    tagger::write_tags(file, &merged)?;

    println!("태그가 업데이트되었습니다: {}", file.display());
    Ok(())
}

fn cmd_fetch(path: &PathBuf) -> Result<()> {
    let cfg = config::load_config();

    if !cfg.spotify.is_configured() {
        println!("Spotify가 설정되지 않았습니다. 먼저 'mp3tag config'를 실행하세요.");
        return Ok(());
    }

    let client = SpotifyClient::new(&cfg.spotify)?;
    let files = scanner::scan_path(path)?;
    let targets: Vec<_> = files.into_iter().filter(|f| !f.has_tags).collect();

    if targets.is_empty() {
        println!("모든 파일에 이미 태그가 있습니다.");
        return Ok(());
    }

    println!("태그가 없는 파일 {}개를 찾았습니다.\n", targets.len());

    for file in &targets {
        println!("--- {} ---", file.filename());

        let parsed = parser::parse_filename(&file.path);
        let query = parser::build_search_query(&parsed);

        if query.is_empty() {
            println!("  파일명에서 검색어를 생성할 수 없습니다. 건너뜁니다.\n");
            continue;
        }

        println!("  검색 중: {}", query);

        let results = match client.search(&query) {
            Ok(r) => r,
            Err(e) => {
                println!("  검색 실패: {}. 건너뜁니다.\n", e);
                continue;
            }
        };

        if results.is_empty() {
            println!("  검색 결과가 없습니다. 건너뜁니다.\n");
            continue;
        }

        let items: Vec<String> = results.iter().map(|r| r.summary()).collect();
        let mut items_with_skip = items.clone();
        items_with_skip.push("이 파일 건너뛰기".to_string());

        let selection = Select::new()
            .with_prompt("  트랙을 선택하세요")
            .items(&items_with_skip)
            .default(0)
            .interact()?;

        if selection >= results.len() {
            println!("  건너뛰었습니다.\n");
            continue;
        }

        let mut track = results[selection].clone();

        // Fetch album art
        match client.fetch_album_art(&track) {
            Ok(art) => {
                track.album_art = Some(art);
                println!("  앨범 아트를 다운로드했습니다.");
            }
            Err(e) => {
                println!("  앨범 아트 다운로드 실패: {}", e);
            }
        }

        tagger::write_tags(&file.path, &track)?;
        println!("  태그가 적용되었습니다: {}\n", track.summary());
    }

    println!("완료!");
    Ok(())
}

fn cmd_config() -> Result<()> {
    let mut cfg = config::load_config();

    println!("Spotify API 설정");
    println!("(자격증명은 https://developer.spotify.com/dashboard 에서 발급받으세요)\n");

    let current_id = cfg
        .spotify
        .client_id
        .clone()
        .unwrap_or_default();

    let client_id: String = Input::new()
        .with_prompt("Client ID")
        .with_initial_text(current_id)
        .interact_text()?;

    let current_secret = cfg
        .spotify
        .client_secret
        .clone()
        .unwrap_or_default();

    let client_secret: String = Input::new()
        .with_prompt("Client Secret")
        .with_initial_text(current_secret)
        .interact_text()?;

    cfg.spotify = SpotifyConfig {
        client_id: Some(client_id),
        client_secret: Some(client_secret),
    };

    config::save_config(&cfg)?;
    println!("\n설정이 저장되었습니다!");
    Ok(())
}
