use std::collections::VecDeque;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;

use rosc::{encoder, OscMessage, OscPacket, OscType};

use crate::{BinParamSpec, BinParamType};

#[derive(Clone, Debug)]
pub enum RemoteCommand {
    SetF32 { id: String, value: f32 },
    SetColor3 { id: String, value: [f32; 3] },
    Pulse { velocity: f32 },
    Note { pitch: f32, velocity: f32 },
    Transport { bpm: f32, beat: f32, measure: f32 },
    Discover,
    Subscribe { enabled: bool },
}

#[derive(Clone, Copy, Debug)]
pub enum RemoteValue {
    F32(f32),
    Color3([f32; 3]),
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
            .name(format!("cuneus-remote-{port}"))
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
                        if matches!(command, RemoteCommand::Discover | RemoteCommand::Subscribe { .. })
                            || is_osc_packet(&buffer[..len])
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
        self.send_message("/cuneus/status", vec![OscType::String(status.to_string())]);
    }

    pub fn send_bin(&self, bin: &str) {
        self.send_message("/cuneus/bin", vec![OscType::String(bin.to_string())]);
    }

    pub fn send_param_count(&self, count: usize) {
        self.send_message("/cuneus/param_count", vec![OscType::Int(count as i32)]);
    }

    pub fn send_param_desc(
        &self,
        index: usize,
        id: &str,
        label: &str,
        param_type: &str,
        min: f32,
        max: f32,
        default_value: f32,
    ) {
        self.send_message(
            "/cuneus/param_desc",
            vec![
                OscType::Int(index as i32),
                OscType::String(id.to_string()),
                OscType::String(label.to_string()),
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
            };
            self.send_param_desc(
                index,
                param.id_str(),
                param.label_str(),
                param_type,
                param.min_value,
                param.max_value,
                param.default_value,
            );
        }
    }

    pub fn send_param_f32(&self, id: &str, value: f32) {
        self.send_message(
            &format!("/cuneus/param/{id}"),
            vec![OscType::Float(value)],
        );
    }

    pub fn send_param_color3(&self, id: &str, value: [f32; 3]) {
        self.send_message(
            &format!("/cuneus/param/{id}"),
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
        }
    }

    pub fn send_values(&self, specs: &[BinParamSpec], mut value_for_id: impl FnMut(&str) -> Option<RemoteValue>) {
        for spec in specs {
            let id = spec.id_str();
            if let Some(value) = value_for_id(id) {
                self.send_value(id, value);
            }
        }
    }

    fn send_message(&self, addr: &str, args: Vec<OscType>) {
        let feedback_enabled = self.feedback_enabled.lock().map_or(false, |enabled| *enabled);
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
    let host = std::env::var("CUNEUS_OSC_FEEDBACK_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
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
        "/cuneus/discover" => return Some(RemoteCommand::Discover),
        "/cuneus/subscribe" => {
            return Some(RemoteCommand::Subscribe {
                enabled: osc_bool(args.first()).unwrap_or(true),
            });
        }
        "/cuneus/pulse" => {
            return Some(RemoteCommand::Pulse {
                velocity: osc_f32(args.first())?,
            });
        }
        "/cuneus/note" => {
            return Some(RemoteCommand::Note {
                pitch: osc_f32(args.first())?,
                velocity: osc_f32(args.get(1))?,
            });
        }
        "/cuneus/transport" => {
            return Some(RemoteCommand::Transport {
                bpm: osc_f32(args.first())?,
                beat: osc_f32(args.get(1))?,
                measure: osc_f32(args.get(2))?,
            });
        }
        _ => {}
    }

    let id = addr
        .strip_prefix("/cuneus/param/")
        .or_else(|| addr.strip_prefix("/cuneus/color/"))?;

    if args.len() >= 3 {
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

fn osc_f32(value: Option<&OscType>) -> Option<f32> {
    match value? {
        OscType::Float(value) => Some(*value),
        OscType::Double(value) => Some(*value as f32),
        OscType::Int(value) => Some(*value as f32),
        OscType::Long(value) => Some(*value as f32),
        _ => None,
    }
}

fn osc_bool(value: Option<&OscType>) -> Option<bool> {
    match value? {
        OscType::Bool(value) => Some(*value),
        OscType::Int(value) => Some(*value != 0),
        OscType::Float(value) => Some(*value != 0.0),
        _ => None,
    }
}
