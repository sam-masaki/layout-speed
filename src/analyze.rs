use std::io::Read;

use rayon::{iter::ParallelIterator, str::ParallelString};

use super::layout;

#[derive(Default)]
pub struct Timeline {
  pub fingers: [Vec<Keyframe>; 10],
  pub finger_counts: [u32; 10], // number of presses
  pub total_time: i32,
  pub total_dist: f32, // in u
  pub total_words: u32,
  pub total_chars: u32,
}

impl Timeline {
  pub fn wpm(&self) -> u16 {
    (60000.0 * (self.total_words as f32) / (self.total_time as f32)) as u16
  }

  pub fn usage_percent(&self, i: usize) -> f32 {
    if i >= 10 {
      return -1.0;
    }
    100.0 * (self.finger_counts[i] as f32) / (self.total_chars as f32)
  }
}

#[derive(Default, Clone, Copy)]
pub struct Keyframe {
  pub pos: layout::Pos,
  pub time: i32,
  pub start_press: bool,
  // TODO: use derivative to make this debug-only
  // TODO: make this a String with the name of the key
  on_char: char,
}

static PRESS_DUR: i32 = 250;

pub fn gen_timeline<'a>(string: &str, gen_anim: bool, lay: &'a layout::Layout) -> Timeline {
  let mut fingers = [
    Vec::new(),
    Vec::new(),
    Vec::new(),
    Vec::new(),
    Vec::new(),
    Vec::new(),
    Vec::new(),
    Vec::new(),
    Vec::new(),
    Vec::new(),
  ];

  let mut finger_usage_cnt = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

  for i in 0..10 {
    fingers[i].push(Keyframe {
      pos: layout::Pos {
        x: lay.homes[i].pos.x,
        y: lay.homes[i].pos.y,
      },
      time: 0,
      start_press: false,
      on_char: lay.homes[i].pressed,
    });
  }

  let mut total_dist = 0.0;

  // The next move must start at or after min_start
  let mut min_start = 0;
  let mut total_time = 0;

  for c in string.chars() {
    let combo = match lay.char_keys.get(&c) {
      Some(co) => co,
      None => continue,
    };
    let main_key = combo.key;
    let main_findex = main_key.finger as usize;
    let main_home = lay.homes[main_findex];
    let main_prev = *fingers[main_findex].last().unwrap();

    // The earliest moment all necessary fingers get on their keys
    let mut min_press =
      move_time(&main_prev.pos, &main_key.pos) + std::cmp::max(min_start, main_prev.time);

    // Currently time_end_press is the same between main key and
    // modifiers but time_end_move varies
    let mut time_end_press = 0;
    let mut time_end_move = 0;

    if combo.mods.is_some() {
      let mods = combo.mods.as_ref().unwrap();

      // Calculate min_press
      for modifier in mods {
        let findex = modifier.finger as usize;
        let prev = fingers[findex].last().unwrap();

        let dur = move_time(&prev.pos, &modifier.pos);
        let time_last_move = std::cmp::max(min_start, prev.time);
        min_press = min_press.max(dur + time_last_move);
      }

      // Add keyframes for modifiers
      for modifier in mods {
        let mod_findex = modifier.finger as usize;
        let (this_end_press, this_end_move) = calc_keyframes(
          &fingers[mod_findex].last().unwrap().clone(),
          modifier,
          lay.homes[mod_findex],
          min_start,
          min_press,
          gen_anim,
          &mut fingers[mod_findex],
        );

        time_end_press = time_end_press.max(this_end_press);
        time_end_move = time_end_move.max(this_end_move);
      }
    }

    // Add main frames
    let (this_end_press, this_end_move) = calc_keyframes(
      &main_prev,
      main_key,
      main_home,
      min_start,
      min_press,
      gen_anim,
      &mut fingers[main_findex],
    );

    // Add to stats
    // For now this only includes main finger usage/movement
    finger_usage_cnt[main_findex] += 1;
    total_dist += move_dist(&main_prev.pos, &main_key.pos);
    total_dist += move_dist(&main_key.pos, &main_home.pos);

    time_end_press = time_end_press.max(this_end_press);
    time_end_move = time_end_move.max(this_end_move);

    // TODO: This should first finish the previous move by
    // either moving back home or moving to this press' key. Can give
    // the option to not fencepost so stitching is easier

    if !gen_anim {
      // The animation-less mode still relies on the last keyframe
      fingers[main_findex][0] = Keyframe {
        pos: main_home.pos,
        time: time_end_move,
        start_press: false,
        on_char: main_home.pressed,
      };
    }

    min_start = time_end_press;
    total_time = time_end_move;
  }

  Timeline {
    fingers,
    finger_counts: finger_usage_cnt,
    total_time,
    total_dist,
    total_words: string.split_whitespace().count() as u32,
    total_chars: string.len() as u32,
  }
}

