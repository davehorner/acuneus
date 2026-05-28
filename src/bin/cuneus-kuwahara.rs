use acuneus::compute::*;
use acuneus::prelude::*;

acuneus::uniform_params! {
    struct KuwaharaParams {
    radius: f32,
    q: f32,
    alpha: f32,
    filter_strength: f32,

    sigma_d: f32,
    sigma_r: f32,

    edge_threshold: f32,
    color_enhance: f32,

    blur_samples: f32,
    blur_lod: f32,
    blur_slod: f32,

    filter_mode: i32,
    show_tensors: i32,

    lic_length: f32,
    lic_strength: f32,
    lic_width: f32}
}

struct KuwaharaShader {
    base: RenderKit,
    compute_shader: ComputeShader,
    current_params: KuwaharaParams,
    remote: acuneus::remote::RemoteRuntime,
}

impl ShaderManager for KuwaharaShader {
    fn init(core: &Core) -> Self {
        let initial_params = KuwaharaParams {
            radius: 5.0,
            q: 1.5,
            alpha: 4.0,
            filter_strength: 0.8,
            sigma_d: 0.8,
            sigma_r: 1.2,
            edge_threshold: 0.2,
            color_enhance: 1.0,
            blur_samples: 15.0,
            blur_lod: 2.0,
            blur_slod: 4.0,
            filter_mode: 1,
            show_tensors: 0,
            lic_length: 15.0,
            lic_strength: 0.5,
            lic_width: 1.5,
        };
        let base = RenderKit::new(core);

        let passes = vec![
            PassDescription::new("structure_tensor", &[]),
            PassDescription::new("tensor_field", &["structure_tensor"]),
            PassDescription::new("kuwahara_filter", &["tensor_field"]),
            PassDescription::new("lic_edges", &["tensor_field", "kuwahara_filter"])
                .with_resolution_scale(0.5),
            PassDescription::new("main_image", &["lic_edges"]),
        ];

        let config = ComputeShader::builder()
            .with_entry_point("structure_tensor")
            .with_multi_pass(&passes)
            .with_custom_uniforms::<KuwaharaParams>()
            .with_workgroup_size([16, 16, 1])
            .with_texture_format(COMPUTE_TEXTURE_FORMAT_RGBA16)
            .with_channels(2)
            .with_label("Kuwahara Multi-Pass")
            .build();

        let compute_shader = acuneus::compute_shader!(core, "shaders/kuwahara.wgsl", config);

        compute_shader.set_custom_params(initial_params, &core.queue);

        Self {
            base,
            compute_shader,
            current_params: initial_params,
            remote: acuneus::remote::RemoteRuntime::new("kuwahara", 800, 600),
        }
    }

    fn update(&mut self, core: &Core) {
        let current_time = self.remote.time(&self.base);
        let delta = self.remote.delta();
        self.compute_shader
            .set_time(current_time, delta, &core.queue);

        self.base.update_current_texture(core, &core.queue);
        if let Some(texture_manager) = self.base.get_current_texture_manager() {
            self.compute_shader.update_channel_texture(
                0,
                &texture_manager.view,
                &texture_manager.sampler,
                &core.device,
                &core.queue,
            );
        }

        self.compute_shader.handle_export(core, &mut self.base);
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

        let current_fps = self.base.fps_tracker.fps();

        let full_output = if self.base.key_handler.show_ui {
            self.base.render_ui(core, |ctx| {
                RenderKit::apply_default_style(ctx);

                egui::Window::new("Filter")
                    .collapsible(true)
                    .resizable(true)
                    .default_width(320.0)
                    .show(ctx, |ui| {
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

                        let mut anisotropy_enabled = params.filter_mode == 1;
                        if ui
                            .checkbox(&mut anisotropy_enabled, "Anisotropy?")
                            .changed()
                        {
                            params.filter_mode = if anisotropy_enabled { 1 } else { 0 };
                            changed = true;
                        }

                        egui::CollapsingHeader::new("Filter Parameters")
                            .default_open(true)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.radius, 2.0..=16.0)
                                            .text("Radius"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.filter_strength, 0.0..=16.0)
                                            .text("Filter Strength"),
                                    )
                                    .changed();

                                if params.filter_mode == 1 {
                                    ui.separator();
                                    ui.label("Anisotropic Controls:");
                                    changed |= ui
                                        .add(
                                            egui::Slider::new(&mut params.alpha, 0.1..=16.0)
                                                .text("Anisotropy"),
                                        )
                                        .changed();
                                }
                            });
                        egui::CollapsingHeader::new("Blur Settings")
                            .default_open(false)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.blur_samples, 5.0..=25.0)
                                            .text("Samples"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.blur_lod, 0.0..=5.0)
                                            .text("LOD"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.blur_slod, 2.0..=5.0)
                                            .text("Step"),
                                    )
                                    .changed();
                            });

                        egui::CollapsingHeader::new("Brush Strokes (LIC)")
                            .default_open(true)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.lic_strength, 0.0..=1.0)
                                            .text("Strength"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.lic_length, 3.0..=40.0)
                                            .text("Stroke Length"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.lic_width, 0.5..=4.0)
                                            .text("Stroke Width"),
                                    )
                                    .changed();
                            });

                        egui::CollapsingHeader::new("Post-Processing")
                            .default_open(false)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.color_enhance, 0.5..=2.0)
                                            .text("Color Filter"),
                                    )
                                    .changed();

                                ui.separator();
                                if ui.button("Reset to Defaults").clicked() {
                                    params = KuwaharaParams {
                                        radius: 8.0,
                                        q: 8.0,
                                        alpha: 1.0,
                                        filter_strength: 1.0,
                                        sigma_d: 1.0,
                                        sigma_r: 2.0,
                                        edge_threshold: 0.2,
                                        color_enhance: 1.0,
                                        blur_samples: 35.0,
                                        blur_lod: 2.0,
                                        blur_slod: 4.0,
                                        filter_mode: params.filter_mode,
                                        show_tensors: 0,
                                        lic_length: 15.0,
                                        lic_strength: 0.5,
                                        lic_width: 1.5,
                                    };
                                    changed = true;
                                }
                            });

                        ui.separator();

                        ShaderControls::render_controls_widget(ui, &mut controls_request);

                        ui.separator();

                        should_start_export =
                            ExportManager::render_export_ui_widget(ui, &mut export_request);

                        ui.separator();
                        ui.label(format!(
                            "Resolution: {}x{}",
                            core.size.width, core.size.height
                        ));
                        ui.label(format!("FPS: {current_fps:.1}"));
                    });
            })
        } else {
            self.base.render_ui(core, |_ctx| {})
        };

        self.base.apply_media_requests(core, &controls_request);

        self.base.export_manager.apply_ui_request(export_request);
        if should_start_export {
            self.base.export_manager.start_export();
        }

        if changed {
            self.current_params = params;
            self.compute_shader.set_custom_params(params, &core.queue);
            self.remote.send_values(&params);
        }

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
    let (app, event_loop) = ShaderApp::new("Kuwahara Filter", 800, 600);

    app.run(event_loop, KuwaharaShader::init)
}
