use crate::hash_muxer::{HashAlgorithm, HashDigest, HashMuxer};
use avutil::{AvError, AvErrorKind, AvResult, Packet};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamHashStreamType {
    Audio,
    Video,
    Subtitle,
    Data,
    Unknown,
}

impl StreamHashStreamType {
    pub fn code(self) -> char {
        match self {
            Self::Audio => 'a',
            Self::Video => 'v',
            Self::Subtitle => 's',
            Self::Data => 'd',
            Self::Unknown => '?',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamHashRecord {
    stream_index: usize,
    stream_type: StreamHashStreamType,
    algorithm: HashAlgorithm,
    digest: HashDigest,
    packets: u64,
    bytes: u64,
}

impl StreamHashRecord {
    pub fn stream_index(&self) -> usize {
        self.stream_index
    }

    pub fn stream_type(&self) -> StreamHashStreamType {
        self.stream_type
    }

    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    pub fn digest(&self) -> &HashDigest {
        &self.digest
    }

    pub fn digest_hex(&self) -> String {
        self.digest.hex()
    }

    pub fn packets(&self) -> u64 {
        self.packets
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn line(&self) -> String {
        format!(
            "{},{},{}={}\n",
            self.stream_index,
            self.stream_type.code(),
            self.algorithm.name(),
            self.digest_hex()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamHashMuxer {
    algorithm: HashAlgorithm,
    streams: BTreeMap<usize, StreamHashState>,
    finished: bool,
}

impl StreamHashMuxer {
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            algorithm,
            streams: BTreeMap::new(),
            finished: false,
        }
    }

    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    pub fn write_packet(
        &mut self,
        packet: &Packet,
        stream_type: StreamHashStreamType,
    ) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after streamhash muxer is finished",
            ));
        }

        let state = self
            .streams
            .entry(packet.stream_index())
            .or_insert_with(|| StreamHashState::new(self.algorithm, stream_type));
        if state.stream_type != stream_type {
            return Err(AvError::new(
                AvErrorKind::InvalidArgument,
                format!(
                    "stream {} changed streamhash type from {} to {}",
                    packet.stream_index(),
                    state.stream_type.code(),
                    stream_type.code()
                ),
            ));
        }
        state.muxer.write_packet(packet)
    }

    pub fn records(&self) -> Vec<StreamHashRecord> {
        self.streams
            .iter()
            .map(|(&stream_index, state)| {
                let report = state.muxer.report();
                StreamHashRecord {
                    stream_index,
                    stream_type: state.stream_type,
                    algorithm: report.algorithm(),
                    digest: report.digest().clone(),
                    packets: report.packets(),
                    bytes: report.bytes(),
                }
            })
            .collect()
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        for record in self.records() {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamHashState {
    stream_type: StreamHashStreamType,
    muxer: HashMuxer,
}

impl StreamHashState {
    fn new(algorithm: HashAlgorithm, stream_type: StreamHashStreamType) -> Self {
        Self {
            stream_type,
            muxer: HashMuxer::new(algorithm),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{digest_to_hex, md5, sha256, AvErrorKind};

    #[test]
    fn hashes_each_stream_independently_in_stream_index_order() {
        let mut muxer = StreamHashMuxer::new(HashAlgorithm::Sha256);
        muxer
            .write_packet(&Packet::new(b"a".to_vec(), 2), StreamHashStreamType::Video)
            .unwrap();
        muxer
            .write_packet(&Packet::new(b"b".to_vec(), 0), StreamHashStreamType::Audio)
            .unwrap();
        muxer
            .write_packet(&Packet::new(b"c".to_vec(), 2), StreamHashStreamType::Video)
            .unwrap();
        muxer
            .write_packet(&Packet::new(b"d".to_vec(), 0), StreamHashStreamType::Audio)
            .unwrap();

        let records = muxer.records();

        assert_eq!(muxer.algorithm(), HashAlgorithm::Sha256);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].stream_index(), 0);
        assert_eq!(records[0].stream_type(), StreamHashStreamType::Audio);
        assert_eq!(records[0].algorithm(), HashAlgorithm::Sha256);
        assert_eq!(
            records[0].digest(),
            &HashDigest::Bytes(sha256(b"bd").to_vec())
        );
        assert_eq!(records[0].packets(), 2);
        assert_eq!(records[0].bytes(), 2);
        assert_eq!(
            records[0].line(),
            format!("0,a,SHA256={}\n", digest_to_hex(&sha256(b"bd")))
        );
        assert_eq!(records[1].stream_index(), 2);
        assert_eq!(records[1].stream_type(), StreamHashStreamType::Video);
        assert_eq!(
            records[1].digest(),
            &HashDigest::Bytes(sha256(b"ac").to_vec())
        );
        assert_eq!(
            muxer.render(),
            format!(
                "0,a,SHA256={}\n2,v,SHA256={}\n",
                digest_to_hex(&sha256(b"bd")),
                digest_to_hex(&sha256(b"ac"))
            )
        );
    }

    #[test]
    fn supports_md5_and_empty_packets() {
        let mut muxer = StreamHashMuxer::new(HashAlgorithm::Md5);

        muxer
            .write_packet(&Packet::new(Vec::new(), 0), StreamHashStreamType::Video)
            .unwrap();
        let records = muxer.records();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].digest(), &HashDigest::Bytes(md5(b"").to_vec()));
        assert_eq!(records[0].packets(), 1);
        assert_eq!(records[0].bytes(), 0);
        assert_eq!(
            records[0].line(),
            format!("0,v,MD5={}\n", digest_to_hex(&md5(b"")))
        );
    }

    #[test]
    fn rejects_conflicting_stream_types_without_changing_digest() {
        let mut muxer = StreamHashMuxer::new(HashAlgorithm::Sha256);
        muxer
            .write_packet(
                &Packet::new(b"abc".to_vec(), 0),
                StreamHashStreamType::Video,
            )
            .unwrap();
        let before = muxer.records();

        let err = muxer
            .write_packet(
                &Packet::new(b"def".to_vec(), 0),
                StreamHashStreamType::Audio,
            )
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.records(), before);
    }

    #[test]
    fn finish_prevents_more_writes() {
        let mut muxer = StreamHashMuxer::new(HashAlgorithm::Sha256);
        let packet = Packet::new(vec![1], 0);

        muxer
            .write_packet(&packet, StreamHashStreamType::Unknown)
            .unwrap();
        let output = muxer.finish();
        let err = muxer
            .write_packet(&packet, StreamHashStreamType::Unknown)
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert!(muxer.is_finished());
        assert_eq!(muxer.render(), output);
    }
}
