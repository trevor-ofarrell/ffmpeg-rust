use avutil::{AvError, AvErrorKind, AvResult, ByteReader, Packet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavInfo {
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    data_size: usize,
}

impl WavInfo {
    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn byte_rate(&self) -> u32 {
        self.byte_rate
    }

    pub fn block_align(&self) -> u16 {
        self.block_align
    }

    pub fn bits_per_sample(&self) -> u16 {
        self.bits_per_sample
    }

    pub fn data_size(&self) -> usize {
        self.data_size
    }

    pub fn samples_per_channel(&self) -> usize {
        self.data_size / usize::from(self.block_align)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavDemuxer<'a> {
    info: WavInfo,
    data: &'a [u8],
    consumed: bool,
}

impl<'a> WavDemuxer<'a> {
    pub fn open(input: &'a [u8]) -> AvResult<Self> {
        let mut reader = ByteReader::new(input);
        expect_fourcc(&mut reader, b"RIFF")?;
        let riff_size = reader.read_u32_le()?;
        expect_fourcc(&mut reader, b"WAVE")?;

        let riff_end = usize::try_from(riff_size)
            .ok()
            .and_then(|size| size.checked_add(8))
            .ok_or_else(|| AvError::invalid_data("WAV RIFF size is out of range"))?;
        if riff_end > input.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "WAV RIFF size exceeds input length",
            ));
        }

        let mut fmt = None;
        let mut data = None;

        while reader.position() < riff_end {
            let chunk_id = read_fourcc(&mut reader)?;
            let chunk_size = usize::try_from(reader.read_u32_le()?)
                .map_err(|_| AvError::invalid_data("WAV chunk size is out of range"))?;
            let chunk_start = reader.position();
            let chunk_end = chunk_start
                .checked_add(chunk_size)
                .ok_or_else(|| AvError::invalid_data("WAV chunk size overflow"))?;
            if chunk_end > riff_end || chunk_end > input.len() {
                return Err(AvError::new(
                    AvErrorKind::EndOfFile,
                    "WAV chunk exceeds RIFF bounds",
                ));
            }

            match &chunk_id {
                b"fmt " => fmt = Some(parse_fmt(reader.read_exact(chunk_size)?)?),
                b"data" => {
                    data = Some(&input[chunk_start..chunk_end]);
                    reader.skip(chunk_size)?;
                }
                _ => reader.skip(chunk_size)?,
            }

            if chunk_size % 2 == 1 && reader.position() < riff_end {
                reader.skip(1)?;
            }
        }

        let mut info = fmt.ok_or_else(|| AvError::invalid_data("WAV missing fmt chunk"))?;
        let data = data.ok_or_else(|| AvError::invalid_data("WAV missing data chunk"))?;
        info.data_size = data.len();
        validate_pcm_s16le(&info)?;

        Ok(Self {
            info,
            data,
            consumed: false,
        })
    }

    pub fn info(&self) -> &WavInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        if self.consumed {
            return Ok(None);
        }

        self.consumed = true;
        let mut packet = Packet::new(self.data.to_vec(), 0);
        packet.set_pts(Some(0));
        packet.set_dts(Some(0));
        packet.set_duration(
            i64::try_from(self.info.samples_per_channel())
                .map_err(|_| AvError::invalid_data("WAV sample count does not fit i64"))?,
        )?;
        Ok(Some(packet))
    }
}

fn parse_fmt(data: &[u8]) -> AvResult<WavInfo> {
    if data.len() < 16 {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "WAV fmt chunk is shorter than 16 bytes",
        ));
    }

    let mut reader = ByteReader::new(data);
    let audio_format = reader.read_u16_le()?;
    if audio_format != 1 {
        return Err(AvError::unsupported(format!(
            "unsupported WAV audio format {audio_format}"
        )));
    }

    Ok(WavInfo {
        channels: reader.read_u16_le()?,
        sample_rate: reader.read_u32_le()?,
        byte_rate: reader.read_u32_le()?,
        block_align: reader.read_u16_le()?,
        bits_per_sample: reader.read_u16_le()?,
        data_size: 0,
    })
}

fn validate_pcm_s16le(info: &WavInfo) -> AvResult<()> {
    if info.channels == 0 {
        return Err(AvError::invalid_data("WAV channel count must be non-zero"));
    }
    if info.sample_rate == 0 {
        return Err(AvError::invalid_data("WAV sample rate must be non-zero"));
    }
    if info.bits_per_sample != 16 {
        return Err(AvError::unsupported(format!(
            "unsupported WAV bits per sample {}",
            info.bits_per_sample
        )));
    }

    let expected_block_align = info
        .channels
        .checked_mul(2)
        .ok_or_else(|| AvError::invalid_data("WAV block align overflow"))?;
    if info.block_align != expected_block_align {
        return Err(AvError::invalid_data(format!(
            "WAV block align {} does not match expected {expected_block_align}",
            info.block_align
        )));
    }

    let expected_byte_rate = info
        .sample_rate
        .checked_mul(u32::from(info.block_align))
        .ok_or_else(|| AvError::invalid_data("WAV byte rate overflow"))?;
    if info.byte_rate != expected_byte_rate {
        return Err(AvError::invalid_data(format!(
            "WAV byte rate {} does not match expected {expected_byte_rate}",
            info.byte_rate
        )));
    }

    if info.data_size % usize::from(info.block_align) != 0 {
        return Err(AvError::invalid_data(
            "WAV data chunk does not contain whole sample frames",
        ));
    }

    Ok(())
}

