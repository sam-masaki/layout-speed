use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;

mod analyze;
mod display;
mod layout;
mod playback;

pub fn main() {
  let (context, canvas, ttf) = display::init("Layout Speed").unwrap();
  let font = display::init_font(&ttf);
  let mut disp = display::Data {
    context,
    canvas,
    ttf: &ttf,
    font,
  };

  // TODO: Think of a better way dealing with Layout.homes
  // Not all fingers have a home
  let mut lay = layout::Layout::default();

  let lay = match layout::init(&mut lay, "qwerty.layout") {
    Some(l) => l,
    None => return,
  };

  let tl = analyze::gen_timeline("qwert yuiop", lay);
  analyze::print_timeline(&tl);

  let mut playhead = playback::Playhead {
    time: 0,
    idxs: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  };

  let mut playdata = playback::PlayData::default();

  let mut curr_time = 0;

  let mut event_pump = disp.context.event_pump().unwrap();
  'main: loop {
    disp.canvas.set_draw_color(Color::RGB(0, 0, 0));
    disp.canvas.clear();

    for event in event_pump.poll_iter() {
      match event {
        Event::Quit { .. }
        | Event::KeyDown {
          keycode: Some(Keycode::Escape),
          ..
        } => break 'main,
        _ => {}
      }
    }

    playback::calc_playback(&playhead, &tl, &mut playdata);
    playback::inc_head(&mut playhead, &tl, 16);

    display::draw_layout(lay, &mut disp);
    display::draw_playdata(&playdata, &mut disp);
    disp.canvas.present();
    ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
  }
}
