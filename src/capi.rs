use std::ffi::{CStr, CString};
use std::net::UdpSocket;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;

use crate::bin_registry::{
    default_dimensions_for_bin, info_for_bin, params_for_bin, title_for_bin, BinParamType, BINS,
    BIN_FLAG_USES_KEYBOARD, BIN_FLAG_USES_MOUSE, BIN_INFOS,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
    Action = 2,
    String = 3,
    Bool = 4,
    Select = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CuneusParamDesc {
    pub id: *const c_char,
    pub label: *const c_char,
    pub group: *const c_char,
    pub param_type: CuneusParamType,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub flags: u32,
    pub options: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CuneusBinDesc {
    pub name: *const c_char,
    pub title: *const c_char,
    pub source_file: *const c_char,
    pub default_width: u32,
    pub default_height: u32,
    pub flags: u32,
    pub keys: *const c_char,
}

pub struct CuneusInstance {
    bin_name: &'static str,
    remote_port: u16,
    socket: UdpSocket,
    child: Option<Child>,
    embedded_thread: Option<JoinHandle<()>>,
    window_x: i32,
    window_y: i32,
    window_scale: f32,
    window_base_width: u32,
    window_base_height: u32,
}

static LAST_ERROR: OnceLock<Mutex<CString>> = OnceLock::new();

fn capi_param_type(param_type: BinParamType) -> CuneusParamType {
    match param_type {
        BinParamType::F32 => CuneusParamType::F32,
        BinParamType::Color3 => CuneusParamType::Color3,
        BinParamType::Action => CuneusParamType::Action,
        BinParamType::String => CuneusParamType::String,
        BinParamType::Bool => CuneusParamType::Bool,
        BinParamType::Select => CuneusParamType::Select,
    }
}

fn set_last_error(message: impl Into<String>) {
    let sanitized = message.into().replace('\0', " ");
    let error =
        CString::new(sanitized).unwrap_or_else(|_| CString::new("unknown cuneus error").unwrap());
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
    match instance
        .socket
        .send_to(message.as_bytes(), ("127.0.0.1", instance.remote_port))
    {
        Ok(_) => CuneusStatus::Ok,
        Err(error) => {
            set_last_error(error.to_string());
            CuneusStatus::IoError
        }
    }
}

fn default_window_dimensions(bin_name: &str) -> (u32, u32) {
    default_dimensions_for_bin(bin_name).unwrap_or((800, 600))
}

#[cfg(windows)]
fn child_window_handle(child: &Child) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct FindWindowState {
        process_id: u32,
        hwnd: HWND,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam as *mut FindWindowState);
        let mut process_id = 0_u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == state.process_id && IsWindowVisible(hwnd) != 0 {
            state.hwnd = hwnd;
            return 0;
        }
        1
    }

    let mut state = FindWindowState {
        process_id: child.id(),
        hwnd: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(
            Some(enum_proc),
            &mut state as *mut FindWindowState as LPARAM,
        );
    }
    (!state.hwnd.is_null()).then_some(state.hwnd)
}

#[cfg(windows)]
fn set_child_window_geometry(instance: &CuneusInstance) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

    let Some(child) = instance.child.as_ref() else {
        return false;
    };
    let Some(hwnd) = child_window_handle(child) else {
        return false;
    };
    let width = ((instance.window_base_width as f32) * instance.window_scale.max(0.1))
        .round()
        .max(1.0) as i32;
    let height = ((instance.window_base_height as f32) * instance.window_scale.max(0.1))
        .round()
        .max(1.0) as i32;
    unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            instance.window_x,
            instance.window_y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ) != 0
    }
}

#[cfg(windows)]
fn get_child_window_position(instance: &CuneusInstance) -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let child = instance.child.as_ref()?;
    let hwnd = child_window_handle(child)?;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe { (GetWindowRect(hwnd, &mut rect) != 0).then_some((rect.left, rect.top)) }
}

#[cfg(not(windows))]
fn get_child_window_position(_instance: &CuneusInstance) -> Option<(i32, i32)> {
    None
}

#[cfg(not(windows))]
fn set_child_window_geometry(_instance: &CuneusInstance) -> bool {
    false
}

