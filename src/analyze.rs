use super::layout;

pub struct Timeline {
  pub fingers: [Vec<Keyframe>; 10],
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

  // The next press must start after min_press
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
    let end_move_dur = move_time(&key.pos, &home_key.pos);

    let time_start_move = if min_press > prev_frame.time {
      min_press
    } else {
      prev_frame.time
    };

    if time_start_move != prev_frame.time {
      let start_move = Keyframe {
        pos: layout::Pos {
          x: prev_frame.pos.x,
          y: prev_frame.pos.y,
        },
        time: time_start_move,
        start_press: false,
        on_char: prev_frame.on_char,
      };
      fingers[findex].push(start_move);
    }

    let time_start_press = time_start_move + start_move_dur;
    let time_end_press = time_start_press + PRESS_DUR;
    let time_end_move = time_end_press + end_move_dur;

    let start_press = Keyframe {
      pos: layout::Pos {
        x: key.pos.x,
        y: key.pos.y,
      },
      time: time_start_press,
      start_press: true,
      on_char: key.pressed,
    };
    fingers[findex].push(start_press);

    let end_press = Keyframe {
      pos: layout::Pos {
        x: key.pos.x,
        y: key.pos.y,
      },
      time: time_end_press,
      start_press: false,
      on_char: key.pressed,
    };
    fingers[findex].push(end_press);

    let end_move = Keyframe {
      pos: layout::Pos {
        x: home_key.pos.x,
        y: home_key.pos.y,
      },
      time: time_end_move,
      start_press: false,
      on_char: home_key.pressed,
    };
    fingers[findex].push(end_move);

    min_press = time_end_press;
  }

  Timeline { fingers }
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

fn move_time(start: &layout::Pos, end: &layout::Pos) -> i32 {
  let x_diff = start.x - end.x;
  let y_diff = start.y - end.y;

  let dist = (x_diff.powi(2) + y_diff.powi(2)).sqrt();

  //  std::cmp::max((dist * 250.0) as i32, 250)
  (dist * 250.0) as i32
}

#[cfg(test)]
mod tests {
  use super::*;

  fn common_invariants(tl: &Timeline, string: &str) {
    for i in 0..10 {
      let mut prev_time = 0;
      let mut curr_char = 0;
      for kf in &tl.fingers[i] {
        assert!(kf.time >= prev_time, "A keyframe goes backwards in time");
        prev_time = kf.time;

        // FIXME: This doesn't work because it's going through each
        // finger individually. Need a utility function for turning a
        // Timeline into a flat list of KeyFrames
        if kf.start_press {
          assert_eq!(
            kf.on_char,
            string.chars().nth(curr_char).unwrap(),
            "A key is pressed out of order"
          );
          curr_char += 1;
        }
      }
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
    // Test text that's all on the home row
    let mut lay = layout::Layout::default();
    let lay = layout::init(&mut lay, "qwerty.layout").unwrap();

    let text = "asdf jkl;";
    let tl = gen_timeline(text, lay);
    // TODO: after fixing, check this again
    //common_invariants(&tl, text);

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
}
