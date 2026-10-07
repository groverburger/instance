//! OpenGL 3.3 backend (glow + glutin).
//!
//! * Every render target (back buffer and surfaces) is a framebuffer object with
//!   an RGBA8 colour texture and a 24-bit depth renderbuffer. Target pixel row y
//!   is stored at FBO row y (no flip), so a target's texture can be sampled with
//!   the same top-first convention as uploaded images; only the final blit to the
//!   window flips.
//! * Vertices arrive in pixel-homogeneous space; the vertex shader maps them to
//!   clip space with the target size. Draws are batched while the state
//!   (target, texture, blend, scissor, depth, cull, fog, sampler) is unchanged.
//! * Image textures are uploaded on first use and cached by allocation (a
//!   `Weak` keeps the address from being reused); dead entries are swept.

use super::backend::{Backend, Cull, Prim, RState, Target, Tex, Vertex};
use super::{Clip, Texture};
use glow::HasContext;
use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version};
use glutin::display::{Display, DisplayApiPreference, GlDisplay};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::{Rc, Weak};

const VS: &str = r#"#version 330 core
layout(location = 0) in vec4 a_pos;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec4 a_col;
layout(location = 3) in float a_fog;
uniform vec2 u_size;
uniform float u_eps;
out vec2 v_uv;
out vec4 v_col;
out float v_fog;
void main() {
    v_uv = a_uv;
    v_col = a_col;
    v_fog = a_fog;
    // A tiny shift up-left makes ties on pixel centres resolve like the
    // software rasteriser (top-left fill rule in y-down pixel space, exact
    // texel boundaries rounding up).
    float e = u_eps;
    gl_Position = vec4((a_pos.x - e * a_pos.w) * 2.0 / u_size.x - a_pos.w,
                       (a_pos.y - e * a_pos.w) * 2.0 / u_size.y - a_pos.w,
                       a_pos.z * 2.0 - a_pos.w,
                       a_pos.w);
}
"#;

const FS: &str = r#"#version 330 core
in vec2 v_uv;
in vec4 v_col;
in float v_fog;
uniform sampler2D u_tex;
uniform vec4 u_fog;
uniform float u_atest;
out vec4 o_col;
void main() {
    vec4 c = v_col * texture(u_tex, v_uv);
    if (c.a <= u_atest) discard;
    if (u_fog.a > 0.5) c.rgb = mix(c.rgb, u_fog.rgb, clamp(v_fog, 0.0, 1.0));
    o_col = c;
}
"#;

const FLOATS_PER_VERTEX: usize = 11;

