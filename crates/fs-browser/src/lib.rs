use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use library_core::is_supported_audio_path;

#[derive(Debug, Clone, Serialize)]
pub struct VolumeInfo {
    pub name: String,
    pub path: String,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryListing {
    pub path: String,
    pub parent: Option<String>,
    pub directories: Vec<FsEntry>,
    pub audio_files: Vec<FsEntry>,
}

fn is_audio_file(path: &Path) -> bool {
    is_supported_audio_path(path)
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn canonicalize_if_exists(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn volume_display_name(path: &str, removable: bool) -> String {
    if path == "/" {
        return "System".to_string();
    }
    if let Some(name) = Path::new(path).file_name().and_then(|n| n.to_str()) {
        if !name.is_empty() {
            if removable {
                return name.to_string();
            }
            return name.to_string();
        }
    }
    path.to_string()
}

pub fn list_volumes() -> Result<Vec<VolumeInfo>, String> {
    let mut volumes = Vec::new();
    let mut seen = HashSet::new();

    #[cfg(target_os = "linux")]
    {
        volumes.extend(list_linux_volumes(&mut seen)?);
    }

    #[cfg(target_os = "macos")]
    {
        volumes.extend(list_macos_volumes(&mut seen)?);
    }

    #[cfg(target_os = "windows")]
    {
        volumes.extend(list_windows_volumes(&mut seen)?);
    }

    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let home_path = PathBuf::from(&home);
        if home_path.is_dir() {
            let path = canonicalize_if_exists(&home_path)
                .to_string_lossy()
                .into_owned();
            if seen.insert(path.clone()) {
                volumes.insert(
                    0,
                    VolumeInfo {
                        name: "Home".to_string(),
                        path,
                        is_removable: false,
                    },
                );
            }
        }
    }

    Ok(volumes)
}

#[cfg(target_os = "linux")]
fn list_linux_volumes(seen: &mut HashSet<String>) -> Result<Vec<VolumeInfo>, String> {
    let content = std::fs::read_to_string("/proc/mounts")
        .map_err(|e| format!("Failed to read /proc/mounts: {e}"))?;

    let interesting_fstypes = [
        "ext4", "ext3", "ext2", "btrfs", "xfs", "vfat", "exfat", "ntfs", "fuseblk", "fuse", "cifs",
        "nfs", "iso9660", "udf",
    ];

    let skip_prefixes = [
        "/proc",
        "/sys",
        "/dev",
        "/run/user",
        "/snap",
        "/var/lib",
        "/tmp",
        "/boot/efi",
        "/run",
    ];

    let mut mounts = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let device = parts[0];
        let mount_point = parts[1].replace("\\040", " ");
        let fstype = parts[2];

        if !interesting_fstypes
            .iter()
            .any(|candidate| fstype.starts_with(candidate))
        {
            continue;
        }
        if skip_prefixes
            .iter()
            .any(|prefix| mount_point.starts_with(prefix))
        {
            continue;
        }

        let removable = is_linux_removable_mount(&mount_point, device);
        mounts.push((mount_point, removable));
    }

    mounts.sort_by(|a, b| {
        let a_root = a.0 == "/";
        let b_root = b.0 == "/";
        match (a_root, b_root) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            },
        }
    });

    let mut volumes = Vec::new();
    for (mount_point, removable) in mounts {
        let path = canonicalize_if_exists(Path::new(&mount_point));
        if !path.is_dir() {
            continue;
        }
        let path_str = path.to_string_lossy().into_owned();
        if !seen.insert(path_str.clone()) {
            continue;
        }
        volumes.push(VolumeInfo {
            name: volume_display_name(&path_str, removable),
            path: path_str,
            is_removable: removable,
        });
    }

    Ok(volumes)
}

