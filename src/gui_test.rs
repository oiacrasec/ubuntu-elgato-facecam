//! Opt-in developer capture of the actual egui framebuffer, for synthetic UI tests.
//! Compiled out of release builds. Create OUTPUT.request to request OUTPUT (PPM).
use eframe::egui;
use std::path::PathBuf;

pub fn capture(ctx: &egui::Context) {
    let Some(path) = std::env::var_os("FACECAM_TEST_UI_CAPTURE").map(PathBuf::from) else {
        return;
    };
    if let Some(image) = ctx.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    }) {
        let mut bytes = format!("P6\n{} {}\n255\n", image.size[0], image.size[1]).into_bytes();
        for pixel in &image.pixels {
            bytes.extend_from_slice(&pixel.to_array()[..3]);
        }
        if let Err(error) = std::fs::write(&path, bytes) {
            eprintln!("UI test capture: {error}");
        }
    }
    if std::fs::remove_file(path.with_extension("request")).is_ok() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        ctx.request_repaint();
    }
}
