use crate::{rescale_q, AvError, AvResult, BufferRef, Rational};

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

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn shrink(&mut self, len: usize) -> AvResult<()> {
        if len > self.data.len() {
            return Err(AvError::invalid_argument(
                "packet side data cannot be shrunk to a larger size",
            ));
        }

        self.data.truncate(len);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    data: BufferRef,
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
        Self::with_buffer(BufferRef::from_vec(data), stream_index)
    }

    pub fn with_buffer(data: BufferRef, stream_index: usize) -> Self {
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
        self.data.as_slice()
    }

    pub fn data_buffer(&self) -> &BufferRef {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_data_writable(&self) -> bool {
        self.data.is_writable()
    }

    pub fn data_mut(&mut self) -> Option<&mut [u8]> {
        self.data.get_mut()
    }

    pub fn make_data_writable(&mut self) -> &mut [u8] {
        self.data.make_mut()
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

    pub fn side_data_by_kind(&self, kind: &str) -> Option<&SideData> {
        self.side_data
            .iter()
            .find(|side_data| side_data.kind() == kind)
    }

    pub fn side_data_mut_by_kind(&mut self, kind: &str) -> Option<&mut SideData> {
        self.side_data
            .iter_mut()
            .find(|side_data| side_data.kind() == kind)
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

    pub fn shrink_side_data(&mut self, kind: &str, len: usize) -> AvResult<bool> {
        let Some(side_data) = self.side_data_mut_by_kind(kind) else {
            return Ok(false);
        };

        side_data.shrink(len)?;
        Ok(true)
    }

    pub fn take_side_data(&mut self, kind: &str) -> Option<SideData> {
        let index = self
            .side_data
            .iter()
            .position(|side_data| side_data.kind() == kind)?;
        Some(self.side_data.remove(index))
    }

    pub fn remove_side_data(&mut self, kind: &str) -> bool {
        self.take_side_data(kind).is_some()
    }

    pub fn clear_side_data(&mut self) {
        self.side_data.clear();
    }

    pub fn unref(&mut self) {
        *self = Self::default();
    }

    pub fn ref_from(&mut self, src: &Self) {
        *self = src.clone();
    }

    pub fn move_ref_from(&mut self, src: &mut Self) {
        *self = std::mem::take(src);
    }

    pub fn rescale_ts(&mut self, src: Rational, dst: Rational) -> AvResult<()> {
        rescale_q(0, src, dst)?;

        let pts = if self.pts == AV_NOPTS_VALUE {
            AV_NOPTS_VALUE
        } else {
            rescale_q(self.pts, src, dst)?
        };
        let dts = if self.dts == AV_NOPTS_VALUE {
            AV_NOPTS_VALUE
        } else {
            rescale_q(self.dts, src, dst)?
        };
        let duration = if self.duration == 0 {
            0
        } else {
            rescale_q(self.duration, src, dst)?
        };

        self.pts = pts;
        self.dts = dts;
        self.duration = duration;
        Ok(())
    }
}

impl Default for Packet {
    fn default() -> Self {
        Self::new(Vec::new(), 0)
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
    use std::sync::{Arc, Mutex};

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
        assert_eq!(packet.side_data()[0].len(), 3);
        assert!(!packet.side_data()[0].is_empty());
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

    #[test]
    fn packet_side_data_lookup_and_shrink_preserve_order() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0, 1, 2, 3]).unwrap());
        packet.push_side_data(SideData::new("skip_samples", vec![4, 5, 6]).unwrap());
        packet.push_side_data(SideData::new("palette", vec![7, 8]).unwrap());

        assert_eq!(
            packet.side_data_by_kind("palette").unwrap().data(),
            &[0, 1, 2, 3]
        );
        assert_eq!(
            packet.side_data_by_kind("skip_samples").unwrap().data(),
            &[4, 5, 6]
        );
        assert!(packet.side_data_by_kind("missing").is_none());

        assert!(packet.shrink_side_data("palette", 2).unwrap());
        assert_eq!(packet.side_data_by_kind("palette").unwrap().data(), &[0, 1]);
        assert_eq!(packet.side_data()[1].kind(), "skip_samples");
        assert_eq!(packet.side_data()[2].data(), &[7, 8]);
        assert!(!packet.shrink_side_data("missing", 0).unwrap());
    }

    #[test]
    fn packet_side_data_shrink_errors_do_not_mutate_payload() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0, 1, 2]).unwrap());

        let err = packet.shrink_side_data("palette", 4).unwrap_err();

        assert_eq!(err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(
            packet.side_data_by_kind("palette").unwrap().data(),
            &[0, 1, 2]
        );
    }

    #[test]
    fn packet_side_data_take_remove_and_clear_are_scoped() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.push_side_data(SideData::new("palette", vec![0]).unwrap());
        packet.push_side_data(SideData::new("skip_samples", vec![1]).unwrap());
        packet.push_side_data(SideData::new("palette", vec![2]).unwrap());

        let taken = packet.take_side_data("palette").unwrap();
        assert_eq!(taken.data(), &[0]);
        assert_eq!(packet.side_data().len(), 2);
        assert_eq!(packet.side_data_by_kind("palette").unwrap().data(), &[2]);

        assert!(packet.remove_side_data("skip_samples"));
        assert!(!packet.remove_side_data("missing"));
        assert_eq!(packet.side_data().len(), 1);

        packet.clear_side_data();
        assert!(packet.side_data().is_empty());
    }

    #[test]
    fn packet_ref_from_shares_payload_and_copies_side_data() {
        let mut src = Packet::new(vec![1, 2, 3], 4);
        src.set_pts(Some(12));
        src.set_dts(Some(10));
        src.set_duration(2).unwrap();
        src.set_pos(Some(42)).unwrap();
        src.set_key(true);
        src.push_side_data(SideData::new("palette", vec![5, 6]).unwrap());

        let mut dst = Packet::new(vec![9], 99);
        dst.push_side_data(SideData::new("old", vec![8]).unwrap());
        dst.ref_from(&src);

        assert_eq!(dst.data(), &[1, 2, 3]);
        assert!(dst.data_buffer().shares_storage(src.data_buffer()));
        assert_eq!(dst.stream_index(), 4);
        assert_eq!(dst.pts(), Some(12));
        assert_eq!(dst.dts(), Some(10));
        assert_eq!(dst.duration(), 2);
        assert_eq!(dst.pos(), Some(42));
        assert!(dst.flags().contains(PacketFlags::KEY));
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[5, 6]);

        dst.shrink_side_data("palette", 1).unwrap();
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[5]);
        assert_eq!(src.side_data_by_kind("palette").unwrap().data(), &[5, 6]);
    }

    #[test]
    fn packet_make_data_writable_detaches_shared_payload() {
        let src = Packet::new(vec![1, 2, 3], 0);
        let mut dst = Packet::default();
        dst.ref_from(&src);

        assert!(dst.data_buffer().shares_storage(src.data_buffer()));
        assert!(!dst.is_data_writable());
        assert!(dst.data_mut().is_none());

        dst.make_data_writable()[0] = 9;

        assert_eq!(dst.data(), &[9, 2, 3]);
        assert_eq!(src.data(), &[1, 2, 3]);
        assert!(!dst.data_buffer().shares_storage(src.data_buffer()));
        assert!(dst.is_data_writable());
        assert!(src.is_data_writable());
    }

    #[test]
    fn packet_move_ref_and_unref_reset_packets_and_release_payloads() {
        let released = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let capture_old = Arc::clone(&released);
        let mut dst = Packet::with_buffer(
            BufferRef::from_vec_with_release_callback(vec![9], move |data| {
                capture_old.lock().unwrap().push(data);
            }),
            8,
        );
        dst.push_side_data(SideData::new("old", vec![0]).unwrap());

        let capture_src = Arc::clone(&released);
        let mut src = Packet::with_buffer(
            BufferRef::from_vec_with_release_callback(vec![1, 2], move |data| {
                capture_src.lock().unwrap().push(data);
            }),
            3,
        );
        src.set_pts(Some(7));
        src.set_duration(5).unwrap();
        src.push_side_data(SideData::new("palette", vec![4]).unwrap());

        dst.move_ref_from(&mut src);

        assert_eq!(*released.lock().unwrap(), vec![vec![9]]);
        assert!(src.is_empty());
        assert_eq!(src.stream_index(), 0);
        assert_eq!(src.pts(), None);
        assert!(src.side_data().is_empty());
        assert_eq!(dst.data(), &[1, 2]);
        assert_eq!(dst.stream_index(), 3);
        assert_eq!(dst.pts(), Some(7));
        assert_eq!(dst.duration(), 5);
        assert_eq!(dst.side_data_by_kind("palette").unwrap().data(), &[4]);

        dst.unref();

        assert_eq!(*released.lock().unwrap(), vec![vec![9], vec![1, 2]]);
        assert!(dst.is_empty());
        assert_eq!(dst.stream_index(), 0);
        assert_eq!(dst.pts(), None);
        assert_eq!(dst.dts(), None);
        assert_eq!(dst.duration(), 0);
        assert_eq!(dst.pos(), None);
        assert!(dst.flags().is_empty());
        assert!(dst.side_data().is_empty());
    }

    #[test]
    fn packet_rescales_valid_timestamps_and_duration() {
        let src = Rational::new(1, 90_000).unwrap();
        let dst = Rational::new(1, 1_000).unwrap();
        let mut packet = Packet::new(vec![0], 3);
        packet.set_pts(Some(90_000));
        packet.set_dts(Some(45_000));
        packet.set_duration(3_003).unwrap();
        packet.set_pos(Some(77)).unwrap();
        packet.set_key(true);
        packet.push_side_data(SideData::new("palette", vec![1, 2, 3]).unwrap());

        packet.rescale_ts(src, dst).unwrap();

        assert_eq!(packet.pts(), Some(1_000));
        assert_eq!(packet.dts(), Some(500));
        assert_eq!(packet.duration(), 33);
        assert_eq!(packet.pos(), Some(77));
        assert_eq!(packet.stream_index(), 3);
        assert!(packet.flags().contains(PacketFlags::KEY));
        assert_eq!(packet.side_data()[0].data(), &[1, 2, 3]);
    }

    #[test]
    fn packet_rescale_ignores_unknown_timestamps() {
        let src = Rational::new(1, 48_000).unwrap();
        let dst = Rational::new(1, 1_000).unwrap();
        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_duration(48_000).unwrap();

        packet.rescale_ts(src, dst).unwrap();

        assert_eq!(packet.pts(), None);
        assert_eq!(packet.dts(), None);
        assert_eq!(packet.duration(), 1_000);
    }

    #[test]
    fn packet_rescale_errors_do_not_mutate_timing_fields() {
        let mut packet = Packet::new(Vec::new(), 0);
        packet.set_pts(Some(10));
        packet.set_dts(Some(9));
        packet.set_duration(8).unwrap();

        let invalid_err = packet
            .rescale_ts(Rational::from_raw(1, 0), Rational::ONE)
            .unwrap_err();

        assert_eq!(invalid_err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(packet.pts(), Some(10));
        assert_eq!(packet.dts(), Some(9));
        assert_eq!(packet.duration(), 8);

        packet.set_pts(Some(i64::MAX));
        packet.set_dts(Some(9));
        packet.set_duration(8).unwrap();
        let overflow_err = packet
            .rescale_ts(Rational::ONE, Rational::new(1, 2).unwrap())
            .unwrap_err();

        assert_eq!(overflow_err.kind(), crate::AvErrorKind::InvalidArgument);
        assert_eq!(packet.pts(), Some(i64::MAX));
        assert_eq!(packet.dts(), Some(9));
        assert_eq!(packet.duration(), 8);
    }
}
