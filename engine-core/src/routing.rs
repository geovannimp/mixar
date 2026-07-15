//! Bus/device channel mapping (tech-spec §5.3).
//!
//! Each bus maps to either a stereo pair or a single mono device channel (1-based).
//! Buses that share a device are opened as one multi-channel stream.
//! Mono routes fold stereo bus L+R with a 0.5 scale into one device channel.

use anyhow::Result;
use audio_core::{BusConfig, BusId, ChannelMapping, ChannelMode, DeviceId, DeviceInfo};

/// Special device id that resolves to the backend default output device.
pub const DEFAULT_DEVICE_ID: &str = "default";

/// One bus route with 0-based channel indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BusRoute {
    pub bus_id: BusId,
    /// Primary channel (stereo left, or mono target).
    pub left: usize,
    /// Stereo right channel; `None` means mono fold onto `left`.
    pub right: Option<usize>,
}

impl BusRoute {
    fn from_mapping(bus_id: BusId, mapping: &ChannelMapping) -> Self {
        match mapping.mode {
            ChannelMode::Mono => Self {
                bus_id,
                left: (mapping.left.saturating_sub(1)) as usize,
                right: None,
            },
            ChannelMode::Stereo => {
                let (left, right) = mapping.to_zero_based();
                Self {
                    bus_id,
                    left,
                    right: Some(right),
                }
            }
        }
    }

    fn to_channel_mapping(&self) -> ChannelMapping {
        match self.right {
            None => ChannelMapping::mono((self.left + 1) as u16),
            Some(right) => ChannelMapping::stereo((self.left + 1) as u16, (right + 1) as u16),
        }
    }

    fn highest_zero_based(&self) -> usize {
        match self.right {
            Some(right) => self.left.max(right),
            None => self.left,
        }
    }
}

/// Plan for one output device stream and the buses written into it.
#[derive(Clone, Debug)]
pub(crate) struct DeviceStreamPlan {
    pub device: DeviceId,
    /// Minimum channel count required (max mapped index + 1).
    pub channels: u16,
    pub routes: Vec<BusRoute>,
}

/// Validate a stereo channel pair (1-based, distinct, non-zero).
pub(crate) fn validate_channel_pair(channels: [u16; 2]) -> Result<ChannelMapping> {
    let [left, right] = channels;
    validate_channel_mapping(&ChannelMapping::stereo(left, right))
}

/// Validate a stereo or mono channel mapping.
pub(crate) fn validate_channel_mapping(mapping: &ChannelMapping) -> Result<ChannelMapping> {
    match mapping.mode {
        ChannelMode::Mono => {
            if mapping.left == 0 {
                return Err(anyhow::anyhow!(
                    "Channel indexes are 1-based; got mono channel=0"
                ));
            }
            Ok(ChannelMapping::mono(mapping.left))
        }
        ChannelMode::Stereo => {
            let left = mapping.left;
            let right = mapping.right;
            if left == 0 || right == 0 {
                return Err(anyhow::anyhow!(
                    "Channel indexes are 1-based; got left={}, right={}",
                    left,
                    right
                ));
            }
            if left == right {
                return Err(anyhow::anyhow!(
                    "Left and right channels must be distinct; both are {}",
                    left
                ));
            }
            Ok(ChannelMapping::stereo(left, right))
        }
    }
}

/// Resolve `"default"` (or unknown aliases) to the backend default device id.
pub(crate) fn resolve_device_id(
    device_id: &DeviceId,
    devices: &[DeviceInfo],
) -> Result<DeviceId> {
    if device_id.as_str() != DEFAULT_DEVICE_ID {
        if devices.iter().any(|d| d.id == *device_id) {
            return Ok(device_id.clone());
        }
        return Err(anyhow::anyhow!(
            "Output device not found: {}",
            device_id.as_str()
        ));
    }

    devices
        .iter()
        .find(|d| d.is_default)
        .or_else(|| devices.first())
        .map(|d| d.id.clone())
        .ok_or_else(|| anyhow::anyhow!("No output device available"))
}

