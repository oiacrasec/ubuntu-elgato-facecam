use crate::i18n::tr as t;
use crate::{
    camera::Control,
    config::Config,
    engine::{Request, Snapshot},
    ipc::Controller,
    stream::output_fps,
};
use anyhow::Result;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::{collections::HashMap, path::PathBuf, time::Duration};

use crate::resources::Kind as FileTarget;

type FileSelection = (FileTarget, Option<PathBuf>);

struct Thumbnail {
    path: PathBuf,
    receiver: std::sync::mpsc::Receiver<Result<Vec<u8>, String>>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
}

fn resource_selector(ui: &mut egui::Ui, state: &Snapshot, kind: FileTarget, selected: &mut String) {
    let name = state
        .resources
        .iter()
        .find(|r| r.path.to_str() == Some(selected.as_str()))
        .map(resource_name);
    let label = name.unwrap_or_else(|| {
        t(if selected.is_empty() {
            if kind == FileTarget::Lut {
                "Original"
            } else {
                "Escolha um fundo"
            }
        } else {
            "Recurso selecionado"
        })
    });
    egui::ComboBox::from_id_salt(("resource", kind as u8))
        .width(ui.available_width())
        .truncate()
        .selected_text(&label)
        .show_ui(ui, |ui| {
            if kind == FileTarget::Lut {
                ui.selectable_value(selected, String::new(), t("Original"));
            }
            for resource in state.resources.iter().filter(|r| r.kind == kind) {
                if let Some(path) = resource.path.to_str() {
                    ui.selectable_value(selected, path.to_owned(), resource_name(resource));
                }
            }
        })
        .response
        .on_hover_text(label);
}

fn resource_name(resource: &crate::resources::Resource) -> String {
    if resource.kind == FileTarget::Lut
        && resource
            .path
            .parent()
            .is_some_and(|p| p.ends_with("builtins-v1"))
    {
        t(&resource.name)
    } else {
        resource.name.clone()
    }
}

fn effect_row(ui: &mut egui::Ui, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(104.0, 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(104.0);
                ui.label(RichText::new(t(label)).size(12.0).color(MUTED));
            },
        );
        content(ui);
    });
}

fn effect_slider<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut N,
    range: std::ops::RangeInclusive<N>,
    step: f64,
) {
    effect_row(ui, label, |ui| {
        ui.spacing_mut().slider_width = 99.0;
        ui.add(
            egui::Slider::new(value, range.clone())
                .step_by(step)
                .show_value(false),
        );
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(step)
                .max_decimals(if step < 1.0 { 2 } else { 0 }),
        );
    });
}

