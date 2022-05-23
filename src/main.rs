use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::env;
use std::time::Duration;

mod analyze;
mod display;
mod layout;
mod playback;

struct ProgOptions {
  lay_path: String,
  file_path: Option<String>,
  text: Option<String>,
  animate: bool,
  parallel: bool,
  compare: bool,
}

pub fn main() {
  let raw_args: Vec<String> = env::args().collect();

  let options = match parse_args(&raw_args) {
    Some(o) => o,
    None => return
  };

  if options.compare {
    assert!(options.file_path.is_some(), "Comparing requires a text file");
    let mut lay = layout::Layout::default();

    let lay = match layout::init(&mut lay, options.lay_path.as_str()) {
      Some(l) => l,
      None => return,
    };
    let longest = analyze::compare_lines(options.file_path.as_ref().unwrap(), lay);

    let mut count = 1;
    for word in longest {
      println!("{:3}: {} is {}mm long and {} letters long (u_per_char: {})", count, word.1, word.0.total_dist_mm(), word.0.total_chars, word.0.u_per_char());
      count += 1;
    }
  } else if options.animate {
    play_anim(&options.lay_path, &options.text);
  } else {
    get_stats(&options.lay_path, &options.text, &options.file_path, options.parallel);
  }
}

fn parse_args(args: &[String]) -> Option<ProgOptions> {
  let mut lay_path = "layouts/qwerty.layout".to_string();
  let mut file_path = None;
  let mut text = None;
  let mut animate = true;
  let mut parallel = false;
  let mut compare = false;

  let mut i = 1;
  while i < args.len() {
    // TODO: This doesn't feel like the right way to do this
    match args[i].as_str() {
      "-h" | "--help" => print_help(),
      "-n" => animate = false,
      "-c" => compare = true,
      f => {
        if i + 1 >= args.len() {
          println!("Flag {} needs a value or unrecognized", f);
          return None;
        }
        let val = &args[i + 1];

        match f {
          "-l" => lay_path = val.clone(),
          "-t" => text = Some(val.clone()),
          "-f" => file_path = Some(val.clone()),
          "-p" => parallel = val == "true",
          unknown => {println!("Flag {} unrecognized", unknown); return None;}
        }

        i += 1;
      },
    }

    i += 1;
  }

  return Some(ProgOptions{
    lay_path,
    file_path,
    text,
    animate,
    parallel,
    compare,
  })
}

fn print_help() {
  println!("Usage: layout-speed [OPTIONS] [TEXT]");
  println!("Options:");
  println!("  -h, --help\t\tPrint this message");
  println!("  -l FILE\t\tUse PATH as the keyboard layout instead of the default qwerty.layout");
  println!("  -t STRING\t\tAnalyze the given STRING");
  println!("  -f FILE\t\tAnalyze the contents of FILE");
  println!("  -p true/false\t\tWhether to analyze the text or file in parallel");
  println!("  -n\t\t\tOnly generate statistics on the text, without the animation");
  println!("  -c\t\t\tCompare each line of the given file and output the longest one");
  std::process::exit(0);
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
    idxs: vec![0; tl.fingers.len()],
  };

  let mut playdata = playback::PlayData {
    fingers: vec![playback::FingerData::default(); tl.fingers.len()]
  };

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

    display::draw_text(10, 255, format!("\"{}\"", text).as_str(), &mut disp);
    display::draw_text(10, 275, analyze::stats_string(&tl).as_str(), &mut disp);
    disp.canvas.present();
    ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
  }
}