fn runner_command(executable_dir: &str, bin_name: &str) -> (PathBuf, bool) {
    let path = PathBuf::from(executable_dir);
    if path.is_file() {
        return (path, true);
    }

    let mut acuneus = path.clone();
    acuneus.push("acuneus_runner");
    if cfg!(windows) {
        acuneus.set_extension("exe");
    }
    if Path::new(&acuneus).exists() {
        return (acuneus, true);
    }

    let mut per_bin = path;
    per_bin.push(bin_name);
    if cfg!(windows) {
        per_bin.set_extension("exe");
    }
    (per_bin, false)
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
pub unsafe extern "C" fn cuneus_bin_desc(
    index: usize,
    out_desc: *mut CuneusBinDesc,
) -> CuneusStatus {
    let Some(out_desc) = out_desc.as_mut() else {
        return CuneusStatus::Null;
    };
    let Some(info) = BIN_INFOS.get(index) else {
        return CuneusStatus::NotFound;
    };
    *out_desc = CuneusBinDesc {
        name: info.name.as_ptr(),
        title: info.title.as_ptr(),
        source_file: info.source_file.as_ptr(),
        default_width: info.default_width,
        default_height: info.default_height,
        flags: info.flags,
        keys: info.keys.as_ptr(),
    };
    CuneusStatus::Ok
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_bin_title(bin_name: *const c_char) -> *const c_char {
    let Some(bin_name) = string_from_ptr(bin_name) else {
        return ptr::null();
    };
    title_for_bin(&bin_name).map_or(ptr::null(), |title| title.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_bin_keys(bin_name: *const c_char) -> *const c_char {
    let Some(bin_name) = string_from_ptr(bin_name) else {
        return ptr::null();
    };
    info_for_bin(&bin_name).map_or(ptr::null(), |info| info.keys.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_bin_uses_mouse(bin_name: *const c_char) -> bool {
    let Some(bin_name) = string_from_ptr(bin_name) else {
        return false;
    };
    info_for_bin(&bin_name).is_some_and(|info| info.flags & BIN_FLAG_USES_MOUSE != 0)
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_bin_uses_keyboard(bin_name: *const c_char) -> bool {
    let Some(bin_name) = string_from_ptr(bin_name) else {
        return false;
    };
    info_for_bin(&bin_name).is_some_and(|info| info.flags & BIN_FLAG_USES_KEYBOARD != 0)
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_bin_default_dimensions(
    bin_name: *const c_char,
    out_width: *mut u32,
    out_height: *mut u32,
) -> bool {
    let Some(bin_name) = string_from_ptr(bin_name) else {
        return false;
    };
    let Some((width, height)) = default_dimensions_for_bin(&bin_name) else {
        return false;
    };
    if let Some(out_width) = out_width.as_mut() {
        *out_width = width;
    }
    if let Some(out_height) = out_height.as_mut() {
        *out_height = height;
    }
    true
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
    if params_for_bin(&bin_name_string).is_none() {
        set_last_error(format!("unknown cuneus bin: {bin_name_string}"));
        return ptr::null_mut();
    };

    let socket = match UdpSocket::bind(("127.0.0.1", 0)) {
        Ok(socket) => socket,
        Err(error) => {
            set_last_error(error.to_string());
            return ptr::null_mut();
        }
    };

    let mut child = None;
    if let Some(dir) = string_from_ptr(executable_dir).filter(|value| !value.is_empty()) {
        let (executable, pass_bin_arg) = runner_command(&dir, &bin_name_string);
        let mut command = Command::new(&executable);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        if pass_bin_arg {
            command.arg(&bin_name_string);
        }
        command.env("CUNEUS_REMOTE_PORT", remote_port.to_string());
        if osc_feedback_port != 0 {
            command.env("CUNEUS_OSC_FEEDBACK_PORT", osc_feedback_port.to_string());
        }
        match command.spawn() {
            Ok(process) => child = Some(process),
            Err(error) => {
                set_last_error(format!(
                    "failed to launch {}: {error}",
                    executable.display()
                ));
                return ptr::null_mut();
            }
        }
    }

    let (window_base_width, window_base_height) = default_window_dimensions(&bin_name_string);
    let bin_name_static = Box::leak(bin_name_string.into_boxed_str());

    Box::into_raw(Box::new(CuneusInstance {
        bin_name: bin_name_static,
        remote_port,
        socket,
        child,
        embedded_thread: None,
        window_x: 100,
        window_y: 100,
        window_scale: 1.0,
        window_base_width,
        window_base_height,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_instance_open_embedded(
    bin_name: *const c_char,
    remote_port: u16,
) -> *mut CuneusInstance {
    cuneus_instance_open_embedded_with_feedback(bin_name, remote_port, 0)
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_instance_open_embedded_with_feedback(
    bin_name: *const c_char,
    remote_port: u16,
    osc_feedback_port: u16,
) -> *mut CuneusInstance {
    let Some(bin_name_string) = string_from_ptr(bin_name) else {
        set_last_error("bin_name is null or invalid utf-8");
        return ptr::null_mut();
    };
    if params_for_bin(&bin_name_string).is_none() {
        set_last_error(format!("unknown cuneus bin: {bin_name_string}"));
        return ptr::null_mut();
    };
    if !crate::embedded::can_run_bin(&bin_name_string) {
        set_last_error(format!(
            "embedded cuneus bin is not available: {bin_name_string}"
        ));
        return ptr::null_mut();
    }

    let socket = match UdpSocket::bind(("127.0.0.1", 0)) {
        Ok(socket) => socket,
        Err(error) => {
            set_last_error(error.to_string());
            return ptr::null_mut();
        }
    };

    crate::app::clear_shutdown_request();
    let thread_bin_name = bin_name_string.clone();
    let embedded_thread = match std::thread::Builder::new()
        .name(format!("acuneus-embedded-{thread_bin_name}"))
        .spawn(move || {
            std::env::set_var("CUNEUS_REMOTE_PORT", remote_port.to_string());
            if osc_feedback_port != 0 {
                std::env::set_var("CUNEUS_OSC_FEEDBACK_PORT", osc_feedback_port.to_string());
            } else {
                std::env::remove_var("CUNEUS_OSC_FEEDBACK_PORT");
            }
            if let Err(error) = crate::embedded::run_bin(&thread_bin_name) {
                eprintln!("embedded cuneus {thread_bin_name} exited with error: {error}");
            }
        }) {
        Ok(thread) => thread,
        Err(error) => {
            set_last_error(format!("failed to start embedded cuneus: {error}"));
            return ptr::null_mut();
        }
    };

    let (window_base_width, window_base_height) = default_window_dimensions(&bin_name_string);
    let bin_name_static = Box::leak(bin_name_string.into_boxed_str());

    Box::into_raw(Box::new(CuneusInstance {
        bin_name: bin_name_static,
        remote_port,
        socket,
        child: None,
        embedded_thread: Some(embedded_thread),
        window_x: 100,
        window_y: 100,
        window_scale: 1.0,
        window_base_width,
        window_base_height,
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
    if let Some(thread) = instance.embedded_thread.take() {
        crate::app::request_shutdown();
        let _ = thread.join();
    }
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_instance_poll_child(
    instance: *mut CuneusInstance,
    out_exit_code: *mut i32,
) -> CuneusStatus {
    let Some(instance) = instance.as_mut() else {
        set_last_error("instance is null");
        return CuneusStatus::Null;
    };

    let Some(child) = instance.child.as_mut() else {
        return CuneusStatus::Ok;
    };

    match child.try_wait() {
        Ok(Some(status)) => {
            let exit_code = status.code().unwrap_or(-1);
            if let Some(out_exit_code) = out_exit_code.as_mut() {
                *out_exit_code = exit_code;
            }
            set_last_error(format!(
                "Acuneus child '{}' exited unexpectedly with status {}",
                instance.bin_name, status
            ));
            CuneusStatus::IoError
        }
        Ok(None) => CuneusStatus::Ok,
        Err(error) => {
            set_last_error(format!(
                "failed to query Acuneus child '{}': {}",
                instance.bin_name, error
            ));
            CuneusStatus::IoError
        }
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
        group: param.group.as_ptr(),
        param_type: capi_param_type(param.param_type),
        min_value: param.min_value,
        max_value: param.max_value,
        default_value: param.default_value,
        flags: param.flags,
        options: param
            .options
            .map_or(ptr::null(), |options| options.as_ptr()),
    };
    CuneusStatus::Ok
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_trigger_action(
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
    send(instance, format!("action {id} {value}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_load_media(
    instance: *mut CuneusInstance,
    path: *const c_char,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    let Some(path) = string_from_ptr(path) else {
        return CuneusStatus::InvalidArgument;
    };
    send(instance, format!("load_media {path}"))
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
pub unsafe extern "C" fn cuneus_set_param_string(
    instance: *mut CuneusInstance,
    id: *const c_char,
    value: *const c_char,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    let Some(id) = string_from_ptr(id) else {
        return CuneusStatus::InvalidArgument;
    };
    let Some(value) = string_from_ptr(value) else {
        return CuneusStatus::InvalidArgument;
    };
    send(instance, format!("set_string {id} {value}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_param_bool(
    instance: *mut CuneusInstance,
    id: *const c_char,
    value: bool,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    let Some(id) = string_from_ptr(id) else {
        return CuneusStatus::InvalidArgument;
    };
    send(
        instance,
        format!("set_bool {id} {}", if value { 1 } else { 0 }),
    )
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_pulse(
    instance: *mut CuneusInstance,
    velocity: f32,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, format!("pulse {velocity}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_note(
    instance: *mut CuneusInstance,
    pitch: f32,
    velocity: f32,
) -> CuneusStatus {
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
pub unsafe extern "C" fn cuneus_set_overlay_visible(
    instance: *mut CuneusInstance,
    visible: bool,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(
        instance,
        format!("overlay_visible {}", if visible { 1 } else { 0 }),
    )
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_toggle_overlay(instance: *mut CuneusInstance) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, "toggle_overlay".to_string())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_window_title(
    instance: *mut CuneusInstance,
    title: *const c_char,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    let Some(title) = string_from_ptr(title) else {
        set_last_error("title is null or invalid utf-8");
        return CuneusStatus::InvalidArgument;
    };
    send(instance, format!("window_title {title}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_window_title_bar_visible(
    instance: *mut CuneusInstance,
    visible: bool,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(
        instance,
        format!("title_bar_visible {}", if visible { 1 } else { 0 }),
    )
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_hide_window_title_bar(
    instance: *mut CuneusInstance,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, "hide_title_bar".to_string())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_window_position(
    instance: *mut CuneusInstance,
    x: i32,
    y: i32,
) -> CuneusStatus {
    let Some(instance) = instance.as_mut() else {
        return CuneusStatus::Null;
    };
    instance.window_x = x;
    instance.window_y = y;
    set_child_window_geometry(instance);
    send(instance, format!("window_position {x} {y}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_get_window_position(
    instance: *mut CuneusInstance,
    out_x: *mut i32,
    out_y: *mut i32,
) -> CuneusStatus {
    let Some(instance) = instance.as_mut() else {
        return CuneusStatus::Null;
    };
    let (x, y) =
        get_child_window_position(instance).unwrap_or((instance.window_x, instance.window_y));
    instance.window_x = x;
    instance.window_y = y;
    if let Some(out_x) = out_x.as_mut() {
        *out_x = x;
    }
    if let Some(out_y) = out_y.as_mut() {
        *out_y = y;
    }
    CuneusStatus::Ok
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_window_scale(
    instance: *mut CuneusInstance,
    scale: f32,
) -> CuneusStatus {
    let Some(instance) = instance.as_mut() else {
        return CuneusStatus::Null;
    };
    instance.window_scale = scale.max(0.1);
    set_child_window_geometry(instance);
    send(instance, format!("window_scale {scale}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_window_size(
    instance: *mut CuneusInstance,
    width: u32,
    height: u32,
) -> CuneusStatus {
    let Some(instance) = instance.as_mut() else {
        return CuneusStatus::Null;
    };
    instance.window_base_width = width.max(1);
    instance.window_base_height = height.max(1);
    instance.window_scale = 1.0;
    set_child_window_geometry(instance);
    send(instance, format!("window_size {width} {height}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_time(
    instance: *mut CuneusInstance,
    time_seconds: f32,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, format!("time {time_seconds}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_fps(instance: *mut CuneusInstance, fps: f32) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, format!("fps {fps}"))
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_resolution(
    instance: *mut CuneusInstance,
    width: u32,
    height: u32,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(
        instance,
        format!("resolution {} {}", width.max(1), height.max(1)),
    )
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_set_audio_spectrum(
    instance: *mut CuneusInstance,
    values: *const f32,
    count: usize,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    if values.is_null() || count == 0 {
        return CuneusStatus::InvalidArgument;
    }

    let values = std::slice::from_raw_parts(values, count.min(69));
    let mut message = String::from("audio_spectrum");
    for value in values {
        message.push(' ');
        message.push_str(&value.max(0.0).to_string());
    }
    send(instance, message)
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_discover(instance: *mut CuneusInstance) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(instance, "discover".to_string())
}

#[no_mangle]
pub unsafe extern "C" fn cuneus_subscribe(
    instance: *mut CuneusInstance,
    enabled: bool,
) -> CuneusStatus {
    let Some(instance) = instance.as_ref() else {
        return CuneusStatus::Null;
    };
    send(
        instance,
        format!("subscribe {}", if enabled { 1 } else { 0 }),
    )
}
