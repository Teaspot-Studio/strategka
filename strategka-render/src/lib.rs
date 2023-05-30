use sdl2::{event::Event, video::WindowBuildError};
use std::thread;
use std::time;
use thiserror::Error;
use tiny_skia::*;

pub struct RenderInfo {
    pub width: u32,
    pub height: u32,
    pub window_tittle: String,
    pub fps: u32,
}

impl RenderInfo {
    pub fn new() -> Self {
        RenderInfo {
            width: 800,
            height: 600,
            window_tittle: "Strategka".to_owned(),
            fps: 30,
        }
    }
}

impl Default for RenderInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to init SDL2: {0}")]
    SdlInit(String),
    #[error("Failed to init video subsystem: {0}")]
    VideoInit(String),
    #[error("Failed to create window: {0}")]
    WindowCreation(#[from] WindowBuildError),
    #[error("Failed to create event pump: {0}")]
    EventPump(String),
    #[error("Failed to get window surface: {0}")]
    WindowSurface(String),
    #[error("Failed to blit result to window: {0}")]
    WindowFinish(String),
}

/// High level wrapper that starts endless loop of rendering
///
/// - `event_handler` process events, if returns 'true' the render loop exits
/// - `render` creates next frame based on nanoseconds passed between frames.
pub fn render_loop<E, R, S>(
    info: &RenderInfo,
    mut state: S,
    mut event_handler: E,
    mut render: R,
) -> Result<(), Error>
where
    E: FnMut(&mut S, Event) -> bool,
    R: FnMut(&mut S, f32) -> Pixmap,
{
    let sdl_context = sdl2::init().map_err(Error::SdlInit)?;
    let video_subsystem = sdl_context.video().map_err(Error::VideoInit)?;

    let window = video_subsystem
        .window(&info.window_tittle, info.width, info.height)
        .position_centered()
        .build()?;

    let mut last_tick = time::Instant::now();
    let mut event_pump = sdl_context.event_pump().map_err(Error::EventPump)?;
    'running: loop {
        for event in event_pump.poll_iter() {
            if event_handler(&mut state, event) {
                break 'running;
            }
        }

        let mut surface = window.surface(&event_pump).map_err(Error::WindowSurface)?;
        let dt = ensure_fps(info.fps, &last_tick);
        let pixels = render(&mut state, dt);

        surface.with_lock_mut(|window_pixels| {
            for (i, pixel) in pixels.pixels().iter().enumerate() {
                let c = pixel.demultiply();
                window_pixels[i * 4] = c.blue();
                window_pixels[i * 4 + 1] = c.green();
                window_pixels[i * 4 + 2] = c.red();
                window_pixels[i * 4 + 3] = c.alpha();
            }
        });

        surface.finish().map_err(Error::WindowFinish)?;
        last_tick = time::Instant::now();
    }
    Ok(())
}

// Helper to run loop with given frames per second
fn ensure_fps(fps: u32, last_tick: &time::Instant) -> f32 {
    let t = last_tick.elapsed();
    let passed_nano = t.as_secs() * 1_000_000_000 + t.subsec_nanos() as u64;
    let diff = (1. / fps as f32) * 1_000_000_000. - (passed_nano as f32);
    if diff > 0.0 {
        thread::sleep(time::Duration::new(0, diff as u32))
    };
    diff
}