// Given the starting frame, what to press, where to return, add
// the necessary frames for the whole move
// min_start is the earliest the finger can start moving to the key
// min_press is the earliest the key can start being pressed
// return the time the press ends and when the move ends
fn calc_keyframes(
  prev: &Keyframe,
  press_key: &layout::Key,
  home_key: &layout::Key,
  min_start: i32,
  min_press: i32,
  push_frames: bool,
  frames: &mut Vec<Keyframe>,
) -> (i32, i32) {
  let time_start_move = std::cmp::max(min_start, prev.time);
  let dur_start_move = move_time(&prev.pos, &press_key.pos);
  let time_start_press = min_press;
  let dur_end_move = move_time(&press_key.pos, &home_key.pos);

  if !push_frames {
    return (
      time_start_press + PRESS_DUR,
      time_start_press + PRESS_DUR + dur_end_move,
    );
  }

  // Avoid duplicating end frame of previous move
  if time_start_move != prev.time {
    frames.push(Keyframe {
      pos: prev.pos,
      time: time_start_move,
      start_press: false,
      on_char: prev.on_char,
    });
  }

  // Move to key and wait for other fingers
  if time_start_move + dur_start_move != time_start_press {
    frames.push(Keyframe {
      pos: press_key.pos,
      time: time_start_move + dur_start_move,
      start_press: false,
      on_char: press_key.pressed,
    })
  };

  // Start pressing
  frames.push(Keyframe {
    pos: press_key.pos,
    time: time_start_press,
    start_press: true,
    on_char: press_key.pressed,
  });

  // End pressing
  frames.push(Keyframe {
    pos: press_key.pos,
    time: time_start_press + PRESS_DUR,
    start_press: false,
    on_char: press_key.pressed,
  });

  // Move back home
  frames.push(Keyframe {
    pos: home_key.pos,
    time: time_start_press + PRESS_DUR + dur_end_move,
    start_press: false,
    on_char: home_key.pressed,
  });

  (
    time_start_press + PRESS_DUR,
    time_start_press + PRESS_DUR + dur_end_move,
  )
}

pub fn print_timeline(tl: &Timeline) {
  for i in 0..10 {
    println!("Finger {}", i);
    println!("  Usage %: {}", tl.usage_percent(i));
    for kf in &tl.fingers[i] {
      println!(
        "    {}, {}, {}ms, {}, on \"{}\"",
        kf.pos.x, kf.pos.y, kf.time, kf.start_press, kf.on_char
      );
    }
  }

  println!(
    "Total distance covered: {}u, {}mm",
    tl.total_dist,
    tl.total_dist * 19.05
  );
  println!("Total time {}s", (tl.total_time) / 1000);
  println!("Total words: {}", tl.total_words);
  println!("WPM: {}", tl.wpm());
}

pub fn gen_timeline_file(path: &String, parallel: bool, lay: &layout::Layout) -> Timeline {
  let mut file = match std::fs::File::open(path) {
    Ok(f) => f,
    Err(_) => panic!("file problem"),
  };

  let mut text = String::new();
  file.read_to_string(&mut text).unwrap();

  if parallel {
    gen_timeline_parallel(text.as_str(), lay)
  } else {
    gen_timeline(text.as_str(), false, lay)
  }
}

