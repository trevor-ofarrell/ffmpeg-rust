use avutil::{digest_to_hex, Adler32, AvError, AvResult, Crc32, Md5, Packet, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Adler32,
    Crc32,
    Md5,
    Sha256,
}

impl HashAlgorithm {
    pub fn name(self) -> &'static str {
        match self {
            Self::Adler32 => "ADLER32",
            Self::Crc32 => "CRC32",
            Self::Md5 => "MD5",
            Self::Sha256 => "SHA256",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashDigest {
    U32(u32),
    Bytes(Vec<u8>),
}

impl HashDigest {
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::U32(value) => Some(*value),
            Self::Bytes(_) => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::U32(_) => None,
            Self::Bytes(bytes) => Some(bytes),
        }
    }

    pub fn hex(&self) -> String {
        match self {
            Self::U32(value) => format!("{value:08x}"),
            Self::Bytes(bytes) => digest_to_hex(bytes),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMuxerReport {
    algorithm: HashAlgorithm,
    digest: HashDigest,
    packets: u64,
    bytes: u64,
}

impl HashMuxerReport {
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
        format!("{}={}\n", self.algorithm.name(), self.digest_hex())
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
    Md5(Md5),
    Sha256(Sha256),
}

impl HashState {
    fn new(algorithm: HashAlgorithm) -> Self {
        match algorithm {
            HashAlgorithm::Adler32 => Self::Adler32(Adler32::new()),
            HashAlgorithm::Crc32 => Self::Crc32(Crc32::new()),
            HashAlgorithm::Md5 => Self::Md5(Md5::new()),
            HashAlgorithm::Sha256 => Self::Sha256(Sha256::new()),
        }
    }

    fn algorithm(&self) -> HashAlgorithm {
        match self {
            Self::Adler32(_) => HashAlgorithm::Adler32,
            Self::Crc32(_) => HashAlgorithm::Crc32,
            Self::Md5(_) => HashAlgorithm::Md5,
            Self::Sha256(_) => HashAlgorithm::Sha256,
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Adler32(state) => state.update(data),
            Self::Crc32(state) => state.update(data),
            Self::Md5(state) => state.update(data),
            Self::Sha256(state) => state.update(data),
        }
    }

    fn digest(&self) -> HashDigest {
        match self {
            Self::Adler32(state) => HashDigest::U32(state.finalize()),
            Self::Crc32(state) => HashDigest::U32(state.finalize()),
            Self::Md5(state) => HashDigest::Bytes(state.clone().finalize().to_vec()),
            Self::Sha256(state) => HashDigest::Bytes(state.clone().finalize().to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{adler32, crc32_ieee, md5, sha256, AvErrorKind};

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
        assert_eq!(
            report.digest(),
            &HashDigest::U32(crc32_ieee(b"The quick brown fox"))
        );
        assert_eq!(
            report.digest().as_u32(),
            Some(crc32_ieee(b"The quick brown fox"))
        );
        assert_eq!(report.digest_hex(), "b74574de");
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

        assert_eq!(report.digest(), &HashDigest::U32(adler32(b"123456789")));
        assert_eq!(report.digest_hex(), "091e01de");
        assert_eq!(report.line(), "ADLER32=091e01de\n");
    }

    #[test]
    fn md5_hashes_packet_data_in_write_order() {
        let mut muxer = HashMuxer::new(HashAlgorithm::Md5);

        muxer.write_packet(&Packet::new(b"ab".to_vec(), 0)).unwrap();
        muxer.write_packet(&Packet::new(b"c".to_vec(), 0)).unwrap();
        let report = muxer.finish();
        let expected = md5(b"abc");

        assert_eq!(report.algorithm(), HashAlgorithm::Md5);
        assert_eq!(report.digest(), &HashDigest::Bytes(expected.to_vec()));
        assert_eq!(report.digest().as_bytes(), Some(expected.as_slice()));
        assert_eq!(report.digest_hex(), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(report.packets(), 2);
        assert_eq!(report.bytes(), 3);
        assert_eq!(report.line(), "MD5=900150983cd24fb0d6963f7d28e17f72\n");
    }

    #[test]
    fn sha256_hashes_packet_data_in_write_order() {
        let mut muxer = HashMuxer::new(HashAlgorithm::Sha256);

        muxer.write_packet(&Packet::new(b"ab".to_vec(), 0)).unwrap();
        muxer.write_packet(&Packet::new(b"c".to_vec(), 0)).unwrap();
        let report = muxer.finish();
        let expected = sha256(b"abc");

        assert_eq!(report.algorithm(), HashAlgorithm::Sha256);
        assert_eq!(report.digest(), &HashDigest::Bytes(expected.to_vec()));
        assert_eq!(report.digest().as_bytes(), Some(expected.as_slice()));
        assert_eq!(
            report.digest_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(report.packets(), 2);
        assert_eq!(report.bytes(), 3);
        assert_eq!(
            report.line(),
            "SHA256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n"
        );
    }

    #[test]
    fn empty_packets_count_but_do_not_change_empty_digest() {
        let mut muxer = HashMuxer::new(HashAlgorithm::Crc32);

        muxer.write_packet(&Packet::new(Vec::new(), 0)).unwrap();
        let report = muxer.finish();

        assert_eq!(report.digest(), &HashDigest::U32(crc32_ieee(b"")));
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
