use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use strategka_render::*;
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

pub fn main() -> Result<(), Error> {
    let render_info = RenderInfo {
        width: 1000,
        height: 1000,
        window_tittle: "Triangle".to_owned(),
        ..RenderInfo::default()
    };
    let mut i = 0.0;
    render_loop(
        &render_info,
        |event| match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => true,
            _ => false,
        },
        |dt| {
            i += dt / 1_000_000_000.0; 
            render(render_info.width, render_info.height, i)
        },
    )
}
