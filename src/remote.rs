use std::collections::VecDeque;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;

use rosc::{encoder, OscMessage, OscPacket, OscType};

pub use crate::bin_registry::{params_for_bin, BinParamSpec, BinParamType};
use crate::{compute::ComputeShader, Core, RenderKit};
#[derive(Clone, Debug)]
pub enum RemoteCommand {
    SetF32 { id: String, value: f32 },
    SetColor3 { id: String, value: [f32; 3] },
    SetString { id: String, value: String },
    Pulse { velocity: f32 },
    Note { pitch: f32, velocity: f32 },
    Transport { bpm: f32, beat: f32, measure: f32 },
    OverlayVisible { visible: bool },
    ToggleOverlay,
    TitleBarVisible { visible: bool },
    HideTitleBar,
    WindowTitle { title: String },
    WindowPosition { x: i32, y: i32 },
    WindowScale { scale: f32 },
    WindowSize { width: u32, height: u32 },
    Time { seconds: f32 },
    Fps { fps: f32 },
    Resolution { width: u32, height: u32 },
    Action { id: String, value: f32 },
    LoadMedia { path: String },
    Discover,
    Subscribe { enabled: bool },
}

#[derive(Clone, Debug)]
pub enum RemoteValue {
    F32(f32),
    Color3([f32; 3]),
    String(String),
}

pub trait RemoteField {
    fn apply_remote_field(&mut self, value: RemoteValue) -> bool;
    fn remote_value(&self) -> Option<RemoteValue>;
}

impl RemoteField for f32 {
    fn apply_remote_field(&mut self, value: RemoteValue) -> bool {
        match value {
            RemoteValue::F32(value) => {
                *self = value;
                true
            }
            RemoteValue::Color3(_) => false,
            RemoteValue::String(_) => false,
        }
    }

    fn remote_value(&self) -> Option<RemoteValue> {
        Some(RemoteValue::F32(*self))
    }
}

impl RemoteField for [f32; 3] {
    fn apply_remote_field(&mut self, value: RemoteValue) -> bool {
        match value {
            RemoteValue::Color3(value) => {
                *self = value;
                true
            }
            RemoteValue::F32(_) => false,
            RemoteValue::String(_) => false,
        }
    }

    fn remote_value(&self) -> Option<RemoteValue> {
        Some(RemoteValue::Color3(*self))
    }
}

impl RemoteField for u32 {
    fn apply_remote_field(&mut self, value: RemoteValue) -> bool {
        match value {
            RemoteValue::F32(value) => {
                *self = value.max(0.0).round() as u32;
                true
            }
            RemoteValue::Color3(_) => false,
            RemoteValue::String(_) => false,
        }
    }

    fn remote_value(&self) -> Option<RemoteValue> {
        Some(RemoteValue::F32(*self as f32))
    }
}

impl RemoteField for i32 {
    fn apply_remote_field(&mut self, value: RemoteValue) -> bool {
        match value {
            RemoteValue::F32(value) => {
                *self = value.round() as i32;
                true
            }
            RemoteValue::Color3(_) => false,
            RemoteValue::String(_) => false,
        }
    }

    fn remote_value(&self) -> Option<RemoteValue> {
        Some(RemoteValue::F32(*self as f32))
    }
}

macro_rules! ignored_remote_field {
    ($ty:ty) => {
        impl RemoteField for $ty {
            fn apply_remote_field(&mut self, _value: RemoteValue) -> bool {
                false
            }

            fn remote_value(&self) -> Option<RemoteValue> {
                None
            }
        }
    };
}

ignored_remote_field!([f32; 2]);
ignored_remote_field!([f32; 4]);
ignored_remote_field!([[f32; 4]; 3]);
ignored_remote_field!([[f32; 4]; 4]);
ignored_remote_field!([[f32; 4]; 8]);
ignored_remote_field!([[f32; 4]; 32]);

pub trait RemoteUniform {
    fn apply_remote_value(&mut self, id: &str, value: RemoteValue) -> bool;
    fn remote_value(&self, id: &str) -> Option<RemoteValue>;
}

impl RemoteUniform for () {
    fn apply_remote_value(&mut self, _id: &str, _value: RemoteValue) -> bool {
        false
    }

    fn remote_value(&self, _id: &str) -> Option<RemoteValue> {
        None
    }
}