fn device_info<'a>(devices: &'a [DeviceInfo], id: &DeviceId) -> Result<&'a DeviceInfo> {
    devices
        .iter()
        .find(|d| d.id == *id)
        .ok_or_else(|| anyhow::anyhow!("Output device not found: {}", id.as_str()))
}

/// Build default master bus on the given device (channels 1–2).
pub(crate) fn default_master_bus(device: DeviceId) -> BusConfig {
    BusConfig::new(
        BusId::new("master"),
        "Master".to_string(),
        device,
        ChannelMapping::stereo(1, 2),
    )
}

/// Ensure channel mappings on the same device do not overlap.
pub(crate) fn ensure_no_channel_conflicts(
    buses: &[BusConfig],
    bus_id: &BusId,
    device: &DeviceId,
    mapping: &ChannelMapping,
) -> Result<()> {
    let occupied = mapping.occupied_zero_based();
    for other in buses {
        if &other.id == bus_id || &other.device != device {
            continue;
        }
        let other_occupied = other.channels.occupied_zero_based();
        let overlaps = occupied.iter().any(|ch| other_occupied.contains(ch));
        if overlaps {
            return Err(anyhow::anyhow!(
                "Channel mapping {:?} for bus '{}' overlaps bus '{}' on device '{}'",
                mapping.occupied_zero_based()
                    .iter()
                    .map(|c| c + 1)
                    .collect::<Vec<_>>(),
                bus_id.as_str(),
                other.id.as_str(),
                device.as_str()
            ));
        }
    }
    Ok(())
}

/// Validate that a mapping fits within a device's channel count.
pub(crate) fn ensure_channels_in_range(
    mapping: &ChannelMapping,
    max_channels: u16,
    device: &DeviceId,
) -> Result<()> {
    let highest = mapping.highest_channel();
    if highest > max_channels {
        return Err(anyhow::anyhow!(
            "Channel {} is out of range for device '{}' (max_channels={})",
            highest,
            device.as_str(),
            max_channels
        ));
    }
    Ok(())
}

/// Resolve configured buses into one stream plan per unique device.
///
/// When `buses` is empty, a master bus on the default device (channels 1–2) is used.
pub(crate) fn resolve_device_stream_plans(
    buses: &[BusConfig],
    devices: &[DeviceInfo],
) -> Result<Vec<DeviceStreamPlan>> {
    let default_device = devices
        .iter()
        .find(|d| d.is_default)
        .or_else(|| devices.first())
        .ok_or_else(|| anyhow::anyhow!("No output device available"))?
        .id
        .clone();

    let effective_buses: Vec<BusConfig> = if buses.is_empty() {
        vec![default_master_bus(default_device)]
    } else {
        buses.to_vec()
    };

    let mut plans: Vec<DeviceStreamPlan> = Vec::new();

    for bus in &effective_buses {
        let mapping = validate_channel_mapping(&bus.channels)?;
        let device = resolve_device_id(&bus.device, devices)?;
        let info = device_info(devices, &device)?;
        ensure_channels_in_range(&mapping, info.max_channels, &device)?;

        // Conflict check against other buses already placed on this device.
        let existing_on_device: Vec<BusConfig> = plans
            .iter()
            .filter(|p| p.device == device)
            .flat_map(|p| {
                p.routes.iter().map(|r| {
                    BusConfig::new(
                        r.bus_id.clone(),
                        r.bus_id.as_str().to_string(),
                        device.clone(),
                        r.to_channel_mapping(),
                    )
                })
            })
            .collect();
        ensure_no_channel_conflicts(&existing_on_device, &bus.id, &device, &mapping)?;

        let route = BusRoute::from_mapping(bus.id.clone(), &mapping);
        let needed = (route.highest_zero_based() + 1) as u16;

        if let Some(plan) = plans.iter_mut().find(|p| p.device == device) {
            if plan.routes.iter().any(|r| r.bus_id == bus.id) {
                return Err(anyhow::anyhow!(
                    "Duplicate bus id in configuration: {}",
                    bus.id.as_str()
                ));
            }
            plan.channels = plan.channels.max(needed);
            plan.routes.push(route);
        } else {
            plans.push(DeviceStreamPlan {
                device,
                channels: needed,
                routes: vec![route],
            });
        }
    }

    // Prefer master-hosting device first so producer pacing uses the master clock.
    plans.sort_by(|a, b| {
        let a_master = a.routes.iter().any(|r| r.bus_id.as_str() == "master");
        let b_master = b.routes.iter().any(|r| r.bus_id.as_str() == "master");
        b_master.cmp(&a_master)
    });

    Ok(plans)
}