#[cfg(target_os = "linux")]
fn is_linux_removable_mount(mount_point: &str, device: &str) -> bool {
    if mount_point.starts_with("/media/")
        || mount_point.starts_with("/run/media/")
        || mount_point.starts_with("/mnt/")
    {
        return true;
    }

    let dev_name = device.strip_prefix("/dev/").unwrap_or(device);
    let base = dev_name.trim_end_matches(|c: char| c.is_ascii_digit());
    let sysfs = format!("/sys/block/{base}/removable");
    std::fs::read_to_string(&sysfs)
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn list_macos_volumes(seen: &mut HashSet<String>) -> Result<Vec<VolumeInfo>, String> {
    let mut volumes = Vec::new();
    let root = Path::new("/Volumes");
    if !root.is_dir() {
        return Ok(volumes);
    }

    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() {
            continue;
        }
        let path = canonicalize_if_exists(&entry.path());
        let path_str = path.to_string_lossy().into_owned();
        if !seen.insert(path_str.clone()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        volumes.push(VolumeInfo {
            name,
            path: path_str,
            is_removable: true,
        });
    }

    volumes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(volumes)
}

#[cfg(target_os = "windows")]
fn list_windows_volumes(seen: &mut HashSet<String>) -> Result<Vec<VolumeInfo>, String> {
    let mut volumes = Vec::new();

    for letter in b'A'..=b'Z' {
        let path = format!("{}:\\", letter as char);
        let root = Path::new(&path);
        if !root.is_dir() {
            continue;
        }
        let path_str = canonicalize_if_exists(root).to_string_lossy().into_owned();
        if !seen.insert(path_str.clone()) {
            continue;
        }
        volumes.push(VolumeInfo {
            name: format!("{}:", letter as char),
            path: path_str,
            is_removable: false,
        });
    }

    Ok(volumes)
}

pub fn browse_directory(path: &str) -> Result<DirectoryListing, String> {
    let path = Path::new(path);
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }

    let canonical = canonicalize_if_exists(path);
    let path_str = canonical.to_string_lossy().into_owned();

    let parent = canonical
        .parent()
        .filter(|parent| parent != &canonical)
        .map(|parent| {
            canonicalize_if_exists(parent)
                .to_string_lossy()
                .into_owned()
        });

    let mut directories = Vec::new();
    let mut audio_files = Vec::new();

    let entries = std::fs::read_dir(&canonical)
        .map_err(|e| format!("Failed to read directory {}: {e}", canonical.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if is_hidden(&file_name) {
            continue;
        }
        // DirEntry::file_type does not follow symlinks (Path::is_dir can block on FUSE/NFS).
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let entry_path = entry.path();
        let entry_str = entry_path.to_string_lossy().into_owned();

        if file_type.is_dir() {
            directories.push(FsEntry {
                name: file_name,
                path: entry_str,
            });
        } else if file_type.is_file() && is_audio_file(&entry_path) {
            audio_files.push(FsEntry {
                name: file_name,
                path: entry_str,
            });
        }
    }

    directories.sort_by_key(|a| a.name.to_lowercase());
    audio_files.sort_by_key(|a| a.name.to_lowercase());

    Ok(DirectoryListing {
        path: path_str,
        parent,
        directories,
        audio_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn browse_directory_splits_dirs_and_audio_files() {
        let root = tempdir().expect("temp dir");
        let nested = root.path().join("nested");
        fs::create_dir(&nested).expect("nested dir");
        fs::write(root.path().join("track.wav"), b"RIFF").expect("wav file");
        fs::write(root.path().join("readme.txt"), b"notes").expect("text file");

        let listing = browse_directory(root.path().to_str().unwrap()).expect("browse temp dir");

        assert_eq!(listing.directories.len(), 1);
        assert_eq!(listing.directories[0].name, "nested");
        assert_eq!(listing.audio_files.len(), 1);
        assert_eq!(listing.audio_files[0].name, "track.wav");
    }

    #[cfg(unix)]
    #[test]
    fn browse_does_not_follow_dangling_symlink() {
        let root = tempdir().expect("temp dir");
        std::os::unix::fs::symlink("/no/such/fs-browser-target", root.path().join("broken"))
            .expect("symlink");
        let listing = browse_directory(root.path().to_str().unwrap()).expect("browse");
        assert!(
            listing.directories.iter().all(|e| e.name != "broken"),
            "dangling symlink must not be stat'd as a directory"
        );
    }
}
