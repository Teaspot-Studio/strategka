use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use tiny_skia::*;

fn render(width: u32, height: u32, i: f32) -> Pixmap {
    let mut paint1 = Paint::default();
    paint1.set_color_rgba8(50, 127, 150, 200);
    paint1.anti_alias = true;

    let mut paint2 = Paint::default();
    paint2.set_color_rgba8(220, 140, 75, 180);
    paint1.anti_alias = false;

    let path1 = {
        let mut pb = PathBuilder::new();
        pb.move_to(60.0 + i.cos() * 30.0, 60.0 + i.sin() * 30.0);
        pb.line_to(160.0 + i.cos() * 30.0, 940.0 + i.sin() * 30.0);
        pb.cubic_to(
            380.0,
            840.0,
            660.0,
            800.0,
            940.0 + i.sin() * 30.0,
            800.0 + i.cos() * 30.0,
        );
        pb.cubic_to(
            740.0,
            460.0,
            440.0,
            160.0,
            60.0 + i.cos() * 30.0,
            60.0 + i.sin() * 30.0,
        );
        pb.close();
        pb.finish().unwrap()
    };

    let path2 = {
        let mut pb = PathBuilder::new();
        pb.move_to(940.0, 60.0);
        pb.line_to(840.0, 940.0);
        pb.cubic_to(620.0, 840.0, 340.0, 800.0, 60.0, 800.0);
        pb.cubic_to(260.0, 460.0, 560.0, 160.0, 940.0, 60.0);
        pb.close();
        pb.finish().unwrap()
    };

    let mut pixmap = Pixmap::new(width, height).unwrap();
    pixmap.fill(Color::from_rgba8(0, 0, 0, 255));
    pixmap.fill_path(
        &path1,
        &paint1,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    pixmap.fill_path(
        &path2,
        &paint2,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    pixmap
}

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let width = 1000;
    let height = 1000;
    let window = video_subsystem
        .window("Triangle", width, height)
        .position_centered()
        .build()
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let mut surface = window.surface(&event_pump).expect("Window surface");
        let pixels = render(width, height, i as f32 / 100.0);
        
        surface.with_lock_mut(|window_pixels| {
            for (i, pixel) in pixels.pixels().iter().enumerate() {
                let c = pixel.demultiply();
                window_pixels[i*4] = c.blue();
                window_pixels[i*4+1] = c.green();
                window_pixels[i*4+2] = c.red();
                window_pixels[i*4+3] = c.alpha();
            }
        });

        i += 1;
        surface.finish().expect("blit sufrace to window");
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60)); // sloppy FPS limit
    }
}
