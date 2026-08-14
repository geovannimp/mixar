//! FRB surface for filesystem volume listing and directory browsing.

/// Volume root for the Flutter folder picker.
#[derive(Clone, Debug)]
pub struct FsVolumeInfo {
    pub name: String,
    pub path: String,
    pub is_removable: bool,
}

/// File or directory entry in a browse listing.
#[derive(Clone, Debug)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
}

/// Directory listing split into subdirectories and audio files.
#[derive(Clone, Debug)]
pub struct FsDirectoryListing {
    pub path: String,
    pub parent: Option<String>,
    pub directories: Vec<FsEntry>,
    pub audio_files: Vec<FsEntry>,
}

/// List mount points / home suitable for starting a folder browse.
pub fn list_fs_volumes() -> Result<Vec<FsVolumeInfo>, String> {
    fs_browser::list_volumes().map(|volumes| {
        volumes
            .into_iter()
            .map(|v| FsVolumeInfo {
                name: v.name,
                path: v.path,
                is_removable: v.is_removable,
            })
            .collect()
    })
}

/// Browse one directory; returns sorted subdirs and supported audio files.
pub fn browse_fs_directory(path: String) -> Result<FsDirectoryListing, String> {
    fs_browser::browse_directory(&path).map(|listing| FsDirectoryListing {
        path: listing.path,
        parent: listing.parent,
        directories: listing
            .directories
            .into_iter()
            .map(|e| FsEntry {
                name: e.name,
                path: e.path,
            })
            .collect(),
        audio_files: listing
            .audio_files
            .into_iter()
            .map(|e| FsEntry {
                name: e.name,
                path: e.path,
            })
            .collect(),
    })
}
