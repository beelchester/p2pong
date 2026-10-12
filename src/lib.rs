mod game;

use game::Game;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys},
    window::{Window, WindowId},
};

const SHADER: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (length(in.uv) > 1.0) {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#;

struct Renderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

impl Renderer {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();
        web_sys::console::log_1(&format!("{:?}", adapter.get_info()).into());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .unwrap();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .unwrap();
        surface.configure(&device, &config);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("paddle shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("paddle pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("paddle vertices"),
            size: 72 * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            vertex_buffer,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&self, game: Option<&Game>) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());

        if let Some(game) = game {
            let bytes: Vec<u8> = game
                .vertices()
                .iter()
                .flat_map(|f| f.to_ne_bytes())
                .collect();
            self.queue.write_buffer(&self.vertex_buffer, 0, &bytes);
        }
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            if game.is_some() {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw(0..18, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
    }
}

pub enum AppEvent {
    RendererReady(Renderer),
    StartLocal,
}

fn dom() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

fn set_hidden(id: &str, hidden: bool) {
    let el = dom().get_element_by_id(id).unwrap();
    if hidden {
        el.set_attribute("hidden", "").unwrap();
    } else {
        el.remove_attribute("hidden").unwrap();
    }
}

fn update_wins_hud(wins: [u32; 2]) {
    let show = wins[0] + wins[1] > 0;
    for (i, w) in wins.iter().enumerate() {
        let text = if show {
            format!("Wins: {w}")
        } else {
            String::new()
        };
        dom()
            .get_element_by_id(&format!("wins{}", i + 1))
            .unwrap()
            .set_text_content(Some(&text));
    }
}

fn focus_canvas() {
    if let Some(el) = dom().query_selector("canvas").unwrap() {
        if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
            let _ = el.focus();
        }
    }
}

fn on_click(id: &str, f: impl FnMut() + 'static) {
    let cb = Closure::<dyn FnMut()>::new(f);
    dom()
        .get_element_by_id(id)
        .unwrap()
        .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
        .unwrap();
    cb.forget();
}

struct App {
    proxy: EventLoopProxy<AppEvent>,
    renderer: Option<Renderer>,
    game: Game,
    last_frame: Instant,
    playing: bool,
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_append(true);
        let window = Arc::new(el.create_window(attrs).unwrap());
        let proxy = self.proxy.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let renderer = Renderer::new(window).await;
            let _ = proxy.send_event(AppEvent::RendererReady(renderer)); // sends to user_event when finished in bg
        });
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::RendererReady(mut renderer) => {
                let size = renderer.window.inner_size();
                renderer.resize(size.width, size.height);
                self.game.aspect = size.width as f32 / size.height.max(1) as f32;
                renderer.window.request_redraw();
                self.renderer = Some(renderer);
            }
            AppEvent::StartLocal => {
                self.playing = true;
                self.last_frame = Instant::now();
                set_hidden("menu", true);
                set_hidden("hud", false);
                focus_canvas();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                renderer.resize(s.width, s.height);
                self.game.aspect = s.width as f32 / s.height.max(1) as f32;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.repeat {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        self.game
                            .set_key(code, event.state == ElementState::Pressed);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().min(0.05);
                self.last_frame = now;
                if self.playing && self.game.update(dt).is_some() {
                    update_wins_hud(self.game.wins);
                }
                renderer.render(self.playing.then_some(&self.game));
                renderer.window.request_redraw();
            }
            _ => {}
        }
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    let el = EventLoop::<AppEvent>::with_user_event().build().unwrap();
    let proxy = el.create_proxy();

    let p = proxy.clone();
    on_click("btn-local", move || {
        let _ = p.send_event(AppEvent::StartLocal);
    });

    let app = App {
        proxy,
        renderer: None,
        game: Game::new(),
        last_frame: Instant::now(),
        playing: false,
    };
    el.spawn_app(app);
}
