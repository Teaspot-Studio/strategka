use sdl2::EventPump;
use sdl2::{event::Event, video::WindowBuildError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::thread;
use std::time;
use strategka_core::Replay;
use strategka_core::World;
use thiserror::Error;
use tiny_skia::*;

pub struct RenderInfo {
    pub width: u32,
    pub height: u32,
    pub window_tittle: String,
    pub fps: u32,
    pub save_replay: Option<PathBuf>,
}

impl RenderInfo {
    pub fn new() -> Self {
        RenderInfo {
            width: 800,
            height: 600,
            window_tittle: "Strategka".to_owned(),
            fps: 30,
            save_replay: None,
        }
    }
}

impl Default for RenderInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum Error<WE: Debug + Display> {
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
    #[error("Replay error: {0}")]
    Replay(#[from] strategka_core::replay::error::ErrorOwned),
    #[error("Event handler error: {0}")]
    EventHandler(WE),
    #[error("Input handler error: {0}")]
    InputHandler(WE),
    #[error("Simulation error: {0}")]
    Simulation(WE),
    #[error("Render error: {0}")]
    Render(WE),
}

/// High level wrapper that starts endless loop of rendering
///
/// - `event_handler` process events and turns them into inputs that are recored in the simulation, if returns 'true' the render loop exits
/// - `input_handler`
/// - `render` creates next frame based on nanoseconds passed between frames.
pub fn render_loop<E, I, S, R, W, Err>(
    info: &RenderInfo,
    mut state: W,
    mut event_handler: E,
    mut input_handler: I,
    mut simulate: S,
    mut render: R,
) -> Result<(), Error<Err>>
where
    W: World + Default + Clone + Serialize + DeserializeOwned,
    E: FnMut(&W, Event) -> Result<Vec<W::Input>, Err>,
    I: FnMut(&mut W, &W::Input) -> Result<bool, Err>,
    S: FnMut(&mut W, f32) -> Result<(), Err>,
    R: FnMut(&W) -> Result<Pixmap, Err>,
    Err: Debug + Display,
{
    let sdl_context = sdl2::init().map_err(Error::SdlInit)?;
    let video_subsystem = sdl_context.video().map_err(Error::VideoInit)?;

    let window = video_subsystem
        .window(&info.window_tittle, info.width, info.height)
        .position_centered()
        .build()?;

    let mut replay = Replay::new(&state, info.fps);
    let mut turn: u64 = 0;
    let mut last_tick = time::Instant::now();
    let mut event_pump = sdl_context.event_pump().map_err(Error::EventPump)?;
    'running: loop {
        let need_exit = process_inputs(
            info,
            &mut state,
            &mut replay,
            turn,
            &mut event_pump,
            &mut event_handler,
            &mut input_handler,
        )?;
        if need_exit {
            break 'running;
        }

        let mut surface = window.surface(&event_pump).map_err(Error::WindowSurface)?;
        let dt = ensure_fps(info.fps, &last_tick);
        simulate(&mut state, dt).map_err(Error::Simulation)?;
        let pixels = render(&mut state).map_err(Error::Render)?;

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
        turn += 1;
    }
    Ok(())
}

/// Helper to process all events from outside of simulation, turn them into inputs and apply them to simulation.
/// Also, the function mantains record of all inputs inside the replay structure.
fn process_inputs<W, E, I, Err>(
    info: &RenderInfo,
    state: &mut W,
    replay: &mut Replay<W>,
    turn: u64,
    event_pump: &mut EventPump,
    event_handler: &mut E,
    input_handler: &mut I,
) -> Result<bool, Error<Err>>
where
    W: World + Default + Clone + Serialize + DeserializeOwned,
    E: FnMut(&W, Event) -> Result<Vec<W::Input>, Err>,
    I: FnMut(&mut W, &W::Input) -> Result<bool, Err>,
    Err: Debug + Display,
{
    let mut inputs = vec![];
    let mut need_exit = false;
    let mut save_error_replay = |inputs: &[W::Input]| {
        if !inputs.is_empty() {
            replay.record(turn, inputs).map_err(|e| e.into_owned())?;
        }
        if let Some(replay_path) = &info.save_replay {
            replay.save(replay_path).map_err(|e| e.into_owned())?;
        }
        Result::<_, Error<Err>>::Ok(())
    };
    for event in event_pump.poll_iter() {
        match event_handler(state, event).map_err(Error::EventHandler) {
            Ok(new_inputs) => {
                for input in new_inputs {
                    let exit = match input_handler(state, &input).map_err(Error::InputHandler) {
                        Ok(exit) => exit,
                        Err(e) => {
                            inputs.push(input);
                            save_error_replay(&inputs)?;
                            return Err(e);
                        }
                    };
                    inputs.push(input);
                    if exit {
                        need_exit = true;
                    }
                }
            }
            Err(e) => {
                save_error_replay(&inputs)?;
                return Err(e);
            }
        }
    }
    if !inputs.is_empty() {
        replay.record(turn, &inputs).map_err(|e| e.into_owned())?;
    }
    if need_exit {
        if let Some(replay_path) = &info.save_replay {
            replay.save(replay_path).map_err(|e| e.into_owned())?;
        }
    }
    Ok(need_exit)
}

// Helper to run loop with given frames per second
fn ensure_fps(fps: u32, last_tick: &time::Instant) -> f32 {
    let t = last_tick.elapsed();
    let passed_nano = t.as_secs() * 1_000_000_000 + t.subsec_nanos() as u64;
    let fps_dt = (1. / fps as f32) * 1_000_000_000.;
    let diff = fps_dt - (passed_nano as f32);
    if diff > 0.0 {
        thread::sleep(time::Duration::new(0, diff as u32))
    };
    fps_dt
}
