//! Load and validate a mapping bundle directory.

use std::path::{Path, PathBuf};

use crate::device::DeviceFile;
use crate::error::LoadError;
use crate::map_file::MapFile;

/// Loaded mapping directory: device + map + optional script.
#[derive(Clone, Debug)]
pub struct MappingBundle {
    pub device: DeviceFile,
    pub map: MapFile,
    pub script_source: Option<String>,
    pub root: PathBuf,
}

pub fn load_bundle(dir: &Path) -> Result<MappingBundle, LoadError> {
    let device_path = dir.join("device.toml");
    let map_path = dir.join("map.toml");
    if !device_path.is_file() {
        return Err(LoadError::MissingFile(device_path));
    }
    if !map_path.is_file() {
        return Err(LoadError::MissingFile(map_path));
    }

    let device_text = std::fs::read_to_string(&device_path).map_err(|source| LoadError::Io {
        path: device_path.clone(),
        source,
    })?;
    let map_text = std::fs::read_to_string(&map_path).map_err(|source| LoadError::Io {
        path: map_path.clone(),
        source,
    })?;

    let device = DeviceFile::parse(&device_text, &device_path)?;
    let map = MapFile::parse(&map_text, &map_path)?;

    let script_path = dir.join("script.rhai");
    let script_source = if script_path.is_file() {
        Some(
            std::fs::read_to_string(&script_path).map_err(|source| LoadError::Io {
                path: script_path,
                source,
            })?,
        )
    } else {
        None
    };

    map.validate_against(&device, script_source.is_some())?;

    Ok(MappingBundle {
        device,
        map,
        script_source,
        root: dir.to_path_buf(),
    })
}
