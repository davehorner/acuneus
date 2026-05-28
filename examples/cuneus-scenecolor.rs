use acuneus::compute::*;
use acuneus::prelude::*;

acuneus::uniform_params! {
    struct SceneColorParams {
    num_segments: f32,
    palette_height: f32,
    samples_x: i32,
    samples_y: i32,

    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
    _pad4: f32}
}

struct SceneColorShader {
    base: RenderKit,
    compute_shader: ComputeShader,
    current_params: SceneColorParams,
    remote: acuneus::remote::RemoteRuntime,
}

impl SceneColorShader {
    fn clear_buffers(&mut self, core: &Core) {
        self.compute_shader.clear_all_buffers(core);
    }
}

impl ShaderManager for SceneColorShader {
    fn init(core: &Core) -> Self {
        let initial_params = SceneColorParams {
            num_segments: 16.0,
            palette_height: 0.2,
            samples_x: 8,
            samples_y: 8,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
            _pad4: 0.0,
        };

        let base = RenderKit::new(core);

        let config = ComputeShader::builder()
            .with_entry_point("main")
            .with_input_texture() // Enable input texture support
            .with_custom_uniforms::<SceneColorParams>()
            .with_workgroup_size([16, 16, 1])
            .with_texture_format(COMPUTE_TEXTURE_FORMAT_RGBA16)
            .with_label("Scene Color Unified")
            .build();

        let compute_shader = acuneus::compute_shader!(core, "shaders/scenecolor.wgsl", config);

        compute_shader.set_custom_params(initial_params, &core.queue);

        Self {
            base,
            compute_shader,
            current_params: initial_params,
            remote: acuneus::remote::RemoteRuntime::new("scenecolor", 800, 600),
        }
    }

    fn update(&mut self, core: &Core) {
        // Update time
        let current_time = self.remote.time(&self.base);
        let delta = self.remote.delta();
        self.compute_shader
            .set_time(current_time, delta, &core.queue);

        // Update input textures for media processing
        self.base.update_current_texture(core, &core.queue);
        if let Some(texture_manager) = self.base.get_current_texture_manager() {
            self.compute_shader.update_input_texture(
                &texture_manager.view,
                &texture_manager.sampler,
                &core.device,
            );
        }
    }

    fn resize(&mut self, core: &Core) {
        self.base.default_resize(core, &mut self.compute_shader);
    }

    fn render(&mut self, core: &Core) -> Result<(), acuneus::SurfaceError> {
        let mut frame = self.base.begin_frame(core)?;

        let mut params = self.current_params;
        let mut changed = false;
        changed |= self
            .remote
            .drain(core, &mut self.base, &mut self.compute_shader, &mut params);
        let mut should_start_export = false;
        let mut export_request = self.base.export_manager.get_ui_request();
        let remote_size = self.remote.resolution_size(core);
        let mut controls_request = self.base.controls.get_ui_request(
            &self.base.start_time,
            &remote_size,
            self.base.fps_tracker.fps(),
        );
        self.remote.apply_to_controls_request(&mut controls_request);

        let using_video_texture = self.base.using_video_texture;
        let using_hdri_texture = self.base.using_hdri_texture;
        let using_webcam_texture = self.base.using_webcam_texture;
        let video_info = self.base.get_video_info();
        let hdri_info = self.base.get_hdri_info();
        let webcam_info = self.base.get_webcam_info();

        let full_output = if self.base.key_handler.show_ui {
            self.base.render_ui(core, |ctx| {
                RenderKit::apply_default_style(ctx);

                egui::Window::new("Scene Color Palette")
                    .collapsible(true)
                    .resizable(true)
                    .default_width(280.0)
                    .show(ctx, |ui| {
                        // Media controls
                        ShaderControls::render_media_panel(
                            ui,
                            &mut controls_request,
                            using_video_texture,
                            video_info,
                            using_hdri_texture,
                            hdri_info,
                            using_webcam_texture,
                            webcam_info,
                        );

                        ui.separator();

                        egui::CollapsingHeader::new("Palette Parameters")
                            .default_open(true)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.num_segments, 1.0..=64.0)
                                            .text("Segments"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.palette_height, 0.05..=0.5)
                                            .text("Height"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.samples_x, 1..=32)
                                            .text("Samples X"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.samples_y, 1..=32)
                                            .text("Samples Y"),
                                    )
                                    .changed();
                            });

                        ui.separator();
                        ShaderControls::render_controls_widget(ui, &mut controls_request);

                        ui.separator();
                        should_start_export =
                            ExportManager::render_export_ui_widget(ui, &mut export_request);

                        ui.separator();
                        ui.label("Color palette extractor from scene");
                    });
            })
        } else {
            self.base.render_ui(core, |_ctx| {})
        };

        // Apply controls
        self.base.export_manager.apply_ui_request(export_request);
        if controls_request.should_clear_buffers {
            self.clear_buffers(core);
        }
        self.base.apply_media_requests(core, &controls_request);

        if changed {
            self.current_params = params;
            self.compute_shader.set_custom_params(params, &core.queue);
            self.remote.send_values(&params);
        }

        if should_start_export {
            self.base.export_manager.start_export();
        }

        // Single stage dispatch
        self.compute_shader.dispatch(&mut frame.encoder, core);

        self.base.renderer.render_to_view(
            &mut frame.encoder,
            &frame.view,
            &self.compute_shader.get_output_texture().bind_group,
        );

        self.base.end_frame(core, frame, full_output);

        Ok(())
    }

    fn handle_input(&mut self, core: &Core, event: &WindowEvent) -> bool {
        self.base.default_handle_input(core, event)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    acuneus::gst::init()?;
    let _ = env_logger::try_init();
    let (app, event_loop) = ShaderApp::new("Scene Color Palette", 800, 600);
    app.run(event_loop, SceneColorShader::init)
}
