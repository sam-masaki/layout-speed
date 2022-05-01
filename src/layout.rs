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

pub struct Combo<'a> {
  pub key: &'a Key,
  pub mods: Option<Vec<&'a Key>>,
}

pub struct Layout<'a> {
  pub keys: Vec<Key>, // Stores text-inputting keys
  pub char_keys: HashMap<char, Combo<'a>>,
  pub homes: [&'a Key; 10],
  pub mod_map: HashMap<String, Key>, // Stores modifiers
}

impl<'a> Default for Layout<'a> {
  fn default() -> Self {
    Self {
      keys: Vec::new(),
      char_keys: HashMap::new(),
      homes: [
        &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY,
        &DUMMY_KEY, &DUMMY_KEY, &DUMMY_KEY,
      ],
      mod_map: HashMap::new(),
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

    if key.visual.name == "lshift" {
      lay.mod_map.insert("lshift".to_string(), key);
    } else if key.visual.name == "rshift" {
      lay.mod_map.insert("rshift".to_string(), key);
    } else {
      lay.keys.push(key);
    }
  }

  let lshift = match lay.mod_map.get("lshift") {
    Some(s) => s,
    None => &DUMMY_KEY,
  };
  let rshift = match lay.mod_map.get("rshift") {
    Some(s) => s,
    None => &DUMMY_KEY,
  };

  for key in &lay.keys {
    if key.pressed != '\0' {
      lay.char_keys.insert(key.pressed, Combo { key, mods: None });
    }
    if key.shifted != '\0' {
      let mut mods = Vec::new();
      if key.finger < 5 {
        mods.push(rshift);
      } else {
        mods.push(lshift);
      }

      lay.char_keys.insert(
        key.shifted,
        Combo {
          key,
          mods: Some(mods),
        },
      );
    }

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

    // Check keys aren't overlapping based on width
    assert_eq!(lay.char_keys.get(&'d').unwrap().key.pos.x, 9.5);
    assert_eq!(lay.char_keys.get(&'h').unwrap().key.pos.x, 7.5);

    // Check sizes
    assert_eq!(lay.char_keys.get(&'a').unwrap().key.visual.height, 1.0);
    assert_eq!(lay.char_keys.get(&'a').unwrap().key.visual.width, 1.5);
    assert_eq!(lay.char_keys.get(&'f').unwrap().key.visual.height, 1.0);
    assert_eq!(lay.char_keys.get(&'f').unwrap().key.visual.width, 3.0);
    assert_eq!(lay.char_keys.get(&'d').unwrap().key.visual.height, 2.0);
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
      lay.char_keys.get(&'a').unwrap().key,
      lay.char_keys.get(&'A').unwrap().key
    ));
    assert!(std::ptr::eq(
      lay.char_keys.get(&'=').unwrap().key,
      lay.char_keys.get(&'%').unwrap().key
    ));
  }

  #[test]
  // Shifted chars get assigned correctly
  fn test_shifts() {
    let mut lay = Layout::default();

    let lay = match init(&mut lay, "test/shifts.layout") {
      Some(l) => l,
      None => return,
    };

    assert!(lay.char_keys.get(&'a').unwrap().mods.is_none());
    assert!(lay.char_keys.get(&'z').unwrap().mods.is_none());

    let a_shift = lay.char_keys.get(&'A').unwrap();
    let z_shift = lay.char_keys.get(&'Z').unwrap();

    let a_mods = a_shift.mods.as_ref().unwrap();
    let z_mods = z_shift.mods.as_ref().unwrap();

    assert_eq!(a_mods.len(), 1);
    assert_eq!(z_mods.len(), 1);

    assert_eq!(a_mods.last().unwrap().visual.name, "rshift");
    assert_eq!(z_mods.last().unwrap().visual.name, "lshift");
  }

  #[test]
  fn test_properties() {
    let mut lay = Layout::default();

    let lay = match init(&mut lay, "test/properties.layout") {
      Some(l) => l,
      None => return,
    };

    let c = lay.char_keys.get(&'a').unwrap();
    assert_eq!(c.key.visual.name, "key0");
    assert_eq!(c.key.pressed, 'a');
    assert_eq!(c.key.shifted, 'A');
    assert_eq!(c.key.finger, 2);
    assert!(c.key.is_home);
    assert_eq!(c.key.pos.x, 2.0);
    assert_eq!(c.key.pos.y, 3.0);
    assert_eq!(c.key.visual.width, 3.0);
    assert_eq!(c.key.visual.height, 2.5);
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
