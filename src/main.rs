use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::env;
use std::time::Duration;

mod analyze;
mod display;
mod layout;
mod playback;

pub fn main() {
  let raw_args: Vec<String> = env::args().collect();
  let args = parse_args(&raw_args);

  let mut lay_path = "qwerty.layout".to_string();
  let mut text: Option<String> = None;
  let mut file_path: Option<String> = None;
  let mut anim = true;
  let mut parallel = true;
  let mut compare = false;

  for opt in args {
    match opt.0.as_str() {
      "-l" => lay_path = opt.1,
      "-t" => text = Some(opt.1),
      "-f" => file_path = Some(opt.1),
      "-p" => parallel = opt.1 == "true",
      "-c" => compare = true,
      "-n" => anim = false,
      x => println!("Unknown option: {}", x),
    }
  }

  if compare {
    let mut lay = layout::Layout::default();

    let lay = match layout::init(&mut lay, lay_path.as_str()) {
      Some(l) => l,
      None => return,
    };
    let longest = analyze::compare_lines(file_path.as_ref().unwrap(), lay);

    for word in longest {
      println!("{} is {}mm long", word.1, word.0.total_dist_mm());
    }
    return;
  }

  if anim {
    play_anim(&lay_path, &text);
  } else {
    get_stats(&lay_path, &text, &file_path, parallel);
  }
}

fn parse_args(args: &[String]) -> Vec<(String, String)> {
  let mut res = Vec::new();

  // TODO: make this better
  let mut i = 1;
  while i < args.len() {
    if i + 1 < args.len() {
      res.push((args[i].clone(), args[i + 1].clone()));
      i += 2;
    } else {
      res.push((args[i].clone(), args[i].clone()));
      break;
    }
  }

  res
}

fn get_stats(lay_path: &str, text: &Option<String>, file_path: &Option<String>, parallel: bool) {
  let text = match text {
    Some(t) => t,
    None => "The quick brown fox jumps over the lazy dog.",
  };
  let mut lay = layout::Layout::default();

  let lay = match layout::init(&mut lay, lay_path) {
    Some(l) => l,
    None => return,
  };

  let tl = match file_path {
    Some(p) => analyze::gen_timeline_file(p, parallel, lay),
    None => analyze::gen_timeline(text, false, lay),
  };

  analyze::print_timeline(&tl);
}

fn play_anim(lay_path: &str, text: &Option<String>) {
  let text = match text {
    Some(t) => t,
    None => "The quick brown fox jumps over the lazy dog.",
  };
  let (context, canvas, ttf) = display::init("Layout Speed").unwrap();
  let font = display::init_font(&ttf);
  let mut disp = display::Data {
    context,
    canvas,
    ttf: &ttf,
    font,
  };

  let mut lay = layout::Layout::default();

  let lay = match layout::init(&mut lay, lay_path) {
    Some(l) => l,
    None => return,
  };

  let tl = analyze::gen_timeline(text, true, lay);
  analyze::print_timeline(&tl);

  let mut playhead = playback::Playhead {
    time: 0,
    idxs: [0; 10],
  };

  let mut playdata = playback::PlayData::default();

  let mut event_pump = disp.context.event_pump().unwrap();
  'main: loop {
    display::clear_screen(&mut disp);

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

    display::draw_text(10, 250, text, &mut disp);
    display::draw_text(
      10,
      275,
      format!(
        "Total Distance: {}u, {}mm",
        tl.total_dist,
        tl.total_dist * 19.05
      )
      .as_str(),
      &mut disp,
    );
    display::draw_text(10, 300, format!("WPM: {}", tl.wpm()).as_str(), &mut disp);
    disp.canvas.present();
    ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
  }
}