fn read_fourcc(reader: &mut ByteReader<'_>) -> AvResult<[u8; 4]> {
    let bytes = reader.read_exact(4)?;
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn expect_fourcc(reader: &mut ByteReader<'_>, expected: &[u8; 4]) -> AvResult<()> {
    let actual = read_fourcc(reader)?;
    if &actual != expected {
        return Err(AvError::invalid_data(format!(
            "expected FourCC `{}`, found `{}`",
            String::from_utf8_lossy(expected),
            String::from_utf8_lossy(&actual)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pcm_s16le_wav_and_returns_single_packet() {
        let bytes = wav_bytes(2, 48_000, &[0, 0, 1, 0, 2, 0, 3, 0]);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().channels(), 2);
        assert_eq!(demuxer.info().sample_rate(), 48_000);
        assert_eq!(demuxer.info().bits_per_sample(), 16);
        assert_eq!(demuxer.info().samples_per_channel(), 2);

        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), &[0, 0, 1, 0, 2, 0, 3, 0]);
        assert_eq!(packet.pts(), Some(0));
        assert_eq!(packet.dts(), Some(0));
        assert_eq!(packet.duration(), 2);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn skips_unknown_chunks_and_odd_chunk_padding() {
        let bytes = wav_bytes_with_unknown_chunk(1, 44_100, &[1, 0, 2, 0]);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().channels(), 1);
        assert_eq!(demuxer.read_packet().unwrap().unwrap().duration(), 2);
    }

    #[test]
    fn rejects_missing_or_invalid_required_chunks() {
        assert_eq!(
            WavDemuxer::open(b"not a wav").unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            WavDemuxer::open(&wav_with_bad_wave_fourcc())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert!(WavDemuxer::open(&wav_without_data_chunk()).is_err());
        assert!(WavDemuxer::open(&wav_with_audio_format(3)).is_err());
    }

    #[test]
    fn validates_pcm_s16le_format_consistency() {
        assert!(WavDemuxer::open(&wav_with_zero_channels()).is_err());
        assert!(WavDemuxer::open(&wav_with_zero_sample_rate()).is_err());
        assert!(WavDemuxer::open(&wav_with_bits_per_sample(24)).is_err());
        assert!(WavDemuxer::open(&wav_with_bad_block_align()).is_err());
        assert!(WavDemuxer::open(&wav_with_bad_byte_rate()).is_err());
        assert!(WavDemuxer::open(&wav_bytes(2, 48_000, &[0, 1, 2])).is_err());
    }

    fn wav_bytes(channels: u16, sample_rate: u32, data: &[u8]) -> Vec<u8> {
        wav_bytes_inner(channels, sample_rate, 1, 16, data, &[])
    }

    fn wav_bytes_with_unknown_chunk(channels: u16, sample_rate: u32, data: &[u8]) -> Vec<u8> {
        wav_bytes_inner(channels, sample_rate, 1, 16, data, b"JUNK\x03\0\0\0abc\0")
    }

    fn wav_without_data_chunk() -> Vec<u8> {
        let mut body = fmt_chunk(1, 1, 48_000, 16);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.append(&mut body);
        out
    }

    fn wav_with_bad_wave_fourcc() -> Vec<u8> {
        let mut out = wav_bytes(1, 48_000, &[0, 0]);
        out[8..12].copy_from_slice(b"WEBM");
        out
    }

    fn wav_with_audio_format(audio_format: u16) -> Vec<u8> {
        wav_bytes_inner(1, 48_000, audio_format, 16, &[0, 0], &[])
    }

    fn wav_with_zero_channels() -> Vec<u8> {
        wav_bytes_inner(0, 48_000, 1, 16, &[], &[])
    }

    fn wav_with_zero_sample_rate() -> Vec<u8> {
        wav_bytes_inner(1, 0, 1, 16, &[0, 0], &[])
    }

    fn wav_with_bits_per_sample(bits_per_sample: u16) -> Vec<u8> {
        wav_bytes_inner(1, 48_000, 1, bits_per_sample, &[0, 0, 0], &[])
    }

    fn wav_with_bad_block_align() -> Vec<u8> {
        let mut out = wav_bytes(2, 48_000, &[0, 0, 1, 0]);
        let block_align_offset = 12 + 8 + 12;
        out[block_align_offset..block_align_offset + 2].copy_from_slice(&2_u16.to_le_bytes());
        out
    }

    fn wav_with_bad_byte_rate() -> Vec<u8> {
        let mut out = wav_bytes(1, 48_000, &[0, 0]);
        let byte_rate_offset = 12 + 8 + 8;
        out[byte_rate_offset..byte_rate_offset + 4].copy_from_slice(&1_u32.to_le_bytes());
        out
    }

    fn wav_bytes_inner(
        channels: u16,
        sample_rate: u32,
        audio_format: u16,
        bits_per_sample: u16,
        data: &[u8],
        extra_chunk: &[u8],
    ) -> Vec<u8> {
        let mut body = fmt_chunk(audio_format, channels, sample_rate, bits_per_sample);
        body.extend_from_slice(extra_chunk);
        body.extend_from_slice(b"data");
        body.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            body.push(0);
        }

        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(&body);
        out
    }

    fn fmt_chunk(
        audio_format: u16,
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u16,
    ) -> Vec<u8> {
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * u32::from(block_align);
        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"fmt ");
        chunk.extend_from_slice(&16_u32.to_le_bytes());
        chunk.extend_from_slice(&audio_format.to_le_bytes());
        chunk.extend_from_slice(&channels.to_le_bytes());
        chunk.extend_from_slice(&sample_rate.to_le_bytes());
        chunk.extend_from_slice(&byte_rate.to_le_bytes());
        chunk.extend_from_slice(&block_align.to_le_bytes());
        chunk.extend_from_slice(&bits_per_sample.to_le_bytes());
        chunk
    }
}
