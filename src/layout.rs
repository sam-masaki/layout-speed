use std::collections::HashMap;

pub struct Key {
  pub pressed: char,
  pub shifted: char,
  // TODO: Looks like I can have polymorphic enums for modifiers, but
  // that seems like the kind of rabbit hole I don't need for this proj
  pub finger: i16,
  pub is_home: bool,
  pub pos: Pos,
  pub visual: VisKey,
}

#[derive(Default, Copy, Clone)]
pub struct Pos {
  pub x: f32,
  pub y: f32,
}

// For drawing key to screen
pub struct VisKey {
  pub width: f32,
  pub height: f32,
  pub name: String,
}

pub struct Layout<'a> {
  pub keys: Vec<Key>,
  pub str_keys: HashMap<char, &'a Key>,
  pub homes: [&'a Key; 10],
}

impl<'a> Default for Layout<'a> {
  fn default() -> Self {
    Self {
      keys: Vec::new(),
      str_keys: HashMap::new(),
      homes: [
        &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY,
        &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY,
      ],
    }
  }
}

pub static DUMMY_KEY: Key = Key {
  pressed: '\0',
  shifted: '\0',
  finger: -1,
  is_home: false,
  pos: Pos { x: 0.0, y: 0.0 },
  visual: VisKey {
    width: 0.0,
    height: 0.0,
    name: String::new(),
  },
};

// Fill lay with the layout info from path
pub fn init<'a>(lay: &'a mut Layout<'a>, path: &str) -> Option<&'a Layout<'a>> {
  let mut reader;
  match csv::ReaderBuilder::new().from_path(path) {
    Ok(r) => reader = r,
    Err(e) => panic!("{}", e),
  }

  let mut prev_x = 0.0;
  let mut prev_y = 0.0;
  let mut prev_w = 0.0;

  //let mut all_keys = Vec::new();
  //let mut str_keys = HashMap::new();

  for res in reader.records() {
    let record = match res {
      Ok(r) => r,
      Err(_) => return None,
    };

    let name = record.get(0)?.to_string();
    let pressed = record.get(1)?.chars().next().unwrap_or('\0');
    let shifted = record.get(2)?.chars().next().unwrap_or('\0');

    let finger = record.get(3)?.parse::<i16>().unwrap_or(-1);
    let is_home = !record.get(4)?.is_empty();

    // Assume keys continue going right
    let x = record.get(5)?.parse::<f32>().unwrap_or(prev_x + prev_w);
    let y = record.get(6)?.parse::<f32>().unwrap_or(prev_y);
    let w = record.get(7)?.parse::<f32>().unwrap_or(1.0);
    let h = record.get(8)?.parse::<f32>().unwrap_or(1.0);

    prev_x = x;
    prev_y = y;
    prev_w = w;

    let key = Key {
      pressed,
      shifted,
      finger,
      is_home,
      pos: Pos { x, y },
      visual: VisKey {
        width: w,
        height: h,
        name,
      },
    };

    lay.keys.push(key);
  }

  for key in &lay.keys {
    lay.str_keys.insert(key.pressed, key);
    lay.str_keys.insert(key.shifted, key);
    if key.is_home && key.finger >= 0 && key.finger < 10 {
      lay.homes[key.finger as usize] = key;
    }
  }

  Some(lay)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sizes() {
    let mut lay = Layout::default();

    let lay = match init(&mut lay, "test/size.layout") {
      Some(l) => l,
      None => return,
    };

    // Check keys don't overlapping based on width
    assert_eq!(lay.str_keys.get(&'d').unwrap().pos.x, 9.5);
    assert_eq!(lay.str_keys.get(&'h').unwrap().pos.x, 7.5);

    // Check sizes
    assert_eq!(lay.str_keys.get(&'a').unwrap().visual.height, 1.0);
    assert_eq!(lay.str_keys.get(&'a').unwrap().visual.width, 1.5);
    assert_eq!(lay.str_keys.get(&'f').unwrap().visual.height, 1.0);
    assert_eq!(lay.str_keys.get(&'f').unwrap().visual.width, 3.0);
    assert_eq!(lay.str_keys.get(&'d').unwrap().visual.height, 2.0);
  }

  #[test]
  // Shifted and unshifted point to the same key
  fn test_str_keys() {
    let mut lay = Layout::default();

    let lay = match init(&mut lay, "test/str_keys.layout") {
      Some(l) => l,
      None => return,
    };

    assert!(std::ptr::eq(
      *lay.str_keys.get(&'a').unwrap(),
      *lay.str_keys.get(&'A').unwrap()
    ));
    assert!(std::ptr::eq(
      *lay.str_keys.get(&'=').unwrap(),
      *lay.str_keys.get(&'%').unwrap()
    ));
  }

  #[test]
  fn test_properties() {
    let mut lay = Layout::default();

    let lay = match init(&mut lay, "test/properties.layout") {
      Some(l) => l,
      None => return,
    };

    let c = lay.str_keys.get(&'A').unwrap();
    assert_eq!(c.visual.name, "key0");
    assert_eq!(c.pressed, 'a');
    assert_eq!(c.shifted, 'A');
    assert_eq!(c.finger, 2);
    assert!(c.is_home);
    assert_eq!(c.pos.x, 2.0);
    assert_eq!(c.pos.y, 3.0);
    assert_eq!(c.visual.width, 3.0);
    assert_eq!(c.visual.height, 2.5);
  }

  #[test]
  fn test_homes() {
    let mut lay = Layout::default();

    let lay = match init(&mut lay, "test/homes.layout") {
      Some(l) => l,
      None => return,
    };

    assert_eq!(lay.homes[0].visual.name, "a");
    assert_eq!(lay.homes[1].visual.name, "s");
    assert_eq!(lay.homes[2].visual.name, "d");
    assert_eq!(lay.homes[3].visual.name, "f");
    assert!(std::ptr::eq(lay.homes[4], &DUMMY_KEY));
    assert!(std::ptr::eq(lay.homes[5], &DUMMY_KEY));
    assert_eq!(lay.homes[6].visual.name, "j");
    assert_eq!(lay.homes[7].visual.name, "k");
    assert_eq!(lay.homes[8].visual.name, "l");
    assert_eq!(lay.homes[9].visual.name, "semicolon");
  }
}
