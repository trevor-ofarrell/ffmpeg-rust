use avutil::{AvError, AvResult, Packet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullStreamStats {
    stream_index: usize,
    packets: u64,
    bytes: u64,
    duration: i64,
    last_pts: Option<i64>,
    last_dts: Option<i64>,
}

impl NullStreamStats {
    fn new(stream_index: usize) -> Self {
        Self {
            stream_index,
            packets: 0,
            bytes: 0,
            duration: 0,
            last_pts: None,
            last_dts: None,
        }
    }

    pub fn stream_index(&self) -> usize {
        self.stream_index
    }

    pub fn packets(&self) -> u64 {
        self.packets
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn duration(&self) -> i64 {
        self.duration
    }

    pub fn last_pts(&self) -> Option<i64> {
        self.last_pts
    }

    pub fn last_dts(&self) -> Option<i64> {
        self.last_dts
    }

    fn record_packet(&mut self, packet: &Packet) -> AvResult<()> {
        let packet_bytes = u64::try_from(packet.data().len())
            .map_err(|_| AvError::invalid_argument("packet size does not fit u64"))?;
        self.bytes = self
            .bytes
            .checked_add(packet_bytes)
            .ok_or_else(|| AvError::invalid_argument("null muxer byte count overflow"))?;
        self.packets = self
            .packets
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("null muxer packet count overflow"))?;
        self.duration = self
            .duration
            .checked_add(packet.duration())
            .ok_or_else(|| AvError::invalid_argument("null muxer duration overflow"))?;
        self.last_pts = packet.pts();
        self.last_dts = packet.dts();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullMuxerReport {
    total_packets: u64,
    total_bytes: u64,
    streams: Vec<NullStreamStats>,
}

impl NullMuxerReport {
    pub fn total_packets(&self) -> u64 {
        self.total_packets
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn streams(&self) -> &[NullStreamStats] {
        &self.streams
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NullMuxer {
    streams: Vec<NullStreamStats>,
    total_packets: u64,
    total_bytes: u64,
    finished: bool,
}

impl NullMuxer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after null muxer is finished",
            ));
        }

        let packet_bytes = u64::try_from(packet.data().len())
            .map_err(|_| AvError::invalid_argument("packet size does not fit u64"))?;
        self.ensure_stream(packet.stream_index());
        self.streams[packet.stream_index()].record_packet(packet)?;
        self.total_packets = self
            .total_packets
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("null muxer packet count overflow"))?;
        self.total_bytes = self
            .total_bytes
            .checked_add(packet_bytes)
            .ok_or_else(|| AvError::invalid_argument("null muxer byte count overflow"))?;
        Ok(())
    }

    pub fn finish(&mut self) -> NullMuxerReport {
        self.finished = true;
        self.report()
    }

    pub fn report(&self) -> NullMuxerReport {
        NullMuxerReport {
            total_packets: self.total_packets,
            total_bytes: self.total_bytes,
            streams: self.streams.clone(),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    fn ensure_stream(&mut self, stream_index: usize) {
        while self.streams.len() <= stream_index {
            let next = self.streams.len();
            self.streams.push(NullStreamStats::new(next));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::AvErrorKind;

    #[test]
    fn tracks_packet_and_stream_statistics() {
        let mut muxer = NullMuxer::new();
        let mut first = Packet::new(vec![1, 2, 3], 0);
        first.set_pts(Some(10));
        first.set_dts(Some(9));
        first.set_duration(2).unwrap();
        let mut second = Packet::new(vec![4, 5], 1);
        second.set_pts(Some(22));
        second.set_duration(3).unwrap();

        muxer.write_packet(&first).unwrap();
        muxer.write_packet(&second).unwrap();
        muxer.write_packet(&first).unwrap();
        let report = muxer.finish();

        assert_eq!(report.total_packets(), 3);
        assert_eq!(report.total_bytes(), 8);
        assert_eq!(report.streams().len(), 2);
        assert_eq!(report.streams()[0].packets(), 2);
        assert_eq!(report.streams()[0].bytes(), 6);
        assert_eq!(report.streams()[0].duration(), 4);
        assert_eq!(report.streams()[0].last_pts(), Some(10));
        assert_eq!(report.streams()[0].last_dts(), Some(9));
        assert_eq!(report.streams()[1].packets(), 1);
        assert_eq!(report.streams()[1].bytes(), 2);
    }

    #[test]
    fn empty_packets_count_without_bytes() {
        let mut muxer = NullMuxer::new();
        let packet = Packet::new(Vec::new(), 2);

        muxer.write_packet(&packet).unwrap();
        let report = muxer.report();

        assert_eq!(report.total_packets(), 1);
        assert_eq!(report.total_bytes(), 0);
        assert_eq!(report.streams().len(), 3);
        assert_eq!(report.streams()[2].stream_index(), 2);
        assert_eq!(report.streams()[2].packets(), 1);
    }

    #[test]
    fn finish_prevents_more_writes() {
        let mut muxer = NullMuxer::new();
        let packet = Packet::new(vec![1], 0);

        muxer.write_packet(&packet).unwrap();
        muxer.finish();
        let err = muxer.write_packet(&packet).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert!(muxer.is_finished());
        assert_eq!(muxer.report().total_packets(), 1);
    }
}
