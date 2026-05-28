use acuneus::compute::{ComputeShader, COMPUTE_TEXTURE_FORMAT_RGBA16};
use acuneus::prelude::*;
use acuneus::remote::{params_for_bin, RemoteCommand, RemoteControl, RemoteValue};
use acuneus::WindowEvent;

acuneus::uniform_params! {
    pub struct RotoParams {
        square_size: f32,
        circle_radius: f32,
        edge_thickness: f32,
        animation_speed: f32,
        background_color: [f32; 3],
        edge_color_intensity: f32,
        _pad1: f32,
        _pad2: f32,
        _pad3: f32,
        _pad4: f32,
    }
}

struct RotoShader {
    base: RenderKit,
    compute_shader: ComputeShader,
    current_params: RotoParams,
    remote_control: Option<RemoteControl>,
    transport: Option<TransportState>,
    window_scale: f32,
    remote_time: Option<f32>,
    remote_fps: f32,
    remote_resolution: Option<(u32, u32)>,
}

#[derive(Clone, Copy, Debug)]
struct TransportState {
    bpm: f32,
    beat: f32,
    measure: f32,
}

impl ShaderManager for RotoShader {
    fn init(core: &Core) -> Self {
        let initial_params = RotoParams {
            square_size: 0.2,
            circle_radius: 0.11,
            edge_thickness: 0.003,
            animation_speed: 12.0,
            background_color: [0.5, 0.5, 0.5],
            edge_color_intensity: 1.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
            _pad4: 0.0,
        };
        let base = RenderKit::new(core);
        let config = ComputeShader::builder()
            .with_entry_point("main")
            .with_custom_uniforms::<RotoParams>()
            .with_workgroup_size([16, 16, 1])
            .with_texture_format(COMPUTE_TEXTURE_FORMAT_RGBA16)
            .with_label("Roto")
            .build();
        let compute_shader = acuneus::compute_shader!(core, "shaders/roto.wgsl", config);
        compute_shader.set_custom_params(initial_params, &core.queue);

        Self {
            base,
            compute_shader,
            current_params: initial_params,
            remote_control: RemoteControl::from_env(),
            transport: None,
            window_scale: 1.0,
            remote_time: None,
            remote_fps: 60.0,
            remote_resolution: None,
        }
    }

    fn update(&mut self, core: &Core) {
        self.compute_shader.handle_export(core, &mut self.base);
        let current_time = self
            .remote_time
            .or_else(|| {
                self.transport
                    .map(|transport| transport.beat * 60.0 / transport.bpm.max(1.0))
            })
            .unwrap_or_else(|| self.base.controls.get_time(&self.base.start_time));
        self.compute_shader
            .set_time(current_time, 1.0 / self.remote_fps.max(1.0), &core.queue);
    }

    fn resize(&mut self, core: &Core) {
        self.base.default_resize(core, &mut self.compute_shader);
    }

