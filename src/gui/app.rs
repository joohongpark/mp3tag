use std::path::PathBuf;
use std::sync::mpsc;

use egui::{ColorImage, TextureHandle};

use crate::config;
use crate::core::{parser, scanner, tagger};
use crate::models::{Mp3File, TrackInfo};
use crate::sources::spotify::SpotifyClient;
use crate::sources::MusicSource;

enum BgResult {
    ScanDone(Vec<Mp3File>),
    SearchDone(Vec<TrackInfo>),
    AlbumArtDone(usize, Vec<u8>),
    Error(String),
}

pub struct Mp3TagApp {
    // File list
    dir_path: String,
    files: Vec<Mp3File>,
    selected_index: Option<usize>,

    // Tag editing
    edit_title: String,
    edit_artist: String,
    edit_album: String,
    edit_album_artist: String,
    edit_track: String,
    edit_year: String,
    edit_genre: String,

    // Search
    search_query: String,
    search_results: Vec<TrackInfo>,
    selected_result: Option<usize>,

    // Album art
    album_art_texture: Option<TextureHandle>,
    result_art_textures: Vec<Option<TextureHandle>>,

    // Background tasks
    tx: mpsc::Sender<BgResult>,
    rx: mpsc::Receiver<BgResult>,
    is_loading: bool,
    status_msg: String,
}

impl Mp3TagApp {
    pub fn new(cc: &eframe::CreationContext<'_>, directory: Option<PathBuf>) -> Self {
        Self::setup_korean_fonts(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();

        let dir_path = directory
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let mut app = Self {
            dir_path,
            files: Vec::new(),
            selected_index: None,
            edit_title: String::new(),
            edit_artist: String::new(),
            edit_album: String::new(),
            edit_album_artist: String::new(),
            edit_track: String::new(),
            edit_year: String::new(),
            edit_genre: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            selected_result: None,
            album_art_texture: None,
            result_art_textures: Vec::new(),
            tx,
            rx,
            is_loading: false,
            status_msg: String::new(),
        };

        if directory.is_some() {
            app.start_scan();
        }

        app
    }

    fn setup_korean_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // macOS 시스템 한글 폰트 경로들
        let font_paths = [
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
            // Linux
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        ];

        for path in &font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "korean_font".to_string(),
                    egui::FontData::from_owned(font_data),
                );

                // 기본 폰트 패밀리에 한글 폰트 추가 (기존 폰트 뒤에)
                if let Some(family) = fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                {
                    family.push("korean_font".to_string());
                }
                if let Some(family) = fonts
                    .families
                    .get_mut(&egui::FontFamily::Monospace)
                {
                    family.push("korean_font".to_string());
                }

