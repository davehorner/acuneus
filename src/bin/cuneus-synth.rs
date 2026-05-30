use acuneus::audio::PcmStreamManager;
use acuneus::compute::*;
use acuneus::prelude::*;
use log::error;

const MAX_SAMPLES_PER_FRAME: u32 = 1024;
const SAMPLE_RATE: u32 = 44100;

acuneus::uniform_params! {
    struct SynthParams {
        tempo: f32,
        waveform_type: u32,
        octave: f32,
        volume: f32,
        beat_enabled: u32,
        reverb_mix: f32,
        delay_time: f32,
        delay_feedback: f32,
        filter_cutoff: f32,
        filter_resonance: f32,
        distortion_amount: f32,
        chorus_rate: f32,
        chorus_depth: f32,
        attack_time: f32,
        decay_time: f32,
        sustain_level: f32,
        release_time: f32,
        sample_offset: u32,
        samples_to_generate: u32,
        sample_rate: u32,
        local_audio_enabled: u32,
        _padding0: u32,
        _padding1: u32,
        _padding2: u32,
        key_states: [[f32; 4]; 3],
        key_decay: [[f32; 4]; 3],
    }
}

struct SynthManager {
    base: RenderKit,
    compute_shader: ComputeShader,
    current_params: SynthParams,
    remote: acuneus::remote::RemoteRuntime,
    pcm_stream: Option<PcmStreamManager>,
    keys_held: [bool; 9],
    remote_keys_held: [bool; 9],
    audio_start: std::time::Instant,
    last_samples_generated: u32,
    samples_written: u64,
}

impl SynthManager {
    fn start_pcm_stream() -> Option<PcmStreamManager> {
        match PcmStreamManager::new(Some(SAMPLE_RATE)) {
            Ok(mut stream) => {
                if let Err(e) = stream.start() {
                    error!("Failed to start PCM stream: {e}");
                    None
                } else {
                    Some(stream)
                }
            }
            Err(e) => {
                error!("Failed to create PCM stream: {e}");
                None
            }
        }
    }

    fn sync_local_audio_stream(&mut self) {
        if self.current_params.local_audio_enabled > 0 {
            if self.pcm_stream.is_none() {
                self.pcm_stream = Self::start_pcm_stream();
            }
        } else if let Some(mut stream) = self.pcm_stream.take() {
            let _ = stream.stop();
        }
    }

    /// key_states stores note-on time (>0 = pressed at this time, 0 = never pressed)
    fn set_key_press_time(&mut self, key_index: usize, time: f32) {
        if key_index < 9 {
            self.current_params.key_states[key_index / 4][key_index % 4] = time;
        }
    }

    /// key_decay stores release time (>0 = released at this time, 0 = still held or idle)
    fn set_key_release_time(&mut self, key_index: usize, time: f32) {
        if key_index < 9 {
            self.current_params.key_decay[key_index / 4][key_index % 4] = time;
        }
    }

    fn key_index_for_pitch(pitch: f32) -> Option<usize> {
        let mut offset = pitch.round() as i32 - 60;
        while offset < 0 {
            offset += 12;
        }

        match offset {
            0 => Some(0),
            2 => Some(1),
            4 => Some(2),
            5 => Some(3),
            7 => Some(4),
            9 => Some(5),
            11 => Some(6),
            12 => Some(7),
            14 => Some(8),
            _ => match offset % 12 {
                0 => Some(0),
                2 => Some(1),
                4 => Some(2),
                5 => Some(3),
                7 => Some(4),
                9 => Some(5),
                11 => Some(6),
                _ => None,
            },
        }
    }

    fn set_key_pressed(&mut self, index: usize, current_time: f32) {
        if index >= 9 {
            return;
        }

        let has_previous = self.current_params.key_states[index / 4][index % 4] > 0.0;
        let in_release = self.current_params.key_decay[index / 4][index % 4] > 0.0;
        self.keys_held[index] = true;
        if has_previous && in_release {
            self.set_key_release_time(index, 0.0);
        } else {
            self.set_key_press_time(index, current_time);
            self.set_key_release_time(index, 0.0);
        }
    }

    fn set_key_released(&mut self, index: usize, current_time: f32) {
        if index >= 9 {
            return;
        }

        self.keys_held[index] = false;
        self.set_key_release_time(index, current_time);
    }

    fn apply_remote_note(&mut self, pitch: f32, velocity: f32, current_time: f32) {
        let Some(index) = Self::key_index_for_pitch(pitch) else {
            return;
        };

        if velocity > 0.0 {
            self.set_key_pressed(index, current_time);
        } else {
            self.set_key_released(index, current_time);
        }
    }

    fn sync_remote_keys(&mut self, current_time: f32) -> bool {
        let mut changed = false;
        for index in 0..9 {
            let id = format!("key_{}", index + 1);
            let down = self.remote.key_down(&id);
            if down == self.remote_keys_held[index] {
                continue;
            }

            self.remote_keys_held[index] = down;
            if down {
                self.set_key_pressed(index, current_time);
            } else {
                self.set_key_released(index, current_time);
            }
            changed = true;
        }
        changed
    }
}