    fn render(&mut self, core: &Core) -> Result<(), SurfaceError> {
        let mut frame = self.base.begin_frame(core)?;
        let mut params = self.current_params;
        let mut changed = false;
        let mut pause_override = None;
        let mut should_reset_remote = false;
        let mut should_clear_remote = false;

        if let Some(remote_control) = &self.remote_control {
            for command in remote_control.drain() {
                match command {
                    RemoteCommand::SetF32 { id, value } => {
                        if id == "control_pause" {
                            pause_override = Some(value >= 0.5);
                        } else {
                            changed |= apply_remote_f32(&mut params, &id, value);
                        }
                    }
                    RemoteCommand::SetColor3 { id, value } => {
                        if id == "background_color" {
                            params.background_color = value;
                            changed = true;
                        }
                    }
                    RemoteCommand::Transport { bpm, beat, measure } => {
                        if bpm > 0.0 {
                            self.transport = Some(TransportState { bpm, beat, measure });
                        }
                    }
                    RemoteCommand::Discover => {
                        send_remote_discovery(remote_control, &params);
                    }
                    RemoteCommand::Subscribe { enabled } => {
                        remote_control.set_feedback_enabled(enabled);
                        if enabled {
                            send_remote_discovery(remote_control, &params);
                        }
                    }
                    RemoteCommand::OverlayVisible { visible } => {
                        self.base.key_handler.show_ui = visible;
                    }
                    RemoteCommand::ToggleOverlay => {
                        self.base.key_handler.show_ui = !self.base.key_handler.show_ui;
                    }
                    RemoteCommand::TitleBarVisible { visible } => {
                        core.window().set_decorations(visible);
                    }
                    RemoteCommand::HideTitleBar => {
                        core.window().set_decorations(false);
                    }
                    RemoteCommand::WindowTitle { title } => {
                        core.window().set_title(&title);
                    }
                    RemoteCommand::WindowPosition { x, y } => {
                        core.window()
                            .set_outer_position(acuneus::winit::dpi::PhysicalPosition::new(x, y));
                    }
                    RemoteCommand::WindowScale { scale } => {
                        self.window_scale = scale.max(0.1);
                        let width = (800.0 * self.window_scale).round().max(1.0) as u32;
                        let height = (600.0 * self.window_scale).round().max(1.0) as u32;
                        let _ = core.window().request_inner_size(
                            acuneus::winit::dpi::LogicalSize::new(width, height),
                        );
                    }
                    RemoteCommand::WindowSize { width, height } => {
                        let _ = core.window().request_inner_size(
                            acuneus::winit::dpi::LogicalSize::new(width.max(1), height.max(1)),
                        );
                    }
                    RemoteCommand::Time { seconds } => {
                        self.remote_time = Some(seconds.max(0.0));
                    }
                    RemoteCommand::Fps { fps } => {
                        self.remote_fps = fps.max(1.0);
                    }
                    RemoteCommand::Resolution { width, height } => {
                        let width = width.max(1);
                        let height = height.max(1);
                        self.remote_resolution = Some((width, height));
                        self.base.update_resolution(
                            &core.queue,
                            acuneus::winit::dpi::PhysicalSize::new(width, height),
                        );
                        self.compute_shader.resize(core, width, height);
                    }
                    RemoteCommand::Action { id, value } => {
                        if value >= 0.5 {
                            if id == "control_reset" {
                                should_reset_remote = true;
                                should_clear_remote = true;
                            } else if id == "control_clear_buffers" {
                                should_clear_remote = true;
                            }
                        }
                    }
                    RemoteCommand::LoadMedia { .. } => {}
                    RemoteCommand::SetString { .. } => {}
                    RemoteCommand::Pulse { .. } | RemoteCommand::Note { .. } => {}
                }
            }
        }

        let mut should_start_export = false;
        let mut export_request = self.base.export_manager.get_ui_request();
        let mut controls_request = self.base.controls.get_ui_request(
            &self.base.start_time,
            &self
                .remote_resolution
                .map(|(width, height)| acuneus::winit::dpi::PhysicalSize::new(width, height))
                .unwrap_or(core.size),
            self.base.fps_tracker.fps(),
        );
        if let Some(time) = self.remote_time {
            controls_request.current_time = Some(time);
        }
        controls_request.current_fps = Some(self.remote_fps);
        if let Some(is_paused) = pause_override {
            controls_request.is_paused = is_paused;
        }
        controls_request.should_reset |= should_reset_remote;
        controls_request.should_clear_buffers |= should_clear_remote;

        let full_output = if self.base.key_handler.show_ui {
            self.base.render_ui(core, |ctx| {
                RenderKit::apply_default_style(ctx);
                egui::Window::new("Rotation Illusion")
                    .collapsible(true)
                    .resizable(true)
                    .default_width(280.0)
                    .show(ctx, |ui| {
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut params.square_size, 0.05..=0.5)
                                    .text("Square Size"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut params.circle_radius, 0.05..=0.2)
                                    .text("Circle Radius"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut params.edge_thickness, 0.001..=0.01)
                                    .text("Edge Thickness"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut params.animation_speed, 1.0..=30.0)
                                    .text("Animation Speed"),
                            )
                            .changed();
                        ui.horizontal(|ui| {
                            ui.label("Background");
                            changed |= ui
                                .color_edit_button_rgb(&mut params.background_color)
                                .changed();
                        });
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut params.edge_color_intensity, 0.1..=2.0)
                                    .text("Edge Brightness"),
                            )
                            .changed();

                        ui.separator();
                        if let Some(remote_control) = &self.remote_control {
                            render_transport_widget(ui, remote_control, self.transport);
                            ui.separator();
                        }
                        ShaderControls::render_controls_widget(ui, &mut controls_request);
                        ui.separator();
                        should_start_export =
                            ExportManager::render_export_ui_widget(ui, &mut export_request);
                    });
            })
        } else {
            self.base.render_ui(core, |_ctx| {})
        };

        self.compute_shader.dispatch(&mut frame.encoder, core);
        self.base.renderer.render_to_view(
            &mut frame.encoder,
            &frame.view,
            &self.compute_shader.get_output_texture().bind_group,
        );

        self.base.apply_control_request(controls_request);
        self.base.export_manager.apply_ui_request(export_request);

        if changed {
            self.current_params = params;
            self.compute_shader.set_custom_params(params, &core.queue);
            if let Some(remote_control) = &self.remote_control {
                send_remote_values(remote_control, &params);
            }
        }

        if should_start_export {
            self.base.export_manager.start_export();
        }

        self.base.end_frame(core, frame, full_output);
        Ok(())
    }

    fn handle_input(&mut self, core: &Core, event: &WindowEvent) -> bool {
        self.base.default_handle_input(core, event)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let (app, event_loop) = ShaderApp::new("rotation_illusion", 800, 600);
    app.run(event_loop, RotoShader::init)
}

