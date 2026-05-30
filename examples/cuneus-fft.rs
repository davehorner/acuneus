use acuneus::compute::{
    ComputeShader, PassDescription, StorageBufferSpec, COMPUTE_TEXTURE_FORMAT_RGBA16,
};
use acuneus::WindowEvent;
use acuneus::{Core, ExportManager, RenderKit, ShaderControls, ShaderManager};
use log::error;

acuneus::uniform_params! {
    struct FFTParams {
    filter_type: i32,
    filter_strength: f32,
    filter_direction: f32,
    filter_radius: f32,
    show_freqs: i32,
    resolution: u32,
    is_bw: i32,
    _padding: u32}
}

struct FFTShader {
    base: RenderKit,
    compute_shader: ComputeShader,
    should_initialize: bool,
    current_params: FFTParams, // Store current parameters
    remote: acuneus::remote::RemoteRuntime,
}

impl FFTShader {
    fn sanitize_params(params: &mut FFTParams) -> bool {
        let old_filter_type = params.filter_type;
        let old_filter_direction = params.filter_direction;
        let old_filter_radius = params.filter_radius;
        let old_show_freqs = params.show_freqs;
        let old_resolution = params.resolution;
        let old_is_bw = params.is_bw;

        params.filter_type = params.filter_type.clamp(0, 3);
        params.filter_direction = params.filter_direction.clamp(0.0, std::f32::consts::TAU);
        params.filter_radius = params.filter_radius.clamp(0.0, std::f32::consts::TAU);
        params.show_freqs = if params.show_freqs >= 1 { 1 } else { 0 };
        params.is_bw = if params.is_bw >= 1 { 1 } else { 0 };

        const RESOLUTIONS: [u32; 4] = [256, 512, 1024, 2048];
        params.resolution = RESOLUTIONS
            .into_iter()
            .min_by_key(|resolution| resolution.abs_diff(params.resolution))
            .unwrap_or(1024);

        old_filter_type != params.filter_type
            || old_filter_direction != params.filter_direction
            || old_filter_radius != params.filter_radius
            || old_show_freqs != params.show_freqs
            || old_resolution != params.resolution
            || old_is_bw != params.is_bw
    }
}

impl ShaderManager for FFTShader {
    fn init(core: &Core) -> Self {
        let initial_params = FFTParams {
            filter_type: 1,
            filter_strength: 0.3,
            filter_direction: 0.0,
            filter_radius: 3.0,
            show_freqs: 0,
            resolution: 1024,
            is_bw: 0,
            _padding: 0,
        };
        let base = RenderKit::new(core);

        // Define the FFT multi-pass pipeline
        let passes = vec![
            PassDescription::new("initialize_data", &[]), // Stage 0: Initialize from input texture
            PassDescription::new("fft_horizontal", &["initialize_data"]), // Stage 1: FFT horizontal pass
            PassDescription::new("fft_vertical", &["fft_horizontal"]), // Stage 2: FFT vertical pass
            PassDescription::new("modify_frequencies", &["fft_vertical"]), // Stage 3: Apply frequency domain filters
            PassDescription::new("ifft_horizontal", &["modify_frequencies"]), // Stage 4: Inverse FFT horizontal
            PassDescription::new("ifft_vertical", &["ifft_horizontal"]), // Stage 5: Inverse FFT vertical
            PassDescription::new("main_image", &["ifft_vertical"]),      // Stage 6: Final display
        ];

        let config = ComputeShader::builder()
            .with_entry_point("initialize_data") // Start with data initialization
            .with_multi_pass(&passes)
            .with_input_texture() // Re-enable input texture support
            .with_custom_uniforms::<FFTParams>()
            .with_audio_spectrum(69)
            .with_storage_buffer(StorageBufferSpec::new("image_data", 2048 * 2048 * 3 * 8)) // FFT working memory: max res to avoid crash
            .with_workgroup_size([16, 16, 1])
            .with_texture_format(COMPUTE_TEXTURE_FORMAT_RGBA16)
            .with_label("FFT Multi-Pass")
            .build();

        let compute_shader = acuneus::compute_shader!(core, "shaders/fft.wgsl", config);

        // Initialize custom uniform with initial parameters
        compute_shader.set_custom_params(initial_params, &core.queue);

        Self {
            base,
            compute_shader,
            should_initialize: true,
            current_params: initial_params,
            remote: acuneus::remote::RemoteRuntime::new("fft", 800, 600),
        }
    }

