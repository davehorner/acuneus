use crate::{Core, ShaderManager};
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowAttributes,
};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn clear_shutdown_request() {
    SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
}

pub fn request_shutdown() {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

pub struct ShaderApp {
    window_title: String,
    window_size: (u32, u32),
    core: Option<Core>,
}

impl ShaderApp {
    pub fn new(window_title: &str, width: u32, height: u32) -> (Self, EventLoop<()>) {
        let mut event_loop_builder = EventLoop::builder();
        #[cfg(target_os = "windows")]
        event_loop_builder.with_any_thread(true);
        let event_loop = event_loop_builder
            .build()
            .expect("Failed to create event loop");

        //note: No window creation here - will happen in resumed event
        let app = Self {
            window_title: String::from(window_title),
            window_size: (width, height),
            core: None,
        };

        (app, event_loop)
    }

    pub fn run<S: ShaderManager + 'static>(
        self,
        event_loop: EventLoop<()>,
        shader_creator: impl FnOnce(&Core) -> S + 'static,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut handler = ShaderAppHandler {
            app: self,
            shader_creator: Some(Box::new(shader_creator)),
            shader: None,
            first_render: true,
        };

        Ok(event_loop.run_app(&mut handler)?)
    }

    pub fn core(&self) -> Option<&Core> {
        self.core.as_ref()
    }
}

// This struct implements ApplicationHandler to handle winit events
struct ShaderAppHandler<S: ShaderManager> {
    app: ShaderApp,
    shader_creator: Option<Box<dyn FnOnce(&Core) -> S + 'static>>,
    shader: Option<S>,
    first_render: bool,
}

impl<S: ShaderManager> ApplicationHandler for ShaderAppHandler<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("creating shader window: {}", self.app.window_title);
        let window_attributes = WindowAttributes::default()
            .with_inner_size(LogicalSize::new(
                self.app.window_size.0,
                self.app.window_size.1,
            ))
            .with_title(&self.app.window_title)
            .with_resizable(true);
        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");
        window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);
        let core = pollster::block_on(Core::new(window));
        // Initialize the shader with the core if it hasn't been initialized yet
        if let Some(shader_creator) = self.shader_creator.take() {
            let shader = shader_creator(&core);
            self.shader = Some(shader);
        }

        self.app.core = Some(core);
        info!("shader window initialized");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Only process events if core and shader are initialized
        if let (Some(core), Some(shader)) = (&self.app.core, &mut self.shader) {
            if window_id == core.window().id() && !shader.handle_input(core, &event) {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("window close requested");
                        event_loop.exit();
                    }
                    WindowEvent::Resized(size) => {
                        if let Some(core) = &mut self.app.core {
                            if core.size == size {
                                return;
                            }
                            core.resize(size);
                            shader.resize(core);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        shader.update(core);
                        match shader.render(core) {
                            Ok(_) => {
                                if self.first_render {
                                    self.first_render = false;
                                }
                            }
                            Err(crate::SurfaceError::SkipFrame) => {}
                            Err(crate::SurfaceError::Lost | crate::SurfaceError::Outdated) => {
                                if let Some(core) = &mut self.app.core {
                                    core.resize(core.size);
                                }
                            }
                            Err(crate::SurfaceError::OutOfMemory) => {
                                error!("GPU out of memory, exiting");
                                event_loop.exit();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
            info!("shutdown requested");
            event_loop.exit();
            return;
        }

        if let Some(core) = &self.app.core {
            core.window().request_redraw();
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        // No special handling needed for new events
    }
}
