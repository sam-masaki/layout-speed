use super::layout;

pub struct Timeline {
    pub fingers: [Vec<Keyframe>; 10],
}

pub struct Keyframe {
    pub pos: layout::Pos,
    pub time: i32,
    pub start_press: bool,
}

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

        let start_move_time = min_press;
        let start_press_time = start_move_time + move_time(&prev_frame.pos, &key.pos);
        let end_press_time = start_press_time + 250;
        let end_move_time = end_press_time + move_time(&key.pos, &home_key.pos);

        let start_move = Keyframe {
            pos: layout::Pos {
                x: prev_frame.pos.x,
                y: prev_frame.pos.y,
            },
            time: start_move_time,
            start_press: false,
        };
        let start_press = Keyframe {
            pos: layout::Pos {
                x: key.pos.x,
                y: key.pos.y,
            },
            time: start_press_time,
            start_press: true,
        };
        let end_press = Keyframe {
            pos: layout::Pos {
                x: key.pos.x,
                y: key.pos.y,
            },
            time: end_press_time,
            start_press: false,
        };
        let end_move = Keyframe {
            pos: layout::Pos {
                x: home_key.pos.x,
                y: home_key.pos.y,
            },
            time: end_move_time,
            start_press: false,
        };

        fingers[findex].push(start_move);
        fingers[findex].push(start_press);
        fingers[findex].push(end_press);
        fingers[findex].push(end_move);

        min_press = end_press_time;
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

    std::cmp::max((dist * 250.0) as i32, 250)
}