impl ShaderManager for SynthManager {
    fn init(core: &Core) -> Self {
        let base = RenderKit::new(core);
        let remote = acuneus::remote::RemoteRuntime::new("synth", 800, 600);

        let initial_params = SynthParams {
            tempo: 120.0,
            waveform_type: 1,
            octave: 4.0,
            volume: 0.7,
            beat_enabled: 0,
            reverb_mix: 0.15,
            delay_time: 0.3,
            delay_feedback: 0.3,
            filter_cutoff: 0.9,
            filter_resonance: 0.1,
            distortion_amount: 0.0,
            chorus_rate: 2.0,
            chorus_depth: 0.1,
            attack_time: 0.02,
            decay_time: 0.15,
            sustain_level: 0.7,
            release_time: 0.4,
            sample_offset: 0,
            samples_to_generate: MAX_SAMPLES_PER_FRAME,
            sample_rate: SAMPLE_RATE,
            local_audio_enabled: u32::from(!remote.has_feedback_target()),
            _padding0: 0,
            _padding1: 0,
            _padding2: 0,
            key_states: [[0.0; 4]; 3],
            key_decay: [[0.0; 4]; 3],
        };

        let audio_buffer_size = (MAX_SAMPLES_PER_FRAME * 2) as usize;

        let config = ComputeShader::builder()
            .with_entry_point("main")
            .with_custom_uniforms::<SynthParams>()
            .with_audio(audio_buffer_size)
            .with_workgroup_size([16, 16, 1])
            .with_texture_format(COMPUTE_TEXTURE_FORMAT_RGBA16)
            .with_label("Synth")
            .build();

        let compute_shader = acuneus::compute_shader!(core, "shaders/synth.wgsl", config);
        compute_shader.set_custom_params(initial_params, &core.queue);

        let pcm_stream = if initial_params.local_audio_enabled > 0 {
            Self::start_pcm_stream()
        } else {
            None
        };

        Self {
            base,
            compute_shader,
            current_params: initial_params,
            remote,
            pcm_stream,
            keys_held: [false; 9],
            remote_keys_held: [false; 9],
            audio_start: std::time::Instant::now(),
            last_samples_generated: 0,
            samples_written: 0,
        }
    }

