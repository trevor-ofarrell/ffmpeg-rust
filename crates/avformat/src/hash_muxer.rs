use avutil::{Adler32, AvError, AvResult, Crc32, Packet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Adler32,
    Crc32,
}

impl HashAlgorithm {
    pub fn name(self) -> &'static str {
        match self {
            Self::Adler32 => "ADLER32",
            Self::Crc32 => "CRC32",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMuxerReport {
    algorithm: HashAlgorithm,
    digest: u32,
    packets: u64,
    bytes: u64,
}

impl HashMuxerReport {
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    pub fn digest(&self) -> u32 {
        self.digest
    }

    pub fn packets(&self) -> u64 {
        self.packets
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn line(&self) -> String {
        format!("{}={:08x}\n", self.algorithm.name(), self.digest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMuxer {
    state: HashState,
    packets: u64,
    bytes: u64,
    finished: bool,
}

impl HashMuxer {
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            state: HashState::new(algorithm),
            packets: 0,
            bytes: 0,
            finished: false,
        }
    }

    pub fn algorithm(&self) -> HashAlgorithm {
        self.state.algorithm()
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after hash muxer is finished",
            ));
        }

        let packet_bytes = u64::try_from(packet.data().len())
            .map_err(|_| AvError::invalid_argument("packet size does not fit u64"))?;
        self.state.update(packet.data());
        self.packets = self
            .packets
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("hash muxer packet count overflow"))?;
        self.bytes = self
            .bytes
            .checked_add(packet_bytes)
            .ok_or_else(|| AvError::invalid_argument("hash muxer byte count overflow"))?;
        Ok(())
    }

    pub fn finish(&mut self) -> HashMuxerReport {
        self.finished = true;
        self.report()
    }

    pub fn report(&self) -> HashMuxerReport {
        HashMuxerReport {
            algorithm: self.algorithm(),
            digest: self.state.digest(),
            packets: self.packets,
            bytes: self.bytes,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HashState {
    Adler32(Adler32),
    Crc32(Crc32),
}

impl HashState {
    fn new(algorithm: HashAlgorithm) -> Self {
        match algorithm {
            HashAlgorithm::Adler32 => Self::Adler32(Adler32::new()),
            HashAlgorithm::Crc32 => Self::Crc32(Crc32::new()),
        }
    }

    fn algorithm(&self) -> HashAlgorithm {
        match self {
            Self::Adler32(_) => HashAlgorithm::Adler32,
            Self::Crc32(_) => HashAlgorithm::Crc32,
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Adler32(state) => state.update(data),
            Self::Crc32(state) => state.update(data),
        }
    }

    fn digest(&self) -> u32 {
        match self {
            Self::Adler32(state) => state.finalize(),
            Self::Crc32(state) => state.finalize(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{adler32, crc32_ieee, AvErrorKind};

    #[test]
    fn crc32_hashes_packet_data_in_write_order() {
        let mut muxer = HashMuxer::new(HashAlgorithm::Crc32);

        muxer
            .write_packet(&Packet::new(b"The quick ".to_vec(), 0))
            .unwrap();
        muxer
            .write_packet(&Packet::new(b"brown fox".to_vec(), 1))
            .unwrap();
        let report = muxer.finish();

        assert_eq!(report.algorithm(), HashAlgorithm::Crc32);
        assert_eq!(report.digest(), crc32_ieee(b"The quick brown fox"));
        assert_eq!(report.packets(), 2);
        assert_eq!(report.bytes(), 19);
        assert_eq!(report.line(), "CRC32=b74574de\n");
    }

    #[test]
    fn adler32_hashes_packet_data_in_write_order() {
        let mut muxer = HashMuxer::new(HashAlgorithm::Adler32);

        muxer
            .write_packet(&Packet::new(b"1234".to_vec(), 0))
            .unwrap();
        muxer
            .write_packet(&Packet::new(b"56789".to_vec(), 0))
            .unwrap();
        let report = muxer.finish();

        assert_eq!(report.digest(), adler32(b"123456789"));
        assert_eq!(report.line(), "ADLER32=091e01de\n");
    }

    #[test]
    fn empty_packets_count_but_do_not_change_empty_digest() {
        let mut muxer = HashMuxer::new(HashAlgorithm::Crc32);

        muxer.write_packet(&Packet::new(Vec::new(), 0)).unwrap();
        let report = muxer.finish();

        assert_eq!(report.digest(), crc32_ieee(b""));
        assert_eq!(report.packets(), 1);
        assert_eq!(report.bytes(), 0);
    }

    #[test]
    fn finish_prevents_more_writes() {
        let mut muxer = HashMuxer::new(HashAlgorithm::Adler32);
        let packet = Packet::new(vec![1, 2, 3], 0);

        muxer.write_packet(&packet).unwrap();
        muxer.finish();
        let err = muxer.write_packet(&packet).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert!(muxer.is_finished());
        assert_eq!(muxer.report().packets(), 1);
    }
}
