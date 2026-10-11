mod game;

use game::Game;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_time::Instant;
use wgpu::web_sys;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys},
    window::{Window, WindowId},
};

struct Renderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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
        Self {
            window,
            surface,
            device,
            queue,
            config,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&self) {
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
        {
            let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        }
        self.queue.submit([enc.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
    }
}

struct App {
    proxy: EventLoopProxy<Renderer>,
    renderer: Option<Renderer>,
    game: Game,
    last_frame: Instant,
}

impl ApplicationHandler<Renderer> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_append(true);
        let window = Arc::new(el.create_window(attrs).unwrap());
        let proxy = self.proxy.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let renderer = Renderer::new(window).await;
            let _ = proxy.send_event(renderer); // sends to user_event when finished in bg
        });
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, mut renderer: Renderer) {
        let size = renderer.window.inner_size();
        renderer.resize(size.width, size.height);
        renderer.window.request_redraw();
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => renderer.resize(s.width, s.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.game
                        .set_key(code, event.state == ElementState::Pressed);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().min(0.05);
                self.last_frame = now;
                self.game.update(dt);
                renderer.window.request_redraw();
            }
            _ => {}
        }
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    let el = EventLoop::<Renderer>::with_user_event().build().unwrap();
    let app = App {
        proxy: el.create_proxy(),
        renderer: None,
        game: Game::default(),
        last_frame: Instant::now(),
    };
    el.spawn_app(app);
}