#[derive(Clone, Default)]
struct RemoteControlsPatch {
    is_paused: Option<bool>,
    should_reset: bool,
    should_clear_buffers: bool,
    load_media_path: Option<std::path::PathBuf>,
    play_video: bool,
    pause_video: bool,
    restart_video: bool,
    seek_position: Option<f64>,
    set_loop: Option<bool>,
    set_volume: Option<f64>,
    mute_audio: Option<bool>,
    toggle_mute: bool,
    start_webcam: bool,
    stop_webcam: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RemoteRuntimeConfig {
    pub bin_name: &'static str,
    pub native_width: u32,
    pub native_height: u32,
}

#[derive(Clone)]
pub struct RemoteRuntime {
    control: Option<RemoteControl>,
    config: RemoteRuntimeConfig,
    transport: Option<(f32, f32, f32)>,
    window_scale: f32,
    remote_time: Option<f32>,
    remote_fps: f32,
    remote_resolution: Option<(u32, u32)>,
    controls_patch: RemoteControlsPatch,
}

impl RemoteRuntime {
    pub fn new(bin_name: &'static str, native_width: u32, native_height: u32) -> Self {
        Self {
            control: RemoteControl::from_env(),
            config: RemoteRuntimeConfig {
                bin_name,
                native_width,
                native_height,
            },
            transport: None,
            window_scale: 1.0,
            remote_time: None,
            remote_fps: 60.0,
            remote_resolution: None,
            controls_patch: RemoteControlsPatch::default(),
        }
    }

    pub fn time(&self, base: &RenderKit) -> f32 {
        self.remote_time
            .or_else(|| {
                self.transport
                    .map(|(bpm, beat, _)| beat * 60.0 / bpm.max(1.0))
            })
            .unwrap_or_else(|| base.controls.get_time(&base.start_time))
    }

    pub fn delta(&self) -> f32 {
        1.0 / self.remote_fps.max(1.0)
    }

    pub fn resolution_size(&self, core: &Core) -> winit::dpi::PhysicalSize<u32> {
        self.remote_resolution
            .map(|(width, height)| winit::dpi::PhysicalSize::new(width, height))
            .unwrap_or(core.size)
    }

    pub fn apply_to_controls_request(&mut self, request: &mut crate::ControlsRequest) {
        if let Some(time) = self.remote_time {
            request.current_time = Some(time);
        }
        if self.remote_time.is_some() || (self.remote_fps - 60.0).abs() > f32::EPSILON {
            request.current_fps = Some(self.remote_fps);
        }
        if let Some(size) = self.remote_resolution {
            request.window_size = Some(size);
        }
        if let Some(is_paused) = self.controls_patch.is_paused.take() {
            request.is_paused = is_paused;
        }
        request.should_reset |= self.controls_patch.should_reset;
        request.should_clear_buffers |= self.controls_patch.should_clear_buffers;
        if self.controls_patch.load_media_path.is_some() {
            request.load_media_path = self.controls_patch.load_media_path.take();
        }
        request.play_video |= self.controls_patch.play_video;
        request.pause_video |= self.controls_patch.pause_video;
        request.restart_video |= self.controls_patch.restart_video;
        if self.controls_patch.seek_position.is_some() {
            request.seek_position = self.controls_patch.seek_position.take();
        }
        if self.controls_patch.set_loop.is_some() {
            request.set_loop = self.controls_patch.set_loop.take();
        }
        if self.controls_patch.set_volume.is_some() {
            request.set_volume = self.controls_patch.set_volume.take();
        }
        if self.controls_patch.mute_audio.is_some() {
            request.mute_audio = self.controls_patch.mute_audio.take();
        }
        request.toggle_mute |= self.controls_patch.toggle_mute;
        request.start_webcam |= self.controls_patch.start_webcam;
        request.stop_webcam |= self.controls_patch.stop_webcam;
        self.controls_patch.should_reset = false;
        self.controls_patch.should_clear_buffers = false;
        self.controls_patch.play_video = false;
        self.controls_patch.pause_video = false;
        self.controls_patch.restart_video = false;
        self.controls_patch.toggle_mute = false;
        self.controls_patch.start_webcam = false;
        self.controls_patch.stop_webcam = false;
    }