                ctx.set_fonts(fonts);
                return;
            }
        }
    }

    fn start_scan(&mut self) {
        let dir = PathBuf::from(&self.dir_path);
        let tx = self.tx.clone();
        self.is_loading = true;
        self.status_msg = "스캔 중...".to_string();

        std::thread::spawn(move || {
            match scanner::scan_directory(&dir) {
                Ok(files) => {
                    let _ = tx.send(BgResult::ScanDone(files));
                }
                Err(e) => {
                    let _ = tx.send(BgResult::Error(format!("스캔 실패: {}", e)));
                }
            }
        });
    }

    fn start_search(&mut self) {
        let query = self.search_query.clone();
        let tx = self.tx.clone();
        let cfg = config::load_config();
        self.is_loading = true;
        self.status_msg = "검색 중...".to_string();

        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<Vec<TrackInfo>> {
                let client = SpotifyClient::new(&cfg.spotify)?;
                client.search(&query)
            })();

            match result {
                Ok(tracks) => {
                    let _ = tx.send(BgResult::SearchDone(tracks));
                }
                Err(e) => {
                    let _ = tx.send(BgResult::Error(format!("검색 실패: {}", e)));
                }
            }
        });
    }

    fn fetch_result_art(&self, index: usize, track: &TrackInfo) {
        let tx = self.tx.clone();
        let track = track.clone();
        let cfg = config::load_config();

        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<Vec<u8>> {
                let client = SpotifyClient::new(&cfg.spotify)?;
                client.fetch_album_art(&track)
            })();

            match result {
                Ok(data) => {
                    let _ = tx.send(BgResult::AlbumArtDone(index, data));
                }
                Err(e) => {
                    let _ = tx.send(BgResult::Error(format!("앨범 아트 실패: {}", e)));
                }
            }
        });
    }

    fn load_edit_fields(&mut self) {
        if let Some(idx) = self.selected_index {
            if let Some(file) = self.files.get(idx) {
                if let Some(ref tags) = file.current_tags {
                    self.edit_title = tags.title.clone().unwrap_or_default();
                    self.edit_artist = tags.artist.clone().unwrap_or_default();
                    self.edit_album = tags.album.clone().unwrap_or_default();
                    self.edit_album_artist = tags.album_artist.clone().unwrap_or_default();
                    self.edit_track = tags
                        .track_number
                        .map(|n| n.to_string())
                        .unwrap_or_default();
                    self.edit_year = tags.year.map(|y| y.to_string()).unwrap_or_default();
                    self.edit_genre = tags.genre.clone().unwrap_or_default();

                    // Build search query from current tags
                    let query = parser::build_search_query(tags);
                    if !query.is_empty() {
                        self.search_query = query;
                    }
                    return;
                }
                // No tags — parse filename for search query
                let parsed = parser::parse_filename(&file.path);
                self.search_query = parser::build_search_query(&parsed);
                self.edit_title = parsed.title.unwrap_or_default();
                self.edit_artist = parsed.artist.unwrap_or_default();
                self.edit_album.clear();
                self.edit_album_artist.clear();
                self.edit_track.clear();
                self.edit_year.clear();
                self.edit_genre.clear();
                return;
            }
        }
        self.clear_edit_fields();
    }

    fn clear_edit_fields(&mut self) {
        self.edit_title.clear();
        self.edit_artist.clear();
        self.edit_album.clear();
        self.edit_album_artist.clear();
        self.edit_track.clear();
        self.edit_year.clear();
        self.edit_genre.clear();
        self.search_query.clear();
    }

    fn save_current_tags(&mut self) {
        let Some(idx) = self.selected_index else {
            return;
        };
        let Some(file) = self.files.get_mut(idx) else {
            return;
        };

        let info = TrackInfo {
            title: non_empty(&self.edit_title),
            artist: non_empty(&self.edit_artist),
            album: non_empty(&self.edit_album),
            album_artist: non_empty(&self.edit_album_artist),
            track_number: self.edit_track.parse().ok(),
            year: self.edit_year.parse().ok(),
            genre: non_empty(&self.edit_genre),
            album_art: file.current_tags.as_ref().and_then(|t| t.album_art.clone()),
            album_art_url: None,
            source: "manual".to_string(),
        };

        match tagger::write_tags(&file.path, &info) {
            Ok(_) => {
                file.current_tags = Some(info);
                file.has_tags = true;
                self.status_msg = "태그가 저장되었습니다!".to_string();
            }
            Err(e) => {
                self.status_msg = format!("저장 실패: {}", e);
            }
        }
    }

    fn apply_search_result(&mut self, result_idx: usize) {
        let Some(file_idx) = self.selected_index else {
            return;
        };

        let track = match self.search_results.get(result_idx) {
            Some(t) => t.clone(),
            None => return,
        };

        self.edit_title = track.title.clone().unwrap_or_default();
        self.edit_artist = track.artist.clone().unwrap_or_default();
        self.edit_album = track.album.clone().unwrap_or_default();
        self.edit_album_artist = track.album_artist.clone().unwrap_or_default();
        self.edit_track = track
            .track_number
            .map(|n| n.to_string())
            .unwrap_or_default();
        self.edit_year = track.year.map(|y| y.to_string()).unwrap_or_default();
        self.edit_genre = track.genre.clone().unwrap_or_default();

        // Write tags including album art if we have it
        if let Some(file) = self.files.get_mut(file_idx) {
            match tagger::write_tags(&file.path, &track) {
                Ok(_) => {
                    file.current_tags = Some(track);
                    file.has_tags = true;
                    self.status_msg = "Spotify에서 태그가 적용되었습니다!".to_string();
                }
                Err(e) => {
                    self.status_msg = format!("적용 실패: {}", e);
                }
            }
        }
    }

    fn load_album_art_texture(&mut self, ctx: &egui::Context) {
        self.album_art_texture = None;

        let art_data = self
            .selected_index
            .and_then(|idx| self.files.get(idx))
            .and_then(|f| f.current_tags.as_ref())
            .and_then(|t| t.album_art.as_ref());

        if let Some(data) = art_data {
            if let Ok(img) = image::load_from_memory(data) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                self.album_art_texture =
                    Some(ctx.load_texture("album_art", color_image, Default::default()));
            }
        }
    }

    fn process_bg_results(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                BgResult::ScanDone(files) => {
                    self.files = files;
                    self.selected_index = None;
                    self.is_loading = false;
                    self.status_msg = format!("MP3 파일 {}개를 찾았습니다", self.files.len());
                }
                BgResult::SearchDone(results) => {
                    // Fetch album art for each result
                    for (i, track) in results.iter().enumerate() {
                        if track.album_art_url.is_some() {
                            self.fetch_result_art(i, track);
                        }
                    }
                    self.result_art_textures = vec![None; results.len()];
                    self.search_results = results;
                    self.selected_result = None;
                    self.is_loading = false;
                    self.status_msg =
                        format!("검색 결과 {}건", self.search_results.len());
                }
                BgResult::AlbumArtDone(index, data) => {
                    // Store art in search result
                    if let Some(track) = self.search_results.get_mut(index) {
                        track.album_art = Some(data.clone());
                    }
                    // Create texture
                    if let Ok(img) = image::load_from_memory(&data) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_image =
                            ColorImage::from_rgba_unmultiplied(size, &pixels);
                        let texture = ctx.load_texture(
                            format!("result_art_{}", index),
                            color_image,
                            Default::default(),
                        );
                        if index < self.result_art_textures.len() {
                            self.result_art_textures[index] = Some(texture);
                        }
                    }
                }
                BgResult::Error(msg) => {
                    self.is_loading = false;
                    self.status_msg = msg;
                }
            }
        }
    }
}

