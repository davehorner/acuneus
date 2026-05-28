use acuneus::compute::*;
use acuneus::prelude::*;

acuneus::uniform_params! {
    struct TreeParams {
    pixel_offset: f32,
    pixel_offset2: f32,
    lights: f32,
    exp: f32,
    frame: f32,
    col1: f32,
    col2: f32,
    decay: f32}
}

struct TreeShader {
    base: RenderKit,
    compute_shader: ComputeShader,
    current_params: TreeParams,
    remote: acuneus::remote::RemoteRuntime,
}

impl ShaderManager for TreeShader {
    fn init(core: &Core) -> Self {
        let initial_params = TreeParams {
            pixel_offset: 1.35,
            pixel_offset2: 1.0,
            lights: 2.2,
            exp: 4.0,
            frame: 0.5,
            col1: 205.0,
            col2: 5.5,
            decay: 0.96,
        };
        let base = RenderKit::new(core);

        // Create multipass system: fractal -> gradient -> trace -> main_image
        let passes = vec![
            PassDescription::new("fractal", &[]), // no dependencies, generates fractal
            PassDescription::new("gradient", &["fractal"]), // reads fractal
            PassDescription::new("trace", &["trace", "gradient"]), // self-feedback + gradient
            PassDescription::new("main_image", &["trace"]),
        ];

        let config = ComputeShader::builder()
            .with_entry_point("fractal")
            .with_multi_pass(&passes)
            .with_custom_uniforms::<TreeParams>()
            .with_workgroup_size([16, 16, 1])
            .with_texture_format(COMPUTE_TEXTURE_FORMAT_RGBA16)
            .with_label("Tree Unified")
            .build();

        let compute_shader = acuneus::compute_shader!(core, "shaders/tree.wgsl", config);

        compute_shader.set_custom_params(initial_params, &core.queue);

        Self {
            base,
            compute_shader,
            current_params: initial_params,
            remote: acuneus::remote::RemoteRuntime::new("tree", 800, 600),
        }
    }

    fn update(&mut self, core: &Core) {
        // Handle export
        self.compute_shader.handle_export(core, &mut self.base);

        // Update time uniform - this is crucial for accumulation!
        let current_time = self.remote.time(&self.base);
        let delta = self.remote.delta(); // Approximate delta time
        self.compute_shader
            .set_time(current_time, delta, &core.queue);
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

        let full_output = if self.base.key_handler.show_ui {
            self.base.render_ui(core, |ctx| {
                RenderKit::apply_default_style(ctx);

                egui::Window::new("Fractal Tree")
                    .collapsible(true)
                    .resizable(true)
                    .default_width(280.0)
                    .show(ctx, |ui| {
                        egui::CollapsingHeader::new("Fractal Parameters")
                            .default_open(true)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.pixel_offset, -3.14..=3.14)
                                            .text("Pixel Offset Y"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.pixel_offset2, -3.14..=3.14)
                                            .text("Pixel Offset X"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.lights, 0.0..=12.2)
                                            .text("Lights"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.exp, 1.0..=120.0).text("Exp"),
                                    )
                                    .changed();
                            });

                        egui::CollapsingHeader::new("Visual Settings")
                            .default_open(false)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.frame, 0.0..=2.2)
                                            .text("Frame"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.col1, 0.0..=300.0)
                                            .text("Iterations"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.col2, 0.0..=10.0)
                                            .text("Color 2"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.decay, 0.0..=1.0)
                                            .text("Feedback"),
                                    )
                                    .changed();
                            });

                        ui.separator();
                        ShaderControls::render_controls_widget(ui, &mut controls_request);

                        ui.separator();
                        should_start_export =
                            ExportManager::render_export_ui_widget(ui, &mut export_request);

                        ui.separator();
                        ui.label(format!("Frame: {}", self.compute_shader.current_frame));
                        ui.label("Multi-buffer fractal tree with particle tracing");
                    });
            })
        } else {
            self.base.render_ui(core, |_ctx| {})
        };

        // Handle controls and clear buffers if requested
        if controls_request.should_clear_buffers {
            // Reset frame count to restart accumulation
            self.compute_shader.current_frame = 0;
        }

        // Execute multi-pass compute shader: fractal -> gradient -> trace -> main_image
        self.compute_shader.dispatch(&mut frame.encoder, core);

        self.base.renderer.render_to_view(
            &mut frame.encoder,
            &frame.view,
            &self.compute_shader.get_output_texture().bind_group,
        );

        // Apply UI changes
        self.base.apply_control_request(controls_request);
        self.base.export_manager.apply_ui_request(export_request);

        if changed {
            self.current_params = params;
            self.compute_shader.set_custom_params(params, &core.queue);
            self.remote.send_values(&params);
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
    let (app, event_loop) = ShaderApp::new("Fractal Tree", 800, 600);
    app.run(event_loop, TreeShader::init)
}
