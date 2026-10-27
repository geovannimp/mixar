//! Bus/device channel mapping (tech-spec §5.3).
//!
//! Each bus maps to either a stereo pair or a single mono device channel (1-based).
//! Buses that share a device are opened as one multi-channel stream.
//! Mono routes fold stereo bus L+R with a 0.5 scale into one device channel.

use crate::config::DeviceConfig;
use crate::engine::Engine;
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
    /// Rate the stream is opened at. The first bus on this device with an
    /// explicit rate decides it; `None` follows the device's mix rate.
    pub sample_rate: Option<u32>,
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
pub(crate) fn resolve_device_id(device_id: &DeviceId, devices: &[DeviceInfo]) -> Result<DeviceId> {
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

pub(crate) fn device_info<'a>(devices: &'a [DeviceInfo], id: &DeviceId) -> Result<&'a DeviceInfo> {
    devices
        .iter()
        .find(|d| d.id == *id)
        .ok_or_else(|| anyhow::anyhow!("Output device not found: {}", id.as_str()))
}

/// Rate a device stream opens at: the buses' explicit rate when one is set,
/// otherwise the rate the system mixes this device at.
pub(crate) fn resolve_plan_rate(plan: &DeviceStreamPlan, devices: &[DeviceInfo]) -> Result<u32> {
    if let Some(rate) = plan.sample_rate {
        return Ok(rate);
    }
    device_info(devices, &plan.device)?
        .mix_sample_rate
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Device '{}' does not report the rate it mixes at; set an explicit rate for its bus in Settings → Audio",
                plan.device.as_str()
            )
        })
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
                mapping
                    .occupied_zero_based()
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
            // Buses on one device share one stream, so they share one rate. An
            // explicit rate wins over a bus that follows the device; two
            // explicit, different rates cannot both be honoured.
            match (plan.sample_rate, bus.sample_rate) {
                (Some(planned), Some(requested)) if planned != requested => {
                    let shared = plan
                        .routes
                        .iter()
                        .map(|r| r.bus_id.as_str())
                        .collect::<Vec<_>>()
                        .join("', '");
                    return Err(anyhow::anyhow!(
                        "Buses '{shared}' and '{}' share device '{}' but ask for {planned} Hz and {requested} Hz; one stream runs at one rate",
                        bus.id.as_str(),
                        device.as_str()
                    ));
                }
                (None, Some(requested)) => plan.sample_rate = Some(requested),
                _ => {}
            }
            plan.channels = plan.channels.max(needed);
            plan.routes.push(route);
        } else {
            plans.push(DeviceStreamPlan {
                device,
                channels: needed,
                sample_rate: bus.sample_rate,
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

impl Engine {
    /// List available audio devices for the engine's current backend
    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        self.backend.list_output_devices()
    }

    /// Get the default audio device (first device with `is_default`, or first in list).
    pub fn default_device(&self) -> Result<DeviceInfo> {
        let devices = self.backend.list_output_devices()?;
        devices
            .iter()
            .find(|d| d.is_default)
            .cloned()
            .or_else(|| devices.first().cloned())
            .ok_or_else(|| anyhow::anyhow!("No output device available"))
    }

    /// Get bus configuration
    pub fn get_bus_config(&self, bus_id: &BusId) -> Option<&BusConfig> {
        self.config.buses.iter().find(|bus| &bus.id == bus_id)
    }

    fn validate_bus_device_mapping(
        &self,
        bus_id: &BusId,
        device: &DeviceId,
        mapping: &ChannelMapping,
    ) -> Result<DeviceId> {
        let devices = self.backend.list_output_devices()?;
        let resolved = crate::routing::resolve_device_id(device, &devices)?;
        let info = devices
            .iter()
            .find(|d| d.id == resolved)
            .ok_or_else(|| anyhow::anyhow!("Output device not found: {}", resolved.as_str()))?;
        crate::routing::ensure_channels_in_range(mapping, info.max_channels, &resolved)?;
        crate::routing::ensure_no_channel_conflicts(
            &self.config.buses,
            bus_id,
            &resolved,
            mapping,
        )?;
        Ok(resolved)
    }

    /// Update bus configuration
    pub fn update_bus_config(&mut self, bus_id: &BusId, new_config: BusConfig) -> Result<()> {
        let mapping = crate::routing::validate_channel_mapping(&new_config.channels)?;
        let resolved = self.validate_bus_device_mapping(bus_id, &new_config.device, &mapping)?;

        if let Some(bus) = self.config.buses.iter_mut().find(|bus| &bus.id == bus_id) {
            bus.name = new_config.name;
            bus.device = resolved;
            bus.channels = mapping;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Bus not found: {}", bus_id.as_str()))
        }
    }

    /// Get device configuration
    pub fn get_device_config(&self, device_id: &DeviceId) -> Option<&DeviceConfig> {
        self.config
            .devices
            .as_ref()?
            .iter()
            .find(|device| device.id == device_id.as_str())
    }

    /// Update device configuration
    pub fn update_device_config(
        &mut self,
        device_id: &DeviceId,
        new_config: DeviceConfig,
    ) -> Result<()> {
        if let Some(devices) = self.config.devices.as_mut() {
            if let Some(device) = devices
                .iter_mut()
                .find(|device| device.id == device_id.as_str())
            {
                *device = new_config;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Device not found: {}", device_id.as_str()))
            }
        } else {
            Err(anyhow::anyhow!("No devices configured"))
        }
    }

    /// Set bus device mapping to a stereo channel pair (1-based).
    pub fn set_bus_device(
        &mut self,
        bus: BusId,
        device: DeviceId,
        channels: [u16; 2],
    ) -> Result<()> {
        let mapping = crate::routing::validate_channel_pair(channels)?;
        self.set_bus_channel_mapping(bus, device, mapping)
    }

    /// Set bus device mapping (stereo pair or mono fold).
    pub fn set_bus_channel_mapping(
        &mut self,
        bus: BusId,
        device: DeviceId,
        channels: ChannelMapping,
    ) -> Result<()> {
        let mapping = crate::routing::validate_channel_mapping(&channels)?;
        let resolved = self.validate_bus_device_mapping(&bus, &device, &mapping)?;

        if let Some(existing) = self.config.buses.iter_mut().find(|b| b.id == bus) {
            existing.device = resolved;
            existing.channels = mapping;
        } else {
            let name = match bus.as_str() {
                "master" => "Master".to_string(),
                "cue" => "Preview".to_string(),
                other => other.to_string(),
            };
            self.config
                .buses
                .push(BusConfig::new(bus, name, resolved, mapping));
        }
        Ok(())
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
        assert!(plans[0]
            .routes
            .iter()
            .any(|r| { r.bus_id.as_str() == "master" && r.left == 2 && r.right == Some(3) }));
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

    /// Two devices mixing at different rates: the case per-bus rates exist for.
    fn two_devices() -> Vec<DeviceInfo> {
        vec![
            DeviceInfo::new(
                DeviceId::new("speakers"),
                "Speakers".to_string(),
                4,
                vec![44_100, 48_000],
                true,
            )
            .with_mix_sample_rate(Some(48_000)),
            DeviceInfo::new(
                DeviceId::new("headphones"),
                "Headphones".to_string(),
                2,
                vec![44_100, 48_000],
                false,
            )
            .with_mix_sample_rate(Some(44_100)),
        ]
    }

    fn bus_on(id: &str, name: &str, device: &str) -> BusConfig {
        BusConfig::new(
            BusId::new(id),
            name.to_string(),
            DeviceId::new(device),
            ChannelMapping::stereo(1, 2),
        )
    }

    #[test]
    fn unset_bus_rate_follows_each_device_mix_rate() {
        let devices = two_devices();
        let buses = vec![
            bus_on("master", "Master", "speakers"),
            bus_on("cue", "Cue", "headphones"),
        ];
        let plans = resolve_device_stream_plans(&buses, &devices).unwrap();

        let master = plans
            .iter()
            .find(|plan| plan.device.as_str() == "speakers")
            .expect("master plan");
        let cue = plans
            .iter()
            .find(|plan| plan.device.as_str() == "headphones")
            .expect("cue plan");

        assert_eq!(master.sample_rate, None);
        assert_eq!(resolve_plan_rate(master, &devices).unwrap(), 48_000);
        assert_eq!(resolve_plan_rate(cue, &devices).unwrap(), 44_100);
    }

    #[test]
    fn explicit_bus_rate_overrides_the_device_mix_rate() {
        let devices = two_devices();
        let buses = vec![bus_on("master", "Master", "headphones").with_sample_rate(Some(96_000))];
        let plans = resolve_device_stream_plans(&buses, &devices).unwrap();

        assert_eq!(plans[0].sample_rate, Some(96_000));
        assert_eq!(resolve_plan_rate(&plans[0], &devices).unwrap(), 96_000);
    }

    #[test]
    fn buses_sharing_a_device_must_agree_on_rate() {
        let devices = two_devices();
        let mut buses = vec![
            bus_on("master", "Master", "speakers").with_sample_rate(Some(48_000)),
            bus_on("cue", "Cue", "speakers").with_sample_rate(Some(44_100)),
        ];
        // Distinct channel pairs so the rate check, not the overlap check, decides.
        buses[1].channels = ChannelMapping::stereo(3, 4);
        let err = resolve_device_stream_plans(&buses, &devices).unwrap_err();
        assert!(
            err.to_string().contains("one stream runs at one rate"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn unset_rate_without_a_reported_mix_rate_is_an_error() {
        let devices = vec![DeviceInfo::new(
            DeviceId::new("mute"),
            "Mute".to_string(),
            2,
            vec![48_000],
            true,
        )];
        let buses = vec![bus_on("master", "Master", "mute")];
        let plans = resolve_device_stream_plans(&buses, &devices).unwrap();

        let err = resolve_plan_rate(&plans[0], &devices).unwrap_err();
        assert!(
            err.to_string()
                .contains("does not report the rate it mixes at"),
            "unexpected error: {err}"
        );
    }
}
