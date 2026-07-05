//! SeaORM entity definitions for the library database.

pub mod collection_tracks;
pub mod collections;
pub mod track_analysis;
pub mod tracks;

pub use collection_tracks::Entity as CollectionTrackEntity;
pub use collections::Entity as CollectionEntity;
pub use track_analysis::Entity as TrackAnalysisEntity;
pub use tracks::Entity as TrackEntity;