struct Fbo {
    fbo: glow::Framebuffer,
    tex: glow::Texture,
    depth: glow::Renderbuffer,
    w: u32,
    h: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum TexKey {
    None,
    Img(usize),
    Surf(i32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct BatchKey {
    st: RState,
    tex: TexKey,
    mode: u32,
}

/// What keeps the native window alive.
enum WindowHold {
    #[allow(dead_code)]
    Winit(std::sync::Arc<winit::window::Window>),
    #[cfg(windows)]
    Hidden(isize),
    #[allow(dead_code)]
    None,
}

impl Drop for WindowHold {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let WindowHold::Hidden(h) = self {
            win32::destroy(*h);
        }
    }
}

pub struct GlBackend {
    gl: glow::Context,
    // field order matters for drop: GL objects die with the context
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    _display: Display,
    _hold: WindowHold,
    prog: glow::Program,
    u_size: Option<glow::UniformLocation>,
    u_fog: Option<glow::UniformLocation>,
    u_atest: Option<glow::UniformLocation>,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    /// [nearest/linear][clamp/repeat]
    samplers: [[glow::Sampler; 2]; 2],
    white: glow::Texture,
    back: Fbo,
    surfaces: HashMap<i32, Fbo>,
    target: Target,
    images: HashMap<usize, (Weak<Texture>, glow::Texture)>,
    sweep_at: usize,
    batch: Vec<f32>,
    key: Option<BatchKey>,
    /// Temporary texture used when a surface is drawn onto itself.
    scratch: Option<(glow::Texture, u32, u32)>,
    pub draw_calls: u64,
    win_size: (u32, u32),
}

fn gl_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl GlBackend {
    /// Creates a context for a winit window (vsync per `vsync`).
    pub fn for_window(win: std::sync::Arc<winit::window::Window>, vsync: bool) -> Result<GlBackend, String> {
        use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
        let rwh = win.window_handle().map_err(gl_err)?.as_raw();
        let rdh = win.display_handle().map_err(gl_err)?.as_raw();
        let size = win.inner_size();
        Self::create(rdh, rwh, size.width.max(1), size.height.max(1), vsync, WindowHold::Winit(win))
    }

    /// Creates a context on a hidden native window (offscreen rendering for
    /// tests and headless runs).
    #[cfg(windows)]
    pub fn hidden() -> Result<GlBackend, String> {
        let hwnd = win32::create_hidden()?;
        let mut h = raw_window_handle::Win32WindowHandle::new(std::num::NonZeroIsize::new(hwnd).ok_or("CreateWindowEx failed")?);
        h.hinstance = std::num::NonZeroIsize::new(win32::module());
        let rwh = RawWindowHandle::Win32(h);
        let rdh = RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new());
        Self::create(rdh, rwh, 16, 16, false, WindowHold::Hidden(hwnd))
    }

    #[cfg(not(windows))]
    pub fn hidden() -> Result<GlBackend, String> {
        Err("hidden OpenGL contexts are only implemented on Windows".into())
    }

    fn create(rdh: RawDisplayHandle, rwh: RawWindowHandle, w: u32, h: u32, vsync: bool, hold: WindowHold) -> Result<GlBackend, String> {
        #[cfg(windows)]
        let pref = DisplayApiPreference::Wgl(Some(rwh));
        #[cfg(not(windows))]
        let pref = DisplayApiPreference::Egl;
        // SAFETY: the handles are valid for the lifetime of `hold`, which the
        // backend keeps alive (dropped after the context and surface).
        unsafe {
            let display = Display::new(rdh, pref).map_err(gl_err)?;
            let tmpl = ConfigTemplateBuilder::new()
                .compatible_with_native_window(rwh)
                .with_alpha_size(8)
                .with_depth_size(24)
                .build();
            let config = display
                .find_configs(tmpl)
                .map_err(gl_err)?
                .reduce(|a, b| if b.num_samples() < a.num_samples() { b } else { a })
                .ok_or("no suitable OpenGL pixel format")?;
            let attrs = ContextAttributesBuilder::new()
                .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
                .build(Some(rwh));
            let ctx = display.create_context(&config, &attrs).map_err(gl_err)?;
            let sattrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
                rwh,
                NonZeroU32::new(w.max(1)).unwrap(),
                NonZeroU32::new(h.max(1)).unwrap(),
            );
            let surface = display.create_window_surface(&config, &sattrs).map_err(gl_err)?;
            let context = ctx.make_current(&surface).map_err(gl_err)?;
            let _ = surface.set_swap_interval(
                &context,
                if vsync { SwapInterval::Wait(NonZeroU32::new(1).unwrap()) } else { SwapInterval::DontWait },
            );
            let gl = glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s));
            let ver = gl.get_parameter_string(glow::VERSION);
            let renderer = gl.get_parameter_string(glow::RENDERER);
            eprintln!("gm-runner: OpenGL {} ({})", ver, renderer);
            let prog = Self::program(&gl)?;
            let vao = gl.create_vertex_array().map_err(gl_err)?;
            let vbo = gl.create_buffer().map_err(gl_err)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = (FLOATS_PER_VERTEX * 4) as i32;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 16);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 24);
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, glow::FLOAT, false, stride, 40);
            let mut sl = Vec::new();
            for filter in [glow::NEAREST, glow::LINEAR] {
                for wrap in [glow::CLAMP_TO_EDGE, glow::REPEAT] {
                    let s = gl.create_sampler().map_err(gl_err)?;
                    gl.sampler_parameter_i32(s, glow::TEXTURE_MIN_FILTER, filter as i32);
                    gl.sampler_parameter_i32(s, glow::TEXTURE_MAG_FILTER, filter as i32);
                    gl.sampler_parameter_i32(s, glow::TEXTURE_WRAP_S, wrap as i32);
                    gl.sampler_parameter_i32(s, glow::TEXTURE_WRAP_T, wrap as i32);
                    sl.push(s);
                }
            }
            let samplers = [[sl[0], sl[1]], [sl[2], sl[3]]];
            let white = Self::upload_raw(&gl, 1, 1, &[255, 255, 255, 255])?;
            let back = Self::make_fbo(&gl, 640, 480)?;
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            let u_size = gl.get_uniform_location(prog, "u_size");
            let u_fog = gl.get_uniform_location(prog, "u_fog");
            let u_atest = gl.get_uniform_location(prog, "u_atest");
            let u_tex = gl.get_uniform_location(prog, "u_tex");
            gl.use_program(Some(prog));
            gl.uniform_1_i32(u_tex.as_ref(), 0);
            // one sub-pixel step of the rasteriser (vertices are on a coarser grid)
            let bits = gl.get_parameter_i32(glow::SUBPIXEL_BITS).clamp(4, 12);
            eprintln!("gm-runner: OpenGL sub-pixel bits: {}", bits);
            gl.uniform_1_f32(gl.get_uniform_location(prog, "u_eps").as_ref(), 1.0 / (1u32 << bits) as f32);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(back.fbo));
            Ok(GlBackend {
                gl,
                surface,
                context,
                _display: display,
                _hold: hold,
                prog,
                u_size,
                u_fog,
                u_atest,
                vao,
                vbo,
                samplers,
                white,
                back,
                surfaces: HashMap::new(),
                target: Target::Back,
                images: HashMap::new(),
                sweep_at: 1024,
                batch: Vec::with_capacity(1 << 16),
                key: None,
                scratch: None,
                draw_calls: 0,
                win_size: (w, h),
            })
        }
    }

    unsafe fn program(gl: &glow::Context) -> Result<glow::Program, String> {
        let prog = gl.create_program()?;
        let mut shaders = Vec::new();
        for (kind, src) in [(glow::VERTEX_SHADER, VS), (glow::FRAGMENT_SHADER, FS)] {
            let s = gl.create_shader(kind)?;
            gl.shader_source(s, src);
            gl.compile_shader(s);
            if !gl.get_shader_compile_status(s) {
                return Err(format!("shader: {}", gl.get_shader_info_log(s)));
            }
            gl.attach_shader(prog, s);
            shaders.push(s);
        }
        gl.link_program(prog);
        if !gl.get_program_link_status(prog) {
            return Err(format!("link: {}", gl.get_program_info_log(prog)));
        }
        for s in shaders {
            gl.detach_shader(prog, s);
            gl.delete_shader(s);
        }
        Ok(prog)
    }

    unsafe fn upload_raw(gl: &glow::Context, w: u32, h: u32, px: &[u8]) -> Result<glow::Texture, String> {
        let t = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(t));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            w as i32,
            h as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(px)),
        );
        Ok(t)
    }

    unsafe fn make_fbo(gl: &glow::Context, w: u32, h: u32) -> Result<Fbo, String> {
        let (w, h) = (w.max(1), h.max(1));
        let zeros = vec![0u8; (w * h * 4) as usize];
        let tex = Self::upload_raw(gl, w, h, &zeros)?;
        let depth = gl.create_renderbuffer()?;
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
        gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, w as i32, h as i32);
        let fbo = gl.create_framebuffer()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        gl.framebuffer_renderbuffer(glow::FRAMEBUFFER, glow::DEPTH_ATTACHMENT, glow::RENDERBUFFER, Some(depth));
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            return Err("incomplete framebuffer".into());
        }
        gl.disable(glow::SCISSOR_TEST);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear_depth_f32(1.0);
        gl.depth_mask(true);
        gl.color_mask(true, true, true, true);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        Ok(Fbo { fbo, tex, depth, w, h })
    }

    unsafe fn delete_fbo(gl: &glow::Context, f: &Fbo) {
        gl.delete_framebuffer(f.fbo);
        gl.delete_texture(f.tex);
        gl.delete_renderbuffer(f.depth);
    }

    fn fbo(&self, t: Target) -> Option<&Fbo> {
        match t {
            Target::Back => Some(&self.back),
            Target::Surface(id) => self.surfaces.get(&id),
        }
    }

    fn bind_target(&self) {
        if let Some(f) = self.fbo(self.target) {
            // SAFETY: valid GL objects owned by this backend; context is current.
            unsafe {
                self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f.fbo));
                self.gl.viewport(0, 0, f.w as i32, f.h as i32);
            }
        }
    }

    /// GL texture of an image, uploading it on first use.
    fn image_texture(&mut self, img: &Rc<Texture>) -> glow::Texture {
        let key = Rc::as_ptr(img) as usize;
        if let Some((w, t)) = self.images.get(&key) {
            if w.strong_count() > 0 {
                return *t;
            }
        }
        if let Some((_, old)) = self.images.remove(&key) {
            // SAFETY: texture created by this backend.
            unsafe { self.gl.delete_texture(old) };
        }
        // SAFETY: context is current; data length matches w*h*4.
        let t = unsafe {
            let bytes: &[u8] = std::slice::from_raw_parts(img.px.as_ptr() as *const u8, img.px.len() * 4);
            Self::upload_raw(&self.gl, img.w, img.h, bytes).unwrap_or(self.white)
        };
        self.images.insert(key, (Rc::downgrade(img), t));
        if self.images.len() >= self.sweep_at {
            self.sweep();
            self.sweep_at = (self.images.len() * 2).max(1024);
        }
        t
    }

    /// Frees textures of images that no longer exist.
    pub fn sweep(&mut self) {
        let dead: Vec<usize> = self.images.iter().filter(|(_, (w, _))| w.strong_count() == 0).map(|(k, _)| *k).collect();
        for k in dead {
            if let Some((_, t)) = self.images.remove(&k) {
                // SAFETY: texture created by this backend.
                unsafe { self.gl.delete_texture(t) };
            }
        }
    }

    fn push(&mut self, v: &Vertex) {
        let v = &super::backend::snap(v);
        self.batch.extend_from_slice(&[v.x, v.y, v.z, v.w, v.u, v.v, v.c[0], v.c[1], v.c[2], v.c[3], v.fog]);
    }

    fn blend_factor(f: u8) -> u32 {
        match f {
            1 => glow::ZERO,
            2 => glow::ONE,
            3 => glow::SRC_COLOR,
            4 => glow::ONE_MINUS_SRC_COLOR,
            5 => glow::SRC_ALPHA,
            6 => glow::ONE_MINUS_SRC_ALPHA,
            7 => glow::DST_ALPHA,
            8 => glow::ONE_MINUS_DST_ALPHA,
            9 => glow::DST_COLOR,
            10 => glow::ONE_MINUS_DST_COLOR,
            11 => glow::SRC_ALPHA_SATURATE,
            _ => glow::ONE,
        }
    }

    fn tex_key(tex: Option<&Tex>) -> TexKey {
        match tex {
            None => TexKey::None,
            Some(Tex::Img(i)) => TexKey::Img(Rc::as_ptr(i) as usize),
            Some(Tex::Surf { id, .. }) => TexKey::Surf(*id),
        }
    }

    /// Executes the pending batch.
    fn flush_batch(&mut self) {
        let Some(key) = self.key.take() else { return };
        if self.batch.is_empty() {
            return;
        }
        let gl = &self.gl;
        let (tw, th) = self.fbo(self.target).map(|f| (f.w, f.h)).unwrap_or((1, 1));
        // SAFETY: all objects belong to this backend and the context is current.
        unsafe {
            let tex = match key.tex {
                TexKey::None => self.white,
                TexKey::Img(k) => self.images.get(&k).map(|e| e.1).unwrap_or(self.white),
                TexKey::Surf(id) => {
                    if self.target == Target::Surface(id) {
                        // feedback: copy the surface first
                        let (w, h) = (tw, th);
                        let need_new = !matches!(self.scratch, Some((_, sw, sh)) if sw == w && sh == h);
                        if need_new {
                            if let Some((t, _, _)) = self.scratch.take() {
                                gl.delete_texture(t);
                            }
                            let z = vec![0u8; (w * h * 4) as usize];
                            let t = Self::upload_raw(gl, w, h, &z).unwrap_or(self.white);
                            self.scratch = Some((t, w, h));
                        }
                        let t = self.scratch.unwrap().0;
                        gl.bind_texture(glow::TEXTURE_2D, Some(t));
                        gl.copy_tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, 0, 0, w as i32, h as i32);
                        t
                    } else {
                        self.surfaces.get(&id).map(|f| f.tex).unwrap_or(self.white)
                    }
                }
            };
            let st = &key.st;
            gl.use_program(Some(self.prog));
            gl.bind_vertex_array(Some(self.vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.bind_sampler(0, Some(self.samplers[st.bilinear as usize][st.repeat as usize]));
            gl.uniform_2_f32(self.u_size.as_ref(), tw as f32, th as f32);
            match st.fog {
                Some(c) => gl.uniform_4_f32(self.u_fog.as_ref(), c[0], c[1], c[2], 1.0),
                None => gl.uniform_4_f32(self.u_fog.as_ref(), 0.0, 0.0, 0.0, 0.0),
            }
            gl.uniform_1_f32(self.u_atest.as_ref(), st.alpha_test.unwrap_or(-1.0));
            let c = st.clip;
            let (x0, y0) = (c.x0.max(0), c.y0.max(0));
            let (x1, y1) = (c.x1.min(tw as i32), c.y1.min(th as i32));
            if x1 <= x0 || y1 <= y0 {
                self.batch.clear();
                return;
            }
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(x0, y0, x1 - x0, y1 - y0);
            gl.enable(glow::BLEND);
            gl.blend_func(Self::blend_factor(st.blend.src), Self::blend_factor(st.blend.dst));
            if st.depth_test || st.depth_write {
                gl.enable(glow::DEPTH_TEST);
                gl.depth_func(if st.depth_test { glow::LEQUAL } else { glow::ALWAYS });
                gl.depth_mask(st.depth_write);
            } else {
                gl.disable(glow::DEPTH_TEST);
                gl.depth_mask(false);
            }
            match st.cull {
                Cull::None => gl.disable(glow::CULL_FACE),
                Cull::KeepCcw => {
                    // pixel space has y down and maps to NDC without a flip, so a
                    // counter-clockwise-on-screen triangle is clockwise in NDC
                    gl.enable(glow::CULL_FACE);
                    gl.front_face(glow::CW);
                    gl.cull_face(glow::BACK);
                }
            }
            gl.color_mask(st.mask[0], st.mask[1], st.mask[2], st.mask[3]);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            let bytes = std::slice::from_raw_parts(self.batch.as_ptr() as *const u8, self.batch.len() * 4);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            gl.draw_arrays(key.mode, 0, (self.batch.len() / FLOATS_PER_VERTEX) as i32);
            gl.color_mask(true, true, true, true);
        }
        self.draw_calls += 1;
        self.batch.clear();
    }

    /// Presents the back buffer in the window: clears to `outside` and blits the
    /// region at (ox, oy) scaled by (sx, sy), then swaps.
    #[allow(clippy::too_many_arguments)]
    pub fn present(&mut self, win_w: u32, win_h: u32, ox: f64, oy: f64, sx: f64, sy: f64, outside: [u8; 4], linear: bool) {
        self.flush_batch();
        let (ww, wh) = (win_w.max(1), win_h.max(1));
        if self.win_size != (ww, wh) {
            self.surface.resize(&self.context, NonZeroU32::new(ww).unwrap(), NonZeroU32::new(wh).unwrap());
            self.win_size = (ww, wh);
        }
        let (bw, bh) = (self.back.w as f64, self.back.h as f64);
        // SAFETY: valid objects, current context.
        unsafe {
            let gl = &self.gl;
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            gl.disable(glow::SCISSOR_TEST);
            gl.viewport(0, 0, ww as i32, wh as i32);
            gl.color_mask(true, true, true, true);
            gl.clear_color(outside[0] as f32 / 255.0, outside[1] as f32 / 255.0, outside[2] as f32 / 255.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.back.fbo));
            let dx0 = ox.round() as i32;
            let dx1 = (ox + bw * sx).round() as i32;
            let top = wh as f64 - oy;
            let dy0 = top.round() as i32;
            let dy1 = (top - bh * sy).round() as i32;
            gl.blit_framebuffer(
                0,
                0,
                self.back.w as i32,
                self.back.h as i32,
                dx0,
                dy0,
                dx1,
                dy1,
                glow::COLOR_BUFFER_BIT,
                if linear { glow::LINEAR } else { glow::NEAREST },
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        let _ = self.surface.swap_buffers(&self.context);
        self.bind_target();
    }

    pub fn set_vsync(&mut self, on: bool) {
        let _ = self.surface.set_swap_interval(
            &self.context,
            if on { SwapInterval::Wait(NonZeroU32::new(1).unwrap()) } else { SwapInterval::DontWait },
        );
    }

    /// Clips a rectangle to a target; returns (x, y, w, h) inside it.
    fn clip_rect(&self, t: Target, x: i32, y: i32, w: i32, h: i32) -> Option<(i32, i32, i32, i32)> {
        let f = self.fbo(t)?;
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(f.w as i32);
        let y1 = (y + h).min(f.h as i32);
        if x1 <= x0 || y1 <= y0 {
            None
        } else {
            Some((x0, y0, x1 - x0, y1 - y0))
        }
    }
}

impl Backend for GlBackend {
    fn name(&self) -> &'static str {
        "opengl"
    }

    fn resize_back(&mut self, w: u32, h: u32) {
        self.flush_batch();
        // SAFETY: current context, objects owned by us.
        unsafe {
            if let Ok(f) = Self::make_fbo(&self.gl, w, h) {
                Self::delete_fbo(&self.gl, &self.back);
                self.back = f;
            }
        }
        self.bind_target();
    }

    fn back_size(&self) -> (u32, u32) {
        (self.back.w, self.back.h)
    }

    fn surface_create(&mut self, id: i32, w: u32, h: u32) {
        self.flush_batch();
        // SAFETY: as above.
        unsafe {
            if let Some(old) = self.surfaces.remove(&id) {
                Self::delete_fbo(&self.gl, &old);
            }
            if let Ok(f) = Self::make_fbo(&self.gl, w, h) {
                self.surfaces.insert(id, f);
            }
        }
        self.bind_target();
    }

    fn surface_free(&mut self, id: i32) {
        self.flush_batch();
        if let Some(f) = self.surfaces.remove(&id) {
            // SAFETY: as above.
            unsafe { Self::delete_fbo(&self.gl, &f) };
        }
        if self.target == Target::Surface(id) {
            self.target = Target::Back;
        }
        self.bind_target();
    }

    fn set_target(&mut self, t: Target) {
        let t = if self.fbo(t).is_some() { t } else { Target::Back };
        if t != self.target {
            self.flush_batch();
            self.target = t;
            self.bind_target();
        }
    }

    fn target(&self) -> Target {
        self.target
    }

    fn clear(&mut self, clip: Clip, c: [u8; 4], depth: bool) {
        self.flush_batch();
        let Some(f) = self.fbo(self.target) else { return };
        let (x0, y0) = (clip.x0.max(0), clip.y0.max(0));
        let (x1, y1) = (clip.x1.min(f.w as i32), clip.y1.min(f.h as i32));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        // SAFETY: current context.
        unsafe {
            let gl = &self.gl;
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(x0, y0, x1 - x0, y1 - y0);
            gl.color_mask(true, true, true, true);
            gl.clear_color(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0);
            let mut bits = glow::COLOR_BUFFER_BIT;
            if depth {
                gl.depth_mask(true);
                gl.clear_depth_f32(1.0);
                bits |= glow::DEPTH_BUFFER_BIT;
            }
            gl.clear(bits);
        }
    }

    fn clear_depth(&mut self) {
        self.flush_batch();
        // SAFETY: current context.
        unsafe {
            let gl = &self.gl;
            gl.disable(glow::SCISSOR_TEST);
            gl.depth_mask(true);
            gl.clear_depth_f32(1.0);
            gl.clear(glow::DEPTH_BUFFER_BIT);
        }
    }

    fn draw(&mut self, st: &RState, tex: Option<&Tex>, prim: Prim, v: &[Vertex]) {
        if let Some(Tex::Surf { id, .. }) = tex {
            if !self.surfaces.contains_key(id) {
                return;
            }
        }
        if let Some(Tex::Img(i)) = tex {
            if i.w == 0 || i.h == 0 {
                return;
            }
            self.image_texture(i);
        }
        let plain2d = v.iter().all(|p| p.w == 1.0);
        let mode = match prim {
            Prim::Lines if !plain2d => glow::LINES,
            Prim::Points if !plain2d => glow::POINTS,
            _ => glow::TRIANGLES,
        };
        let key = BatchKey { st: *st, tex: Self::tex_key(tex), mode };
        if self.key != Some(key) || self.batch.len() > (1 << 20) {
            self.flush_batch();
            self.key = Some(key);
        }
        match prim {
            Prim::Triangles => {
                for p in v.chunks_exact(3) {
                    self.push(&p[0]);
                    self.push(&p[1]);
                    self.push(&p[2]);
                }
            }
            Prim::Quads => {
                for q in v.chunks_exact(4) {
                    for i in [0, 1, 2, 0, 2, 3] {
                        self.push(&q[i]);
                    }
                }
            }
            Prim::Lines if mode == glow::TRIANGLES => {
                for l in v.chunks_exact(2) {
                    let q = super::soft::line_quad(&l[0], &l[1]);
                    for i in [0, 1, 2, 0, 2, 3] {
                        self.push(&q[i]);
                    }
                }
            }
            Prim::Points if mode == glow::TRIANGLES => {
                for p in v {
                    let (x, y) = (p.x.floor(), p.y.floor());
                    let mk = |x: f32, y: f32| Vertex { x, y, ..*p };
                    let q = [mk(x, y), mk(x + 1.0, y), mk(x + 1.0, y + 1.0), mk(x, y + 1.0)];
                    for i in [0, 1, 2, 0, 2, 3] {
                        self.push(&q[i]);
                    }
                }
            }
            Prim::Lines => {
                for p in v.chunks_exact(2) {
                    self.push(&p[0]);
                    self.push(&p[1]);
                }
            }
            Prim::Points => {
                for p in v {
                    self.push(p);
                }
            }
        }
    }

    fn read(&mut self, t: Target, x: i32, y: i32, w: u32, h: u32) -> Texture {
        self.flush_batch();
        let mut out = Texture::new(w, h);
        let Some((cx, cy, cw, ch)) = self.clip_rect(t, x, y, w as i32, h as i32) else { return out };
        let f = self.fbo(t).unwrap();
        let mut buf = vec![0u8; (cw * ch * 4) as usize];
        // SAFETY: buffer is large enough for cw*ch RGBA pixels (PACK_ALIGNMENT 1).
        unsafe {
            self.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(f.fbo));
            self.gl.read_pixels(cx, cy, cw, ch, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut buf)));
        }
        for row in 0..ch {
            for col in 0..cw {
                let s = ((row * cw + col) * 4) as usize;
                let (dx, dy) = (cx - x + col, cy - y + row);
                out.px[(dy as u32 * w + dx as u32) as usize] = [buf[s], buf[s + 1], buf[s + 2], buf[s + 3]];
            }
        }
        self.bind_target();
        out
    }

    fn write(&mut self, t: Target, x: i32, y: i32, img: &Texture) {
        self.flush_batch();
        let Some((cx, cy, cw, ch)) = self.clip_rect(t, x, y, img.w as i32, img.h as i32) else { return };
        let tex = self.fbo(t).unwrap().tex;
        let mut buf = Vec::with_capacity((cw * ch * 4) as usize);
        for row in 0..ch {
            for col in 0..cw {
                let p = img.px[((cy - y + row) as u32 * img.w + (cx - x + col) as u32) as usize];
                buf.extend_from_slice(&p);
            }
        }
        // SAFETY: buffer holds cw*ch RGBA pixels.
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                cx,
                cy,
                cw,
                ch,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&buf)),
            );
        }
    }

    fn copy(&mut self, dst: Target, dx: i32, dy: i32, src: Target, sx: i32, sy: i32, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        // clip source then destination
        let Some((cx, cy, cw, ch)) = self.clip_rect(src, sx, sy, w, h) else { return };
        let (dx, dy) = (dx + (cx - sx), dy + (cy - sy));
        let Some((ex, ey, ew, eh)) = self.clip_rect(dst, dx, dy, cw, ch) else { return };
        let (cx, cy) = (cx + (ex - dx), cy + (ey - dy));
        if src == dst {
            let img = self.read(src, cx, cy, ew as u32, eh as u32);
            self.write(dst, ex, ey, &img);
            return;
        }
        self.flush_batch();
        let (sf, df) = (self.fbo(src).unwrap().fbo, self.fbo(dst).unwrap().fbo);
        // SAFETY: valid framebuffers; rectangles inside both.
        unsafe {
            let gl = &self.gl;
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(sf));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(df));
            gl.blit_framebuffer(cx, cy, cx + ew, cy + eh, ex, ey, ex + ew, ey + eh, glow::COLOR_BUFFER_BIT, glow::NEAREST);
        }
        self.bind_target();
    }

    fn flush(&mut self) {
        self.flush_batch();
    }

    fn supports_3d(&self) -> bool {
        true
    }

    fn as_gl(&mut self) -> Option<&mut GlBackend> {
        Some(self)
    }
}

