//! Bus/device channel mapping (tech-spec §5.3).
//!
//! Each bus is a stereo pair mapped to two 1-based device channels on a target device.
//! Buses that share a device are opened as one multi-channel stream.

use anyhow::Result;
use audio_core::{BusConfig, BusId, ChannelMapping, DeviceId, DeviceInfo};

/// Special device id that resolves to the backend default output device.
pub const DEFAULT_DEVICE_ID: &str = "default";

/// One bus route with 0-based channel indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BusRoute {
    pub bus_id: BusId,
    pub left: usize,
    pub right: usize,
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
    Ok(ChannelMapping::new(left, right))
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
        ChannelMapping::new(1, 2),
    )
}

/// Ensure channel mappings on the same device do not overlap.
pub(crate) fn ensure_no_channel_conflicts(
    buses: &[BusConfig],
    bus_id: &BusId,
    device: &DeviceId,
    mapping: &ChannelMapping,
) -> Result<()> {
    let (left, right) = mapping.to_zero_based();
    for other in buses {
        if &other.id == bus_id || &other.device != device {
            continue;
        }
        let (other_left, other_right) = other.channels.to_zero_based();
        let overlaps = left == other_left
            || left == other_right
            || right == other_left
            || right == other_right;
        if overlaps {
            return Err(anyhow::anyhow!(
                "Channel mapping [{}, {}] for bus '{}' overlaps bus '{}' on device '{}'",
                mapping.left,
                mapping.right,
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
    let highest = mapping.left.max(mapping.right);
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
        let mapping = validate_channel_pair([bus.channels.left, bus.channels.right])?;
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
                        ChannelMapping::new((r.left + 1) as u16, (r.right + 1) as u16),
                    )
                })
            })
            .collect();
        ensure_no_channel_conflicts(&existing_on_device, &bus.id, &device, &mapping)?;

        let (left, right) = mapping.to_zero_based();
        let needed = (left.max(right) + 1) as u16;

        if let Some(plan) = plans.iter_mut().find(|p| p.device == device) {
            if plan.routes.iter().any(|r| r.bus_id == bus.id) {
                return Err(anyhow::anyhow!(
                    "Duplicate bus id in configuration: {}",
                    bus.id.as_str()
                ));
            }
            plan.channels = plan.channels.max(needed);
            plan.routes.push(BusRoute {
                bus_id: bus.id.clone(),
                left,
                right,
            });
        } else {
            plans.push(DeviceStreamPlan {
                device,
                channels: needed,
                routes: vec![BusRoute {
                    bus_id: bus.id.clone(),
                    left,
                    right,
                }],
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
            if src + 1 >= bus_buf.len() || dst + route.left.max(route.right) >= device_out.len() {
                break;
            }
            device_out[dst + route.left] = bus_buf[src];
            device_out[dst + route.right] = bus_buf[src + 1];
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
        assert_eq!((plans[0].routes[0].left, plans[0].routes[0].right), (0, 1));
    }

    #[test]
    fn resolves_default_device_alias() {
        let buses = vec![BusConfig::new(
            BusId::new("master"),
            "Master".to_string(),
            DeviceId::new(DEFAULT_DEVICE_ID),
            ChannelMapping::new(1, 2),
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
                ChannelMapping::new(1, 2),
            ),
            BusConfig::new(
                BusId::new("master"),
                "Master".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::new(3, 4),
            ),
        ];
        let plans = resolve_device_stream_plans(&buses, &null_devices()).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].channels, 4);
        assert_eq!(plans[0].routes.len(), 2);
        // Master-hosting plan is sorted first; only one plan here.
        assert!(plans[0]
            .routes
            .iter()
            .any(|r| r.bus_id.as_str() == "master" && r.left == 2 && r.right == 3));
    }

    #[test]
    fn rejects_overlapping_channels() {
        let buses = vec![
            BusConfig::new(
                BusId::new("master"),
                "Master".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::new(1, 2),
            ),
            BusConfig::new(
                BusId::new("cue"),
                "Cue".to_string(),
                DeviceId::new("null-device"),
                ChannelMapping::new(2, 3),
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
            ChannelMapping::new(7, 9),
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
                right: 1,
            },
            BusRoute {
                bus_id: BusId::new("master"),
                left: 2,
                right: 3,
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
    fn validate_channel_pair_rejects_zero_and_duplicates() {
        assert!(validate_channel_pair([0, 1]).is_err());
        assert!(validate_channel_pair([1, 1]).is_err());
        assert!(validate_channel_pair([1, 2]).is_ok());
    }
}