    fn update(&mut self, core: &Core) {
        let current_time = self.remote.time(&self.base);
        let delta = self.remote.delta();
        self.compute_shader
            .set_time(current_time, delta, &core.queue);
        self.sync_local_audio_stream();

        if let Some(ref mut stream) = self.pcm_stream {
            stream.set_master_volume(self.current_params.volume as f64);
        }

        let prev = self.last_samples_generated;
        if prev > 0 {
            if let Ok(audio_data) = pollster::block_on(
                self.compute_shader
                    .read_audio_buffer(&core.device, &core.queue),
            ) {
                let count = (prev * 2) as usize;
                if audio_data.len() >= count {
                    self.remote.send_audio_pcm(&audio_data[..count]);
                    if let Some(ref mut stream) = self.pcm_stream {
                        let _ = stream.push_samples(&audio_data[..count]);
                    }
                    self.samples_written = self.samples_written.saturating_add(prev as u64);
                }
            }
        }

        let elapsed = self.audio_start.elapsed().as_secs_f64();
        let target_samples = (elapsed * SAMPLE_RATE as f64) as u64;
        let written = self.samples_written;
        let needed = (target_samples.saturating_sub(written) as u32).min(MAX_SAMPLES_PER_FRAME);
        self.current_params.sample_offset = written as u32;
        self.current_params.samples_to_generate = needed;
        self.last_samples_generated = needed;

        self.compute_shader
            .set_custom_params(self.current_params, &core.queue);
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

                egui::Window::new("GPU Synth")
                    .collapsible(true)
                    .resizable(true)
                    .default_width(280.0)
                    .show(ctx, |ui| {
                        egui::CollapsingHeader::new("Playback")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label("Keys 1-9: C D E F G A B C D");

                                let mut beat_enabled = params.beat_enabled > 0;
                                if ui.checkbox(&mut beat_enabled, "Background Beat").changed() {
                                    params.beat_enabled = u32::from(beat_enabled);
                                    changed = true;
                                }
                                let mut local_audio_enabled = params.local_audio_enabled > 0;
                                if ui
                                    .checkbox(&mut local_audio_enabled, "Local Audio")
                                    .changed()
                                {
                                    params.local_audio_enabled = u32::from(local_audio_enabled);
                                    changed = true;
                                }

                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.tempo, 60.0..=180.0)
                                            .text("Tempo"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.octave, 2.0..=7.0)
                                            .text("Octave"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.volume, 0.0..=1.0)
                                            .text("Volume"),
                                    )
                                    .changed();

                                ui.horizontal(|ui| {
                                    ui.label("Wave:");
                                    for (i, name) in
                                        ["Sin", "Saw", "Sqr", "Tri", "Nse"].iter().enumerate()
                                    {
                                        if ui
                                            .selectable_label(
                                                params.waveform_type == i as u32,
                                                *name,
                                            )
                                            .clicked()
                                        {
                                            params.waveform_type = i as u32;
                                            changed = true;
                                        }
                                    }
                                });
                            });

                        egui::CollapsingHeader::new("Envelope (ADSR)")
                            .default_open(true)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.attack_time, 0.001..=0.5)
                                            .logarithmic(true)
                                            .text("Attack")
                                            .suffix("s"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.decay_time, 0.01..=1.0)
                                            .logarithmic(true)
                                            .text("Decay")
                                            .suffix("s"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.sustain_level, 0.0..=1.0)
                                            .text("Sustain"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.release_time, 0.01..=2.0)
                                            .logarithmic(true)
                                            .text("Release")
                                            .suffix("s"),
                                    )
                                    .changed();

                                ui.separator();
                                if ui.small_button("Piano").clicked() {
                                    params.attack_time = 0.01;
                                    params.decay_time = 0.3;
                                    params.sustain_level = 0.5;
                                    params.release_time = 0.8;
                                    changed = true;
                                }
                                ui.horizontal(|ui| {
                                    if ui.small_button("Pad").clicked() {
                                        params.attack_time = 0.2;
                                        params.decay_time = 0.5;
                                        params.sustain_level = 0.8;
                                        params.release_time = 1.5;
                                        changed = true;
                                    }
                                    if ui.small_button("Pluck").clicked() {
                                        params.attack_time = 0.005;
                                        params.decay_time = 0.1;
                                        params.sustain_level = 0.3;
                                        params.release_time = 0.2;
                                        changed = true;
                                    }
                                });
                            });

                        egui::CollapsingHeader::new("Filter")
                            .default_open(false)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.filter_cutoff, 0.0..=1.0)
                                            .text("Cutoff"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.filter_resonance, 0.0..=0.9)
                                            .text("Resonance"),
                                    )
                                    .changed();
                            });

                        egui::CollapsingHeader::new("Effects")
                            .default_open(false)
                            .show(ui, |ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.reverb_mix, 0.0..=0.8)
                                            .text("Reverb"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.delay_time, 0.01..=1.0)
                                            .text("Delay"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.delay_feedback, 0.0..=0.8)
                                            .text("Feedback"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.distortion_amount, 0.0..=0.9)
                                            .text("Distortion"),
                                    )
                                    .changed();
                                ui.separator();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.chorus_rate, 0.1..=10.0)
                                            .text("Chorus Rate"),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut params.chorus_depth, 0.0..=0.5)
                                            .text("Chorus Depth"),
                                    )
                                    .changed();
                            });

                        ui.separator();
                        ShaderControls::render_controls_widget(ui, &mut controls_request);
                    });
            })
        } else {
            self.base.render_ui(core, |_ctx| {})
        };

        if changed {
            // Preserve audio fields that are managed by update()
            params.sample_offset = self.current_params.sample_offset;
            params.samples_to_generate = self.current_params.samples_to_generate;
            params.sample_rate = self.current_params.sample_rate;
            params.local_audio_enabled = self.current_params.local_audio_enabled;
            params._padding0 = self.current_params._padding0;
            params._padding1 = self.current_params._padding1;
            params._padding2 = self.current_params._padding2;
            params.key_states = self.current_params.key_states;
            params.key_decay = self.current_params.key_decay;
            self.current_params = params;
        }

        let current_time = self.remote.time(&self.base);
        let remote_notes = self.remote.take_notes();
        let had_remote_notes = !remote_notes.is_empty();
        if had_remote_notes {
            for (pitch, velocity) in remote_notes {
                self.apply_remote_note(pitch, velocity, current_time);
            }
        }
        let remote_keys_changed = self.sync_remote_keys(current_time);
        if had_remote_notes || remote_keys_changed {
            self.compute_shader
                .set_custom_params(self.current_params, &core.queue);
        }

        self.base.apply_control_request(controls_request);

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
        if self.base.forward_to_egui(core, event) {
            return true;
        }

        if let WindowEvent::KeyboardInput { event, .. } = event {
            if let winit::keyboard::Key::Character(ref s) = event.logical_key {
                if let Some(key_index) = s.chars().next().and_then(|c| c.to_digit(10)) {
                    if (1..=9).contains(&key_index) {
                        let index = (key_index - 1) as usize;

                        let current_time = self.remote.time(&self.base);
                        if event.state == winit::event::ElementState::Pressed
                            && !self.keys_held[index]
                        {
                            self.set_key_pressed(index, current_time);
                            self.compute_shader
                                .set_custom_params(self.current_params, &core.queue);
                        } else if event.state == winit::event::ElementState::Released {
                            self.set_key_released(index, current_time);
                            self.compute_shader
                                .set_custom_params(self.current_params, &core.queue);
                        }
                        return true;
                    }
                }
            }
            return self
                .base
                .key_handler
                .handle_keyboard_input(core.window(), event);
        }

        false
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    acuneus::gst::init()?;

    let (app, event_loop) = ShaderApp::new("Synth", 800, 600);
    app.run(event_loop, SynthManager::init)
}
