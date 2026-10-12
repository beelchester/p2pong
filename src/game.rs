use wgpu::web_sys;
use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct Game {
    left_y: f32,
    right_y: f32,
    left_up: bool,
    left_down: bool,
    right_up: bool,
    right_down: bool,
}

const PADDLE_HALF_W: f32 = 0.015;
const PADDLE_HALF_H: f32 = 0.15;
const PADDLE_MARGIN: f32 = 0.05;
const PADDLE_SPEED: f32 = 1.5;

impl Game {
    pub fn set_key(&mut self, code: KeyCode, pressed: bool) {
        web_sys::console::log_1(&format!("{:?}", &code).into());
        match code {
            KeyCode::KeyK => self.left_up = pressed,
            KeyCode::KeyJ => self.left_down = pressed,
            KeyCode::ArrowUp => self.right_up = pressed,
            KeyCode::ArrowDown => self.right_down = pressed,
            _ => {}
        }
    }

    pub fn update(&mut self, dt: f32) {
        let limit = 1.0 - PADDLE_HALF_H;
        let l = self.left_up as i32 - self.left_down as i32;
        let r = self.right_up as i32 - self.right_down as i32;
        self.left_y = (self.left_y + l as f32 * PADDLE_SPEED * dt).clamp(-limit, limit);
        self.right_y = (self.right_y + r as f32 * PADDLE_SPEED * dt).clamp(-limit, limit);
        web_sys::console::log_1(&format!("({}, {})", self.left_y, self.right_y).into());
    }
    pub fn vertices(&self) -> [f32; 24] {
        let lx = -1.0 + PADDLE_MARGIN + PADDLE_HALF_W;
        let rx = 1.0 - PADDLE_MARGIN - PADDLE_HALF_W;
        let mut v = [0.0; 24];
        v[..12].copy_from_slice(&quad(lx, self.left_y));
        v[12..].copy_from_slice(&quad(rx, self.right_y));
        v
    }
}

fn quad(cx: f32, cy: f32) -> [f32; 12] {
    let (l, r) = (cx - PADDLE_HALF_W, cx + PADDLE_HALF_W);
    let (b, t) = (cy - PADDLE_HALF_H, cy + PADDLE_HALF_H);
    [l, b, r, b, r, t, l, b, r, t, l, t]
}
