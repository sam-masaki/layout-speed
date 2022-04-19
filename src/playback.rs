use super::analyze;

pub struct Playhead {
  pub time: i32,
  pub idxs: [usize; 10],
}

#[cfg(test)]
mod tests {
  use super::*;
}