fn gen_timeline_parallel<'a>(string: &'a str, lay: &layout::Layout) -> Timeline {
  let coll: Vec<Timeline> = string
    .par_lines()
    .map(|line| gen_timeline(line, false, lay))
    .collect();

  let mut res = Timeline::default();

  // This is slightly inaccurate, w/ <1% error in total_time, and
  // ~0.05% error in distance covered, both overestimating.  TODO:
  // Mesh tl's together better with first and last moves for each
  // finger
  for tl in coll {
    for i in 0..10 {
      res.finger_counts[i] += tl.finger_counts[i];
    }

    res.total_time += tl.total_time;
    res.total_dist += tl.total_dist;
    res.total_words += tl.total_words;
    res.total_chars += tl.total_chars;
  }

  res
}

fn move_dist(start: &layout::Pos, end: &layout::Pos) -> f32 {
  let x_diff = start.x - end.x;

  let y_diff = start.y - end.y;

  (x_diff.powi(2) + y_diff.powi(2)).sqrt()
}

fn move_time(start: &layout::Pos, end: &layout::Pos) -> i32 {
  (move_dist(start, end) * 250.0) as i32
}

#[cfg(test)]
mod tests {
  use super::*;

  // Turn a timeline into a flat list of Vec<Keyframes> for testing
  // Multiple Keyframes at the same time are put into the same inner Vec<>
  // Not very efficient, but for testing it's fine
  fn flatten_timeline(tl: &Timeline) -> Vec<Vec<Keyframe>> {
    let mut earliest_time = i32::MAX;
    let mut earliest_indices = Vec::new();

    let mut finger_frontier = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    let mut flattened = Vec::new();

    // Loop through all 10 fingers and find the earliest frames
    'outer: loop {
      let mut this_frame = Vec::new();
      let mut frames_left = false;
      for i in 0..10 {
        if tl.fingers[i].len() > finger_frontier[i] {
          if tl.fingers[i][finger_frontier[i]].time < earliest_time {
            frames_left = true;

            earliest_time = tl.fingers[i][finger_frontier[i]].time;
            earliest_indices.clear();
            earliest_indices.push(i);
          } else if tl.fingers[i][finger_frontier[i]].time == earliest_time {
            earliest_indices.push(i);
          }
        }
      }
      if !frames_left {
        break 'outer;
      }

      // Copy every frame that occured at earliest_time milliseconds
      for idx in &earliest_indices {
        let original = &tl.fingers[*idx][finger_frontier[*idx]];
        this_frame.push(Keyframe {
          pos: layout::Pos {
            x: original.pos.x,
            y: original.pos.y,
          },
          time: original.time,
          start_press: original.start_press,
          on_char: original.on_char,
        });
        finger_frontier[*idx] += 1;
      }

      flattened.push(this_frame);

      earliest_time = i32::MAX;
    }