    fn update(&mut self, core: &Core) {
        // Update time
        let current_time = self.remote.time(&self.base);
        let delta = self.remote.delta();
        self.compute_shader
            .set_time(current_time, delta, &core.queue);

        // Update input textures for image proc.
        self.base.update_current_texture(core, &core.queue);
        if let Some(texture_manager) = self.base.get_current_texture_manager() {
            // Update input texture in unified ComputeShader
            self.compute_shader.update_input_texture(
                &texture_manager.view,
                &texture_manager.sampler,
                &core.device,
            );
        }
        self.base.update_audio_spectrum(&core.queue);
        self.compute_shader
            .update_audio_spectrum(&self.base.resolution_uniform.data, &core.queue);
        self.remote
            .send_audio_spectrum(&self.base.resolution_uniform.data);
        // Handle export
        self.compute_shader.handle_export(core, &mut self.base);
    }

    fn resize(&mut self, core: &Core) {
        self.compute_shader
            .resize(core, core.size.width, core.size.height);
    }

    fn render(&mut self, core: &Core) -> Result<(), acuneus::SurfaceError> {
        let mut frame = self.base.begin_frame(core)?;

        // Handle UI and controls - using original transparent UI design
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

                egui::Window::new("fourier workflow")
                    .collapsible(true)
                    .resizable(true)
                    .default_width(250.0)
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

                        egui::CollapsingHeader::new("FFT Settings")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.label("Resolution:");

                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .radio_value(&mut params.resolution, 256, "256")
                                        .changed();
                                    changed |= ui
                                        .radio_value(&mut params.resolution, 512, "512")
                                        .changed();
                                    changed |= ui
                                        .radio_value(&mut params.resolution, 1024, "1024")
                                        .changed();
                                    changed |= ui
                                        .radio_value(&mut params.resolution, 2048, "2048")
                                        .changed();
                                });

                                if changed {
                                    self.should_initialize = true;
                                }

                                ui.separator();
                                ui.label("View Mode:");
                                changed |= ui
                                    .radio_value(&mut params.show_freqs, 0, "Filtered")
                                    .changed();
                                changed |= ui
                                    .radio_value(&mut params.show_freqs, 1, "Frequency Domain")
                                    .changed();

                                let mut is_bw_bool = params.is_bw != 0;
                                if ui.checkbox(&mut is_bw_bool, "Black & White").changed() {
                                    params.is_bw = if is_bw_bool { 1 } else { 0 };
                                    changed = true;
                                }

                                ui.separator();
                            });

                        egui::CollapsingHeader::new("Filter Settings")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.label("Filter Type:");
                                // Keep the improved ComboBox as requested
                                changed |= egui::ComboBox::from_label("")
                                    .selected_text(match params.filter_type {
                                        0 => "LP",
                                        1 => "HP",
                                        2 => "BP",
                                        3 => "Directional",
                                        _ => "None",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut params.filter_type, 0, "LP")
                                            .changed()
                                            || ui
                                                .selectable_value(&mut params.filter_type, 1, "HP")
                                                .changed()
                                            || ui
                                                .selectable_value(&mut params.filter_type, 2, "BP")
                                                .changed()
                                            || ui
                                                .selectable_value(
                                                    &mut params.filter_type,
                                                    3,
                                                    "Directional",
                                                )
                                                .changed()
                                    })
                                    .inner
                                    .unwrap_or(false);

                                ui.separator();

                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.filter_strength, 0.0..=1.0)
                                            .text("Filter Strength"),
                                    )
                                    .changed();

                                if params.filter_type == 2 {
                                    changed |= ui
                                        .add(
                                            egui::Slider::new(
                                                &mut params.filter_radius,
                                                0.0..=6.28,
                                            )
                                            .text("Band Radius"),
                                        )
                                        .changed();
                                }

                                if params.filter_type == 3 {
                                    changed |= ui
                                        .add(
                                            egui::Slider::new(
                                                &mut params.filter_direction,
                                                0.0..=6.28,
                                            )
                                            .text("Direction"),
                                        )
                                        .changed();
                                }
                            });

                        ui.separator();

                        ShaderControls::render_controls_widget(ui, &mut controls_request);

                        ui.separator();

                        should_start_export =
                            ExportManager::render_export_ui_widget(ui, &mut export_request);
                    });
            })
        } else {
            self.base.render_ui(core, |_ctx| {})
        };

        // Keep current parameters - don't reset to defaults
        // The UI will modify 'params' directly, and we'll apply changes at the end

        // Apply controls
        self.base.apply_media_requests(core, &controls_request);

        // Handle export requests
        self.base.export_manager.apply_ui_request(export_request);
        if should_start_export {
            self.base.export_manager.start_export();
        }

        if controls_request.load_media_path.is_some() {
            self.should_initialize = true;
        }
        if controls_request.start_webcam {
            self.should_initialize = true;
        }

        if Self::sanitize_params(&mut params) {
            changed = true;
        }

        // Apply parameter changes
        if changed {
            self.current_params = params;
            self.compute_shader.set_custom_params(params, &core.queue);
            self.should_initialize = true; // Trigger FFT reprocessing
        }

        // FFT dispatch - only run full pipeline when needed, otherwise just display
        let mut should_run_full_fft = self.should_initialize
            || self.base.using_video_texture
            || self.base.using_webcam_texture
            || changed; // Also run when parameters change

        // FORCE run FFT if there's any texture to debug the issue
        let has_any_texture = self.base.get_current_texture_manager().is_some();
        if has_any_texture && !should_run_full_fft {
            should_run_full_fft = true;
        }
        // Get FFT resolution for proper workgroup calculation
        let n = params.resolution;
        if should_run_full_fft {
            // Stage 0: Initialize data from input texture (16x16 workgroups)
            self.compute_shader.dispatch_stage_with_workgroups(
                &mut frame.encoder,
                0,
                [n.div_ceil(16), n.div_ceil(16), 1],
            );

            // Stage 1: FFT horizontal (Nx1 workgroups)
            self.compute_shader
                .dispatch_stage_with_workgroups(&mut frame.encoder, 1, [n, 1, 1]);

            // Stage 2: FFT vertical (Nx1 workgroups)
            self.compute_shader
                .dispatch_stage_with_workgroups(&mut frame.encoder, 2, [n, 1, 1]);

            // Stage 3: Modify frequencies - apply filter (16x16 workgroups)
            self.compute_shader.dispatch_stage_with_workgroups(
                &mut frame.encoder,
                3,
                [n.div_ceil(16), n.div_ceil(16), 1],
            );

            if params.show_freqs == 0 {
                // Stage 4: Inverse FFT horizontal
                self.compute_shader.dispatch_stage_with_workgroups(
                    &mut frame.encoder,
                    4,
                    [n, 1, 1],
                );

                // Stage 5: Inverse FFT vertical
                self.compute_shader.dispatch_stage_with_workgroups(
                    &mut frame.encoder,
                    5,
                    [n, 1, 1],
                );
            }

            self.should_initialize = false;
            log::info!("Completed full FFT pipeline");
        } else {
            log::debug!("Skipping full FFT pipeline - using cached result");
        }

        // Stage 6: Main rendering - always run for display (uses screen size)
        self.compute_shader
            .dispatch_stage(&mut frame.encoder, core, 6);

        self.base.renderer.render_to_view(
            &mut frame.encoder,
            &frame.view,
            &self.compute_shader.get_output_texture().bind_group,
        );

        self.base.end_frame(core, frame, full_output);

        Ok(())
    }

    fn handle_input(&mut self, core: &Core, event: &WindowEvent) -> bool {
        if let WindowEvent::DroppedFile(path) = event {
            if let Err(e) = self.base.load_media(core, path) {
                error!("Failed to load dropped file: {e:?}");
            } else {
                self.should_initialize = true;
            }
            return true;
        }
        self.base.default_handle_input(core, event)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    acuneus::gst::init()?;
    let _ = env_logger::try_init();
    let (app, event_loop) = acuneus::ShaderApp::new("FFT", 800, 600);
    app.run(event_loop, FFTShader::init)
}
