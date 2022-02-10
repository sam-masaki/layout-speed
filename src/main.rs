use csv::ReaderBuilder;
use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator, TextureQuery, WindowCanvas};
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::{Window, WindowContext};
use sdl2::Sdl;
use sdl2::VideoSubsystem;
use std::collections::HashMap;
use std::ops::Index;
use std::path::Path;
use std::time::Duration;

static SCREEN_WIDTH: u32 = 1280;
static SCREEN_HEIGHT: u32 = 720;

// Finger speed in u/s
static SPEED_UP: f32 = 1.0;
static SPEED_DN: f32 = 1.5;
static SPEED_LT: f32 = 2.0;
static SPEED_RT: f32 = 2.0;

static PRESS_DUR: u32 = 100;

struct Key {
    pos: (f32, f32),
    width: f32,
    height: f32,
    shifted: char,
    unshifted: char,
    finger: i16,
    home: bool,
}

struct Move {
    modifiers: Vec<String>,
    key: String,
}

enum FingAct {
    MoveTo { x: f32, y: f32, t: u32 }, // move to (x, y) in t millis
    Wait { t: u32 },                   // wait t millis
    Press { t: u32 },                  // press for t millis
}

// This is just awful, replace with separate lists for positions and pressing
fn last_pos(moves: &Vec<FingAct>) -> Option<(f32, f32)> {
    let mut res = (0.0, 0.0);

    for m in moves {
        match m {
            FingAct::MoveTo { x, y, t: _ } => res = (x.clone(), y.clone()),
            FingAct::Wait { t: _ } => continue,
            FingAct::Press { t: _ } => continue,
        }
    }

    Some(res)
}

struct KeyLayout {
    keys: HashMap<String, Key>,
    homes: [String; 10],
}

// TODO: Add wait time?
struct FingPos {
    x: f32,
    y: f32,
    t: u32,
}

struct FingPrs {
    start: u32,    // ms
    duration: u32, // ms
}

struct FingTimeline {
    pos: [Vec<FingPos>; 10],
    prs: [Vec<FingPrs>; 10],
}

