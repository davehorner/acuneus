# Acuneus

Acuneus is a controllable WGSL shader runner and C ABI bridge for using Cuneus-style GPU examples from hosts such as BespokeSynth. It builds the shader examples, exposes their parameters through a stable C DLL, and lets an external host control the running window through OSC/UDP.

The main goals are:

- Run shader examples standalone through the `acuneus` runner.
- Run the same examples from a C host through `acuneus.dll`.
- Keep the C catalog generated from Rust examples so it is easy to maintain.
- Let BespokeSynth open, automate, and control Acuneus windows without embedding Bespoke-specific code into every example.

## What It Builds

Acuneus produces:

- `acuneus.dll`: the C ABI used by hosts.
- `acuneus.exe`: the standalone runner.
- Shader executables/examples such as `roto`, `cuneus-roto`, `cuneus-fluid`, and the rest of the generated catalog.
- `include/acuneus.h` and `include/acuneus_capi.h`: public C headers.
- `examples/generated/cuneus_examples.cmake`: generated list of examples for CMake consumers.

The runner is always built. Its contents are configurable with `ACUNEUS_RUNNER_CONTENT`:

```powershell
$env:ACUNEUS_RUNNER_CONTENT = "both"      # bins and examples
$env:ACUNEUS_RUNNER_CONTENT = "bins"      # compiled bins only
$env:ACUNEUS_RUNNER_CONTENT = "examples"  # embedded examples only
```

## Quick Start

Run a shader directly:

```powershell
cargo run --bin acuneus -- roto
```

Run an example binary:

```powershell
cargo run --bin roto
```

Regenerate the C catalog after adding or changing examples:

```powershell
cargo run --bin acuneus-gen_registry -- --write
```

Build the library and runner:

```powershell
cargo build --release
```

## BespokeSynth

The Bespoke side lives in `R:\w\cpp\BespokeSynth` as `Source/Acuneus.cpp` and `Source/Acuneus.h`.

The Acuneus module can:

- Select any generated shader from the dropdown.
- Open/close an out-of-process Acuneus window.
- Optionally run embedded through the C DLL.
- Auto reopen when the shader dropdown changes.
- Show generated parameter sliders with readable labels.
- Show generated checkbox params for boolean controls.
- Automate overlay, title bar, window position, scale, time, FPS, and render resolution.
- Remember per-shader window position, scale, and resolution.
- Control Bespoke transport bidirectionally for remote-aware shaders such as `roto`.
- Accept audio from a Bespoke patch cable, pass it through to downstream audio modules, and use that signal to automate shader controls.
- Feed Bespoke audio into the `audiovis` shader's spectrum buffer so Audio Visualizer bars respond to generated or live Bespoke audio, not only media/webcam audio.

The Bespoke welcome screen includes an `acuneus/stableaudio` shortcut. It creates this patch:

```text
stableaudio -> acuneus/audio visualizer -> gain -> output
```

StableAudio is put into auto-generation mode, Acuneus opens the `audiovis` shader, and the Acuneus module sends a 69-value spectrum frame to the shader while passing audio through to `gain`.

The welcome screen also includes an `acuneus/synth` shortcut:

```text
keyboarddisplay -> acuneus/synth -> gain -> output
```

Keyboard notes are sent to the Acuneus synth over the remote note path. The synth returns interleaved stereo PCM feedback to Bespoke, and Bespoke plays that PCM from the Acuneus node's audio output cable. The synth exposes a `Local Audio` checkbox: standalone synths start with local audio on, while Bespoke-hosted synths default it off so the audible route is the Bespoke patch. Enable `Local Audio` if you want the synth process to play through its own audio device too.

From the BespokeSynth repo, test with:

```powershell
task run
```

## C ABI

The C ABI is declared in `include/acuneus.h`. The exported type names still use the existing `Cuneus*` ABI names for compatibility with the current host code, but the project, headers, runner, and Bespoke module are Acuneus-specific.

Important entry points:

```c
size_t cuneus_bin_count(void);
const char* cuneus_bin_name(size_t index);
bool cuneus_bin_default_dimensions(const char* bin_name, uint32_t* out_width, uint32_t* out_height);

CuneusInstance* cuneus_instance_open_with_feedback(
    const char* bin_name,
    const char* executable_dir,
    uint16_t remote_port,
    uint16_t osc_feedback_port);

CuneusStatus cuneus_set_param_f32(CuneusInstance* instance, const char* id, float value);
CuneusStatus cuneus_set_param_color3(CuneusInstance* instance, const char* id, float r, float g, float b);
CuneusStatus cuneus_set_param_bool(CuneusInstance* instance, const char* id, bool value);
CuneusStatus cuneus_set_transport(CuneusInstance* instance, float bpm, float beat, float measure);

CuneusStatus cuneus_set_overlay_visible(CuneusInstance* instance, bool visible);
CuneusStatus cuneus_set_window_title(CuneusInstance* instance, const char* title);
CuneusStatus cuneus_set_window_title_bar_visible(CuneusInstance* instance, bool visible);
CuneusStatus cuneus_set_window_position(CuneusInstance* instance, int32_t x, int32_t y);
CuneusStatus cuneus_set_window_scale(CuneusInstance* instance, float scale);
CuneusStatus cuneus_set_window_size(CuneusInstance* instance, uint32_t width, uint32_t height);
CuneusStatus cuneus_set_time(CuneusInstance* instance, float time_seconds);
CuneusStatus cuneus_set_fps(CuneusInstance* instance, float fps);
CuneusStatus cuneus_set_resolution(CuneusInstance* instance, uint32_t width, uint32_t height);
CuneusStatus cuneus_set_audio_spectrum(CuneusInstance* instance, const float* values, size_t count);
```

