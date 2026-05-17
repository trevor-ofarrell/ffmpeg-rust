use crate::{AvError, AvResult};

pub const AV_NOPTS_VALUE: i64 = i64::MIN;
pub const AV_PACKET_POS_UNKNOWN: i64 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PacketFlags {
    bits: u32,
}

impl PacketFlags {
    pub const KEY: Self = Self { bits: 0x0001 };
    pub const CORRUPT: Self = Self { bits: 0x0002 };
    pub const DISCARD: Self = Self { bits: 0x0004 };
    pub const TRUSTED: Self = Self { bits: 0x0008 };
    pub const DISPOSABLE: Self = Self { bits: 0x0010 };
    const KNOWN_BITS: u32 = Self::KEY.bits
        | Self::CORRUPT.bits
        | Self::DISCARD.bits
        | Self::TRUSTED.bits
        | Self::DISPOSABLE.bits;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn all() -> Self {
        Self {
            bits: Self::KNOWN_BITS,
        }
    }

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self {
            bits: bits & Self::KNOWN_BITS,
        }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    pub fn insert(&mut self, other: Self) {
        self.bits |= other.bits;
    }

    pub fn remove(&mut self, other: Self) {
        self.bits &= !other.bits;
    }

    pub fn set(&mut self, other: Self, enabled: bool) {
        if enabled {
            self.insert(other);
        } else {
            self.remove(other);
        }
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
        if kind.contains('\0') {
            return Err(AvError::invalid_argument(
                "packet side data kind must not contain NUL",
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
    pos: i64,
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
            pos: AV_PACKET_POS_UNKNOWN,
            stream_index,
            flags: PacketFlags::empty(),
            side_data: Vec::new(),
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
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

    pub fn pos(&self) -> Option<i64> {
        pos_option(self.pos)
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

    pub fn set_pos(&mut self, pos: Option<i64>) -> AvResult<()> {
        if let Some(pos) = pos {
            if pos < 0 {
                return Err(AvError::invalid_argument(
                    "packet byte position must not be negative",
                ));
            }
        }

        self.pos = pos.unwrap_or(AV_PACKET_POS_UNKNOWN);
        Ok(())
    }

    pub fn set_flag(&mut self, flag: PacketFlags, enabled: bool) {
        self.flags.set(flag, enabled);
    }

    pub fn set_key(&mut self, is_key: bool) {
        self.set_flag(PacketFlags::KEY, is_key);
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

fn pos_option(value: i64) -> Option<i64> {
    if value == AV_PACKET_POS_UNKNOWN {
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
        assert_eq!(packet.pos(), None);
        assert_eq!(packet.len(), 3);
        assert!(!packet.is_empty());
    }

    #[test]
    fn packet_tracks_timestamps_position_flags_and_side_data() {
        let mut packet = Packet::new(vec![0xaa], 0);
        packet.set_pts(Some(12));
        packet.set_dts(Some(10));
        packet.set_duration(2).unwrap();
        packet.set_pos(Some(42)).unwrap();
        packet.set_key(true);
        packet.set_flag(PacketFlags::CORRUPT, true);
        packet.set_flag(PacketFlags::DISCARD, true);
        packet.set_flag(PacketFlags::TRUSTED, true);
        packet.set_flag(PacketFlags::DISPOSABLE, true);
        packet.push_side_data(SideData::new("palette", vec![0, 1, 2]).unwrap());

        assert_eq!(packet.pts(), Some(12));
        assert_eq!(packet.dts(), Some(10));
        assert_eq!(packet.duration(), 2);
        assert_eq!(packet.pos(), Some(42));
        assert!(packet.flags().contains(PacketFlags::KEY));
        assert!(packet.flags().contains(PacketFlags::CORRUPT));
        assert!(packet.flags().contains(PacketFlags::DISCARD));
        assert!(packet.flags().contains(PacketFlags::TRUSTED));
        assert!(packet.flags().contains(PacketFlags::DISPOSABLE));
        assert_eq!(packet.side_data()[0].kind(), "palette");
    }

    #[test]
    fn packet_flags_can_be_cleared() {
        let mut flags = PacketFlags::empty();
        assert!(flags.is_empty());
        flags.insert(PacketFlags::KEY);
        flags.insert(PacketFlags::DISCARD);
        assert!(flags.contains(PacketFlags::KEY));
        assert!(flags.contains(PacketFlags::DISCARD));

        flags.remove(PacketFlags::KEY);
        assert!(!flags.contains(PacketFlags::KEY));
        assert!(flags.contains(PacketFlags::DISCARD));

        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_key(true);
        assert!(packet.flags().contains(PacketFlags::KEY));
        packet.set_key(false);
        assert!(!packet.flags().contains(PacketFlags::KEY));

        packet.set_flag(PacketFlags::CORRUPT, true);
        assert!(packet.flags().contains(PacketFlags::CORRUPT));
        packet.set_flag(PacketFlags::CORRUPT, false);
        assert!(!packet.flags().contains(PacketFlags::CORRUPT));

        let truncated = PacketFlags::from_bits_truncate(0xffff_ffff);
        assert_eq!(truncated.bits(), PacketFlags::all().bits());
    }

    #[test]
    fn packet_rejects_negative_duration_position_and_invalid_side_data_kind() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_duration(5).unwrap();
        packet.set_pos(Some(9)).unwrap();

        assert!(packet.set_duration(-1).is_err());
        assert_eq!(packet.duration(), 5);
        assert!(packet.set_pos(Some(-1)).is_err());
        assert_eq!(packet.pos(), Some(9));
        packet.set_pos(None).unwrap();
        assert_eq!(packet.pos(), None);
        assert!(SideData::new(" ", Vec::new()).is_err());
        assert!(SideData::new("bad\0kind", Vec::new()).is_err());
    }
}
