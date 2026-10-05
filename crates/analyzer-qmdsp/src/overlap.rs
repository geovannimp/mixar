//! Mixxx `DownmixAndOverlapHelper` framing, reproduced for parity.
//!
//! Windows are emitted when the buffer fills; the first window is centred (the
//! write position starts at `windowSize / 2`) and `finalize` appends enough
//! silence to complete the last window. The input is already a mono downmix.

pub fn feed(x: &[f64], window_size: usize, step_size: usize, mut callback: impl FnMut(&[f64])) {
    if window_size == 0 || step_size == 0 || step_size > window_size {
        return;
    }
    let mut buffer = vec![0.0f64; window_size];
    let mut write_pos = window_size / 2;
    let mut read = 0usize;

    while read < x.len() {
        let write_available = window_size - write_pos;
        let n = (x.len() - read).min(write_available);
        buffer[write_pos..write_pos + n].copy_from_slice(&x[read..read + n]);
        write_pos += n;
        read += n;
        if write_pos == window_size {
            callback(&buffer);
            buffer.copy_within(step_size..window_size, 0);
            write_pos -= step_size;
        }
    }

    // finalize: append max(windowSize - writePos, windowSize / 2 - 1) silence.
    let frames_to_fill = window_size - write_pos;
    let min_tail = (window_size / 2).saturating_sub(1);
    let need = frames_to_fill.max(min_tail);
    let mut done = 0usize;
    while done < need {
        let write_available = window_size - write_pos;
        let n = (need - done).min(write_available);
        for k in 0..n {
            buffer[write_pos + k] = 0.0;
        }
        write_pos += n;
        done += n;
        if write_pos == window_size {
            callback(&buffer);
            buffer.copy_within(step_size..window_size, 0);
            write_pos -= step_size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_centred_first_window_and_pads_tail() {
        let x: Vec<f64> = (0..10).map(|i| i as f64 + 1.0).collect();
        let mut windows: Vec<Vec<f64>> = Vec::new();
        feed(&x, 4, 2, |w| windows.push(w.to_vec()));
        assert!(!windows.is_empty());
        // First window: writePos starts at 2, so [0,0,1,2].
        assert_eq!(windows[0], vec![0.0, 0.0, 1.0, 2.0]);
        // Every window has the configured length.
        assert!(windows.iter().all(|w| w.len() == 4));
    }
}
