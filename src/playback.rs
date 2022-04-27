use super::analyze;
use super::layout;

pub struct Playhead {
  pub time: i32,
  pub idxs: [usize; 10],
}

#[derive(Default)]
pub struct FingerData {
  pub pos: layout::Pos,
  pub pressing: bool,
}

#[derive(Default)]
pub struct PlayData {
  pub fingers: [FingerData; 10],
}

// Calculate where each finger is positioned, and its pressing state
// based on head
pub fn calc_playback(head: &Playhead, timeline: &analyze::Timeline, data: &mut PlayData) {
  // finger will be somewhere between [idx] and [idx + 1]
  for i in 0..10 {
    let prev_frame = &timeline.fingers[i][head.idxs[i]];
    if timeline.fingers[i].len() == head.idxs[i] + 1 {
      data.fingers[i].pos.x = prev_frame.pos.x;
      data.fingers[i].pos.y = prev_frame.pos.y;
      data.fingers[i].pressing = prev_frame.start_press;
      continue;
    }
    let next_frame = &timeline.fingers[i][head.idxs[i] + 1];

    // Percent of the move to next_frame we have completed
    // head.time will always be < next_frame.time so no divide by zero
    let time_ratio =
      1.0 - (((next_frame.time - head.time) as f32) / ((next_frame.time - prev_frame.time) as f32));
    let x_diff = next_frame.pos.x - prev_frame.pos.x;
    let y_diff = next_frame.pos.y - prev_frame.pos.y;

    data.fingers[i].pos.x = prev_frame.pos.x + (x_diff * time_ratio);
    data.fingers[i].pos.y = prev_frame.pos.y + (y_diff * time_ratio);
    data.fingers[i].pressing = prev_frame.start_press;
  }
}

// Given a valid Playhead, increment time by inc_ms, and update each
// head.idxs[i] to point to the most recently passed keyframe
pub fn inc_head(head: &mut Playhead, timeline: &analyze::Timeline, inc_ms: i32) {
  let new_time = head.time + inc_ms;

  for i in 0..10 {
    let mut new_frame_idx = head.idxs[i];

    // Check future frames, if they exist, until the first one that
    // has been passed by the playhead is found
    while timeline.fingers[i].len() > new_frame_idx + 1
      && timeline.fingers[i][new_frame_idx + 1].time <= new_time
    {
      new_frame_idx += 1;
    }

    head.idxs[i] = new_frame_idx;
  }

  head.time = new_time;
}

#[cfg(test)]
mod tests {
  use super::*;
}