impl FingTimeline {
    pub fn construct(layout: &KeyLayout) -> Self {
        let mut pos = [
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

        let prs = [
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

        (0..10).for_each(|i| match layout.keys.get(&layout.homes[i]) {
            Some(k) => pos[i].push(FingPos {
                x: k.pos.0,
                y: k.pos.1,
                t: 0,
            }),
            None => pos[i].push(FingPos {
                x: 0.0,
                y: 0.0,
                t: 0,
            }),
        });

        FingTimeline { pos, prs }
    }
}

fn dist(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
}

// Stores timelines of finger actions for typing something
struct TypeSeq {
    fingers: FingTimeline,
}

impl TypeSeq {
    pub fn construct(
        word: String,
        chartomove: &HashMap<char, Move>,
        layout: &KeyLayout,
    ) -> Option<Self> {
        let mut res = FingTimeline::construct(layout);

        let mut start_time: u32 = 0; // Earliest time next move can start
        let mut prev_press: u32 = 0; // Time previous letter finished being pressed
        for letter in word.chars() {
            let key_move = chartomove.get(&letter)?;

            let key = layout.keys.get(&key_move.key)?;
            let fing = key.finger as usize;

            let last_x;
            let last_y;
            let last_t;

            {
                let last_pos = res.pos[fing].last()?;
                last_x = last_pos.x;
                last_y = last_pos.y;

                // Add wait if the move cannot start yet
                if last_pos.t < start_time {
                    res.pos[fing].push(FingPos {
                        x: last_x,
                        y: last_y,
                        t: start_time,
                    });
                    last_t = start_time;
                } else {
                    last_t = last_pos.t;
                }
            }

            let dist_x = key.pos.0 - last_x;
            let dist_y = key.pos.1 - last_y;

            let h_speed;
            let v_speed;
            if dist_x > 0.0 {
                h_speed = SPEED_RT;
            } else {
                h_speed = SPEED_LT;
            }
            if dist_y > 0.0 {
                v_speed = SPEED_DN;
            } else {
                v_speed = SPEED_UP;
            }

            // horizontal & vertical duration in seconds
            let h_dur_s = (dist_x / h_speed).abs();
            let v_dur_s = (dist_y / v_speed).abs();

            let dur_s;
            if h_dur_s > v_dur_s {
                dur_s = h_dur_s;
            } else {
                dur_s = v_dur_s;
            }

            // move-to-key duration in ms
            let dur = (dur_s * 1000.0).round() as u32;

            // time when finger finishes moving
            let move_end = last_t + dur;
            let press_start;

            // press needs to start after the previous press
            if move_end < prev_press {
                press_start = prev_press;
            } else {
                press_start = move_end;
            }

            let press_end = press_start + PRESS_DUR;

            res.pos[fing].push(FingPos {
                x: key.pos.0,
                y: key.pos.1,
                t: press_start,
            });
            res.pos[fing].push(FingPos {
                x: key.pos.0,
                y: key.pos.1,
                t: press_end,
            });
            res.prs[fing].push(FingPrs {
                start: press_start,
                duration: PRESS_DUR,
            });

            prev_press = press_end;
            start_time += 100;
        }

        Some(Self { fingers: res })
    }

    fn print(&self) {
        for i in 0..10 {
            println!("finger {}", i);

            for pos in &self.fingers.pos[i] {
                println!("{}, {}, at {}", pos.x, pos.y, pos.t);
            }

            for prs in &self.fingers.prs[i] {
                println!("pressing at {}ms for {}ms", prs.start, prs.duration);
            }
        }
    }
}

fn init_sdl(title: &str) -> Result<(Sdl, WindowCanvas, Sdl2TtfContext), String> {
    let context = sdl2::init()?;
    let video = context.video()?;
    let window = video
        .window(title, SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    Ok((context, canvas, ttf_context))
}

fn draw_text(x: i32, y: i32, text: &str, font: &Font, canvas: &mut Canvas<Window>) {
    let surface = font
        .render(text)
        .blended(Color::RGBA(255, 0, 0, 255))
        .unwrap();
    let creator = canvas.texture_creator();
    let texture = creator.create_texture_from_surface(&surface).unwrap();

    let TextureQuery { width, height, .. } = texture.query();
    let pos = Rect::new(x, y, width, height);
    canvas.copy(&texture, None, pos).unwrap();
}

// Read a layout file and parse it into a map from key name to Key
fn init_layout() -> Option<KeyLayout> {
    let mut reader;
    match csv::ReaderBuilder::new().from_path("qwerty.layout") {
        Ok(r) => reader = r,
        Err(e) => panic!("{}", e),
    }

    let mut keys = HashMap::new();
    let mut homes = [
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];

    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    let mut prev_w = 0.0;

    for result in reader.records() {
        let record;
        match result {
            Ok(r) => record = r,
            Err(_) => return None,
        }

        //let label = record.get(0)?;
        let hash_key = record.get(0)?.to_string();
        //println!("{}", label);
        let unshifted = record.get(1)?.chars().next().unwrap_or('\0');
        let shifted = record.get(2)?.chars().next().unwrap_or('\0');

        let finger = record.get(3)?.parse::<i16>().unwrap_or(0);
        let home = !record.get(4)?.is_empty();

        if home && finger >= 0 && finger < 10 {
            homes[finger as usize] = hash_key.clone();
        }

        // format assumes keys are 1u and go left-to-right unless position specified
        let x = record.get(5)?.parse::<f32>().unwrap_or(prev_x + prev_w);
        let y = record.get(6)?.parse::<f32>().unwrap_or(prev_y);
        let w = record.get(7)?.parse::<f32>().unwrap_or(1.0);
        let h = record.get(8)?.parse::<f32>().unwrap_or(1.0);

        keys.insert(
            hash_key,
            Key {
                pos: (x, y),
                width: w,
                height: h,
                shifted,
                unshifted,
                finger,
                home,
            },
        );

        prev_x = x;
        prev_y = y;
        prev_w = w;
    }

    //    print!("{}", layout.len());

    Some(KeyLayout { keys, homes })
}

fn init_charmap(layout: &HashMap<String, Key>) -> HashMap<char, Move> {
    let mut res = HashMap::new();

    for (hashkey, key) in layout {
        // Skip non-alphanumeric keys
        if key.unshifted == '\0' {
            continue;
        }

        res.insert(
            key.unshifted,
            Move {
                modifiers: Vec::new(),
                key: hashkey.clone(),
            },
        );

        if key.shifted == '\0' {
            continue;
        }

        // Use opposite hand's pinky for shift
        let shift;
        if key.finger < 5 {
            shift = "rshift".to_string();
        } else {
            shift = "lshift".to_string();
        }

        res.insert(
            key.shifted,
            Move {
                modifiers: vec![shift],
                key: hashkey.clone(),
            },
        );
    }

    res
}

fn draw_layout(layout: &HashMap<String, Key>, font: &Font, canvas: &mut Canvas<Window>) {
    let w = 50.0;
    let r = 10;
    let color = Color::RGB(0, 0, 255);

    for (name, key) in layout {
        canvas
            .rounded_rectangle(
                (key.pos.0 * w) as i16,
                (key.pos.1 * w) as i16,
                ((key.pos.0 * w) + (w * key.width)) as i16,
                ((key.pos.1 * w) + w) as i16,
                r,
                color,
            )
            .unwrap();

        draw_text(
            (key.pos.0 * w) as i32 + 5,
            (key.pos.1 * w) as i32 + 5,
            name,
            font,
            canvas,
        );

        if key.shifted != '\0' {
            draw_text(
                (key.pos.0 * w) as i32 + 5,
                (key.pos.1 * w) as i32 + 17,
                &String::from(key.shifted),
                font,
                canvas,
            );
        }
        draw_text(
            (key.pos.0 * w) as i32 + 5,
            (key.pos.1 * w) as i32 + 29,
            &format!("{}", key.finger),
            font,
            canvas,
        );

        if key.home {
            draw_text(
                (key.pos.0 * w) as i32 + 17,
                (key.pos.1 * w) as i32 + 29,
                "*",
                font,
                canvas,
            );
        }
    }
}

fn handle_key(keycode: Keycode) {}

fn word_to_moves(word: String, moves: &HashMap<char, Move>) -> Option<Vec<String>> {
    let mut res = Vec::new();

    for c in word.chars() {
        let modifiers = &moves.get(&c)?.modifiers;
        let key = &moves.get(&c)?.key;

        for m in modifiers {
            res.push(m.clone());
        }
        res.push(key.clone());
    }

    Some(res)
}

pub fn main() {
    let (context, mut canvas, ttf) = init_sdl("Layout Speed").unwrap();

    let font = ttf
        .load_font(
            Path::new("/usr/share/fonts/noto/NotoSansMono-Regular.ttf"),
            12,
        )
        .unwrap();

    let layout;
    match init_layout() {
        Some(l) => layout = l,
        None => return,
    }

    let chartomove = init_charmap(&layout.keys);

    for (k, v) in &chartomove {
        print!("{}: ", k);

        // for s in &v.keys {
        //     print!("{}, ", s);
        // }

        println!();
    }

    match word_to_moves("aBcDEf".to_string(), &chartomove) {
        Some(r) => {
            println!("start of word");
            for s in r {
                println!("{}", s);
            }
        }
        None => println!("it failed"),
    }

    let abc;
    match TypeSeq::construct("abcdefgh".to_string(), &chartomove, &layout) {
        Some(r) => {
            abc = r;
            abc.print();
        }
        None => return,
    }

    let mut vis_pos_keyframes: [u32; 10] = [0; 10];
    let mut vis_prs_keyframes: [u32; 10] = [0; 10];
    let mut vis_curtime: u32 = 0;

    let mut event_pump = context.event_pump().unwrap();
    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => handle_key(key),
                _ => {}
            }
        }

        let mut has_moved = false;
        for findex in 0..10 {
            let pos_keyframe;
            match abc.fingers.pos[findex].get(vis_pos_keyframes[findex] as usize) {
                Some(k) => pos_keyframe = k,
                None => continue,
            }

            // let prs_keyframe;
            // match abc.fingers.prs[findex].get(vis_prs_keyframes[findex] as usize) {
            //     Some(k) => prs_keyframe = k,
            //     None => continue,
            // }

            let prev_x = pos_keyframe.x;
            let prev_y = pos_keyframe.y;

            let vis_x;
            let vis_y;
            // If we are after the current keyframe and before a coming frame

            if findex == 2 {
                println!(
                    "=== on {} keyframe, at {}. keyframe time is {}",
                    findex, vis_curtime, pos_keyframe.t
                );
            }

            if vis_curtime > pos_keyframe.t {
                // If current keyframe is not the last

                if vis_pos_keyframes[findex] < abc.fingers.pos[findex].len() as u32 - 1 {
                    let next_pos = abc.fingers.pos[findex]
                        .get(vis_pos_keyframes[findex] as usize + 1)
                        .unwrap();

                    let total_time_diff = next_pos.t - pos_keyframe.t;
                    let cur_time_diff = vis_curtime - pos_keyframe.t;

                    println!(
                        "{}: total_time_diff {}, cur_time_diff {}",
                        findex, total_time_diff, cur_time_diff
                    );

                    if total_time_diff < cur_time_diff || total_time_diff - cur_time_diff < 16 {
                        vis_pos_keyframes[findex] += 1;
                    }
                    if total_time_diff == 0 {
                        continue;
                    }

                    let x_diff = next_pos.x - pos_keyframe.x;
                    let y_diff = next_pos.y - pos_keyframe.y;

                    vis_x = prev_x + (x_diff * (cur_time_diff as f32 / total_time_diff as f32));
                    vis_y = prev_y + (y_diff * (cur_time_diff as f32 / total_time_diff as f32));
                } else {
                    vis_x = prev_x;
                    vis_y = prev_y;
                }

                // CONTINUE HERE calculate the current position of the finger and draw it as a circle or something
            } else {
                // We are at the keyframe
                vis_x = prev_x;
                vis_y = prev_y;

                vis_pos_keyframes[findex] += 1;
            }

            has_moved = true;

            println!(
                "fing {} at {}, {}, at time {}",
                findex, vis_x, vis_y, vis_curtime
            );

            canvas
                .circle(
                    (vis_x * 50.0) as i16,
                    (vis_y * 50.0) as i16,
                    10,
                    Color::RGB(0, 255, 0),
                )
                .unwrap();
        }

        vis_curtime += 16;

        //draw_text(0, 0, "abc", &font, &mut canvas);

        //canvas.circle(100, 1, 1, Color::RGB(0, 255, 0)).unwrap();

        draw_layout(&layout.keys, &font, &mut canvas);

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