    fn apply_builtin_value(&mut self, id: &str, value: f32) -> bool {
        match id {
            "control_pause" => self.controls_patch.is_paused = Some(value >= 0.5),
            "video_loop" => self.controls_patch.set_loop = Some(value >= 0.5),
            "video_mute" => self.controls_patch.mute_audio = Some(value >= 0.5),
            "video_volume" => self.controls_patch.set_volume = Some(value.clamp(0.0, 1.0) as f64),
            "video_seek" => self.controls_patch.seek_position = Some(value.max(0.0) as f64),
            _ => return false,
        }
        true
    }

    fn apply_builtin_string(&mut self, id: &str, value: String) -> bool {
        match id {
            "media_path" | "load_media" => {
                self.controls_patch.load_media_path = Some(value.into());
                self.controls_patch.play_video = true;
                true
            }
            _ => false,
        }
    }

    fn apply_action(&mut self, id: &str, value: f32) {
        if value < 0.5 {
            return;
        }
        match id {
            "control_reset" => {
                self.controls_patch.should_reset = true;
                self.controls_patch.should_clear_buffers = true;
            }
            "control_clear_buffers" => self.controls_patch.should_clear_buffers = true,
            "media_start_webcam" => self.controls_patch.start_webcam = true,
            "media_stop_webcam" => self.controls_patch.stop_webcam = true,
            "video_play" => self.controls_patch.play_video = true,
            "video_pause" => self.controls_patch.pause_video = true,
            "video_restart" => self.controls_patch.restart_video = true,
            "video_mute_toggle" => self.controls_patch.toggle_mute = true,
            _ => {}
        }
    }

    pub fn drain<T: RemoteUniform>(
        &mut self,
        core: &Core,
        base: &mut RenderKit,
        compute_shader: &mut ComputeShader,
        params: &mut T,
    ) -> bool {
        let mut changed = false;
        let Some(remote_control) = self.control.clone() else {
            return false;
        };
        for command in remote_control.drain() {
            match command {
                RemoteCommand::SetF32 { id, value } => {
                    if !self.apply_builtin_value(&id, value) {
                        changed |= params.apply_remote_value(&id, RemoteValue::F32(value));
                    }
                }
                RemoteCommand::SetColor3 { id, value } => {
                    changed |= params.apply_remote_value(&id, RemoteValue::Color3(value));
                }
                RemoteCommand::SetString { id, value } => {
                    if !self.apply_builtin_string(&id, value.clone()) {
                        changed |= params.apply_remote_value(&id, RemoteValue::String(value));
                    }
                }
                RemoteCommand::Transport { bpm, beat, measure } => {
                    if bpm > 0.0 {
                        self.transport = Some((bpm, beat, measure));
                    }
                }
                RemoteCommand::Discover => {
                    self.send_discovery(params);
                }
                RemoteCommand::Subscribe { enabled } => {
                    remote_control.set_feedback_enabled(enabled);
                    if enabled {
                        self.send_discovery(params);
                    }
                }
                RemoteCommand::OverlayVisible { visible } => {
                    base.key_handler.show_ui = visible;
                }
                RemoteCommand::ToggleOverlay => {
                    base.key_handler.show_ui = !base.key_handler.show_ui;
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
                        .set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
                }
                RemoteCommand::WindowScale { scale } => {
                    self.window_scale = scale.max(0.1);
                    let width = (self.config.native_width as f32 * self.window_scale)
                        .round()
                        .max(1.0) as u32;
                    let height = (self.config.native_height as f32 * self.window_scale)
                        .round()
                        .max(1.0) as u32;
                    let _ = core
                        .window()
                        .request_inner_size(winit::dpi::LogicalSize::new(width, height));
                }
                RemoteCommand::WindowSize { width, height } => {
                    let _ = core
                        .window()
                        .request_inner_size(winit::dpi::LogicalSize::new(
                            width.max(1),
                            height.max(1),
                        ));
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
                    base.update_resolution(
                        &core.queue,
                        winit::dpi::PhysicalSize::new(width, height),
                    );
                    compute_shader.resize(core, width, height);
                }
                RemoteCommand::Action { id, value } => {
                    self.apply_action(&id, value);
                }
                RemoteCommand::LoadMedia { path } => {
                    self.controls_patch.load_media_path = Some(path.into());
                    self.controls_patch.play_video = true;
                }
                RemoteCommand::Pulse { .. } | RemoteCommand::Note { .. } => {}
            }
        }
        if changed {
            self.send_values(params);
        }
        changed
    }

