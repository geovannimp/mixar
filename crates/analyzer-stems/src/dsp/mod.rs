//! STFT/iSTFT and HTDemucs channel-as-complex (CaC) packing.
//!
//! CaC layout for `[4, F, T]`: channel 0 = left real, 1 = left imag, 2 = right real, 3 = right imag.
//! Planar index: `c * F * T + f * T + t`.

mod cac;
mod stft;

pub use cac::{cac_planar_to_complex, stft_to_cac_planar};
pub use stft::{Stft, HOP_LENGTH, N_FFT};
