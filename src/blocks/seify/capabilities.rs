use crate::runtime::Error;
use crate::runtime::Result;
use futuresdr_types::Pmt;
use seify::Device;
use seify::DeviceTrait;
use seify::Direction;
use seify::Range;
use std::collections::HashMap;

/// Record describing the reported capabilities of a seify [`Device`].
#[derive(Debug, Clone)]
pub struct Capabilities {
    /// Frequency range supported by the device.
    pub frequency_range: Option<Range>,
    /// Sample rate range supported by the device.
    pub sample_rate_range: Option<Range>,
    /// Bandwidth range supported by the device.
    pub bandwidth_range: Option<Range>,
    /// Antennas identified by the device.
    pub antennas: Option<Vec<String>>,
    /// General gain ranges supported by the device.
    pub gain_range: Option<Range>,
    /// Whether the device supports automatic gain control.
    pub supports_agc: Option<bool>,
    // TODO: Frequency components, gain elements, etc.
}

impl Capabilities {
    /// Extracts a [`Capabilities`] from a [`Device`], [`Direction`], and channel id.
    pub fn try_from<D: DeviceTrait + Clone>(
        dev: &Device<D>,
        dir: Direction,
        channel: usize,
    ) -> Result<Self, Error> {
        let inner = dev.impl_ref::<D>()?;
        Ok(Capabilities {
            frequency_range: inner.frequency_range(dir, channel).ok(),
            sample_rate_range: inner.get_sample_rate_range(dir, channel).ok(),
            bandwidth_range: inner.get_bandwidth_range(dir, channel).ok(),
            antennas: inner.antennas(dir, channel).ok(),
            gain_range: inner.gain_range(dir, channel).ok(),
            supports_agc: inner.supports_agc(dir, channel).ok(),
        })
    }
}

impl From<&Capabilities> for Pmt {
    fn from(value: &Capabilities) -> Self {
        let mut m = HashMap::new();

        if let Some(r) = &value.frequency_range {
            m.insert("frequency_range".to_owned(), r.into());
        }
        if let Some(r) = &value.sample_rate_range {
            m.insert("sample_rate_range".to_owned(), r.into());
        }
        if let Some(r) = &value.bandwidth_range {
            m.insert("bandwidth_range".to_owned(), r.into());
        }
        if let Some(v) = &value.antennas {
            m.insert(
                "antennas".to_owned(),
                Pmt::VecPmt(v.iter().map(|v| Pmt::String(v.to_string())).collect()),
            );
        }
        if let Some(r) = &value.gain_range {
            m.insert("gain_range".to_owned(), r.into());
        }
        if let Some(v) = &value.supports_agc {
            m.insert("supports_agc".to_owned(), Pmt::Bool(*v));
        }

        Pmt::MapStrPmt(m)
    }
}

impl TryFrom<&Pmt> for Capabilities {
    type Error = anyhow::Error;

    fn try_from(value: &Pmt) -> Result<Self> {
        match value {
            Pmt::MapStrPmt(m) => {
                let frequency_range = m.get("frequency_range").and_then(|v| v.try_into().ok());
                let sample_rate_range = m.get("sample_rate_range").and_then(|v| v.try_into().ok());
                let bandwidth_range = m.get("bandwidth_range").and_then(|v| v.try_into().ok());
                let antennas = m.get("antennas").and_then(|v| {
                    if let Pmt::VecPmt(v) = v {
                        Some(
                            v.iter()
                                .map(|v| {
                                    if let Pmt::String(s) = v {
                                        Ok(s.to_string())
                                    } else {
                                        Err(anyhow::Error::msg("unexpected pmt type"))
                                    }
                                })
                                .collect::<Result<Vec<String>>>()
                                .ok()?,
                        )
                    } else {
                        None
                    }
                });
                let gain_range = m.get("gain_range").and_then(|v| v.try_into().ok());
                let supports_agc = m.get("supports_agc").and_then(|v| {
                    if let Pmt::Bool(v) = v {
                        Some(*v)
                    } else {
                        None
                    }
                });

                Ok(Capabilities {
                    frequency_range,
                    sample_rate_range,
                    bandwidth_range,
                    antennas,
                    gain_range,
                    supports_agc,
                })
            }
            o => Err(Error::PmtValueError(format!("unexpected Pmt value: {o:?}")).into()),
        }
    }
}

impl TryFrom<Pmt> for Capabilities {
    type Error = anyhow::Error;
    fn try_from(value: Pmt) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}