    pub fn send_discovery<T: RemoteUniform>(&self, params: &T) {
        if let (Some(remote_control), Some(specs)) =
            (&self.control, params_for_bin(self.config.bin_name))
        {
            remote_control.send_discovery(self.config.bin_name, specs);
            remote_control.send_values(specs, |id| params.remote_value(id));
        }
    }

    pub fn send_values<T: RemoteUniform>(&self, params: &T) {
        if let (Some(remote_control), Some(specs)) =
            (&self.control, params_for_bin(self.config.bin_name))
        {
            remote_control.send_values(specs, |id| params.remote_value(id));
        }
    }
}

#[derive(Clone)]
pub struct RemoteControl {
    commands: Arc<Mutex<VecDeque<RemoteCommand>>>,
    feedback: Arc<Mutex<Option<SocketAddr>>>,
    feedback_enabled: Arc<Mutex<bool>>,
}

impl Default for RemoteControl {
    fn default() -> Self {
        Self {
            commands: Arc::new(Mutex::new(VecDeque::new())),
            feedback: Arc::new(Mutex::new(None)),
            feedback_enabled: Arc::new(Mutex::new(true)),
        }
    }
}

impl RemoteControl {
    pub fn from_env() -> Option<Self> {
        let port = std::env::var("CUNEUS_REMOTE_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())?;
        Self::listen(port).ok()
    }

    pub fn listen(port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(("127.0.0.1", port))?;
        socket.set_nonblocking(false)?;

        let remote = Self::default();
        let commands = remote.commands.clone();
        let feedback = remote.feedback.clone();

        if let Some(addr) = feedback_addr_from_env() {
            if let Ok(mut target) = feedback.lock() {
                *target = Some(addr);
            }
        }

        thread::Builder::new()
            .name(format!("acuneus-remote-{port}"))
            .spawn(move || {
                let mut buffer = [0_u8; 1024];
                loop {
                    let Ok((len, addr)) = socket.recv_from(&mut buffer) else {
                        continue;
                    };

                    let command = parse_osc_command(&buffer[..len]).or_else(|| {
                        std::str::from_utf8(&buffer[..len])
                            .ok()
                            .and_then(|text| parse_text_command(text.trim()))
                    });

                    if let Some(command) = command {
                        if matches!(
                            command,
                            RemoteCommand::Discover | RemoteCommand::Subscribe { .. }
                        ) || is_osc_packet(&buffer[..len])
                        {
                            if let Ok(mut target) = feedback.lock() {
                                *target = Some(addr);
                            }
                        }
                        if let Ok(mut queue) = commands.lock() {
                            queue.push_back(command);
                        }
                    }
                }
            })?;

        Ok(remote)
    }

    pub fn drain(&self) -> Vec<RemoteCommand> {
        let Ok(mut queue) = self.commands.lock() else {
            return Vec::new();
        };
        queue.drain(..).collect()
    }

    pub fn set_feedback_enabled(&self, enabled: bool) {
        if let Ok(mut feedback_enabled) = self.feedback_enabled.lock() {
            *feedback_enabled = enabled;
        }
    }

    pub fn send_status(&self, status: &str) {
        self.send_message(
            "/acuneus/cuneus/status",
            vec![OscType::String(status.to_string())],
        );
    }

    pub fn send_bin(&self, bin: &str) {
        self.send_message(
            "/acuneus/cuneus/bin",
            vec![OscType::String(bin.to_string())],
        );
    }

    pub fn send_param_count(&self, count: usize) {
        self.send_message(
            "/acuneus/cuneus/param_count",
            vec![OscType::Int(count as i32)],
        );
    }

    pub fn send_param_desc(
        &self,
        index: usize,
        id: &str,
        label: &str,
        group: &str,
        param_type: &str,
        min: f32,
        max: f32,
        default_value: f32,
    ) {
        self.send_message(
            "/acuneus/cuneus/param_desc",
            vec![
                OscType::Int(index as i32),
                OscType::String(id.to_string()),
                OscType::String(label.to_string()),
                OscType::String(group.to_string()),
                OscType::String(param_type.to_string()),
                OscType::Float(min),
                OscType::Float(max),
                OscType::Float(default_value),
            ],
        );
    }

    pub fn send_discovery(&self, bin_name: &str, params: &[BinParamSpec]) {
        self.send_status("ready");
        self.send_bin(bin_name);
        self.send_param_count(params.len());
        for (index, param) in params.iter().enumerate() {
            let param_type = match param.param_type {
                BinParamType::F32 => "f32",
                BinParamType::Color3 => "color3",
                BinParamType::Action => "action",
                BinParamType::String => "string",
            };
            self.send_param_desc(
                index,
                param.id_str(),
                param.label_str(),
                param.group_str(),
                param_type,
                param.min_value,
                param.max_value,
                param.default_value,
            );
        }
    }

    pub fn send_param_f32(&self, id: &str, value: f32) {
        self.send_message(
            &format!("/acuneus/cuneus/param/{id}"),
            vec![OscType::Float(value)],
        );
    }

    pub fn send_param_color3(&self, id: &str, value: [f32; 3]) {
        self.send_message(
            &format!("/acuneus/cuneus/param/{id}"),
            vec![
                OscType::Float(value[0]),
                OscType::Float(value[1]),
                OscType::Float(value[2]),
            ],
        );
    }

    pub fn send_value(&self, id: &str, value: RemoteValue) {
        match value {
            RemoteValue::F32(value) => self.send_param_f32(id, value),
            RemoteValue::Color3(value) => self.send_param_color3(id, value),
            RemoteValue::String(value) => self.send_message(
                &format!("/acuneus/cuneus/param/{id}"),
                vec![OscType::String(value)],
            ),
        }
    }

    pub fn send_values(
        &self,
        specs: &[BinParamSpec],
        mut value_for_id: impl FnMut(&str) -> Option<RemoteValue>,
    ) {
        for spec in specs {
            let id = spec.id_str();
            if let Some(value) = value_for_id(id) {
                self.send_value(id, value);
            }
        }
    }

    pub fn send_transport(&self, bpm: f32, beat: f32, measure: f32) {
        self.send_message(
            "/acuneus/cuneus/transport",
            vec![
                OscType::Float(bpm),
                OscType::Float(beat),
                OscType::Float(measure),
            ],
        );
    }

    pub fn send_transport_tempo(&self, bpm: f32) {
        self.send_message("/acuneus/cuneus/transport/tempo", vec![OscType::Float(bpm)]);
    }

    pub fn send_transport_playing(&self, playing: bool) {
        self.send_message(
            "/acuneus/cuneus/transport/play",
            vec![OscType::Int(if playing { 1 } else { 0 })],
        );
    }

    pub fn send_transport_reset(&self) {
        self.send_message("/acuneus/cuneus/transport/reset", Vec::new());
    }

    pub fn send_transport_shift_beats(&self, beats: f32) {
        self.send_message(
            "/acuneus/cuneus/transport/shift_beats",
            vec![OscType::Float(beats)],
        );
    }

    fn send_message(&self, addr: &str, args: Vec<OscType>) {
        let feedback_enabled = self
            .feedback_enabled
            .lock()
            .map_or(false, |enabled| *enabled);
        if !feedback_enabled {
            return;
        }
        let target = self.feedback.lock().ok().and_then(|target| *target);
        let Some(target) = target else {
            return;
        };
        let packet = OscPacket::Message(OscMessage {
            addr: addr.to_string(),
            args,
        });
        let Ok(bytes) = encoder::encode(&packet) else {
            return;
        };
        if let Ok(socket) = UdpSocket::bind(("127.0.0.1", 0)) {
            let _ = socket.send_to(&bytes, target);
        }
    }
}

fn feedback_addr_from_env() -> Option<SocketAddr> {
    let host =
        std::env::var("CUNEUS_OSC_FEEDBACK_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("CUNEUS_OSC_FEEDBACK_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())?;
    (host.as_str(), port).to_socket_addrs().ok()?.next()
}

fn is_osc_packet(bytes: &[u8]) -> bool {
    rosc::decoder::decode_udp(bytes).is_ok()
}

fn parse_text_command(text: &str) -> Option<RemoteCommand> {
    let mut parts = text.split_whitespace();
    match parts.next()? {
        "set_f32" => Some(RemoteCommand::SetF32 {
            id: parts.next()?.to_string(),
            value: parts.next()?.parse().ok()?,
        }),
        "set_color3" => Some(RemoteCommand::SetColor3 {
            id: parts.next()?.to_string(),
            value: [
                parts.next()?.parse().ok()?,
                parts.next()?.parse().ok()?,
                parts.next()?.parse().ok()?,
            ],
        }),
        "set_string" => Some(RemoteCommand::SetString {
            id: parts.next()?.to_string(),
            value: parts.collect::<Vec<_>>().join(" "),
        }),
        "pulse" => Some(RemoteCommand::Pulse {
            velocity: parts.next()?.parse().ok()?,
        }),
        "note" => Some(RemoteCommand::Note {
            pitch: parts.next()?.parse().ok()?,
            velocity: parts.next()?.parse().ok()?,
        }),
        "transport" => Some(RemoteCommand::Transport {
            bpm: parts.next()?.parse().ok()?,
            beat: parts.next()?.parse().ok()?,
            measure: parts.next()?.parse().ok()?,
        }),
        "overlay_visible" => Some(RemoteCommand::OverlayVisible {
            visible: parts.next().map_or(true, |value| value != "0"),
        }),
        "toggle_overlay" => Some(RemoteCommand::ToggleOverlay),
        "title_bar_visible" => Some(RemoteCommand::TitleBarVisible {
            visible: parts.next().map_or(true, |value| value != "0"),
        }),
        "hide_title_bar" => Some(RemoteCommand::HideTitleBar),
        "window_title" => Some(RemoteCommand::WindowTitle {
            title: parts.collect::<Vec<_>>().join(" "),
        }),
        "window_position" => Some(RemoteCommand::WindowPosition {
            x: parts.next()?.parse().ok()?,
            y: parts.next()?.parse().ok()?,
        }),
        "window_scale" => Some(RemoteCommand::WindowScale {
            scale: parts.next()?.parse().ok()?,
        }),
        "window_size" => Some(RemoteCommand::WindowSize {
            width: parts.next()?.parse().ok()?,
            height: parts.next()?.parse().ok()?,
        }),
        "time" => Some(RemoteCommand::Time {
            seconds: parts.next()?.parse().ok()?,
        }),
        "fps" => Some(RemoteCommand::Fps {
            fps: parts.next()?.parse().ok()?,
        }),
        "resolution" => Some(RemoteCommand::Resolution {
            width: parts.next()?.parse().ok()?,
            height: parts.next()?.parse().ok()?,
        }),
        "action" => Some(RemoteCommand::Action {
            id: parts.next()?.to_string(),
            value: parts
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1.0),
        }),
        "load_media" => Some(RemoteCommand::LoadMedia {
            path: parts.collect::<Vec<_>>().join(" "),
        }),
        "discover" => Some(RemoteCommand::Discover),
        "subscribe" => Some(RemoteCommand::Subscribe {
            enabled: parts.next().map_or(true, |value| value != "0"),
        }),
        _ => None,
    }
}

fn parse_osc_command(bytes: &[u8]) -> Option<RemoteCommand> {
    let (_, packet) = rosc::decoder::decode_udp(bytes).ok()?;
    parse_osc_packet(packet)
}

fn parse_osc_packet(packet: OscPacket) -> Option<RemoteCommand> {
    match packet {
        OscPacket::Message(message) => parse_osc_message(message),
        OscPacket::Bundle(bundle) => bundle.content.into_iter().find_map(parse_osc_packet),
    }
}

fn parse_osc_message(message: OscMessage) -> Option<RemoteCommand> {
    let addr = message.addr.trim_end_matches('/');
    let args = message.args;

    match addr {
        "/acuneus/cuneus/discover" => return Some(RemoteCommand::Discover),
        "/acuneus/cuneus/subscribe" => {
            return Some(RemoteCommand::Subscribe {
                enabled: osc_bool(args.first()).unwrap_or(true),
            });
        }
        "/acuneus/cuneus/pulse" => {
            return Some(RemoteCommand::Pulse {
                velocity: osc_f32(args.first())?,
            });
        }
        "/acuneus/cuneus/note" => {
            return Some(RemoteCommand::Note {
                pitch: osc_f32(args.first())?,
                velocity: osc_f32(args.get(1))?,
            });
        }
        "/acuneus/cuneus/transport" => {
            return Some(RemoteCommand::Transport {
                bpm: osc_f32(args.first())?,
                beat: osc_f32(args.get(1))?,
                measure: osc_f32(args.get(2))?,
            });
        }
        "/acuneus/cuneus/overlay" => {
            return Some(RemoteCommand::OverlayVisible {
                visible: osc_bool(args.first()).unwrap_or(true),
            });
        }
        "/acuneus/cuneus/overlay/toggle" => return Some(RemoteCommand::ToggleOverlay),
        "/acuneus/cuneus/window/titlebar" => {
            return Some(RemoteCommand::TitleBarVisible {
                visible: osc_bool(args.first()).unwrap_or(true),
            });
        }
        "/acuneus/cuneus/window/titlebar/hide" => return Some(RemoteCommand::HideTitleBar),
        "/acuneus/cuneus/window/title" => {
            return Some(RemoteCommand::WindowTitle {
                title: osc_string(args.first())?,
            });
        }
        "/acuneus/cuneus/window/position" => {
            return Some(RemoteCommand::WindowPosition {
                x: osc_i32(args.first())?,
                y: osc_i32(args.get(1))?,
            });
        }
        "/acuneus/cuneus/window/scale" => {
            return Some(RemoteCommand::WindowScale {
                scale: osc_f32(args.first())?,
            });
        }
        "/acuneus/cuneus/window/size" => {
            return Some(RemoteCommand::WindowSize {
                width: osc_u32(args.first())?,
                height: osc_u32(args.get(1))?,
            });
        }
        "/acuneus/cuneus/time" => {
            return Some(RemoteCommand::Time {
                seconds: osc_f32(args.first())?,
            });
        }
        "/acuneus/cuneus/fps" => {
            return Some(RemoteCommand::Fps {
                fps: osc_f32(args.first())?,
            });
        }
        "/acuneus/cuneus/resolution" => {
            return Some(RemoteCommand::Resolution {
                width: osc_u32(args.first())?,
                height: osc_u32(args.get(1))?,
            });
        }
        "/acuneus/cuneus/action" => {
            return Some(RemoteCommand::Action {
                id: osc_string(args.first())?,
                value: args
                    .get(1)
                    .and_then(|arg| osc_f32(Some(arg)))
                    .unwrap_or(1.0),
            });
        }
        "/acuneus/cuneus/media/load" => {
            return Some(RemoteCommand::LoadMedia {
                path: osc_string(args.first())?,
            });
        }
        _ => {}
    }

    let id = addr
        .strip_prefix("/acuneus/cuneus/param/")
        .or_else(|| addr.strip_prefix("/acuneus/cuneus/color/"))?;

    if args.first().is_some_and(|arg| matches!(arg, OscType::String(_))) {
        Some(RemoteCommand::SetString {
            id: id.to_string(),
            value: osc_string(args.first())?,
        })
    } else if args.len() >= 3 {
        Some(RemoteCommand::SetColor3 {
            id: id.to_string(),
            value: [
                osc_f32(args.first())?,
                osc_f32(args.get(1))?,
                osc_f32(args.get(2))?,
            ],
        })
    } else {
        Some(RemoteCommand::SetF32 {
            id: id.to_string(),
            value: osc_f32(args.first())?,
        })
    }
}

fn osc_string(value: Option<&OscType>) -> Option<String> {
    match value? {
        OscType::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn osc_f32(value: Option<&OscType>) -> Option<f32> {
    match value? {
        OscType::Float(value) => Some(*value),
        OscType::Double(value) => Some(*value as f32),
        OscType::Int(value) => Some(*value as f32),
        OscType::Long(value) => Some(*value as f32),
        _ => None,
    }
}

fn osc_i32(value: Option<&OscType>) -> Option<i32> {
    match value? {
        OscType::Int(value) => Some(*value),
        OscType::Float(value) => Some(*value as i32),
        _ => None,
    }
}

fn osc_u32(value: Option<&OscType>) -> Option<u32> {
    let value = osc_i32(value)?;
    u32::try_from(value).ok()
}

fn osc_bool(value: Option<&OscType>) -> Option<bool> {
    match value? {
        OscType::Bool(value) => Some(*value),
        OscType::Int(value) => Some(*value != 0),
        OscType::Float(value) => Some(*value != 0.0),
        _ => None,
    }
}
