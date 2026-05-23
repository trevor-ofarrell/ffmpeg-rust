use avutil::{AvError, AvResult, Packet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameCrcRecord {
    stream_index: usize,
    pts: Option<i64>,
    dts: Option<i64>,
    duration: i64,
    size: usize,
    checksum: u32,
}

impl FrameCrcRecord {
    fn from_packet(packet: &Packet) -> Self {
        Self {
            stream_index: packet.stream_index(),
            pts: packet.pts(),
            dts: packet.dts(),
            duration: packet.duration(),
            size: packet.data().len(),
            checksum: ffmpeg_framecrc_checksum(packet.data()),
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

    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn line(&self) -> String {
        format!(
            "{}, {:>10}, {:>10}, {:>8}, {:>8}, 0x{:08x}\n",
            self.stream_index,
            fmt_ts(self.dts),
            fmt_ts(self.pts),
            self.duration,
            self.size,
            self.checksum
        )
    }
}

pub fn ffmpeg_framecrc_checksum(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65_521;
    let mut a = 0_u32;
    let mut b = 0_u32;
    for byte in data {
        a = (a + u32::from(*byte)) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
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
    use avutil::AvErrorKind;

    #[test]
    fn records_packet_checksum_and_timing_fields() {
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
        assert_eq!(record.checksum(), ffmpeg_framecrc_checksum(b"abc"));
        assert_eq!(
            record.line(),
            format!(
                "{}, {:>10}, {:>10}, {:>8}, {:>8}, 0x{:08x}\n",
                2, 8, 10, 2, 3, 0x024a0126_u32
            )
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
        assert!(output.contains("0,        N/A,        N/A,        0,        1"));
        assert!(output.contains("1,        N/A,        N/A,        0,        1"));
        assert!(output.find("0,").unwrap() < output.find("1,").unwrap());
    }

    #[test]
    fn empty_packets_produce_zero_size_checksum_records() {
        let mut muxer = FrameCrcMuxer::new();

        muxer.write_packet(&Packet::new(Vec::new(), 0)).unwrap();
        let record = &muxer.records()[0];

        assert_eq!(record.size(), 0);
        assert_eq!(record.checksum(), ffmpeg_framecrc_checksum(b""));
        assert_eq!(
            record.line(),
            "0,        N/A,        N/A,        0,        0, 0x00000000\n"
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
