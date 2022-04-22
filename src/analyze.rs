use super::layout;

pub struct Timeline {
  pub fingers: [Vec<Keyframe>; 10],
  // Total distance covered by all fingers in u
  pub total_dist: f32,
  pub wpm: u16,
}

pub struct Keyframe {
  pub pos: layout::Pos,
  pub time: i32,
  pub start_press: bool,
  on_char: char,
}

static PRESS_DUR: i32 = 250;

pub fn gen_timeline<'a>(string: &str, lay: &'a layout::Layout) -> Timeline {
  // May want a different data structure for generating vs playback
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

  // The next press must start at or after min_press
  let mut min_press = 0;

  for c in string.chars() {
    let key = match lay.str_keys.get(&c) {
      Some(k) => k,
      None => panic!(),
    };
    let findex = key.finger as usize;

    let home_key = lay.homes[findex];
    let prev_frame = fingers[findex].last().unwrap();

    let start_move_dur = move_time(&prev_frame.pos, &key.pos);
    // Only add starting keyframe if the finger has been at rest since
    // its last move
    let time_start_move = if min_press > prev_frame.time {
      let start_move = Keyframe {
        pos: prev_frame.pos,
        time: min_press,
        start_press: false,
        on_char: prev_frame.on_char,
      };
      total_dist += move_dist(&prev_frame.pos, &key.pos);
      fingers[findex].push(start_move);

      min_press
    } else {
      prev_frame.time
    };

    let time_start_press = time_start_move + start_move_dur;
    let start_press = Keyframe {
      pos: key.pos,
      time: time_start_press,
      start_press: true,
      on_char: key.pressed,
    };
    fingers[findex].push(start_press);

    let time_end_press = time_start_press + PRESS_DUR;
    let end_press = Keyframe {
      pos: key.pos,
      time: time_end_press,
      start_press: false,
      on_char: key.pressed,
    };
    fingers[findex].push(end_press);

    let end_move_dur = move_time(&key.pos, &home_key.pos);
    let time_end_move = time_end_press + end_move_dur;
    let end_move = Keyframe {
      pos: home_key.pos,
      time: time_end_move,
      start_press: false,
      on_char: home_key.pressed,
    };
    total_dist += move_dist(&key.pos, &home_key.pos);
    fingers[findex].push(end_move);

    min_press = time_end_press;
  }

  let word_count = string.split(' ').count() as f32;
  let wpm = (word_count / ((min_press as f32) / 60000.0)) as u16;

  Timeline {
    fingers,
    total_dist,
    wpm,
  }
}

pub fn print_timeline(tl: &Timeline) {
  for i in 0..10 {
    println!("Finger {}", i);
    for kf in &tl.fingers[i] {
      println!(
        "{}, {}, {}ms, {}",
        kf.pos.x, kf.pos.y, kf.time, kf.start_press
      );
    }
  }
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
  // Not very memory efficient, but for testing it's fine
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
        // Note: this assumes there won't be multiple presses at the
        // same time. For now this is true, and this shouldn't have to
        // change when I add key combos
        if frame.start_press {
          assert_eq!(
            frame.on_char,
            string.chars().nth(curr_char).unwrap(),
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
    let tl = gen_timeline(text, lay);
    common_invariants(&tl, text);
  }

  #[test]
  fn moveless_text() {
    // Test text that is all on the home row
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "asdf jkl;";
    let tl = gen_timeline(text, lay);
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
  fn return_to_home() {
    // All fingers' last position should be home
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "qxevy,o/";
    let tl = gen_timeline(text, lay);

    for i in 0..10 {
      assert_eq!(tl.fingers[i].last().unwrap().pos.x, lay.homes[i].pos.x);
      assert_eq!(tl.fingers[i].last().unwrap().pos.y, lay.homes[i].pos.y);
    }
  }
}
