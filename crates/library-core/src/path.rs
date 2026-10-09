//! Filesystem path helpers shared by library backends.

use std::path::Path;

/// Whether `path` lies inside `folder`.
///
/// Comparison is component-wise, so `/a/b` is inside `/a` but `/a/bc` is not.
/// A path equal to `folder` counts as inside.
///
/// This is a raw, unnormalized comparison: neither argument is canonicalized,
/// so `..`/`.` components are treated literally — `/a/b/../c` *does* lexically
/// match the prefix `/a/b` even though it resolves to `/a/c`, so an unnormalized
/// path can incorrectly register as inside `folder`. Callers deciding folder
/// membership must pass already normalized absolute paths. An empty `folder`
/// never matches.
pub fn path_under_folder(path: &Path, folder: &Path) -> bool {
    !folder.as_os_str().is_empty() && path.starts_with(folder)
}
