//! Small conversions shared by the Mixxx reader and migration.

use std::path::Path;

/// Whether `path` lies inside `folder` (component-wise, so `/a/b` is not inside
/// `/a/bc`).
pub(crate) fn path_under_folder(path: &Path, folder: &Path) -> bool {
    path.starts_with(folder)
}
