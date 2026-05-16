use crate::{AvError, AvResult};

pub const AV_NOPTS_VALUE: i64 = i64::MIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacketFlags {
    bits: u32,
}

impl PacketFlags {
    pub const KEY: Self = Self { bits: 0x0001 };
    pub const CORRUPT: Self = Self { bits: 0x0002 };
    pub const DISCARD: Self = Self { bits: 0x0004 };

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn contains(self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    pub fn insert(&mut self, other: Self) {
        self.bits |= other.bits;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideData {
    kind: String,
    data: Vec<u8>,
}

impl SideData {
    pub fn new(kind: impl Into<String>, data: Vec<u8>) -> AvResult<Self> {
        let kind = kind.into();
        if kind.trim().is_empty() {
            return Err(AvError::invalid_argument(
                "packet side data kind must not be empty",
            ));
        }

        Ok(Self { kind, data })
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    data: Vec<u8>,
    pts: i64,
    dts: i64,
    duration: i64,
    stream_index: usize,
    flags: PacketFlags,
    side_data: Vec<SideData>,
}

impl Packet {
    pub fn new(data: Vec<u8>, stream_index: usize) -> Self {
        Self {
            data,
            pts: AV_NOPTS_VALUE,
            dts: AV_NOPTS_VALUE,
            duration: 0,
            stream_index,
            flags: PacketFlags::empty(),
            side_data: Vec::new(),
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn pts(&self) -> Option<i64> {
        pts_option(self.pts)
    }

    pub fn dts(&self) -> Option<i64> {
        pts_option(self.dts)
    }

    pub fn duration(&self) -> i64 {
        self.duration
    }

    pub fn stream_index(&self) -> usize {
        self.stream_index
    }

    pub fn flags(&self) -> PacketFlags {
        self.flags
    }

    pub fn side_data(&self) -> &[SideData] {
        &self.side_data
    }

    pub fn set_pts(&mut self, pts: Option<i64>) {
        self.pts = pts.unwrap_or(AV_NOPTS_VALUE);
    }

    pub fn set_dts(&mut self, dts: Option<i64>) {
        self.dts = dts.unwrap_or(AV_NOPTS_VALUE);
    }

    pub fn set_duration(&mut self, duration: i64) -> AvResult<()> {
        if duration < 0 {
            return Err(AvError::invalid_argument(
                "packet duration must not be negative",
            ));
        }

        self.duration = duration;
        Ok(())
    }

    pub fn set_key(&mut self, is_key: bool) {
        if is_key {
            self.flags.insert(PacketFlags::KEY);
        }
    }

    pub fn push_side_data(&mut self, side_data: SideData) {
        self.side_data.push(side_data);
    }
}

fn pts_option(value: i64) -> Option<i64> {
    if value == AV_NOPTS_VALUE {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_defaults_to_no_timestamps() {
        let packet = Packet::new(vec![1, 2, 3], 7);

        assert_eq!(packet.data(), &[1, 2, 3]);
        assert_eq!(packet.stream_index(), 7);
        assert_eq!(packet.pts(), None);
        assert_eq!(packet.dts(), None);
        assert_eq!(packet.duration(), 0);
    }

    #[test]
    fn packet_tracks_timestamps_flags_and_side_data() {
        let mut packet = Packet::new(vec![0xaa], 0);
        packet.set_pts(Some(12));
        packet.set_dts(Some(10));
        packet.set_duration(2).unwrap();
        packet.set_key(true);
        packet.push_side_data(SideData::new("palette", vec![0, 1, 2]).unwrap());

        assert_eq!(packet.pts(), Some(12));
        assert_eq!(packet.dts(), Some(10));
        assert_eq!(packet.duration(), 2);
        assert!(packet.flags().contains(PacketFlags::KEY));
        assert_eq!(packet.side_data()[0].kind(), "palette");
    }

    #[test]
    fn packet_rejects_negative_duration_and_empty_side_data_kind() {
        let mut packet = Packet::new(Vec::new(), 0);

        assert!(packet.set_duration(-1).is_err());
        assert!(SideData::new(" ", Vec::new()).is_err());
    }
}
