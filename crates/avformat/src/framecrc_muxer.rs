use avutil::{crc32_ieee, AvError, AvResult, Packet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameCrcRecord {
    stream_index: usize,
    pts: Option<i64>,
    dts: Option<i64>,
    duration: i64,
    size: usize,
    crc32: u32,
}

impl FrameCrcRecord {
    fn from_packet(packet: &Packet) -> Self {
        Self {
            stream_index: packet.stream_index(),
            pts: packet.pts(),
            dts: packet.dts(),
            duration: packet.duration(),
            size: packet.data().len(),
            crc32: crc32_ieee(packet.data()),
        }
    }

    pub fn stream_index(&self) -> usize {
        self.stream_index
    }

    pub fn pts(&self) -> Option<i64> {
        self.pts
    }

    pub fn dts(&self) -> Option<i64> {
        self.dts
    }

    pub fn duration(&self) -> i64 {
        self.duration
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    pub fn line(&self) -> String {
        format!(
            "stream={} pts={} dts={} duration={} size={} crc32=0x{:08x}\n",
            self.stream_index,
            fmt_ts(self.pts),
            fmt_ts(self.dts),
            self.duration,
            self.size,
            self.crc32
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameCrcMuxer {
    records: Vec<FrameCrcRecord>,
    finished: bool,
}

impl FrameCrcMuxer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after framecrc muxer is finished",
            ));
        }

        self.records.push(FrameCrcRecord::from_packet(packet));
        Ok(())
    }

    pub fn records(&self) -> &[FrameCrcRecord] {
        &self.records
    }

    pub fn render(&self) -> String {
        let mut output = String::from("# framecrc-rs packet checksums\n");
        for record in &self.records {
            output.push_str(&record.line());
        }
        output
    }

    pub fn finish(&mut self) -> String {
        self.finished = true;
        self.render()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

fn fmt_ts(value: Option<i64>) -> String {
    value.map_or_else(|| "N/A".to_string(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{crc32_ieee, AvErrorKind};

    #[test]
    fn records_packet_crc_and_timing_fields() {
        let mut muxer = FrameCrcMuxer::new();
        let mut packet = Packet::new(b"abc".to_vec(), 2);
        packet.set_pts(Some(10));
        packet.set_dts(Some(8));
        packet.set_duration(2).unwrap();

        muxer.write_packet(&packet).unwrap();
        let record = &muxer.records()[0];

        assert_eq!(record.stream_index(), 2);
        assert_eq!(record.pts(), Some(10));
        assert_eq!(record.dts(), Some(8));
        assert_eq!(record.duration(), 2);
        assert_eq!(record.size(), 3);
        assert_eq!(record.crc32(), crc32_ieee(b"abc"));
        assert_eq!(
            record.line(),
            "stream=2 pts=10 dts=8 duration=2 size=3 crc32=0x352441c2\n"
        );
    }

    #[test]
    fn renders_multiple_records_in_write_order() {
        let mut muxer = FrameCrcMuxer::new();
        muxer.write_packet(&Packet::new(b"a".to_vec(), 0)).unwrap();
        muxer.write_packet(&Packet::new(b"b".to_vec(), 1)).unwrap();

        let output = muxer.finish();

        assert!(muxer.is_finished());
        assert!(output.starts_with("# framecrc-rs packet checksums\n"));
        assert!(output.contains("stream=0 pts=N/A dts=N/A duration=0 size=1"));
        assert!(output.contains("stream=1 pts=N/A dts=N/A duration=0 size=1"));
        assert!(output.find("stream=0").unwrap() < output.find("stream=1").unwrap());
    }

    #[test]
    fn empty_packets_produce_zero_size_crc_records() {
        let mut muxer = FrameCrcMuxer::new();

        muxer.write_packet(&Packet::new(Vec::new(), 0)).unwrap();
        let record = &muxer.records()[0];

        assert_eq!(record.size(), 0);
        assert_eq!(record.crc32(), crc32_ieee(b""));
        assert_eq!(
            record.line(),
            "stream=0 pts=N/A dts=N/A duration=0 size=0 crc32=0x00000000\n"
        );
    }

    #[test]
    fn finish_prevents_more_writes() {
        let mut muxer = FrameCrcMuxer::new();
        let packet = Packet::new(vec![1], 0);

        muxer.write_packet(&packet).unwrap();
        muxer.finish();
        let err = muxer.write_packet(&packet).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.records().len(), 1);
    }
}