impl Drop for GlBackend {
    fn drop(&mut self) {
        // SAFETY: deleting objects we created while the context is current.
        unsafe {
            let gl = &self.gl;
            for (_, (_, t)) in self.images.drain() {
                gl.delete_texture(t);
            }
            for (_, f) in self.surfaces.drain() {
                Self::delete_fbo(gl, &f);
            }
            Self::delete_fbo(gl, &self.back);
            gl.delete_texture(self.white);
            if let Some((t, _, _)) = self.scratch.take() {
                gl.delete_texture(t);
            }
            for row in self.samplers {
                for s in row {
                    gl.delete_sampler(s);
                }
            }
            gl.delete_buffer(self.vbo);
            gl.delete_vertex_array(self.vao);
            gl.delete_program(self.prog);
        }
    }
}

#[cfg(windows)]
mod win32 {
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn CreateWindowExW(
            ex: u32,
            class: *const u16,
            name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            parent: isize,
            menu: isize,
            inst: isize,
            param: *mut c_void,
        ) -> isize;
        fn DestroyWindow(h: isize) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn module() -> isize {
        // SAFETY: null name returns the current module handle.
        unsafe { GetModuleHandleW(std::ptr::null()) }
    }

    /// Creates an invisible top-level window of the predefined STATIC class.
    pub fn create_hidden() -> Result<isize, String> {
        let class = wide("STATIC");
        let name = wide("gm-runner offscreen");
        // SAFETY: valid null-terminated strings; no parent; WS_OVERLAPPED (not visible).
        let h = unsafe { CreateWindowExW(0, class.as_ptr(), name.as_ptr(), 0x00CF_0000, 0, 0, 16, 16, 0, 0, module(), std::ptr::null_mut()) };
        if h == 0 {
            Err("CreateWindowEx failed".into())
        } else {
            Ok(h)
        }
    }

    pub fn destroy(h: isize) {
        // SAFETY: h was created by create_hidden.
        unsafe {
            DestroyWindow(h);
        }
    }
}
