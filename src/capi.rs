use std::ffi::{CStr, CString};
use std::net::UdpSocket;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::bin_registry::{params_for_bin, BinParamType, BINS};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CuneusStatus {
    Ok = 0,
    Null = 1,
    InvalidArgument = 2,
    NotFound = 3,
    IoError = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CuneusParamType {
    F32 = 0,
    Color3 = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CuneusParamDesc {
    pub id: *const c_char,
    pub label: *const c_char,
    pub param_type: CuneusParamType,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub flags: u32,
}

pub struct CuneusInstance {
    bin_name: &'static str,
    remote_port: u16,
    socket: UdpSocket,
    child: Option<Child>,
}

static LAST_ERROR: OnceLock<Mutex<CString>> = OnceLock::new();

fn capi_param_type(param_type: BinParamType) -> CuneusParamType {
    match param_type {
        BinParamType::F32 => CuneusParamType::F32,
        BinParamType::Color3 => CuneusParamType::Color3,
    }
}

fn set_last_error(message: impl Into<String>) {
    let sanitized = message.into().replace('\0', " ");
    let error = CString::new(sanitized).unwrap_or_else(|_| CString::new("unknown cuneus error").unwrap());
    let lock = LAST_ERROR.get_or_init(|| Mutex::new(CString::new("").unwrap()));
    if let Ok(mut last_error) = lock.lock() {
        *last_error = error;
    }
}

unsafe fn string_from_ptr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(ToOwned::to_owned)
}

fn send(instance: &CuneusInstance, message: String) -> CuneusStatus {
    match instance.socket.send_to(message.as_bytes(), ("127.0.0.1", instance.remote_port)) {
        Ok(_) => CuneusStatus::Ok,
        Err(error) => {
            set_last_error(error.to_string());
            CuneusStatus::IoError
        }
    }
}

#[no_mangle]
pub extern "C" fn cuneus_bin_count() -> usize {
    BINS.len()
}

#[no_mangle]
pub extern "C" fn cuneus_bin_name(index: usize) -> *const c_char {
    BINS.get(index).map_or(ptr::null(), |name| name.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_instance_open(
    bin_name: *const c_char,
    executable_dir: *const c_char,
    remote_port: u16,
) -> *mut CuneusInstance {
    cuneus_instance_open_with_feedback(bin_name, executable_dir, remote_port, 0)
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_instance_open_with_feedback(
    bin_name: *const c_char,
    executable_dir: *const c_char,
    remote_port: u16,
    osc_feedback_port: u16,
) -> *mut CuneusInstance {
    let Some(bin_name_string) = string_from_ptr(bin_name) else {
        set_last_error("bin_name is null or invalid utf-8");
        return ptr::null_mut();
    };
    let Some(params) = params_for_bin(&bin_name_string) else {
        set_last_error(format!("unknown cuneus bin: {bin_name_string}"));
        return ptr::null_mut();
    };
    if params.is_empty() {
        set_last_error(format!("no parameters registered for {bin_name_string}"));
        return ptr::null_mut();
    }

    let socket = match UdpSocket::bind(("127.0.0.1", 0)) {
        Ok(socket) => socket,
        Err(error) => {
            set_last_error(error.to_string());
            return ptr::null_mut();
        }
    };

    let mut child = None;
    if let Some(dir) = string_from_ptr(executable_dir).filter(|value| !value.is_empty()) {
        let mut executable = PathBuf::from(dir);
        executable.push(&bin_name_string);
        if cfg!(windows) {
            executable.set_extension("exe");
        }
        let mut command = Command::new(&executable);
        command.env("CUNEUS_REMOTE_PORT", remote_port.to_string());
        if osc_feedback_port != 0 {
            command.env("CUNEUS_OSC_FEEDBACK_PORT", osc_feedback_port.to_string());
        }
        match command.spawn() {
            Ok(process) => child = Some(process),
            Err(error) => {
                set_last_error(format!("failed to launch {}: {error}", executable.display()));
                return ptr::null_mut();
            }
        }
    }

    let bin_name_static = params_for_bin(&bin_name_string)
        .and(Some(bin_name_string.into_boxed_str()))
        .map(Box::leak)
        .map(|value| &*value)
        .unwrap_or("roto");

    Box::into_raw(Box::new(CuneusInstance {
        bin_name: bin_name_static,
        remote_port,
        socket,
        child,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_instance_free(instance: *mut CuneusInstance) {
    if instance.is_null() {
        return;
    }
    let mut instance = Box::from_raw(instance);
    if let Some(mut child) = instance.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[no_mangle]
pub extern "C" fn cuneus_last_error() -> *const c_char {
    let lock = LAST_ERROR.get_or_init(|| Mutex::new(CString::new("").unwrap()));
    lock.lock().map_or(ptr::null(), |value| value.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_param_count(instance: *mut CuneusInstance) -> usize {
    let Some(instance) = instance.as_ref() else {
        return 0;
    };
    params_for_bin(instance.bin_name).map_or(0, |params| params.len())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_param_desc(
    instance: *mut CuneusInstance,
    index: usize,
    out_desc: *mut CuneusParamDesc,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    let Some(out_desc) = out_desc.as_mut() else {
        return CuneusStatus::Null;
    };
    let Some(param) = params_for_bin(instance.bin_name).and_then(|params| params.get(index)) else {
        return CuneusStatus::NotFound;
    };
    *out_desc = CuneusParamDesc {
        id: param.id.as_ptr(),
        label: param.label.as_ptr(),
        param_type: capi_param_type(param.param_type),
        min_value: param.min_value,
        max_value: param.max_value,
        default_value: param.default_value,
        flags: param.flags,
    };
    CuneusStatus::Ok
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_param_f32(
    instance: *mut CuneusInstance,
    id: *const c_char,
    value: f32,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    let Some(id) = string_from_ptr(id) else {
        return CuneusStatus::InvalidArgument;
    };
    send(instance, format!("set_f32 {id} {value}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_param_color3(
    instance: *mut CuneusInstance,
    id: *const c_char,
    r: f32,
    g: f32,
    b: f32,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    let Some(id) = string_from_ptr(id) else {
        return CuneusStatus::InvalidArgument;
    };
    send(instance, format!("set_color3 {id} {r} {g} {b}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_pulse(instance: *mut CuneusInstance, velocity: f32) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, format!("pulse {velocity}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_note(instance: *mut CuneusInstance, pitch: f32, velocity: f32) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, format!("note {pitch} {velocity}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_transport(
    instance: *mut CuneusInstance,
    bpm: f32,
    beat: f32,
    measure: f32,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, format!("transport {bpm} {beat} {measure}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_discover(instance: *mut CuneusInstance) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, "discover".to_string())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_subscribe(instance: *mut CuneusInstance, enabled: bool) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, format!("subscribe {}", if enabled { 1 } else { 0 }))
}
