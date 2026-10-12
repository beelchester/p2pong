use winit::keyboard::KeyCode;

const PADDLE_HALF_W: f32 = 0.015;
const PADDLE_HALF_H: f32 = 0.15;
const PADDLE_MARGIN: f32 = 0.05;
const PADDLE_SPEED: f32 = 1.5;

const BALL_HALF: f32 = 0.02;
const BALL_SPEEDUP: f32 = 1.05;
const BALL_MAX_SPEED: f32 = 2.5;
const MAX_BOUNCE_ANGLE: f32 = 1.05;

pub struct Game {
    pub aspect: f32,
    left_y: f32,
    right_y: f32,
    left_up: bool,
    left_down: bool,
    right_up: bool,
    right_down: bool,
    ball_x: f32,
    ball_y: f32,
    ball_vx: f32,
    ball_vy: f32,
    pub wins: [u32; 2],
    last_touch: Option<usize>, // 0 = player 1, 1 = player 2
    waiting: bool,
}

impl Game {
    pub fn new() -> Self {
        Self {
            aspect: 4.0 / 3.0,
            left_y: 0.0,
            right_y: 0.0,
            left_up: false,
            left_down: false,
            right_up: false,
            right_down: false,
            ball_x: 0.0,
            ball_y: 0.0,
            ball_vx: 1.0,
            ball_vy: 0.5,
            wins: [0, 0],
            last_touch: None,
            waiting: true,
        }
    }

    fn left_x(&self) -> f32 {
        -self.aspect + PADDLE_MARGIN + PADDLE_HALF_W
    }

    fn right_x(&self) -> f32 {
        self.aspect - PADDLE_MARGIN - PADDLE_HALF_W
    }

    pub fn set_key(&mut self, code: KeyCode, pressed: bool) {
        let is_move_key = match code {
            KeyCode::KeyW => {
                self.left_up = pressed;
                true
            }
            KeyCode::KeyS => {
                self.left_down = pressed;
                true
            }
            KeyCode::ArrowUp => {
                self.right_up = pressed;
                true
            }
            KeyCode::ArrowDown => {
                self.right_down = pressed;
                true
            }
            _ => false,
        };
        if is_move_key && pressed {
            self.waiting = false;
        }
    }

    pub fn update(&mut self, dt: f32) -> Option<usize> {
        let limit = 1.0 - PADDLE_HALF_H;
        let l = self.left_up as i32 - self.left_down as i32;
        let r = self.right_up as i32 - self.right_down as i32;
        self.left_y = (self.left_y + l as f32 * PADDLE_SPEED * dt).clamp(-limit, limit);
        self.right_y = (self.right_y + r as f32 * PADDLE_SPEED * dt).clamp(-limit, limit);

        if self.waiting {
            return None;
        }

        self.ball_x += self.ball_vx * dt;
        self.ball_y += self.ball_vy * dt;

        let wall = 1.0 - BALL_HALF;
        if self.ball_y > wall {
            self.ball_y = wall;
            self.ball_vy = -self.ball_vy.abs();
        } else if self.ball_y < -wall {
            self.ball_y = -wall;
            self.ball_vy = self.ball_vy.abs();
        }

        let (lx, rx) = (self.left_x(), self.right_x());
        if self.ball_vx < 0.0 && self.hits(lx, self.left_y) {
            self.bounce(lx + PADDLE_HALF_W + BALL_HALF, self.left_y, 1.0);
            self.last_touch = Some(0);
        } else if self.ball_vx > 0.0 && self.hits(rx, self.right_y) {
            self.bounce(rx - PADDLE_HALF_W - BALL_HALF, self.right_y, -1.0);
            self.last_touch = Some(1);
        }

        if self.ball_x.abs() > self.aspect + BALL_HALF {
            let winner = self.last_touch;
            if let Some(w) = winner {
                self.wins[w] += 1;
            }
            self.reset(winner);
            return winner;
        }
        None
    }

    fn reset(&mut self, winner: Option<usize>) {
        let dir = if winner == Some(1) { -1.0 } else { 1.0 };
        let sign = if (self.wins[0] + self.wins[1]) % 2 == 0 {
            1.0
        } else {
            -1.0
        };
        self.left_y = 0.0;
        self.right_y = 0.0;
        self.ball_x = 0.0;
        self.ball_y = 0.0;
        self.ball_vx = dir * 1.0;
        self.ball_vy = sign * 0.5;
        self.last_touch = None;
        self.waiting = true;
    }

    fn hits(&self, paddle_x: f32, paddle_y: f32) -> bool {
        (self.ball_x - paddle_x).abs() < PADDLE_HALF_W + BALL_HALF
            && (self.ball_y - paddle_y).abs() < PADDLE_HALF_H + BALL_HALF
    }

    fn bounce(&mut self, new_x: f32, paddle_y: f32, dir: f32) {
        let offset = ((self.ball_y - paddle_y) / (PADDLE_HALF_H + BALL_HALF)).clamp(-1.0, 1.0);
        let angle = offset * MAX_BOUNCE_ANGLE;
        let speed = (self.ball_vx.hypot(self.ball_vy) * BALL_SPEEDUP).min(BALL_MAX_SPEED);

        self.ball_x = new_x;
        self.ball_vx = dir * speed * angle.cos();
        self.ball_vy = speed * angle.sin();
    }

    pub fn vertices(&self) -> [f32; 72] {
        let a = self.aspect;
        let mut v = [0.0; 72];
        v[..24].copy_from_slice(&quad(
            self.left_x(),
            self.left_y,
            PADDLE_HALF_W,
            PADDLE_HALF_H,
            a,
            false,
        ));
        v[24..48].copy_from_slice(&quad(
            self.right_x(),
            self.right_y,
            PADDLE_HALF_W,
            PADDLE_HALF_H,
            a,
            false,
        ));
        v[48..].copy_from_slice(&quad(
            self.ball_x,
            self.ball_y,
            BALL_HALF,
            BALL_HALF,
            a,
            true,
        ));
        v
    }
}

fn quad(cx: f32, cy: f32, hw: f32, hh: f32, aspect: f32, round: bool) -> [f32; 24] {
    let (l, r) = ((cx - hw) / aspect, (cx + hw) / aspect);
    let (b, t) = (cy - hh, cy + hh);
    let u = if round { 1.0 } else { 0.0 };
    [
        l, b, -u, -u, r, b, u, -u, r, t, u, u, l, b, -u, -u, r, t, u, u, l, t, -u, u,
    ]
}
