//! Filesystem path helpers shared by library backends.

use std::path::Path;

/// Whether `path` lies inside `folder`.
///
/// Comparison is component-wise, so `/a/b` is inside `/a` but `/a/bc` is not.
/// A path equal to `folder` counts as inside.
pub fn path_under_folder(path: &Path, folder: &Path) -> bool {
    path.starts_with(folder)
}