fn apply_remote_f32(params: &mut RotoParams, id: &str, value: f32) -> bool {
    match id {
        "square_size" => params.square_size = value.clamp(0.05, 0.5),
        "circle_radius" => params.circle_radius = value.clamp(0.05, 0.2),
        "edge_thickness" => params.edge_thickness = value.clamp(0.001, 0.01),
        "animation_speed" => params.animation_speed = value.clamp(1.0, 30.0),
        "edge_color_intensity" => params.edge_color_intensity = value.clamp(0.1, 2.0),
        _ => return false,
    }
    true
}

fn render_transport_widget(
    ui: &mut egui::Ui,
    remote_control: &RemoteControl,
    transport: Option<TransportState>,
) {
    let mut bpm = transport.map_or(120.0, |transport| transport.bpm);
    ui.label("Bespoke Transport");
    ui.horizontal(|ui| {
        if ui.button("Play").clicked() {
            remote_control.send_transport_playing(true);
        }
        if ui.button("Pause").clicked() {
            remote_control.send_transport_playing(false);
        }
        if ui.button("Reset").clicked() {
            remote_control.send_transport_reset();
        }
    });
    ui.horizontal(|ui| {
        if ui.button("- beat").clicked() {
            remote_control.send_transport_shift_beats(-1.0);
        }
        if ui.button("+ beat").clicked() {
            remote_control.send_transport_shift_beats(1.0);
        }
    });
    if ui
        .add(egui::Slider::new(&mut bpm, 20.0..=225.0).text("Tempo"))
        .changed()
    {
        remote_control.send_transport_tempo(bpm);
    }
    if let Some(transport) = transport {
        let mut beat = transport.beat;
        if ui
            .add(egui::Slider::new(&mut beat, 0.0..=4.0).text("Beat"))
            .changed()
        {
            remote_control.send_transport(transport.bpm, beat, transport.measure);
        }
        ui.label(format!(
            "measure {} beat {:.2}",
            transport.measure.floor() as i32,
            transport.beat
        ));
    }
}

fn send_remote_discovery(remote_control: &RemoteControl, params: &RotoParams) {
    if let Some(specs) = params_for_bin("roto") {
        remote_control.send_discovery("roto", specs);
        remote_control.send_values(specs, |id| remote_value(params, id));
    }
}

fn send_remote_values(remote_control: &RemoteControl, params: &RotoParams) {
    if let Some(specs) = params_for_bin("roto") {
        remote_control.send_values(specs, |id| remote_value(params, id));
    }
}

fn remote_value(params: &RotoParams, id: &str) -> Option<RemoteValue> {
    match id {
        "square_size" => Some(RemoteValue::F32(params.square_size)),
        "circle_radius" => Some(RemoteValue::F32(params.circle_radius)),
        "edge_thickness" => Some(RemoteValue::F32(params.edge_thickness)),
        "animation_speed" => Some(RemoteValue::F32(params.animation_speed)),
        "background_color" => Some(RemoteValue::Color3(params.background_color)),
        "edge_color_intensity" => Some(RemoteValue::F32(params.edge_color_intensity)),
        _ => None,
    }
}
