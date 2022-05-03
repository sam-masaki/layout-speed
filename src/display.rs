use super::layout;
use super::playback;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::render::TextureQuery;
use sdl2::ttf::Font;
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::Window;
use sdl2::Sdl;
use std::path::Path;

pub struct Data<'a, 'b> {
  pub context: Sdl,
  pub canvas: Canvas<Window>,
  pub ttf: &'a Sdl2TtfContext,
  pub font: Font<'a, 'b>,
}

static SCREEN_WIDTH: u32 = 1280;
static SCREEN_HEIGHT: u32 = 720;

static KEY_W: f32 = 50.0;
static KEY_H: f32 = 50.0;
static KEY_RAD: i16 = 10;
static KEY_COL: Color = Color::RGB(0, 0, 255);

pub fn init(title: &str) -> Result<(Sdl, Canvas<Window>, Sdl2TtfContext), String> {
  let context = sdl2::init()?;
  let video = context.video()?;
  let window = video
    .window(title, SCREEN_WIDTH, SCREEN_HEIGHT)
    .position_centered()
    .build()
    .map_err(|e| e.to_string())?;
  let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
  let ttf = sdl2::ttf::init().map_err(|e| e.to_string())?;

  Ok((context, canvas, ttf))
}

// TODO: Get the font into Data. Not sure how to make it work with the borrow checker
pub fn init_font(ttf: &Sdl2TtfContext) -> Font {
  ttf
    .load_font(
      Path::new("./NotoSansMono-Regular.ttf"),
      12,
    )
    .unwrap()
}

pub fn draw_text(x: i32, y: i32, text: &str, data: &mut Data) {
  let surface = data
    .font
    .render(text)
    .blended(Color::RGBA(255, 0, 0, 255))
    .unwrap();
  let creator = data.canvas.texture_creator();
  let texture = creator.create_texture_from_surface(&surface).unwrap();

  let TextureQuery { width, height, .. } = texture.query();
  let pos = Rect::new(x, y, width, height);
  data.canvas.copy(&texture, None, pos).unwrap();
}

pub fn draw_playdata(playdata: &playback::PlayData, disp_data: &mut Data) {
  for i in 0..10 {
    let finger = &playdata.fingers[i];
    let x = ((finger.pos.x * KEY_W) + (KEY_H / 2.0)) as i16;
    let y = ((finger.pos.y * KEY_H) + (KEY_H / 2.0)) as i16;

    disp_data.canvas.circle(x, y, 10, KEY_COL).unwrap();
    if finger.pressing {
      disp_data.canvas.circle(x, y, 15, KEY_COL).unwrap();
    }
  }
}

pub fn draw_layout(lay: &layout::Layout, data: &mut Data) {
  for key in &lay.keys {
    draw_key(key, data);
  }
  for key in lay.mod_map.values() {
    draw_key(key, data);
  }
}

fn draw_key(key: &layout::Key, data: &mut Data) {
  let x1 = (key.pos.x * KEY_W) as i16;
  let y1 = (key.pos.y * KEY_H) as i16;
  let x2 = x1 + ((KEY_W * key.visual.width) as i16);
  let y2 = y1 + (KEY_W as i16);

  data
    .canvas
    .rounded_rectangle(x1, y1, x2, y2, KEY_RAD, KEY_COL)
    .unwrap();

  draw_text(
    (x1 + (KEY_RAD / 2)) as i32,
    (y1 + (KEY_RAD / 2)) as i32,
    &key.visual.name,
    data,
  );

  if key.is_home {
    draw_text(
      (x1 + (KEY_RAD / 2)) as i32,
      (y1 + (KEY_RAD / 2) + ((KEY_H as i16) / 2)) as i32,
      "*",
      data,
    )
  }
}