`cuneus_set_audio_spectrum` accepts up to 69 floats. The layout matches `.with_audio_spectrum(69)`: indices `0..63` are frequency magnitudes, index `64` is BPM, and indices `65..68` are bass, mid, high, and total energy.

`cuneus_set_param_bool` is the C ABI path for generated checkbox params. It maps to the text command `set_bool <id> 0|1` and to OSC bool param updates.

On Windows, Acuneus launches child processes without opening a console window. The C DLL can also move and resize out-of-process child windows directly, so host-side window control works even when a shader does not yet implement every remote command internally.

## OSC And UDP

Acuneus uses a strict OSC namespace:

```text
/acuneus/cuneus/*
```

There is no legacy `/cuneus/*` compatibility path.

Common incoming OSC addresses:

```text
/acuneus/cuneus/discover
/acuneus/cuneus/subscribe
/acuneus/cuneus/param/<id>
/acuneus/cuneus/color/<id>
/acuneus/cuneus/bool/<id>
/acuneus/cuneus/checkbox/<id>
/acuneus/cuneus/pulse
/acuneus/cuneus/note
/acuneus/cuneus/transport
/acuneus/cuneus/overlay
/acuneus/cuneus/overlay/toggle
/acuneus/cuneus/window/title
/acuneus/cuneus/window/titlebar
/acuneus/cuneus/window/titlebar/hide
/acuneus/cuneus/window/position
/acuneus/cuneus/window/scale
/acuneus/cuneus/window/size
/acuneus/cuneus/time
/acuneus/cuneus/fps
/acuneus/cuneus/resolution
/acuneus/cuneus/audio_spectrum
```

For bool params, send an OSC bool to `/acuneus/cuneus/param/<id>`, `/acuneus/cuneus/bool/<id>`, or `/acuneus/cuneus/checkbox/<id>`. Text remotes can use `set_bool <id> 0|1`.

Common feedback addresses:

```text
/acuneus/cuneus/status
/acuneus/cuneus/bin
/acuneus/cuneus/param_count
/acuneus/cuneus/param_desc
/acuneus/cuneus/param/<id>
/acuneus/cuneus/audio_pcm
/acuneus/cuneus/transport
/acuneus/cuneus/transport/tempo
/acuneus/cuneus/transport/play
/acuneus/cuneus/transport/reset
/acuneus/cuneus/transport/shift_beats
```

The C ABI sends simple UDP text commands to the runner, and remote-aware examples accept both those text commands and OSC packets.

## Generated Catalog

The catalog is generated from files in `examples/`. The generator scans each example for:

- The example name.
- `uniform_params!` fields.
- Slider labels/ranges/defaults.
- Boolean fields (`bool`) and bool-like `u32` fields ending in `_enabled`, starting with `enable_`, or starting with `use_`.
- Color labels.
- The example's desired default window dimensions from `ShaderApp::new(...)`.

Generated files include:

- `src/bin_registry.rs`
- `src/embedded_generated.rs`
- `examples/generated/cuneus_examples.cmake`

Run the generator whenever examples or their parameter metadata change:

```powershell
cargo run --bin acuneus-gen_registry -- --write
```

## Adding Remote Support To An Example

Use `RemoteControl::from_env()` in the example and drain `RemoteCommand` values each frame. At minimum, handle generated params and the shared UI/window controls:

```rust
match command {
    RemoteCommand::SetF32 { id, value } => { /* update param */ }
    RemoteCommand::SetColor3 { id, value } => { /* update color param */ }
    RemoteCommand::OverlayVisible { visible } => {
        self.base.key_handler.show_ui = visible;
    }
    RemoteCommand::TitleBarVisible { visible } => {
        core.window().set_decorations(visible);
    }
    RemoteCommand::WindowTitle { title } => {
        core.window().set_title(&title);
    }
    RemoteCommand::WindowPosition { x, y } => {
        core.window().set_outer_position(acuneus::winit::dpi::PhysicalPosition::new(x, y));
    }
    RemoteCommand::WindowScale { scale } => { /* resize from native dimensions */ }
    RemoteCommand::WindowSize { width, height } => { /* request window size */ }
    RemoteCommand::Time { seconds } => { /* drive shader time */ }
    RemoteCommand::Fps { fps } => { /* drive delta */ }
    RemoteCommand::Resolution { width, height } => { /* resize render target */ }
    _ => {}
}
```

`roto` is the reference example for the full remote path.

## Keyboard

- `F`: fullscreen/minimal screen.
- `H`: toggle the in-window overlay.

## Dependencies

- Rust stable.
- A GPU supported by `wgpu`.
- GStreamer is used by media-capable shaders and is enabled by default.

Build without media support when you only need pure GPU compute examples:

```powershell
cargo build --release --no-default-features
```

## Project Notes

Acuneus is still built on the original Cuneus shader engine concepts: WGSL compute shaders, `wgpu`, `winit`, `egui`, hot reload, multi-pass compute, atomics, media textures, audio analysis, and export support. The Acuneus layer is the hostable, generator-driven runner and C bridge around that engine.