    flattened
  }

  fn common_invariants(tl: &Timeline, string: &str) {
    let flat = flatten_timeline(tl);

    let mut prev_time = 0;
    let mut curr_char = 0;
    for moment in flat {
      let new_time = moment.first().unwrap().time;
      assert!(new_time >= prev_time, "A keyframe went backwards in time");

      for frame in moment {
        if frame.start_press && frame.on_char != '\0' {
          assert_eq!(
            frame.on_char.to_ascii_lowercase(),
            string.chars().nth(curr_char).unwrap().to_ascii_lowercase(),
            "A key was pressed out of order"
          );
          curr_char += 1;
        }
      }

      prev_time = new_time;
    }
  }

  #[test]
  fn one_finger() {
    // Test back to back single-finger movement
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "rgvf";
    let tl = gen_timeline(text, true, lay);
    common_invariants(&tl, text);
  }

  #[test]
  fn moveless_text() {
    // Test text that is all on the home row
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "asdf jkl;";
    let tl = gen_timeline(text, true, lay);
    common_invariants(&tl, text);

    let mut prev_press_end = 0;
    for i in 0..10 {
      for kf in &tl.fingers[i] {
        if kf.start_press {
          assert_eq!(prev_press_end, kf.time);
          prev_press_end = kf.time + PRESS_DUR;
        }
      }
    }
  }

  #[test]
  fn shifted() {
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "uPpErCaSe AnD lOwErCaSe";
    let tl = gen_timeline(text, true, lay);
    common_invariants(&tl, text);
    let flat = flatten_timeline(&tl);

    let mut shift_on = false;
    let mut curr_char = 0;
    for moment in &flat {
      // Look for shifts
      for frame in moment {
        // TODO: If I add other modifiers, this needs to be changed
        if frame.on_char == '\0' {
          shift_on = frame.start_press;
        }
      }

      // Now check that right chars are shifted
      for frame in moment {
        if frame.on_char != '\0' && frame.start_press {
          let key_mods = lay
            .char_keys
            .get(text.chars().nth(curr_char).as_ref().unwrap())
            .unwrap()
            .mods
            .as_ref();
          if key_mods.is_some() {
            assert!(shift_on);
          }
          curr_char += 1;
        }
      }
    }
  }

  #[test]
  fn return_to_home() {
    // All fingers' last position should be home
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "qxevy,o/";
    let tl = gen_timeline(text, true, lay);

    for i in 0..10 {
      assert_eq!(tl.fingers[i].last().unwrap().pos.x, lay.homes[i].pos.x);
      assert_eq!(tl.fingers[i].last().unwrap().pos.y, lay.homes[i].pos.y);
    }
  }

  #[test]
  fn distance() {
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "qhv";
    let tl = gen_timeline(text, true, lay);

    let q_dist = 2.0 * (0.25_f32.powi(2) + 1.0).sqrt();
    let h_dist = 2.0;
    let v_dist = 2.0 * (0.5_f32.powi(2) + 1.0).sqrt();
    assert_eq!(tl.total_dist, q_dist + h_dist + v_dist);
  }

  #[test]
  fn distance_no_shift() {
    // For now shift movement isn't included in distance
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "QHV";
    let tl = gen_timeline(text, true, lay);

    let q_dist = 2.0 * (0.25_f32.powi(2) + 1.0).sqrt();
    let h_dist = 2.0;
    let v_dist = 2.0 * (0.5_f32.powi(2) + 1.0).sqrt();
    assert_eq!(tl.total_dist, q_dist + h_dist + v_dist);
  }

  #[test]
  fn usage() {
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "qwertyuiop";
    let tl = gen_timeline(text, true, lay);

    assert_eq!(tl.usage_percent(0), 100.0 * (1.0 / 10.0));
    assert_eq!(tl.usage_percent(3), 100.0 * (2.0 / 10.0));
    assert_eq!(tl.usage_percent(4), 0.0);
    assert_eq!(tl.usage_percent(9), 100.0 * (1.0 / 10.0));
  }

  #[test]
  fn usage_no_shift() {
    // For now shifting doesn't get counted as usage
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "QPWO";
    let tl = gen_timeline(text, true, lay);

    assert_eq!(tl.usage_percent(0), 25.0);
    assert_eq!(tl.usage_percent(1), 25.0);
    assert_eq!(tl.usage_percent(8), 25.0);
    assert_eq!(tl.usage_percent(9), 25.0);
  }

  #[test]
  fn no_anim() {
    // Timelines generated without animations should have the same stats
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "The Quick Brown Fox Jumps Over The Lazy Dog.";
    let tl = gen_timeline(text, true, lay);
    let tl_no_anim = gen_timeline(text, false, lay);

    assert_eq!(tl.total_time, tl_no_anim.total_time);
    assert_eq!(tl.total_dist, tl_no_anim.total_dist);
    assert_eq!(tl.total_words, tl_no_anim.total_words);
    assert_eq!(tl.total_chars, tl_no_anim.total_chars);
  }

  #[test]
  fn parallel() {
    // TODO: Fix this. Implement above change to gen_timeline so home
    // row returns can just not be generated in the first place
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "The Quick Brown\nFox Jumps Over\nThe Lazy Dog.";
    let tl = gen_timeline(text, true, lay);
    let tl_parallel = gen_timeline_parallel(text, lay);

    assert_eq!(tl.total_time, tl_parallel.total_time);
    assert_eq!(tl.total_dist, tl_parallel.total_dist);
    assert_eq!(tl.total_words, tl_parallel.total_words);
    assert_eq!(tl.total_chars, tl_parallel.total_chars);
  }
}
