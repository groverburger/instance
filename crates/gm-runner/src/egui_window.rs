//! A winit window drawn with egui through its own OpenGL context on the main
//! thread (egui_glow), styled with the shared GameMaker-style theme of `gm-ui`.
//! The Debug Information window and the error dialog are built on it.

use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, PossiblyCurrentGlContext, Version};
use glutin::display::{Display, DisplayApiPreference, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub struct EguiWindow {
    // field order matters for drop: GL objects before the context, the context
    // before the display and the window
    egui: egui_glow::EguiGlow,
    gl: Arc<glow::Context>,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    _display: Display,
    pub window: Arc<Window>,
}

impl EguiWindow {
    /// Creates the window from `attrs` with an OpenGL context and egui.
    pub fn new(el: &ActiveEventLoop, attrs: winit::window::WindowAttributes) -> Result<EguiWindow, String> {
        let window = Arc::new(el.create_window(attrs).map_err(|e| e.to_string())?);
        let (surface, context, display, gl) = create_gl(&window)?;
        let gl = Arc::new(gl);
        let egui = egui_glow::EguiGlow::new(el, gl.clone(), None, Some(window.scale_factor() as f32), false);
        gm_ui::theme::apply(&egui.egui_ctx);
        Ok(EguiWindow { egui, gl, surface, context, _display: display, window })
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Passes a window event to egui (not RedrawRequested / CloseRequested, which
    /// the owner handles); requests a repaint when egui wants one.
    pub fn event(&mut self, ev: &WindowEvent) {
        if let WindowEvent::Resized(s) = ev {
            self.surface.resize(&self.context, NonZeroU32::new(s.width.max(1)).unwrap(), NonZeroU32::new(s.height.max(1)).unwrap());
        }
        if self.egui.on_window_event(&self.window, ev).repaint {
            self.window.request_redraw();
        }
    }

    /// Runs `ui` for one frame and presents it.
    pub fn redraw(&mut self, ui: impl FnMut(&mut egui::Ui)) {
        if !self.context.is_current() {
            let _ = self.context.make_current(&self.surface);
        }
        self.egui.run(&self.window, ui);
        use glow::HasContext;
        // SAFETY: the context is current on this thread.
        unsafe {
            self.gl.clear_color(0.94, 0.94, 0.94, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        self.egui.paint(&self.window);
        let _ = self.surface.swap_buffers(&self.context);
        if self.egui.egui_ctx.has_requested_repaint() {
            self.window.request_redraw();
        }
    }
}

impl Drop for EguiWindow {
    fn drop(&mut self) {
        if !self.context.is_current() {
            let _ = self.context.make_current(&self.surface);
        }
        self.egui.destroy();
    }
}

/// An OpenGL 3.3 context for the window, current on this (the main) thread,
/// without vsync (the main thread also serves the game window).
fn create_gl(win: &Window) -> Result<(Surface<WindowSurface>, PossiblyCurrentContext, Display, glow::Context), String> {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    let rwh = win.window_handle().map_err(|e| e.to_string())?.as_raw();
    let rdh = win.display_handle().map_err(|e| e.to_string())?.as_raw();
    #[cfg(windows)]
    let pref = DisplayApiPreference::Wgl(Some(rwh));
    #[cfg(target_os = "macos")]
    let pref = DisplayApiPreference::Cgl;
    #[cfg(all(unix, not(target_os = "macos")))]
    let pref = DisplayApiPreference::Egl;
    let size = win.inner_size();
    let e = |e: glutin::error::Error| e.to_string();
    // SAFETY: the handles stay valid while the window lives; EguiWindow drops
    // the surface and context before the window.
    unsafe {
        let display = Display::new(rdh, pref).map_err(e)?;
        let tmpl = ConfigTemplateBuilder::new().compatible_with_native_window(rwh).with_alpha_size(8).build();
        let config = display
            .find_configs(tmpl)
            .map_err(e)?
            .reduce(|a, b| if b.num_samples() < a.num_samples() { b } else { a })
            .ok_or("no suitable OpenGL pixel format")?;
        let attrs = ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3)))).build(Some(rwh));
        let ctx = display.create_context(&config, &attrs).map_err(e)?;
        let sattrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            rwh,
            NonZeroU32::new(size.width.max(1)).unwrap(),
            NonZeroU32::new(size.height.max(1)).unwrap(),
        );
        let surface = display.create_window_surface(&config, &sattrs).map_err(e)?;
        let context = ctx.make_current(&surface).map_err(e)?;
        let _ = surface.set_swap_interval(&context, SwapInterval::DontWait);
        let gl = glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s));
        Ok((surface, context, display, gl))
    }
}
