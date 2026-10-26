//! Presentation rendering only. Playback and settings belong to Engine.
use crate::{
    engine::Request,
    i18n::tr as t,
    prompter::{Action, Settings, State},
};
use eframe::egui::{self, Color32, FontId};
use std::{sync::mpsc::Sender, time::Duration};

pub fn reading(ui: &mut egui::Ui, settings: &Settings, state: &State) {
    let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0, Color32::BLACK);
    if settings.text.trim().is_empty() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            t("Escreva seu roteiro ao lado"),
            FontId::proportional(20.0),
            Color32::GRAY,
        );
        return;
    }
    let width = (rect.width() * 0.84).max(1.0);
    let galley = ui.fonts_mut(|fonts| {
        fonts.layout(
            settings.text.clone(),
            FontId::proportional(settings.font_size as f32),
            Color32::WHITE,
            width,
        )
    });
    let progress = ui.ctx().animate_value_with_time(
        egui::Id::new("prompter-reading-position"),
        state.position as f32,
        0.1,
    );
    let top = rect.center().y - progress * (galley.size().y + rect.height() * 0.5);
    let position = egui::pos2(rect.center().x - width * 0.5, top);
    let painter = ui.painter().with_clip_rect(rect);
    if settings.mirror {
        // Reflect our own text mesh, including glyph geometry, without touching video.
        let shape = egui::Shape::galley(position, galley, Color32::WHITE);
        for primitive in ui.ctx().tessellate(
            vec![egui::epaint::ClippedShape {
                clip_rect: rect,
                shape,
            }],
            ui.ctx().pixels_per_point(),
        ) {
            if let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive {
                for vertex in &mut mesh.vertices {
                    vertex.pos.x = rect.left() + rect.right() - vertex.pos.x;
                }
                painter.add(egui::Shape::mesh(mesh));
            }
        }
    } else {
        painter.galley(position, galley, Color32::WHITE);
    }
    // Reading guide is local to this presentation, never part of camera output.
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.center().y),
            egui::pos2(rect.left() + 8.0, rect.center().y),
        ],
        egui::Stroke::new(2.0, Color32::GRAY),
    );
    if state.playing {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

pub fn keys(ctx: &egui::Context, state: &State, tx: &Sender<Request>) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
        let _ = tx.send(Request::ControlPrompter {
            action: if state.playing {
                Action::Pause
            } else {
                Action::Play
            },
        });
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
        let _ = tx.send(Request::ControlPrompter {
            action: Action::Rewind,
        });
    }
    let step = ctx.input(|i| {
        if i.key_pressed(egui::Key::ArrowUp) {
            -0.02
        } else if i.key_pressed(egui::Key::ArrowDown) {
            0.02
        } else {
            0.0
        }
    });
    if step != 0.0 {
        let _ = tx.send(Request::SeekPrompter {
            position: (state.position + step).clamp(0.0, 1.0),
        });
    }
}

pub fn presentation(
    ctx: &egui::Context,
    shared: &crate::engine::SharedState,
    tx: &Sender<Request>,
    settings: &Settings,
) {
    let shared = shared.clone();
    let tx = tx.clone();
    let mut builder = egui::ViewportBuilder::default()
        .with_title(t("Facecam Hub — Prompter"))
        .with_inner_size([960.0, 640.0])
        .with_fullscreen(settings.monitor.is_some());
    if let Some(index) = settings.monitor {
        builder = builder.with_monitor(index);
    }
    ctx.show_viewport_deferred(
        egui::ViewportId::from_hash_of("prompter-output"),
        builder,
        move |ctx, _| {
            let (settings, state, quit) = {
                let snapshot = shared.read().unwrap_or_else(|e| e.into_inner());
                (
                    snapshot.config.prompter.clone(),
                    snapshot.prompter.clone(),
                    snapshot.quit,
                )
            };
            if !state.presenting || quit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                let _ = tx.send(Request::ControlPrompter {
                    action: Action::Hide,
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            keys(ctx, &state, &tx);
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(Color32::BLACK))
                .show(ctx, |ui| reading(ui, &settings, &state));
            #[cfg(debug_assertions)]
            crate::gui_test::capture(ctx);
        },
    );
}
