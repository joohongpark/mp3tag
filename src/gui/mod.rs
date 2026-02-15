#[cfg(feature = "gui")]
mod app;

#[cfg(feature = "gui")]
pub fn launch(directory: Option<std::path::PathBuf>) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "MP3 태그 편집기",
        options,
        Box::new(move |cc| Ok(Box::new(app::Mp3TagApp::new(cc, directory)))),
    );
}