impl eframe::App for Mp3TagApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_bg_results(ctx);

        // Top panel: directory input
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("디렉토리:");
                let response = ui.text_edit_singleline(&mut self.dir_path);
                if ui.button("폴더 열기").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.dir_path = folder.display().to_string();
                        self.start_scan();
                    }
                }
                if ui.button("스캔").clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    self.start_scan();
                }
                if self.is_loading {
                    ui.spinner();
                }
                ui.label(&self.status_msg);
            });
        });

        // Left panel: file list
        egui::SidePanel::left("file_panel")
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("파일 목록");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut new_selection = None;
                    for (i, file) in self.files.iter().enumerate() {
                        let label = if file.has_tags {
                            format!("[T] {}", file.filename())
                        } else {
                            format!("[ ] {}", file.filename())
                        };

                        let is_selected = self.selected_index == Some(i);
                        if ui.selectable_label(is_selected, &label).clicked() {
                            new_selection = Some(i);
                        }
                    }

                    if let Some(idx) = new_selection {
                        self.selected_index = Some(idx);
                        self.load_edit_fields();
                        self.load_album_art_texture(ctx);
                        self.search_results.clear();
                        self.result_art_textures.clear();
                    }
                });
            });

        // Central panel: tag editor + search
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_index.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("태그를 편집할 파일을 선택하세요");
                });
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Tag editor section
                ui.heading("태그 편집기");
                ui.separator();

                egui::Grid::new("tag_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("제목:");
                        ui.text_edit_singleline(&mut self.edit_title);
                        ui.end_row();

                        ui.label("아티스트:");
                        ui.text_edit_singleline(&mut self.edit_artist);
                        ui.end_row();

                        ui.label("앨범:");
                        ui.text_edit_singleline(&mut self.edit_album);
                        ui.end_row();

                        ui.label("앨범 아티스트:");
                        ui.text_edit_singleline(&mut self.edit_album_artist);
                        ui.end_row();

                        ui.label("트랙 번호:");
                        ui.text_edit_singleline(&mut self.edit_track);
                        ui.end_row();

                        ui.label("연도:");
                        ui.text_edit_singleline(&mut self.edit_year);
                        ui.end_row();

                        ui.label("장르:");
                        ui.text_edit_singleline(&mut self.edit_genre);
                        ui.end_row();
                    });

                ui.horizontal(|ui| {
                    if ui.button("태그 저장").clicked() {
                        self.save_current_tags();
                        self.load_album_art_texture(ctx);
                    }
                });

                // Album art preview
                if let Some(ref texture) = self.album_art_texture {
                    ui.separator();
                    ui.label("현재 앨범 아트:");
                    let size = texture.size_vec2();
                    let scale = (150.0 / size.x).min(150.0 / size.y).min(1.0);
                    ui.image(egui::load::SizedTexture::new(
                        texture.id(),
                        size * scale,
                    ));
                }

                ui.add_space(20.0);
                ui.separator();

                // Search section
                ui.heading("Spotify 검색");
                ui.horizontal(|ui| {
                    ui.label("검색어:");
                    let response = ui.text_edit_singleline(&mut self.search_query);
                    if ui.button("검색").clicked()
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.start_search();
                    }
                });

                if !self.search_results.is_empty() {
                    ui.separator();
                    let mut apply_idx = None;

                    for (i, result) in self.search_results.iter().enumerate() {
                        ui.horizontal(|ui| {
                            // Album art thumbnail
                            if let Some(Some(texture)) = self.result_art_textures.get(i) {
                                let size = texture.size_vec2();
                                let scale = (48.0 / size.x).min(48.0 / size.y).min(1.0);
                                ui.image(egui::load::SizedTexture::new(
                                    texture.id(),
                                    size * scale,
                                ));
                            } else {
                                ui.allocate_space(egui::vec2(48.0, 48.0));
                            }

                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(result.display_title()).strong(),
                                );
                                ui.label(format!(
                                    "{} - {}",
                                    result.display_artist(),
                                    result.display_album()
                                ));
                                if let Some(year) = result.year {
                                    ui.label(format!("연도: {}", year));
                                }
                            });

                            if ui.button("적용").clicked() {
                                apply_idx = Some(i);
                            }
                        });
                        ui.separator();
                    }

                    if let Some(idx) = apply_idx {
                        self.apply_search_result(idx);
                        self.load_album_art_texture(ctx);
                    }
                }
            });
        });
    }
}

fn non_empty(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
