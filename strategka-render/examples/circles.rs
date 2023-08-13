use rand::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use serde::{Deserialize, Serialize};
use std::ops::{AddAssign, Div, Mul, Sub};
use strategka_core::World;
use strategka_render::*;
use thiserror::Error;
use tiny_skia::*;

#[derive(Debug, Error)]
pub enum CircleError {
    #[error("Failed to create canvas to draw on")]
    CanvasCreation,
    #[error("Failed to finish circle path")]
    CircleDraw,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
struct V2 {
    x: f32,
    y: f32,
}

impl AddAssign for V2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
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

impl Mul<f32> for V2 {
    type Output = V2;

    fn mul(self, rhs: f32) -> Self::Output {
        V2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Mul<V2> for f32 {
    type Output = V2;

    fn mul(self, rhs: V2) -> Self::Output {
        V2 {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

impl Div<f32> for V2 {
    type Output = V2;

    fn div(self, rhs: f32) -> Self::Output {
        V2 {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl Div<V2> for f32 {
    type Output = V2;

    fn div(self, rhs: V2) -> Self::Output {
        V2 {
            x: self / rhs.x,
            y: self / rhs.y,
        }
    }
}

impl V2 {
    fn square_dist(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }
}

/// Index of circle
type CircleId = usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Circle {
    pos: V2,
    vel: V2,
    radius: f32,
    selected: bool,
    target: Option<V2>,
}

impl Circle {
    pub fn new(pos: V2, vel: V2) -> Self {
        Circle {
            pos,
            vel,
            radius: 15.0,
            selected: false,
            target: None,
        }
    }

    pub fn rng(rng: &mut StdRng, width: u32, height: u32) -> Self {
        let x = rng.gen_range(0.0..width as f32);
        let y = rng.gen_range(0.0..height as f32);
        let vx = rng.gen_range(0.0..width as f32);
        let vy = rng.gen_range(0.0..height as f32);

        Circle::new(V2 { x, y }, V2 { x: vx, y: vy })
    }

    pub fn render(&self, pixmap: &mut Pixmap) -> Result<(), CircleError> {
        let paint = if self.selected {
            make_paint(50, 127, 150, 200)
        } else {
            make_paint(220, 140, 75, 180)
        };

        let path = {
            let mut pb = PathBuilder::new();
            pb.push_circle(self.pos.x as f32, self.pos.y as f32, self.radius as f32);
            pb.finish().ok_or(CircleError::CircleDraw)?
        };

        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        Ok(())
    }

    pub fn step(&mut self, dt: f32, width: u32, height: u32) {
        self.pos += self.vel * dt;

        if self.pos.x < 0.0 || self.pos.x > width as f32 {
            self.vel.x *= -1.;
        }
        if self.pos.y < 0.0 || self.pos.y > height as f32 {
            self.vel.y *= -1.;
        }

        if let Some(t) = self.target {
            let mass = 1.0;
            let k = 1.0;
            let c = 0.3;
            let dv = t - self.pos;
            let fv = k * dv - c * self.vel;
            self.vel += fv / mass;
        }
    }
}

fn make_paint<'a>(r: u8, g: u8, b: u8, a: u8) -> Paint<'a> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    paint.anti_alias = true;
    paint
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum CirclesInput {
    /// Player selects circle with index i
    Select(CircleId),
    /// Player orders to move to the point
    Move(V2),
    /// Stop simulation
    EndSimulation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CirclesWorld {
    width: u32,
    height: u32,
    circles: Vec<Circle>,
    selected: Option<CircleId>,
}

impl Default for CirclesWorld {
    fn default() -> Self {
        CirclesWorld {
            width: 0,
            height: 0,
            circles: vec![],
            selected: None,
        }
    }
}

impl CirclesWorld {
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

        CirclesWorld {
            width,
            height,
            circles,
            selected: None,
        }
    }

    pub fn render(&self) -> Result<Pixmap, CircleError> {
        let mut pixmap = Pixmap::new(self.width, self.height).ok_or(CircleError::CanvasCreation)?;
        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));
        for circle in self.circles.iter() {
            circle.render(&mut pixmap)?;
        }
        Ok(pixmap)
    }

    pub fn process_input(&mut self, input: &CirclesInput) {
        match input {
            CirclesInput::Select(i) => {
                self.circles[*i].selected = true;
                if let Some(other) = self.selected {
                    self.circles[other].selected = false;
                }
                self.selected = Some(*i);
            }
            CirclesInput::Move(p) => {
                if let Some(i) = self.selected {
                    self.circles[i].target = Some(*p);
                }
            }
            CirclesInput::EndSimulation => {}
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
            .position(|c| (c.pos - pos).square_dist() < (c.radius * c.radius))
    }
}

impl World for CirclesWorld {
    type Input = CirclesInput;

    fn magic_bytes() -> [u8; 4] {
        b"crls".clone()
    }

    fn current_version() -> u32 {
        1
    }
}

pub fn main() -> Result<(), Error<CircleError>> {
    let render_info = RenderInfo {
        width: 1000,
        height: 1000,
        window_tittle: "Circles".to_owned(),
        fps: 120,
        save_replay: Some("circles.replay".into()),
        ..RenderInfo::default()
    };
    let world = CirclesWorld::new(render_info.width, render_info.height, 20, 42);
    render_loop(
        &render_info,
        world,
        |world, event| match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => Ok(vec![CirclesInput::EndSimulation]),
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if let Some(i) = world.circle_at(V2 {
                    x: x as f32,
                    y: y as f32,
                }) {
                    Ok(vec![CirclesInput::Select(i)])
                } else {
                    Ok(vec![])
                }
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Right,
                x,
                y,
                ..
            } => Ok(vec![CirclesInput::Move(V2 {
                x: x as f32,
                y: y as f32,
            })]),
            _ => Ok(vec![]),
        },
        |world, input| {
            world.process_input(input);
            Ok(matches!(input, CirclesInput::EndSimulation))
        },
        |world, dt| {
            world.step(dt / 1_000_000_000.0);
            Ok(())
        },
        |world| world.render(),
    )
}
