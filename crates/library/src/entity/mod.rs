//! SeaORM entity definitions for the library database.

pub mod collection_tracks;
pub mod collections;
pub mod sampler_bank;
pub mod sampler_slot;
pub mod track_analysis;
pub mod track_hot_cue;
pub mod track_loop;
pub mod track_waveform;
pub mod tracks;

pub use collection_tracks::Entity as CollectionTrackEntity;
pub use collections::Entity as CollectionEntity;
pub use sampler_bank::{Entity as SamplerBankEntity, SamplerPlayMode};
pub use sampler_slot::Entity as SamplerSlotEntity;
pub use track_analysis::Entity as TrackAnalysisEntity;
pub use track_hot_cue::Entity as TrackHotCueEntity;
pub use track_loop::Entity as TrackLoopEntity;
pub use track_waveform::Entity as TrackWaveformEntity;
pub use tracks::Entity as TrackEntity;
