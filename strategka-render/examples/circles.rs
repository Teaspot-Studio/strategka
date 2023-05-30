use rand::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use std::ops::Sub;
use strategka_render::*;
use tiny_skia::*;

#[derive(Debug, Copy, Clone)]
struct V2 {
    x: i32,
    y: i32,
}

impl Sub for V2 {
    type Output = V2;

    fn sub(self, rhs: Self) -> Self::Output {
        V2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl V2 {
    fn square_dist(&self) -> i32 {
        self.x * self.x + self.y * self.y
    }
}

/// Index of circle
type CircleId = usize;

#[derive(Debug, Clone)]
struct Circle {
    pos: V2,
    vel: V2,
    radius: u32,
    selected: bool,
    target: Option<V2>,
}

impl Circle {
    pub fn new(pos: V2, vel: V2) -> Self {
        Circle {
            pos,
            vel,
            radius: 15,
            selected: false,
            target: None,
        }
    }

    pub fn rng(rng: &mut StdRng, width: u32, height: u32) -> Self {
        let x = rng.gen_range(0..width as i32);
        let y = rng.gen_range(0..height as i32);
        let vx = rng.gen_range(0..width as i32);
        let vy = rng.gen_range(0..height as i32);

        Circle::new(V2 { x, y }, V2 { x: vx, y: vy })
    }

    pub fn render(&self, pixmap: &mut Pixmap) {
        let paint = if self.selected {
            make_paint(50, 127, 150, 200)
        } else {
            make_paint(220, 140, 75, 180)
        };

        let path = {
            let mut pb = PathBuilder::new();
            pb.push_circle(self.pos.x as f32, self.pos.y as f32, self.radius as f32);
            pb.finish().unwrap()
        };

        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    pub fn step(&mut self, dt: f32, width: u32, height: u32) {
        self.pos.x = (self.pos.x as f32 + self.vel.x as f32 * dt) as i32;
        self.pos.y = (self.pos.y as f32 + self.vel.y as f32 * dt) as i32;

        if self.pos.x < 0 || self.pos.x > width as i32 {
            self.vel.x *= -1;
        }
        if self.pos.y < 0 || self.pos.y > height as i32 {
            self.vel.y *= -1;
        }
    }
}

fn make_paint<'a>(r: u8, g: u8, b: u8, a: u8) -> Paint<'a> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    paint.anti_alias = true;
    paint
}

#[derive(Debug, Clone)]
struct World {
    width: u32,
    height: u32,
    circles: Vec<Circle>,
    input: Option<Input>,
    selected: Option<CircleId>,
}

impl World {
    pub fn new(width: u32, height: u32, circles_num: usize, seed: u64) -> Self {
        let mut rng = {
            let mut streched_seed = [0u8; 32];
            streched_seed[0..8].copy_from_slice(&seed.to_ne_bytes());
            StdRng::from_seed(streched_seed)
        };

        let mut circles = vec![];
        for _ in 0..circles_num {
            circles.push(Circle::rng(&mut rng, width, height));
        }

        World {
            width,
            height,
            circles,
            input: None,
            selected: None,
        }
    }

    pub fn render(&self) -> Pixmap {
        let mut pixmap = Pixmap::new(self.width, self.height).unwrap();
        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));
        for circle in self.circles.iter() {
            circle.render(&mut pixmap);
        }
        pixmap
    }

    pub fn process_input(&mut self) {
        if let Some(input) = &self.input {
            match input {
                Input::Select(i) => {
                    self.circles[*i].selected = true;
                    if let Some(other) = self.selected {
                        self.circles[other].selected = false;
                    }
                    self.selected = Some(*i);
                }
                _ => (),
            }
            self.input = None;
        }
    }

    pub fn step(&mut self, dt: f32) {
        for circle in self.circles.iter_mut() {
            circle.step(dt, self.width, self.height);
        }
    }

    /// Return first circle under the point
    pub fn circle_at(&self, pos: V2) -> Option<CircleId> {
        self.circles
            .iter()
            .position(|c| (c.pos - pos).square_dist() < (c.radius * c.radius) as i32)
    }
}

#[derive(Debug, Clone)]
enum Input {
    /// Player selects circle with index i
    Select(CircleId),
    /// Player orders to move to the point
    Move(V2),
}

pub fn main() -> Result<(), Error> {
    let render_info = RenderInfo {
        width: 1000,
        height: 1000,
        window_tittle: "Triangle".to_owned(),
        fps: 120,
        ..RenderInfo::default()
    };
    let world = World::new(render_info.width, render_info.height, 20, 42);
    render_loop(
        &render_info,
        world,
        |world, event| match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => true,
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if let Some(i) = world.circle_at(V2 { x, y }) {
                    world.input = Some(Input::Select(i))
                }
                false
            }
            _ => false,
        },
        |world, dt| {
            world.process_input();
            world.step(dt / 1_000_000_000.0);
            world.render()
        },
    )
}
