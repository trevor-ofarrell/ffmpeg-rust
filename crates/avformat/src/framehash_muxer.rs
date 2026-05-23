use crate::hash_muxer::{HashAlgorithm, HashDigest, HashMuxer};
use avutil::{AvError, AvResult, Packet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHashRecord {
    algorithm: HashAlgorithm,
    stream_index: usize,
    pts: Option<i64>,
    dts: Option<i64>,
    duration: i64,
    size: usize,
    digest: HashDigest,
}

impl FrameHashRecord {
    fn from_packet(packet: &Packet, algorithm: HashAlgorithm) -> AvResult<Self> {
        let mut muxer = HashMuxer::new(algorithm);
        muxer.write_packet(packet)?;
        let report = muxer.finish();
        Ok(Self {
            algorithm,
            stream_index: packet.stream_index(),
            pts: packet.pts(),
            dts: packet.dts(),
            duration: packet.duration(),
            size: packet.data().len(),
            digest: report.digest().clone(),
        })
    }

    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
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

    pub fn digest(&self) -> &HashDigest {
        &self.digest
    }

    pub fn digest_hex(&self) -> String {
        self.digest.hex()
    }

    pub fn line(&self) -> String {
        format!(
            "{}, {:>10}, {:>10}, {:>8}, {:>8}, {}\n",
            self.stream_index,
            fmt_ts(self.dts),
            fmt_ts(self.pts),
            self.duration,
            self.size,
            self.digest_hex()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHashMuxer {
    algorithm: HashAlgorithm,
    records: Vec<FrameHashRecord>,
    finished: bool,
}

impl FrameHashMuxer {
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            algorithm,
            records: Vec::new(),
            finished: false,
        }
    }

    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after framehash muxer is finished",
            ));
        }

        self.records
            .push(FrameHashRecord::from_packet(packet, self.algorithm)?);
        Ok(())
    }

    pub fn records(&self) -> &[FrameHashRecord] {
        &self.records
    }

    pub fn render(&self) -> String {
        let mut output = format!(
            "#format: frame checksums\n#version: 2\n#hash: {}\n#stream#, dts,        pts, duration,     size, hash\n",
            self.algorithm.name()
        );
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
    use avutil::{digest_to_hex, md5, sha256, AvErrorKind};

    #[test]
    fn records_packet_hash_and_timing_fields() {
        let mut muxer = FrameHashMuxer::new(HashAlgorithm::Sha256);
        let mut packet = Packet::new(b"abc".to_vec(), 2);
        packet.set_pts(Some(10));
        packet.set_dts(Some(8));
        packet.set_duration(2).unwrap();

        muxer.write_packet(&packet).unwrap();
        let record = &muxer.records()[0];

        assert_eq!(muxer.algorithm(), HashAlgorithm::Sha256);
        assert_eq!(record.algorithm(), HashAlgorithm::Sha256);
        assert_eq!(record.stream_index(), 2);
        assert_eq!(record.pts(), Some(10));
        assert_eq!(record.dts(), Some(8));
        assert_eq!(record.duration(), 2);
        assert_eq!(record.size(), 3);
        assert_eq!(record.digest(), &HashDigest::Bytes(sha256(b"abc").to_vec()));
        assert_eq!(
            record.line(),
            format!(
                "{}, {:>10}, {:>10}, {:>8}, {:>8}, {}\n",
                2,
                8,
                10,
                2,
                3,
                digest_to_hex(&sha256(b"abc"))
            )
        );
    }

    #[test]
    fn renders_multiple_records_in_write_order() {
        let mut muxer = FrameHashMuxer::new(HashAlgorithm::Md5);
        muxer.write_packet(&Packet::new(b"a".to_vec(), 0)).unwrap();
        muxer.write_packet(&Packet::new(b"b".to_vec(), 1)).unwrap();

        let output = muxer.finish();

        assert!(muxer.is_finished());
        assert!(output.starts_with(
            "#format: frame checksums\n#version: 2\n#hash: MD5\n#stream#, dts,        pts, duration,     size, hash\n"
        ));
        assert!(output.contains(&format!(
            "{}, {:>10}, {:>10}, {:>8}, {:>8}, ",
            0, "N/A", "N/A", 0, 1
        )));
        assert!(output.contains(&format!(
            "{}, {:>10}, {:>10}, {:>8}, {:>8}, ",
            1, "N/A", "N/A", 0, 1
        )));
        assert!(output.find("0,").unwrap() < output.find("1,").unwrap());
    }

    #[test]
    fn empty_packets_produce_zero_size_hash_records() {
        let mut muxer = FrameHashMuxer::new(HashAlgorithm::Md5);

        muxer.write_packet(&Packet::new(Vec::new(), 0)).unwrap();
        let record = &muxer.records()[0];

        assert_eq!(record.size(), 0);
        assert_eq!(record.digest(), &HashDigest::Bytes(md5(b"").to_vec()));
        assert_eq!(
            record.line(),
            format!(
                "{}, {:>10}, {:>10}, {:>8}, {:>8}, {}\n",
                0,
                "N/A",
                "N/A",
                0,
                0,
                digest_to_hex(&md5(b""))
            )
        );
    }

    #[test]
    fn finish_prevents_more_writes() {
        let mut muxer = FrameHashMuxer::new(HashAlgorithm::Sha256);
        let packet = Packet::new(vec![1], 0);

        muxer.write_packet(&packet).unwrap();
        muxer.finish();
        let err = muxer.write_packet(&packet).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.records().len(), 1);
    }
}
