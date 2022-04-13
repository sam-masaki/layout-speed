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