/// Interleave stereo bus buffers into a multi-channel device frame buffer.
///
/// Mono routes fold L+R with a 0.5 scale to avoid hard clipping.
/// `device_out` length must be `frames * channels`. Unmapped channels are silence.
pub(crate) fn map_buses_to_device_buffer(
    frames: usize,
    channels: usize,
    routes: &[BusRoute],
    output_buses: &std::collections::HashMap<BusId, Vec<audio_core::Sample>>,
    device_out: &mut [audio_core::Sample],
) {
    device_out.fill(0.0);

    for route in routes {
        let Some(bus_buf) = output_buses.get(&route.bus_id) else {
            continue;
        };
        for frame in 0..frames {
            let src = frame * 2;
            let dst = frame * channels;
            if src + 1 >= bus_buf.len() || dst + route.highest_zero_based() >= device_out.len() {
                break;
            }
            match route.right {
                Some(right) => {
                    device_out[dst + route.left] = bus_buf[src];
                    device_out[dst + right] = bus_buf[src + 1];
                }
                None => {
                    let mono = (bus_buf[src] + bus_buf[src + 1]) * 0.5;
                    device_out[dst + route.left] = mono;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn null_devices() -> Vec<DeviceInfo> {
        vec![DeviceInfo::new(
            DeviceId::new("null-device"),
            "Null Audio Device".to_string(),
            8,
            vec![48000],
            true,
        )]
    }

    #[test]
    fn default_plan_is_master_stereo_on_default_device() {
        let plans = resolve_device_stream_plans(&[], &null_devices()).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].device.as_str(), "null-device");
        assert_eq!(plans[0].channels, 2);
        assert_eq!(plans[0].routes.len(), 1);
        assert_eq!(plans[0].routes[0].bus_id.as_str(), "master");
        assert_eq!(
            (plans[0].routes[0].left, plans[0].routes[0].right),
            (0, Some(1))
        );
    }

    #[test]
    fn resolves_default_device_alias() {
        let buses = vec![BusConfig::new(
            BusId::new("master"),
            "Master".to_string(),
            DeviceId::new(DEFAULT_DEVICE_ID),
            ChannelMapping::stereo(1, 2),
        )];
        let plans = resolve_device_stream_plans(&buses, &null_devices()).unwrap();
        assert_eq!(plans[0].device.as_str(), "null-device");
    }

    #[test]
    fn multi_bus_same_device_uses_highest_channel() {
        let buses = vec![
            BusConfig::new(
                BusId::new("cue"),
                "Cue".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::stereo(1, 2),
            ),
            BusConfig::new(
                BusId::new("master"),
                "Master".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::stereo(3, 4),
            ),
        ];
        let plans = resolve_device_stream_plans(&buses, &null_devices()).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].channels, 4);
        assert_eq!(plans[0].routes.len(), 2);
        assert!(plans[0].routes.iter().any(|r| {
            r.bus_id.as_str() == "master" && r.left == 2 && r.right == Some(3)
        }));
    }

    #[test]
    fn mono_master_and_cue_on_adjacent_channels() {
        let buses = vec![
            BusConfig::new(
                BusId::new("master"),
                "Master".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::mono(1),
            ),
            BusConfig::new(
                BusId::new("cue"),
                "Preview".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::mono(2),
            ),
        ];
        let plans = resolve_device_stream_plans(&buses, &null_devices()).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].channels, 2);
        assert!(plans[0]
            .routes
            .iter()
            .any(|r| r.bus_id.as_str() == "master" && r.left == 0 && r.right.is_none()));
        assert!(plans[0]
            .routes
            .iter()
            .any(|r| r.bus_id.as_str() == "cue" && r.left == 1 && r.right.is_none()));
    }

    #[test]
    fn rejects_mono_overlap_with_stereo() {
        let buses = vec![
            BusConfig::new(
                BusId::new("master"),
                "Master".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::stereo(1, 2),
            ),
            BusConfig::new(
                BusId::new("cue"),
                "Preview".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::mono(2),
            ),
        ];
        let err = resolve_device_stream_plans(&buses, &null_devices()).unwrap_err();
        assert!(err.to_string().contains("overlaps"));
    }

    #[test]
    fn rejects_overlapping_channels() {
        let buses = vec![
            BusConfig::new(
                BusId::new("master"),
                "Master".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::stereo(1, 2),
            ),
            BusConfig::new(
                BusId::new("cue"),
                "Cue".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::stereo(2, 3),
            ),
        ];
        let err = resolve_device_stream_plans(&buses, &null_devices()).unwrap_err();
        assert!(err.to_string().contains("overlaps"));
    }

    #[test]
    fn rejects_channel_out_of_range() {
        let buses = vec![BusConfig::new(
            BusId::new("master"),
            "Master".to_string(),
            DeviceId::new("null-device"),
            ChannelMapping::stereo(7, 9),
        )];
        let err = resolve_device_stream_plans(&buses, &null_devices()).unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn map_buses_writes_stereo_pairs_into_device_channels() {
        let routes = vec![
            BusRoute {
                bus_id: BusId::new("cue"),
                left: 0,
                right: Some(1),
            },
            BusRoute {
                bus_id: BusId::new("master"),
                left: 2,
                right: Some(3),
            },
        ];
        let mut buses = HashMap::new();
        buses.insert(BusId::new("cue"), vec![0.1, 0.2, 0.3, 0.4]);
        buses.insert(BusId::new("master"), vec![0.5, 0.6, 0.7, 0.8]);

        let mut out = vec![1.0; 8];
        map_buses_to_device_buffer(2, 4, &routes, &buses, &mut out);

        assert_eq!(out, vec![0.1, 0.2, 0.5, 0.6, 0.3, 0.4, 0.7, 0.8]);
    }

    #[test]
    fn map_buses_folds_stereo_to_mono_channel() {
        let routes = vec![
            BusRoute {
                bus_id: BusId::new("master"),
                left: 0,
                right: None,
            },
            BusRoute {
                bus_id: BusId::new("cue"),
                left: 1,
                right: None,
            },
        ];
        let mut buses = HashMap::new();
        // L=1.0, R=0.0 → mono 0.5; then L=0.4, R=0.6 → mono 0.5
        buses.insert(BusId::new("master"), vec![1.0, 0.0, 0.4, 0.6]);
        buses.insert(BusId::new("cue"), vec![0.2, 0.2, 0.8, 0.0]);

        let mut out = vec![1.0; 4];
        map_buses_to_device_buffer(2, 2, &routes, &buses, &mut out);

        assert!((out[0] - 0.5).abs() < 1e-6);
        assert!((out[1] - 0.2).abs() < 1e-6);
        assert!((out[2] - 0.5).abs() < 1e-6);
        assert!((out[3] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn validate_channel_pair_rejects_zero_and_duplicates() {
        assert!(validate_channel_pair([0, 1]).is_err());
        assert!(validate_channel_pair([1, 1]).is_err());
        assert!(validate_channel_pair([1, 2]).is_ok());
    }

    #[test]
    fn validate_mono_mapping() {
        assert!(validate_channel_mapping(&ChannelMapping::mono(0)).is_err());
        assert!(validate_channel_mapping(&ChannelMapping::mono(1)).is_ok());
    }
}
