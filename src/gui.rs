use crate::{
    camera::Control,
    config::Config,
    engine::{Request, Snapshot},
    ipc::Controller,
    stream::{PREVIEW_HEIGHT, PREVIEW_WIDTH},
};
use anyhow::Result;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::{collections::HashMap, path::PathBuf, time::Duration};

// Camera Hub layout: compact control column on the left, preview on the right.
const CANVAS: Color32 = Color32::from_rgb(18, 18, 18);
const PANEL: Color32 = Color32::from_rgb(39, 39, 39);
const HEADER: Color32 = Color32::from_rgb(32, 32, 32);
const ACCENT: Color32 = Color32::from_rgb(30, 80, 255);
const TEXT: Color32 = Color32::from_rgb(230, 230, 230);
const MUTED: Color32 = Color32::from_rgb(166, 166, 170);

pub fn run(path: PathBuf, minimized: bool, resume_stream: bool) -> Result<()> {
    let controller = Controller::spawn(&path, true)?;
    if resume_stream {
        let _ = controller.tx.send(Request::Start { preview: true });
    }
    let tx = controller.tx.clone();
    ctrlc::set_handler(move || {
        let _ = tx.send(Request::Quit);
    })?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Facecam Hub")
            .with_inner_size([1240.0, 790.0])
            .with_min_inner_size([960.0, 640.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Facecam Hub",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, controller, minimized)))),
    )
    .map_err(|e| anyhow::anyhow!("Interface: {e}"))
}

struct App {
    controller: Controller,
    config: Config,
    applied_effects: crate::effects::Effects,
    texture: Option<egui::TextureHandle>,
    pending: HashMap<String, i64>,
    grid: bool,
    mirror: bool,
    effects: bool,
    settings: bool,
    hidden: bool,
    show_revision: u64,
    #[cfg(feature = "tray")]
    tray: Option<ksni::blocking::Handle<crate::tray::Tray>>,
    #[cfg(feature = "tray")]
    tray_streaming: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, controller: Controller, minimized: bool) -> Self {
        let repaint = cc.egui_ctx.clone();
        *controller.wake.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(Box::new(move || repaint.request_repaint()));
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = CANVAS;
        style.visuals.window_fill = PANEL;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = ACCENT;
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(22, 22, 22);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(62, 62, 65);
        style.visuals.widgets.active.bg_fill = ACCENT;
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(10.0, 6.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
        cc.egui_ctx.set_global_style(style);
        let config = controller
            .shared
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .config
            .clone();
        #[cfg(feature = "tray")]
        let tray = match crate::tray::spawn(controller.tx.clone()) {
            Ok(tray) => Some(tray),
            Err(e) => {
                eprintln!("Bandeja indisponível; a janela continuará acessível: {e}");
                None
            }
        };
        #[cfg(feature = "tray")]
        let hidden = (minimized || config.ui.start_minimized) && tray.is_some();
        #[cfg(not(feature = "tray"))]
        let hidden = {
            let _ = minimized;
            false
        };
        if hidden {
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        Self {
            controller,
            applied_effects: config.effects.clone(),
            config,
            texture: None,
            pending: HashMap::new(),
            grid: false,
            mirror: false,
            effects: false,
            settings: false,
            hidden,
            show_revision: 0,
            #[cfg(feature = "tray")]
            tray,
            #[cfg(feature = "tray")]
            tray_streaming: false,
        }
    }

    fn dispatch(&self, request: Request) {
        let _ = self.controller.tx.send(request);
    }

    fn state(&self) -> Snapshot {
        self.controller
            .shared
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn configure(&self) {
        self.dispatch(Request::Configure {
            config: Box::new(self.config.clone()),
            save: false,
        });
    }

    fn control(&mut self, ui: &mut egui::Ui, controls: &[Control], name: &str, label: &str) {
        let Some(control) = controls.iter().find(|c| c.name == name) else {
            return;
        };
        let mut value = self.pending.get(name).copied().unwrap_or(control.value);
        let mut commit = false;
        ui.add_enabled_ui(control.writable(), |ui| {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(104.0, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(104.0);
                        ui.label(RichText::new(label).size(12.0).color(MUTED));
                    },
                );
                if control.kind == "bool" {
                    let mut on = value != 0;
                    if ui.checkbox(&mut on, "").changed() {
                        value = i64::from(on);
                        commit = true;
                    }
                } else if !control.menu.is_empty() {
                    egui::ComboBox::from_id_salt(name)
                        .width(155.0)
                        .selected_text(control.menu.get(&value).map(String::as_str).unwrap_or("—"))
                        .show_ui(ui, |ui| {
                            for (v, text) in &control.menu {
                                if ui.selectable_value(&mut value, *v, text).changed() {
                                    commit = true;
                                }
                            }
                        });
                } else {
                    ui.spacing_mut().slider_width = 99.0;
                    let response = ui.add(
                        egui::Slider::new(&mut value, control.min..=control.max)
                            .step_by(control.step as f64)
                            .show_value(false),
                    );
                    if response.changed() {
                        self.pending.insert(name.into(), value);
                    }
                    commit |=
                        response.drag_stopped() || (response.changed() && !response.dragged());
                    let response = ui.add(
                        egui::DragValue::new(&mut value)
                            .range(control.min..=control.max)
                            .speed(control.step as f64),
                    );
                    if response.changed() {
                        self.pending.insert(name.into(), value);
                    }
                    commit |=
                        response.drag_stopped() || (response.changed() && !response.dragged());
                }
            });
        });
        if let Some(reason) = &control.write_error {
            ui.label(
                RichText::new("Indisponível nesta configuração")
                    .size(10.0)
                    .color(MUTED),
            )
            .on_hover_text(reason);
        }
        if commit {
            let value = control.min + ((value - control.min) / control.step) * control.step;
            self.pending.remove(name);
            self.dispatch(Request::SetControl {
                name: name.into(),
                value,
            });
        }
    }

    fn section(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        controls: &[Control],
        rows: &[(&str, &str)],
    ) {
        if !rows
            .iter()
            .any(|(name, _)| controls.iter().any(|c| c.name == *name))
        {
            return;
        }
        egui::Frame::new()
            .fill(PANEL)
            .corner_radius(3)
            .inner_margin(10)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                egui::CollapsingHeader::new(RichText::new(title).size(13.0).strong())
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.add_space(5.0);
                        for (name, label) in rows {
                            self.control(ui, controls, name, label);
                        }
                    });
            });
        ui.add_space(3.0);
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, state: &Snapshot) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.effects, false, "Camera");
            ui.selectable_value(&mut self.effects, true, "Effects");
            ui.add_enabled(false, egui::Button::new("Prompter"));
        });
        ui.add_space(3.0);
        if self.effects {
            self.effects_panel(ui, state);
            return;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().fill(PANEL).corner_radius(3).inner_margin(10).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Device");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("SAVE").fill(ACCENT)).on_hover_text("Salvar configuração e ajustes neste computador").clicked() { self.dispatch(Request::Save); }
                        if ui.small_button("R").on_hover_text("Restaurar ajustes salvos neste computador").clicked() { self.dispatch(Request::Restore); }
                        if ui.small_button("⟳").on_hover_text("Atualizar dispositivos e ajustes").clicked() { self.dispatch(Request::Refresh); }
                    });
                });
                ui.separator();
                ui.add_enabled_ui(!state.streaming && !state.preview, |ui| {
                    let select_width = ui.available_width() - 48.0 - ui.spacing().item_spacing.x;
                    egui::Grid::new("device-fields").num_columns(2).min_col_width(0.0).show(ui, |ui| {
                        ui.allocate_ui_with_layout(Vec2::new(48.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Input").color(MUTED));
                        });
                        let text = state.camera.as_ref().map(|c| c.name.as_str()).unwrap_or("Conecte uma Facecam");
                        egui::ComboBox::from_id_salt("device").width(select_width).truncate().selected_text(text).show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.config.camera_device, String::new(), "Automático").changed() { self.configure(); }
                            for camera in &state.cameras {
                                if ui.selectable_value(&mut self.config.camera_device, camera.path.clone(), &camera.name).changed() { self.configure(); }
                            }
                        });
                        ui.end_row();
                        ui.allocate_ui_with_layout(Vec2::new(48.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Format").color(MUTED));
                        });
                        egui::ComboBox::from_id_salt("format").width(select_width).truncate()
                            .selected_text(state.mode.as_ref().map(|m| m.label()).unwrap_or_else(|| "Indisponível".into()))
                            .show_ui(ui, |ui| { for mode in &state.modes {
                                if ui.selectable_label(state.mode.as_ref() == Some(mode), mode.label()).clicked() {
                                    self.config.ffmpeg_params.video_size = format!("{}x{}", mode.width, mode.height);
                                    self.config.ffmpeg_params.framerate = mode.fps.round() as u32;
                                    self.config.ffmpeg_params.input_format = mode.format.clone();
                                    self.configure();
                                }
                            } });
                        ui.end_row();
                    });
                });
            });
            ui.add_space(5.0);
            self.section(ui, "Frame", &state.controls, &[("zoom_absolute", "Zoom / FOV"), ("pan_absolute", "Pan"), ("tilt_absolute", "Tilt")]);
            self.section(ui, "Picture", &state.controls, &[("contrast", "Contrast"), ("saturation", "Saturation"), ("sharpness", "Sharpness"), ("brightness", "Brightness")]);
            self.section(ui, "Exposure", &state.controls, &[("auto_exposure", "Automatic"), ("exposure_time_absolute", "Exposure¹"), ("gain", "Gain²"), ("backlight_compensation", "Backlight"), ("exposure_dynamic_framerate", "Dynamic FPS")]);
            self.section(ui, "White Balance", &state.controls, &[("white_balance_automatic", "Automatic"), ("white_balance_temperature", "Temperature K")]);
            self.section(ui, "Processing", &state.controls, &[("power_line_frequency", "Anti-flicker"), ("privacy", "Privacy")]);
            ui.add_space(8.0);
            ui.label(RichText::new("¹ Exposição em unidades de 100 µs.\n² Ganho do driver; não representa ISO.").size(10.0).color(MUTED));
        });
    }

    fn effects_panel(&mut self, ui: &mut egui::Ui, state: &Snapshot) {
        if state.config.effects != self.applied_effects {
            self.applied_effects = state.config.effects.clone();
            self.config.effects = self.applied_effects.clone();
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().fill(PANEL).corner_radius(3).inner_margin(10).show(ui, |ui| {
                let effects = &mut self.config.effects;
                ui.strong("Color & LUT");
                ui.checkbox(&mut effects.mirror, "Espelhar também a saída virtual");
                ui.add(egui::Slider::new(&mut effects.brightness, -1.0..=1.0).text("Brightness"));
                ui.add(egui::Slider::new(&mut effects.contrast, 0.0..=2.0).text("Contrast"));
                ui.add(egui::Slider::new(&mut effects.saturation, 0.0..=3.0).text("Saturation"));
                ui.add(egui::Slider::new(&mut effects.denoise, 0.0..=10.0).text("Denoise"));
                ui.label("LUT 3D (.cube) — caminho completo");
                ui.text_edit_singleline(&mut effects.lut);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong("AI Background");
                    if ui.small_button("Detectar arquivos").on_hover_text("Preencher caminhos vazios com arquivos de IA disponíveis neste computador").clicked() {
                        crate::ai_paths::autofill(effects);
                    }
                });
                ui.add_enabled_ui(state.ai_compiled, |ui| {
                    ui.checkbox(&mut effects.background_blur, "Desfocar fundo com IA");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut effects.ai_provider, crate::effects::AiProvider::Cuda, "NVIDIA CUDA");
                        ui.selectable_value(&mut effects.ai_provider, crate::effects::AiProvider::Cpu, "CPU");
                    });
                    ui.add(egui::Slider::new(&mut effects.blur_radius, 1..=32).text("Blur"));
                    ui.label("Modelo de segmentação (.onnx)");
                    ui.text_edit_singleline(&mut effects.ai_model);
                    ui.label("Runtime (libonnxruntime.so)");
                    ui.text_edit_singleline(&mut effects.ai_runtime);
                });
                if !state.ai_compiled { ui.label("Compile com --features ai para habilitar IA."); }
                ui.label(RichText::new("CUDA exige ONNX Runtime GPU, CUDA e cuDNN. O modelo e o runtime são carregados somente ao ativar IA.").size(10.0).color(MUTED));
                ui.separator();
                if ui.button("Restaurar efeitos padrão").clicked() { self.config.effects = Default::default(); }
                if ui.add(egui::Button::new("Aplicar e salvar").fill(ACCENT)).clicked() {
                    self.dispatch(Request::SetEffects { effects: self.config.effects.clone() });
                }
                ui.label(RichText::new("Aplicar reinicia brevemente a captura. Os efeitos são enviados à câmera virtual; a grade do preview é apenas local.").size(10.0).color(MUTED));
            });
        });
    }

    fn preview(&mut self, ui: &mut egui::Ui, state: &Snapshot) {
        let available = ui.available_size();
        let height = (available.y - 62.0).max(100.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(available.x, height), egui::Sense::hover());
        let width = rect.width().min(rect.height() * 16.0 / 9.0);
        let image_rect =
            egui::Rect::from_center_size(rect.center(), Vec2::new(width, width * 9.0 / 16.0));
        if let Some(texture) = &self.texture {
            let uv = if self.mirror {
                egui::Rect::from_min_max(egui::pos2(1.0, 0.0), egui::pos2(0.0, 1.0))
            } else {
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))
            };
            ui.painter()
                .image(texture.id(), image_rect, uv, Color32::WHITE);
            if self.grid {
                for n in [1.0_f32, 2.0] {
                    let x = image_rect.left() + image_rect.width() * n / 3.0;
                    let y = image_rect.top() + image_rect.height() * n / 3.0;
                    let stroke = Stroke::new(1.0, Color32::from_white_alpha(120));
                    ui.painter().line_segment(
                        [
                            egui::pos2(x, image_rect.top()),
                            egui::pos2(x, image_rect.bottom()),
                        ],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [
                            egui::pos2(image_rect.left(), y),
                            egui::pos2(image_rect.right(), y),
                        ],
                        stroke,
                    );
                }
            }
        } else {
            ui.painter()
                .rect_filled(image_rect, 2, Color32::from_rgb(23, 23, 23));
            let center = image_rect.center();
            ui.painter().text(
                center - Vec2::new(0.0, 30.0),
                egui::Align2::CENTER_CENTER,
                if state.camera.is_some() {
                    "Preview pausado"
                } else {
                    "Conecte uma Facecam"
                },
                egui::FontId::proportional(20.0),
                TEXT,
            );
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "Controle a câmera. Veja cada ajuste.",
                egui::FontId::proportional(13.0),
                MUTED,
            );
            let button =
                egui::Rect::from_center_size(center + Vec2::new(0.0, 40.0), Vec2::new(150.0, 32.0));
            ui.painter().rect_filled(
                button,
                3,
                if state.camera.is_some() {
                    ACCENT
                } else {
                    HEADER
                },
            );
            ui.painter().text(
                button.center(),
                egui::Align2::CENTER_CENTER,
                "Iniciar preview",
                egui::FontId::proportional(13.0),
                TEXT,
            );
            if ui
                .interact(button, ui.id().with("start-preview"), egui::Sense::click())
                .clicked()
                && state.camera.is_some()
            {
                self.dispatch(Request::Preview);
            }
        }
        egui::Frame::new()
            .fill(PANEL)
            .corner_radius(3)
            .inner_margin(9)
            .show(ui, |ui| {
                ui.set_min_width((available.x - 18.0).max(0.0));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Preview Format").size(12.0));
                    ui.label(
                        RichText::new(
                            state
                                .mode
                                .as_ref()
                                .map(|m| format!("{}p{}", m.height, m.fps))
                                .unwrap_or_else(|| "—".into()),
                        )
                        .size(12.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.toggle_value(&mut self.mirror, "Mirror")
                            .on_hover_text("Espelhar apenas o preview");
                        ui.toggle_value(&mut self.grid, "Grid")
                            .on_hover_text("Grade de enquadramento");
                        if ui
                            .selectable_label(state.preview, "View")
                            .on_hover_text("Mostrar / pausar preview")
                            .clicked()
                        {
                            self.texture = None;
                            self.dispatch(if state.streaming {
                                Request::Start {
                                    preview: !state.preview,
                                }
                            } else if state.preview {
                                Request::Stop
                            } else {
                                Request::Preview
                            });
                        }
                        if ui
                            .add_enabled(
                                state.camera.is_some(),
                                egui::Button::new(if state.streaming {
                                    "Parar câmera virtual"
                                } else {
                                    "Iniciar câmera virtual"
                                })
                                .fill(if state.streaming {
                                    HEADER
                                } else {
                                    ACCENT
                                }),
                            )
                            .clicked()
                        {
                            self.dispatch(if state.streaming {
                                Request::Stop
                            } else {
                                Request::Start { preview: true }
                            });
                        }
                    });
                });
            });
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let state = self.state();
        self.config.saved_controls = state.config.saved_controls.clone();
        if state.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if state.show_revision != self.show_revision {
            self.show_revision = state.show_revision;
            self.hidden = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !state.quit {
            #[cfg(feature = "tray")]
            if self.tray.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.hidden = true;
                self.texture = None;
                if state.streaming {
                    self.dispatch(Request::Start { preview: false });
                } else {
                    self.dispatch(Request::Stop);
                }
            }
        }
        #[cfg(feature = "tray")]
        if self.tray_streaming != state.streaming {
            if let Some(tray) = &self.tray {
                tray.update(|tray| tray.streaming = state.streaming);
            }
            self.tray_streaming = state.streaming;
        }
        if !self.hidden {
            if let Some(bytes) = self
                .controller
                .frames
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
            {
                let image = egui::ColorImage::from_rgb([PREVIEW_WIDTH, PREVIEW_HEIGHT], &bytes);
                if let Some(texture) = &mut self.texture {
                    texture.set(image, egui::TextureOptions::LINEAR);
                } else {
                    self.texture = Some(ctx.load_texture(
                        "camera-preview",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
            if !state.preview {
                self.texture = None;
            }
        }
        ctx.request_repaint_after(if state.preview && !self.hidden {
            Duration::from_millis(67)
        } else {
            Duration::from_secs(5)
        });
    }

    fn ui(&mut self, root: &mut egui::Ui, _: &mut eframe::Frame) {
        let state = self.state();
        let ctx = root.ctx().clone();
        egui::Panel::top("title")
            .frame(egui::Frame::new().fill(CANVAS).inner_margin(14))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Facecam Hub").size(18.0).strong());
                    ui.label(RichText::new("Linux").size(11.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⚙").on_hover_text("Configurações").clicked() {
                            self.settings = !self.settings;
                        }
                        if state.streaming {
                            ui.colored_label(
                                Color32::from_rgb(110, 200, 150),
                                "● Câmera virtual ativa",
                            );
                        }
                    });
                });
            });
        egui::Panel::bottom("status")
            .frame(egui::Frame::new().fill(HEADER).inner_margin(8))
            .show(root, |ui| {
                if let Some(error) = &state.error {
                    ui.colored_label(Color32::from_rgb(255, 170, 150), error);
                } else {
                    ui.label(RichText::new(&state.message).size(11.0).color(MUTED));
                }
            });
        egui::Panel::left("camera-controls")
            .exact_size(330.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(CANVAS).inner_margin(8))
            .show(root, |ui| self.sidebar(ui, &state));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(CANVAS).inner_margin(14))
            .show(root, |ui| self.preview(ui, &state));
        let mut open = self.settings;
        egui::Window::new("Configurações")
            .open(&mut open)
            .resizable(false)
            .show(&ctx, |ui| {
                ui.add_enabled_ui(!state.streaming && !state.preview, |ui| {
                    ui.label("Dispositivo da câmera virtual");
                    ui.text_edit_singleline(&mut self.config.virtual_device);
                    ui.checkbox(&mut self.config.ui.start_minimized, "Iniciar na bandeja");
                    if ui.button("Salvar").clicked() {
                        self.dispatch(Request::Configure {
                            config: Box::new(self.config.clone()),
                            save: true,
                        });
                    }
                });
                ui.label("Pare a captura para alterar o formato ou o dispositivo.");
                if ui.button("Iniciar ao entrar no desktop").clicked()
                    && let Err(e) = crate::desktop::install_autostart()
                {
                    eprintln!("{e}");
                }
                ui.separator();
                if ui.button("Encerrar aplicativo").clicked() {
                    self.dispatch(Request::Quit);
                }
            });
        self.settings = open;
    }
}