fn effect_card(ui: &mut egui::Ui, title: &str, open: bool, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(3)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::CollapsingHeader::new(RichText::new(t(title)).size(13.0).strong())
                .id_salt(("effects", title))
                .default_open(open)
                .show(ui, |ui| {
                    ui.add_space(5.0);
                    content(ui);
                });
        });
    ui.add_space(3.0);
}

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
    effects: bool,
    settings: bool,
    hidden: bool,
    show_revision: u64,
    file_dialog: Option<std::sync::mpsc::Receiver<FileSelection>>,
    file_dialog_error: Option<String>,
    import_revision: u64,
    removing_image: Option<PathBuf>,
    thumbnail: Option<Thumbnail>,
    virtual_camera_persistent: bool,
    #[cfg(feature = "tray")]
    tray: Option<ksni::blocking::Handle<crate::tray::Tray>>,
    #[cfg(feature = "tray")]
    tray_streaming: bool,
    #[cfg(feature = "tray")]
    tray_language: String,
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
            effects: false,
            settings: false,
            hidden,
            show_revision: 0,
            file_dialog: None,
            file_dialog_error: None,
            import_revision: 0,
            removing_image: None,
            thumbnail: None,
            virtual_camera_persistent: true,
            #[cfg(feature = "tray")]
            tray,
            #[cfg(feature = "tray")]
            tray_streaming: false,
            #[cfg(feature = "tray")]
            tray_language: String::new(),
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
                        ui.label(RichText::new(t(label)).size(12.0).color(MUTED));
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
                        .selected_text(t(control
                            .menu
                            .get(&value)
                            .map(String::as_str)
                            .unwrap_or("—")))
                        .show_ui(ui, |ui| {
                            for (v, text) in &control.menu {
                                if ui.selectable_value(&mut value, *v, t(text)).changed() {
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
                RichText::new(t("Indisponível nesta configuração"))
                    .size(10.0)
                    .color(MUTED),
            )
            .on_hover_text(t(reason));
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
                egui::CollapsingHeader::new(RichText::new(t(title)).size(13.0).strong())
                    .id_salt(("camera", title))
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
            ui.selectable_value(&mut self.effects, false, t("Camera"));
            ui.selectable_value(&mut self.effects, true, t("Effects"));
            ui.add_enabled(false, egui::Button::new(t("Prompter")));
        });
        ui.add_space(3.0);
        if self.effects {
            self.effects_panel(ui, state);
            return;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().fill(PANEL).corner_radius(3).inner_margin(10).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(t("Device"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let save = ui.add(egui::Button::new(t("SAVE")).fill(ACCENT)).on_hover_text(t("Salvar configuração e ajustes neste computador"));
                        let action_size = save.rect.size();
                        if save.clicked() { self.dispatch(Request::Save); }
                        if ui.add_sized(action_size, egui::Button::new("R")).on_hover_text(t("Restaurar ajustes salvos neste computador")).clicked() { self.dispatch(Request::Restore); }
                        if ui.add_sized(action_size, egui::Button::new("⟳")).on_hover_text(t("Atualizar dispositivos e ajustes")).clicked() { self.dispatch(Request::Refresh); }
                    });
                });
                ui.separator();
                ui.add_enabled_ui(!state.streaming && !state.preview, |ui| {
                    let select_width = ui.available_width() - 48.0 - ui.spacing().item_spacing.x;
                    egui::Grid::new("device-fields").num_columns(2).min_col_width(0.0).show(ui, |ui| {
                        ui.allocate_ui_with_layout(Vec2::new(48.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(RichText::new(t("Input")).color(MUTED));
                        });
                        let text = state.camera.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| t("Conecte uma Facecam"));
                        egui::ComboBox::from_id_salt("device").width(select_width).truncate().selected_text(text).show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.config.camera_device, String::new(), t("Automático")).changed() { self.configure(); }
                            for camera in &state.cameras {
                                if ui.selectable_value(&mut self.config.camera_device, camera.path.clone(), &camera.name).changed() { self.configure(); }
                            }
                        });
                        ui.end_row();
                        ui.allocate_ui_with_layout(Vec2::new(48.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(RichText::new(t("Format")).color(MUTED));
                        });
                        egui::ComboBox::from_id_salt("format").width(select_width).truncate()
                            .selected_text(state.mode.as_ref().map(|m| m.label()).unwrap_or_else(|| t("Indisponível")))
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
            ui.label(RichText::new(t("¹ Exposição em unidades de 100 µs.\n² Ganho do driver; não representa ISO.")).size(10.0).color(MUTED));
        });
    }

    fn effects_panel(&mut self, ui: &mut egui::Ui, state: &Snapshot) {
        if state.config.effects != self.applied_effects {
            let mut detected = self.applied_effects.clone();
            detected.ai_model = state.config.effects.ai_model.clone();
            detected.ai_runtime = state.config.effects.ai_runtime.clone();
            detected.mirror = state.config.effects.mirror;
            if detected == state.config.effects {
                self.config.effects.ai_model = detected.ai_model;
                self.config.effects.ai_runtime = detected.ai_runtime;
                if self.applied_effects.mirror != detected.mirror {
                    self.config.effects.mirror = detected.mirror;
                }
            } else {
                self.config.effects = state.config.effects.clone();
            }
            self.applied_effects = state.config.effects.clone();
        }
        if let Some(path) = &self.removing_image
            && !state.resources.iter().any(|r| r.path == *path)
        {
            if std::path::Path::new(&self.config.effects.background_image) == path {
                self.config.effects.background_image.clear();
            }
            if self.thumbnail.as_ref().is_some_and(|t| t.path == *path) {
                self.thumbnail = None;
            }
            self.removing_image = None;
        }
        let mut choose_file = None;
        let mut remove_image = None;
        let mut refresh_ai = false;
        let dialog_available = self.file_dialog.is_none() && !state.importing;
        let controls_height = (ui.available_height() - 82.0).max(80.0);
        egui::ScrollArea::vertical().max_height(controls_height).auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().fill(PANEL).corner_radius(3).inner_margin(10).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(t("Look"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(dialog_available, egui::Button::new(t("Adicionar LUT…"))).clicked() {
                            choose_file = Some(FileTarget::Lut);
                        }
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(48.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(RichText::new(t("LUT 3D")).color(MUTED));
                    });
                    resource_selector(ui, state, FileTarget::Lut, &mut self.config.effects.lut);
                });
            });
            ui.add_space(5.0);
            let effects = &mut self.config.effects;
            effect_card(ui, "Picture", true, |ui| {
                effect_slider(ui, "Brightness", &mut effects.brightness, -1.0..=1.0, 0.01);
                effect_slider(ui, "Contrast", &mut effects.contrast, 0.0..=2.0, 0.01);
                effect_slider(ui, "Saturation", &mut effects.saturation, 0.0..=3.0, 0.01);
            });
            effect_card(ui, "Processing", true, |ui| {
                effect_slider(ui, "Denoise", &mut effects.denoise, 0.0..=10.0, 0.1);
                effect_row(ui, "Espelhar vídeo", |ui| {
                    ui.checkbox(&mut effects.mirror, "").on_hover_text(t("Espelha o preview e a saída virtual."));
                });
            });
            effect_card(ui, "AI Background", true, |ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled(state.ai_compiled, egui::Checkbox::new(&mut effects.background_blur, t("Ativar fundo com IA")));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(t("Atualizar")).on_hover_text(t("Detectar recursos e instalações de IA disponíveis neste computador")).clicked() {
                            refresh_ai = true;
                        }
                    });
                });
                ui.add_enabled_ui(state.ai_compiled, |ui| {
                    effect_row(ui, "Fundo", |ui| {
                        egui::ComboBox::from_id_salt("background-mode").width(ui.available_width()).truncate().selected_text(t(effects.background_mode.label())).show_ui(ui, |ui| {
                            use crate::effects::BackgroundMode::*;
                            for mode in [Blur, Solid, Studio, Office, Landscape, Image, Video] {
                                ui.selectable_value(&mut effects.background_mode, mode, t(mode.label()));
                            }
                        });
                    });
                    match effects.background_mode {
                        crate::effects::BackgroundMode::Blur => effect_slider(ui, "Blur", &mut effects.blur_radius, 1..=32, 1.0),
                        crate::effects::BackgroundMode::Solid => effect_row(ui, "Cor de fundo", |ui| { ui.color_edit_button_srgb(&mut effects.background_color); }),
                        crate::effects::BackgroundMode::Image => {
                            effect_row(ui, "Imagem", |ui| { resource_selector(ui, state, FileTarget::Image, &mut effects.background_image); });
                            ui.horizontal(|ui| {
                                if ui.add_enabled(dialog_available, egui::Button::new(t("Adicionar imagem…"))).clicked() {
                                    choose_file = Some(FileTarget::Image);
                                }
                                let removable = state.resources.iter().any(|r| r.kind == FileTarget::Image && r.removable && r.path == std::path::Path::new(&effects.background_image));
                                if ui.add_enabled(dialog_available && removable, egui::Button::new(t("Remover imagem")))
                                    .on_hover_text(t("Remove a cópia da biblioteca e preserva o original. Se estiver aplicada, troca o fundo para desfoque."))
                                    .on_disabled_hover_text(t("Selecione uma imagem importada para a biblioteca deste aplicativo."))
                                    .clicked() {
                                    remove_image = Some(PathBuf::from(&effects.background_image));
                                }
                            });
                            if !effects.background_image.is_empty() { Self::resource_thumbnail(ui, std::path::Path::new(&effects.background_image), &mut self.thumbnail); }
                        }
                        crate::effects::BackgroundMode::Video => {
                            effect_row(ui, "Vídeo", |ui| { resource_selector(ui, state, FileTarget::Video, &mut effects.background_video); });
                            if ui.add_enabled(dialog_available, egui::Button::new(t("Adicionar vídeo…"))).clicked() {
                                choose_file = Some(FileTarget::Video);
                            }
                            if !effects.background_video.is_empty() { Self::resource_thumbnail(ui, std::path::Path::new(&effects.background_video), &mut self.thumbnail); }
                            ui.label(RichText::new(t("Reproduz em loop, sem áudio. Preenche o quadro com recorte central.")).size(10.0).color(MUTED));
                        }
                        _ => { ui.label(RichText::new(t("Cenário ilustrado local, sem download.")).size(10.0).color(MUTED)); }
                    }
                    effect_slider(ui, "FPS da IA", &mut effects.ai_fps, 1..=60, 1.0);
                    effect_row(ui, "Processamento", |ui| { ui.label(t(match state.ai_backend.as_ref().map(|s| s.backend) {
                        Some(crate::effects::AiBackend::Nvidia) => "NVIDIA RTX · temporal",
                        _ => match effects.ai_provider {
                            crate::effects::AiProvider::Cuda => "NVIDIA CUDA",
                            crate::effects::AiProvider::Cpu => "CPU (ONNX)",
                        },
                    })); });
                    if let Some(warning) = state.ai_backend.as_ref().and_then(|s| s.warning.as_deref()) {
                        ui.label(RichText::new(t(warning)).size(10.0).color(MUTED));
                    }
                    ui.label(RichText::new(t(if state.nvidia_prepared { "Componentes NVIDIA RTX detectados" } else if effects.ai_model.is_empty() || effects.ai_runtime.is_empty() { "IA indisponível · prepare os componentes nas configurações" } else { "Modelo e runtime ONNX detectados" })).size(10.0).color(MUTED));
                    ui.label(RichText::new(t("Preview e câmera virtual usam a mesma imagem, resolução e FPS.")).size(10.0).color(MUTED)).on_hover_text(t("FPS da IA limita ambas as saídas. A janela ajusta apenas o tamanho de exibição, preservando a proporção."));
                });
                if !state.ai_compiled { ui.label(RichText::new(t("Esta compilação não inclui IA.")).size(10.0).color(MUTED)); }
            });
            effect_card(ui, "Opções avançadas", false, |ui| {
                ui.add_enabled_ui(state.ai_compiled, |ui| {
                    effect_row(ui, "Provider", |ui| {
                        egui::ComboBox::from_id_salt("ai-provider").width(ui.available_width()).selected_text(t(match effects.ai_provider {
                            crate::effects::AiProvider::Cuda => "NVIDIA CUDA",
                            crate::effects::AiProvider::Cpu => "CPU (alternativa)",
                        })).show_ui(ui, |ui| {
                            ui.selectable_value(&mut effects.ai_provider, crate::effects::AiProvider::Cuda, t("NVIDIA CUDA"));
                            ui.selectable_value(&mut effects.ai_provider, crate::effects::AiProvider::Cpu, t("CPU (alternativa)"));
                        });
                    });
                    effect_row(ui, "Segmentação", |ui| {
                        let label = |b| match b {
                            crate::effects::AiBackend::Auto => "Automática",
                            crate::effects::AiBackend::Onnx => "ONNX",
                            crate::effects::AiBackend::Nvidia => "NVIDIA RTX",
                        };
                        egui::ComboBox::from_id_salt("ai-backend").width(ui.available_width()).selected_text(t(label(effects.ai_backend))).show_ui(ui, |ui| {
                            for backend in [crate::effects::AiBackend::Auto, crate::effects::AiBackend::Onnx, crate::effects::AiBackend::Nvidia] {
                                ui.selectable_value(&mut effects.ai_backend, backend, t(label(backend)));
                            }
                        });
                    });
                    ui.label(RichText::new(t("Automática usa a segmentação RTX preparada para a GPU e mantém ONNX como alternativa. CPU executa somente quando selecionada.")).size(10.0).color(MUTED));
                    ui.label(RichText::new(t("CUDA exige ONNX Runtime GPU, CUDA e cuDNN. O modelo e o runtime são carregados somente ao ativar IA.")).size(10.0).color(MUTED));
                });
            });
            if state.importing { ui.horizontal(|ui| { ui.spinner(); ui.label(t("Adicionando à biblioteca…")); }); }
            if let Some(error) = &self.file_dialog_error { ui.colored_label(Color32::LIGHT_RED, t(error)); }
        });
        ui.add_space(4.0);
        if ui.button(t("Restaurar efeitos padrão")).clicked() {
            self.config.effects = Default::default();
        }
        if ui.add_sized([ui.available_width(), 28.0], egui::Button::new(t("Aplicar e salvar")).fill(ACCENT))
            .on_hover_text(t("Aplica ao preview e à câmera virtual, reiniciando brevemente a captura, e salva a configuração.")).clicked() {
            self.dispatch(Request::SetEffects { effects: self.config.effects.clone() });
        }
        if let Some(target) = choose_file {
            self.open_file_dialog(target, ui.ctx().clone());
        }
        if refresh_ai {
            self.dispatch(Request::Refresh);
        }
        if let Some(path) = remove_image {
            self.removing_image = Some(path.clone());
            self.dispatch(Request::RemoveImage { path });
        }
    }

    fn open_file_dialog(&mut self, target: FileTarget, context: egui::Context) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.file_dialog_error = None;
        match std::thread::Builder::new()
            .name("file-dialog".into())
            .spawn(move || {
                let dialog = match target {
                    FileTarget::Lut => rfd::FileDialog::new()
                        .set_title(t("Adicionar LUT"))
                        .add_filter("LUT 3D", &["cube"]),
                    FileTarget::Image => rfd::FileDialog::new()
                        .set_title(t("Adicionar imagem de fundo"))
                        .add_filter(t("Imagens"), &["png", "jpg", "jpeg", "webp", "bmp"]),
                    FileTarget::Video => rfd::FileDialog::new()
                        .set_title(t("Adicionar vídeo de fundo"))
                        .add_filter(t("Vídeos"), &["mp4", "mov", "webm", "mkv"]),
                };
                let _ = tx.send((target, dialog.pick_file()));
                context.request_repaint();
            }) {
            Ok(_) => self.file_dialog = Some(rx),
            Err(error) => {
                self.file_dialog_error = Some(format!("Não foi possível abrir o seletor: {error}"))
            }
        }
    }

    fn poll_file_dialog(&mut self) {
        let Some(receiver) = &self.file_dialog else {
            return;
        };
        match receiver.try_recv() {
            Ok((target, path)) => {
                self.file_dialog = None;
                if let Some(path) = path {
                    self.dispatch(Request::ImportResource { kind: target, path });
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.file_dialog = None;
                self.file_dialog_error =
                    Some("O seletor de arquivos encerrou; tente novamente".into());
            }
        }
    }

    fn resource_thumbnail(
        ui: &mut egui::Ui,
        path: &std::path::Path,
        state: &mut Option<Thumbnail>,
    ) {
        if state.as_ref().is_none_or(|t| t.path != path) {
            let path = path.to_owned();
            let task_path = path.clone();
            let context = ui.ctx().clone();
            let (tx, receiver) = std::sync::mpsc::channel();
            if std::thread::Builder::new()
                .name("resource-thumbnail".into())
                .spawn(move || {
                    let _ = tx.send(
                        crate::resources::thumbnail(&task_path).map_err(|e| format!("{e:#}")),
                    );
                    context.request_repaint();
                })
                .is_ok()
            {
                *state = Some(Thumbnail {
                    path,
                    receiver,
                    texture: None,
                    error: None,
                });
            }
        }
        if let Some(thumbnail) = state {
            if let Ok(result) = thumbnail.receiver.try_recv() {
                match result {
                    Ok(rgb) => {
                        thumbnail.texture = Some(ui.ctx().load_texture(
                            "resource-thumbnail",
                            egui::ColorImage::from_rgb([128, 72], &rgb),
                            Default::default(),
                        ))
                    }
                    Err(error) => thumbnail.error = Some(error),
                }
            }
            if let Some(texture) = &thumbnail.texture {
                ui.image(texture);
            } else if let Some(error) = &thumbnail.error {
                ui.label(RichText::new(t(error)).size(10.0).color(MUTED));
            } else {
                ui.spinner();
            }
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui, state: &Snapshot) {
        let available = ui.available_size();
        let height = (available.y - 62.0).max(100.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(available.x, height), egui::Sense::hover());
        let size = self.texture.as_ref().map(|t| t.size()).unwrap_or_else(|| {
            state
                .mode
                .as_ref()
                .map(|m| [m.width as usize, m.height as usize])
                .unwrap_or([16, 9])
        });
        let aspect = size[0] as f32 / size[1] as f32;
        let width = rect.width().min(rect.height() * aspect);
        let image_rect =
            egui::Rect::from_center_size(rect.center(), Vec2::new(width, width / aspect));
        if let Some(texture) = &self.texture {
            let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
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
                t(if state.camera.is_some() {
                    "Preview pausado"
                } else {
                    "Conecte uma Facecam"
                }),
                egui::FontId::proportional(20.0),
                TEXT,
            );
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                t("Controle a câmera. Veja cada ajuste."),
                egui::FontId::proportional(13.0),
                MUTED,
            );
            let button =
                egui::Rect::from_center_size(center + Vec2::new(0.0, 40.0), Vec2::new(150.0, 32.0));
            if ui
                .add_enabled_ui(state.camera.is_some(), |ui| {
                    ui.put(button, egui::Button::new(t("Iniciar preview")).fill(ACCENT))
                })
                .inner
                .clicked()
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
                    ui.label(RichText::new(t("Output")).size(12.0))
                        .on_hover_text(t(
                            "Formato compartilhado pelo preview e pela câmera virtual",
                        ));
                    ui.label(
                        RichText::new(
                            state
                                .mode
                                .as_ref()
                                .map(|m| {
                                    format!(
                                        "{}×{} · {:.2} fps",
                                        m.width,
                                        m.height,
                                        output_fps(m, &state.config.effects)
                                    )
                                })
                                .unwrap_or_else(|| "—".into()),
                        )
                        .size(12.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .selectable_label(state.config.effects.mirror, t("Mirror"))
                            .on_hover_text(t("Espelhar preview e câmera virtual"))
                            .clicked()
                        {
                            let mut effects = state.config.effects.clone();
                            effects.mirror = !effects.mirror;
                            self.dispatch(Request::SetEffects { effects });
                        }
                        ui.toggle_value(&mut self.grid, t("Grid"))
                            .on_hover_text(t("Grade de enquadramento"));
                        if ui
                            .selectable_label(state.preview, t("View"))
                            .on_hover_text(t("Mostrar / pausar preview"))
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
                                egui::Button::new(t(if state.streaming {
                                    "Parar câmera virtual"
                                } else {
                                    "Iniciar câmera virtual"
                                }))
                                .fill(if state.streaming {
                                    HEADER
                                } else {
                                    ACCENT
                                }),
                            )
                            .clicked()
                        {
                            if !state.streaming && state.virtual_cameras.is_empty() {
                                self.settings = true;
                            } else {
                                self.dispatch(if state.streaming {
                                    Request::Stop
                                } else {
                                    Request::Start { preview: true }
                                });
                            }
                        }
                    });
                });
            });
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        #[cfg(debug_assertions)]
        crate::gui_test::capture(ctx);
        self.poll_file_dialog();
        let state = self.state();
        self.config.virtual_device = state.config.virtual_device.clone();
        self.config.ui = state.config.ui.clone();
        if state.import_revision != self.import_revision {
            self.import_revision = state.import_revision;
            if let Some(resource) = &state.imported
                && let Some(path) = resource.path.to_str()
            {
                match resource.kind {
                    FileTarget::Lut => self.config.effects.lut = path.into(),
                    FileTarget::Image => {
                        self.config.effects.background_image = path.into();
                        self.config.effects.background_mode = crate::effects::BackgroundMode::Image;
                    }
                    FileTarget::Video => {
                        self.config.effects.background_video = path.into();
                        self.config.effects.background_mode = crate::effects::BackgroundMode::Video;
                    }
                }
            }
        }
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
            if state.installation_running || state.importing || state.preparing_nvidia {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.settings = true;
            }
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
        if self.tray_streaming != state.streaming || self.tray_language != state.config.ui.language
        {
            if let Some(tray) = &self.tray {
                tray.update(|tray| tray.streaming = state.streaming);
            }
            self.tray_streaming = state.streaming;
            self.tray_language = state.config.ui.language.clone();
        }
        if !self.hidden {
            if let Some(frame) = self
                .controller
                .frames
                .frame
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
            {
                let image = egui::ColorImage::from_rgb(frame.size, &frame.rgb);
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
            Duration::from_secs_f64(
                1.0 / state
                    .mode
                    .as_ref()
                    .map(|m| output_fps(m, &state.config.effects))
                    .unwrap_or(30.0)
                    .max(1.0),
            )
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
                        if ui.button("⚙").on_hover_text(t("Configurações")).clicked() {
                            self.settings = !self.settings;
                        }
                        if state.streaming {
                            ui.colored_label(
                                Color32::from_rgb(110, 200, 150),
                                t("● Câmera virtual ativa"),
                            );
                        }
                    });
                });
            });
        egui::Panel::bottom("status")
            .frame(egui::Frame::new().fill(HEADER).inner_margin(8))
            .show(root, |ui| {
                if let Some(error) = &state.error {
                    ui.colored_label(Color32::from_rgb(255, 170, 150), t(error));
                } else {
                    ui.label(RichText::new(t(&state.message)).size(11.0).color(MUTED));
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
        egui::Window::new(t("Configurações"))
            .id(egui::Id::new("settings-window"))
            .vscroll(true)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(560.0)
            .default_height(520.0)
            .max_height((ctx.content_rect().height() - 80.0).max(240.0))
            .resizable(false)
            .show(&ctx, |ui| {
                let mut language = state.config.ui.language.clone();
                ui.horizontal(|ui| {
                    ui.label(t("Idioma"));
                    egui::ComboBox::from_id_salt("interface-language")
                        .width(320.0)
                        .truncate()
                        .selected_text(state.languages.iter().find(|l| l.language == language).map_or(language.as_str(), |l| l.name.as_str()))
                        .show_ui(ui, |ui| {
                            for entry in &state.languages {
                                ui.selectable_value(&mut language, entry.language.clone(), &entry.name);
                            }
                        });
                });
                if language != state.config.ui.language {
                    self.dispatch(Request::UpdateSettings { virtual_device: None, start_minimized: None, language: Some(language) });
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(t("Câmera virtual"));
                    let (rect, response) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                    ui.painter().circle_stroke(rect.center(), 6.5, Stroke::new(1.0, MUTED));
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(11.0), MUTED);
                    response.on_hover_text(t("Pare a captura para alterar o formato ou o dispositivo."));
                });
                let mut virtual_device = state.config.virtual_device.clone();
                ui.add_enabled_ui(!state.streaming && !state.preview, |ui| {
                    let selected = state
                        .virtual_cameras
                        .iter()
                        .find(|c| c.path == virtual_device)
                        .map(|c| c.name.clone()).unwrap_or_else(|| t("Nenhuma câmera virtual detectada"));
                    ui.horizontal(|ui| {
                        let selector_width =
                            (ui.available_width() - 152.0 - ui.spacing().item_spacing.x).max(120.0);
                        egui::ComboBox::from_id_salt("virtual-camera")
                            .width(selector_width)
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                for camera in &state.virtual_cameras {
                                    ui.selectable_value(
                                        &mut virtual_device,
                                        camera.path.clone(),
                                        &camera.name,
                                    );
                                }
                            });
                        if ui
                            .add_sized([152.0, 28.0], egui::Button::new(t("Atualizar dispositivos")))
                            .clicked()
                        {
                            self.dispatch(Request::Refresh);
                        }
                    });
                    if state.virtual_cameras.is_empty() {
                        ui.checkbox(
                            &mut self.virtual_camera_persistent,
                            t("Disponível após reiniciar o Ubuntu"),
                        );
                        if ui
                            .add_enabled(
                                !state.installation_running,
                                egui::Button::new(t("Preparar câmera virtual…")),
                            )
                            .clicked()
                        {
                            self.dispatch(Request::PrepareVirtualCamera {
                                persistent: self.virtual_camera_persistent,
                            });
                        }
                    }
                });
                let mut start_minimized = state.config.ui.start_minimized;
                let start_minimized_changed = ui.checkbox(&mut start_minimized, t("Iniciar na bandeja")).changed();
                let device_changed = virtual_device != state.config.virtual_device;
                if device_changed || start_minimized_changed {
                    self.dispatch(Request::UpdateSettings {
                        language: None,
                        virtual_device: device_changed.then_some(virtual_device),
                        start_minimized: start_minimized_changed.then_some(start_minimized),
                    });
                }
                let mut autostart = state.autostart_enabled.unwrap_or(false);
                let autostart_response = ui.add_enabled(
                    state.autostart_enabled.is_some(),
                    egui::Checkbox::new(&mut autostart, t("Iniciar ao entrar no desktop")),
                ).on_hover_text(t("Marcar ativa o início automático; desmarcar desfaz. A alteração é imediata."));
                if autostart_response.changed() {
                    self.dispatch(Request::SetAutostart { enabled: autostart });
                }
                if state.autostart_enabled.is_none() {
                    ui.label(t("Não foi possível consultar o início automático. Atualize os dispositivos e tente novamente."));
                }
                ui.separator();
                ui.strong(t("Efeitos NVIDIA RTX"));
                ui.label(t("Segmentação temporal, desfoque e troca de fundo · RTX 40 (Ada)."));
                ui.label(t("Preparação no seu usuário: baixa os componentes e testa a GPU. Pode levar vários minutos."));
                if ui.add_enabled(state.ai_compiled && !state.preparing_nvidia && !state.streaming && !state.preview && !state.installation_running,
                    egui::Button::new(t(if state.nvidia_prepared { "Verificar / reparar IA RTX" } else { "Preparar IA RTX" }))).clicked() {
                    self.dispatch(Request::PrepareNvidia);
                }
                if state.preparing_nvidia {
                    ui.horizontal(|ui| { ui.spinner(); ui.label(t(&state.nvidia_progress)); });
                } else if !state.nvidia_progress.is_empty() {
                    ui.label(t(&state.nvidia_progress));
                }
                if let Some(log) = &state.nvidia_log { ui.small(format!("Log: {}", log.display())); }
                ui.separator();
                ui.strong(t("Dependências do Ubuntu"));
                ui.horizontal(|ui| {
                    let width =
                        ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(0.0);
                    for dependency in [
                        crate::dependencies::Dependency::VideoTools,
                        crate::dependencies::Dependency::SdkManager,
                    ] {
                        let status = state
                            .dependencies
                            .iter()
                            .find(|s| s.dependency == dependency);
                        let installed = status.is_some_and(|s| s.installed());
                        let label = format!(
                            "{} {}…",
                            t(if installed { "Desinstalar" } else { "Instalar" }),
                            t(dependency.name())
                        );
                        let response = ui.add_enabled(
                            !state.installation_running
                                && status.is_some()
                                && state.dependency_error.is_none(),
                            egui::Button::new(label).min_size(Vec2::new(width, 32.0)),
                        );
                        if let Some(status) = status {
                            response.clone().on_hover_text(
                                status
                                    .packages
                                    .iter()
                                    .map(|p| {
                                        format!(
                                            "{}: {}",
                                            p.name,
                                            p.version.as_deref().map(str::to_owned).unwrap_or_else(|| t("não instalado"))
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            );
                        }
                        if response.clicked() {
                            self.dispatch(if installed {
                                Request::PrepareUninstall { dependency }
                            } else {
                                Request::PrepareInstall { dependency }
                            });
                        }
                    }
                });
                if let Some(error) = &state.dependency_error {
                    ui.colored_label(Color32::LIGHT_RED, t(error));
                }
                if state.installation_running {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(t("Operação APT em andamento; aguarde concluir."));
                    });
                }
                if let Some(log) = &state.installation_log {
                    ui.label(t(&format!("Log: {}", log.display())));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t("Encerrar aplicativo")).clicked() {
                            self.dispatch(Request::Quit);
                        }
                    });
                });
            });
        self.settings = open;
        if let Some(plan) = &state.installation_plan {
            let mut confirm = false;
            let mut cancel = false;
            let response = egui::Modal::new(egui::Id::new("confirm-apt-install")).show(&ctx, |ui| {
                ui.set_width(560.0);
                ui.heading(t(&plan.title));
                ui.label(t(&plan.explanation));
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(230.0).show(ui, |ui| {
                    for command in &plan.commands {
                        ui.add(egui::Label::new(RichText::new(crate::dependencies::display_command(command)).monospace()).wrap());
                        ui.add_space(5.0);
                    }
                    if let Some(setup) = &plan.virtual_camera {
                        ui.label(t("Etapas após a autorização única do Ubuntu:"));
                        if let Ok(steps) = setup.review_steps() {
                            for step in steps {
                                ui.add(egui::Label::new(RichText::new(crate::dependencies::display_command(&step)).monospace()).wrap());
                                ui.add_space(5.0);
                            }
                        }
                    }
                    if let Some(hash) = &plan.sha256 {
                        ui.label(t("SHA-256 da chave de assinatura NVIDIA:"));
                        ui.monospace(hash);
                    }
                    if let Some(repository) = &plan.repository {
                        ui.label(t("Repositório que será cadastrado:"));
                        ui.add(egui::Label::new(RichText::new(repository).monospace()).wrap());
                    }
                    if !plan.removals.is_empty() {
                        ui.label(t(&format!("Pacotes que serão removidos: {}", plan.removals.join(", "))));
                    }
                });
                ui.label(t("O Ubuntu pedirá autenticação administrativa. O aplicativo não recebe sua senha."));
                ui.horizontal(|ui| {
                    cancel = ui.button(t("Cancelar")).clicked();
                    let label = if plan.virtual_camera.is_some() { "Confirmar preparação" } else if plan.action == crate::dependencies::Action::Remove { "Confirmar desinstalação" } else { "Confirmar instalação" };
                    confirm = ui.add(egui::Button::new(t(label)).fill(ACCENT)).clicked();
                });
            });
            if confirm {
                self.dispatch(Request::ConfirmInstall { id: plan.id });
            } else if cancel || response.should_close() {
                self.dispatch(Request::CancelInstall);
            }
        }
    }
}
