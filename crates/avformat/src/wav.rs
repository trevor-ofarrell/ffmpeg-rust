use crate::AudioStreamParameters;
use avutil::{
    AvError, AvErrorKind, AvResult, ByteReader, ByteWriter, ChannelLayout, Packet, SampleFormat,
};

const PCM_S16LE_FORMAT_TAG: u16 = 1;
const WAV_FORMAT_EXTENSIBLE_FORMAT_TAG: u16 = 0xFFFE;
const WAV_FMT_CHUNK_SIZE: u32 = 16;
const WAV_FORMAT_EXTENSIBLE_CHUNK_SIZE: usize = 40;
const WAV_ENCODER_NAME: &[u8; 14] = b"Lavf62.12.101\0";
const WAV_INFO_LIST_CHUNK_SIZE: u32 = 4 + 8 + WAV_ENCODER_NAME.len() as u32;
const WAV_INFO_LIST_TOTAL_SIZE: usize = 8 + WAV_INFO_LIST_CHUNK_SIZE as usize;
const WAV_HEADER_SIZE: usize = 44 + WAV_INFO_LIST_TOTAL_SIZE;
const MAX_RIFF_DATA_SIZE: usize = u32::MAX as usize - 36 - WAV_INFO_LIST_TOTAL_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavInfo {
    audio: AudioStreamParameters,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    data_size: usize,
}

impl WavInfo {
    pub fn channels(&self) -> u16 {
        self.audio.channels()
    }

    pub fn channel_layout(&self) -> Option<ChannelLayout> {
        self.audio.channel_layout()
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.audio.sample_format()
    }

    pub fn sample_rate(&self) -> u32 {
        self.audio.sample_rate()
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
        self.data_size / self.audio.bytes_per_sample_frame()
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
        if riff_size < 4 {
            return Err(AvError::invalid_data(
                "WAV RIFF size is too small for WAVE form type",
            ));
        }
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
                b"fmt " => {
                    let parsed_fmt = parse_fmt(reader.read_exact(chunk_size)?)?;
                    if fmt.is_none() {
                        fmt = Some(parsed_fmt);
                    }
                }
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

        if self.data.is_empty() {
            self.consumed = true;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavMuxer {
    info: WavInfo,
    data: Vec<u8>,
    packets: u64,
    finished: bool,
}

impl WavMuxer {
    pub fn new_pcm_s16le(channels: u16, sample_rate: u32) -> AvResult<Self> {
        let audio =
            AudioStreamParameters::with_context(sample_rate, channels, SampleFormat::S16, "WAV")?;
        let block_align = block_align_for_audio(&audio, AvErrorKind::InvalidArgument)?;
        let byte_rate = byte_rate_for_audio(&audio, block_align, AvErrorKind::InvalidArgument)?;
        let bits_per_sample = audio.bits_per_sample()?;
        let info = WavInfo {
            audio,
            byte_rate,
            block_align,
            bits_per_sample,
            data_size: 0,
        };
        validate_pcm_s16le(&info)?;

        Ok(Self {
            info,
            data: Vec::new(),
            packets: 0,
            finished: false,
        })
    }

    pub fn info(&self) -> &WavInfo {
        &self.info
    }

    pub fn packets(&self) -> u64 {
        self.packets
    }

    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after WAV muxer is finished",
            ));
        }
        if packet.stream_index() != 0 {
            return Err(AvError::invalid_argument(format!(
                "WAV muxer only accepts stream 0, got stream {}",
                packet.stream_index()
            )));
        }
        let new_len = self
            .data
            .len()
            .checked_add(packet.data().len())
            .ok_or_else(|| AvError::invalid_argument("WAV data size overflow"))?;
        validate_data_len(new_len)?;

        self.data.extend_from_slice(packet.data());
        self.info.data_size = self.data.len();
        self.packets = self
            .packets
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("WAV packet count overflow"))?;
        Ok(())
    }

    pub fn render(&self) -> AvResult<Vec<u8>> {
        validate_data_len(self.data.len())?;
        let data_size = self.data.len();
        let padded_data_size = data_size
            .checked_add(data_size & 1)
            .ok_or_else(|| AvError::invalid_argument("WAV data size overflow"))?;
        let riff_size = u32::try_from(36 + WAV_INFO_LIST_TOTAL_SIZE + padded_data_size)
            .map_err(|_| AvError::invalid_argument("WAV RIFF size does not fit u32"))?;
        let data_size = u32::try_from(data_size)
            .map_err(|_| AvError::invalid_argument("WAV data size does not fit u32"))?;

        let mut writer = ByteWriter::with_capacity(WAV_HEADER_SIZE + self.data.len());
        writer.write_all(b"RIFF");
        writer.write_u32_le(riff_size);
        writer.write_all(b"WAVE");
        writer.write_all(b"fmt ");
        writer.write_u32_le(WAV_FMT_CHUNK_SIZE);
        writer.write_u16_le(PCM_S16LE_FORMAT_TAG);
        writer.write_u16_le(self.info.channels());
        writer.write_u32_le(self.info.sample_rate());
        writer.write_u32_le(self.info.byte_rate);
        writer.write_u16_le(self.info.block_align);
        writer.write_u16_le(self.info.bits_per_sample);
        write_ffmpeg_info_chunk(&mut writer);
        writer.write_all(b"data");
        writer.write_u32_le(data_size);
        writer.write_all(&self.data);
        if data_size % 2 == 1 {
            writer.write_u8(0);
        }
        Ok(writer.into_inner())
    }

    pub fn finish(&mut self) -> AvResult<Vec<u8>> {
        self.finished = true;
        self.render()
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
    match audio_format {
        PCM_S16LE_FORMAT_TAG => {}
        WAV_FORMAT_EXTENSIBLE_FORMAT_TAG => {
            if data.len() < WAV_FORMAT_EXTENSIBLE_CHUNK_SIZE {
                return Err(AvError::invalid_data(
                    "WAV extensible fmt chunk is smaller than required 40 bytes",
                ));
            }

            let cb_size = u16::from_le_bytes([data[16], data[17]]);
            if cb_size < 22 {
                return Err(AvError::invalid_data(
                    "WAV extensible fmt chunk does not include required extension",
                ));
            }

            if &data[24..40] != WAV_FORMAT_EXTENSIBLE_PCM_GUID.as_slice() {
                return Err(AvError::unsupported(
                    "unsupported WAV extensible sub-format",
                ));
            }
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "unsupported WAV audio format {audio_format}"
            )));
        }
    }

    let channels = reader.read_u16_le()?;
    let sample_rate = reader.read_u32_le()?;
    let audio =
        AudioStreamParameters::from_container(sample_rate, channels, SampleFormat::S16, "WAV")?;
    Ok(WavInfo {
        audio,
        byte_rate: reader.read_u32_le()?,
        block_align: reader.read_u16_le()?,
        bits_per_sample: reader.read_u16_le()?,
        data_size: 0,
    })
}

const WAV_FORMAT_EXTENSIBLE_PCM_GUID: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

fn validate_pcm_s16le(info: &WavInfo) -> AvResult<()> {
    if info.sample_format() != SampleFormat::S16 {
        return Err(AvError::unsupported("unsupported WAV sample format"));
    }

    let expected_bits_per_sample = info.audio.bits_per_sample()?;
    if info.bits_per_sample != expected_bits_per_sample {
        return Err(AvError::unsupported(format!(
            "unsupported WAV bits per sample {}",
            info.bits_per_sample
        )));
    }

    let expected_block_align = block_align_for_audio(&info.audio, AvErrorKind::InvalidData)?;
    if info.block_align != expected_block_align {
        return Err(AvError::invalid_data(format!(
            "WAV block align {} does not match expected {expected_block_align}",
            info.block_align
        )));
    }

    let expected_byte_rate =
        byte_rate_for_audio(&info.audio, info.block_align, AvErrorKind::InvalidData)?;
    if info.byte_rate != expected_byte_rate {
        return Err(AvError::invalid_data(format!(
            "WAV byte rate {} does not match expected {expected_byte_rate}",
            info.byte_rate
        )));
    }

    Ok(())
}

fn block_align_for_audio(audio: &AudioStreamParameters, error_kind: AvErrorKind) -> AvResult<u16> {
    u16::try_from(audio.bytes_per_sample_frame())
        .map_err(|_| AvError::new(error_kind, "WAV block align does not fit u16"))
}

fn byte_rate_for_audio(
    audio: &AudioStreamParameters,
    block_align: u16,
    error_kind: AvErrorKind,
) -> AvResult<u32> {
    audio
        .sample_rate()
        .checked_mul(u32::from(block_align))
        .ok_or_else(|| AvError::new(error_kind, "WAV byte rate overflow"))
}

fn validate_data_len(data_len: usize) -> AvResult<()> {
    let padded_data_len = data_len
        .checked_add(data_len & 1)
        .ok_or_else(|| AvError::invalid_argument("WAV data size overflow"))?;
    if padded_data_len > MAX_RIFF_DATA_SIZE {
        return Err(AvError::invalid_argument(
            "WAV data is too large for classic RIFF",
        ));
    }
    Ok(())
}

fn write_ffmpeg_info_chunk(writer: &mut ByteWriter) {
    writer.write_all(b"LIST");
    writer.write_u32_le(WAV_INFO_LIST_CHUNK_SIZE);
    writer.write_all(b"INFO");
    writer.write_all(b"ISFT");
    writer.write_u32_le(WAV_ENCODER_NAME.len() as u32);
    writer.write_all(WAV_ENCODER_NAME);
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
        assert_eq!(
            demuxer.info().channel_layout(),
            Some(ChannelLayout::stereo())
        );
        assert_eq!(demuxer.info().sample_format(), SampleFormat::S16);
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
        assert_eq!(demuxer.info().channel_layout(), Some(ChannelLayout::mono()));
        assert_eq!(demuxer.read_packet().unwrap().unwrap().duration(), 2);
    }

    #[test]
    fn uses_first_fmt_chunk_when_multiple_fmt_chunks_are_present() {
        let data = [0, 0, 1, 0, 2, 0, 3, 0];
        let bytes = wav_bytes_with_duplicate_fmt_chunks(1, 44_100, 2, 48_000, &data);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().channels(), 1);
        assert_eq!(demuxer.info().channel_layout(), Some(ChannelLayout::mono()));
        assert_eq!(demuxer.info().sample_rate(), 44_100);
        assert_eq!(demuxer.info().samples_per_channel(), 4);

        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), &data);
        assert_eq!(packet.duration(), 4);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn uses_last_data_chunk_when_multiple_data_chunks_are_present() {
        let first_payload = [0, 0, 1, 0, 2, 0, 3, 0];
        let second_payload = [0xAA, 0x00, 0xBB, 0x00];
        let bytes =
            wav_bytes_with_duplicate_data_chunks(2, 44_100, &first_payload, &second_payload);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), &second_payload);
        assert_eq!(packet.duration(), 1);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn uses_last_empty_data_chunk_when_multiple_data_chunks_are_present() {
        let first_payload = [0, 0, 1, 0, 2, 0, 3, 0];
        let bytes = wav_bytes_with_duplicate_data_chunks(2, 44_100, &first_payload, &[]);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().data_size(), 0);
        assert!(demuxer.read_packet().unwrap().is_none());
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
        assert_eq!(
            WavDemuxer::open(&wav_with_too_small_riff_size())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert!(WavDemuxer::open(&wav_without_data_chunk()).is_err());
        assert!(WavDemuxer::open(&wav_with_audio_format(3)).is_err());
        assert!(WavDemuxer::open(&wav_with_short_extensible_fmt_chunk()).is_err());
        assert!(WavDemuxer::open(&wav_with_small_extensible_cb_size()).is_err());
        assert!(WavDemuxer::open(&wav_with_extensible_non_pcm_subformat()).is_err());
        assert_eq!(
            WavDemuxer::open(&wav_with_short_pcm_fmt_chunk())
                .unwrap_err()
                .kind(),
            AvErrorKind::EndOfFile
        );
    }

    #[test]
    fn parses_pcm_s16le_wav_extensible_format() {
        let bytes = wav_bytes_extensible(1, 44_100, &[0x00, 0x00, 0x01, 0x00]);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().sample_format(), SampleFormat::S16);
        assert_eq!(demuxer.info().sample_rate(), 44_100);
        assert_eq!(demuxer.info().channels(), 1);

        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), &[0x00, 0x00, 0x01, 0x00]);
        assert_eq!(packet.duration(), 2);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn validates_pcm_s16le_format_consistency() {
        assert!(WavDemuxer::open(&wav_with_zero_channels()).is_err());
        assert!(WavDemuxer::open(&wav_with_zero_sample_rate()).is_err());
        assert!(WavDemuxer::open(&wav_with_bits_per_sample(24)).is_err());
        assert!(WavDemuxer::open(&wav_with_bad_block_align()).is_err());
        assert!(WavDemuxer::open(&wav_with_bad_byte_rate()).is_err());
        assert!(WavDemuxer::open(&wav_bytes(1, 48_000, &[0, 1, 2])).is_ok());
    }

    #[test]
    fn parses_partial_wav_data_without_rejecting_truncated_sample_frames() {
        let bytes = wav_bytes(1, 44_100, &[0, 0, 1]);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().samples_per_channel(), 1);
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), &[0, 0, 1]);
        assert_eq!(packet.duration(), 1);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn returns_no_packet_for_empty_data_chunk() {
        let bytes = wav_bytes(1, 44_100, &[]);
        let mut demuxer = WavDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().samples_per_channel(), 0);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_writes_pcm_s16le_header_and_payload() {
        let mut muxer = WavMuxer::new_pcm_s16le(2, 48_000).unwrap();
        let packet = Packet::new(vec![0, 0, 1, 0, 2, 0, 3, 0], 0);

        assert_eq!(muxer.info().channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(muxer.info().sample_format(), SampleFormat::S16);
        muxer.write_packet(&packet).unwrap();
        let bytes = muxer.finish().unwrap();

        assert!(muxer.is_finished());
        assert_eq!(muxer.packets(), 1);
        assert_eq!(muxer.data_len(), 8);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 78);
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 16);
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 2);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            48_000
        );
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            192_000
        );
        assert_eq!(u16::from_le_bytes(bytes[32..34].try_into().unwrap()), 4);
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
        assert_eq!(&bytes[36..40], b"LIST");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            WAV_INFO_LIST_CHUNK_SIZE
        );
        assert_eq!(&bytes[44..48], b"INFO");
        assert_eq!(&bytes[48..52], b"ISFT");
        assert_eq!(
            u32::from_le_bytes(bytes[52..56].try_into().unwrap()),
            WAV_ENCODER_NAME.len() as u32
        );
        assert_eq!(&bytes[56..70], WAV_ENCODER_NAME);
        assert_eq!(&bytes[70..74], b"data");
        assert_eq!(u32::from_le_bytes(bytes[74..78].try_into().unwrap()), 8);
        assert_eq!(&bytes[78..], &[0, 0, 1, 0, 2, 0, 3, 0]);
    }

    #[test]
    fn muxer_accepts_partial_sample_frames_and_emits_odd_wav_data_padding() {
        let mut muxer = WavMuxer::new_pcm_s16le(1, 44_100).unwrap();
        muxer.write_packet(&Packet::new(vec![0, 0, 1], 0)).unwrap();

        assert_eq!(muxer.info().data_size(), 3);
        let bytes = muxer.finish().unwrap();

        assert_eq!(muxer.packets(), 1);
        assert_eq!(bytes.len(), 82);
        assert_eq!(u32::from_le_bytes(bytes[74..78].try_into().unwrap()), 3);
        assert_eq!(bytes[81], 0);

        let mut demuxer = WavDemuxer::open(&bytes).unwrap();
        assert_eq!(demuxer.info().channels(), 1);
        assert_eq!(demuxer.info().sample_rate(), 44_100);
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), &[0, 0, 1]);
        assert_eq!(packet.duration(), 1);
    }

    #[test]
    fn muxer_renders_empty_wav_header_and_round_trips_through_demuxer() {
        let mut muxer = WavMuxer::new_pcm_s16le(1, 44_100).unwrap();
        let bytes = muxer.finish().unwrap();

        assert!(muxer.is_finished());
        assert_eq!(muxer.packets(), 0);
        assert_eq!(muxer.data_len(), 0);
        assert_eq!(bytes.len(), WAV_HEADER_SIZE);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 70);
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 16);
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            44_100
        );
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            88_200
        );
        assert_eq!(u16::from_le_bytes(bytes[32..34].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
        assert_eq!(&bytes[36..40], b"LIST");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            WAV_INFO_LIST_CHUNK_SIZE
        );
        assert_eq!(&bytes[44..48], b"INFO");
        assert_eq!(&bytes[48..52], b"ISFT");
        assert_eq!(
            u32::from_le_bytes(bytes[52..56].try_into().unwrap()),
            WAV_ENCODER_NAME.len() as u32
        );
        assert_eq!(&bytes[56..70], WAV_ENCODER_NAME);
        assert_eq!(&bytes[70..74], b"data");
        assert_eq!(u32::from_le_bytes(bytes[74..78].try_into().unwrap()), 0);

        let mut demuxer = WavDemuxer::open(&bytes).unwrap();
        assert_eq!(demuxer.info().channels(), 1);
        assert_eq!(demuxer.info().channel_layout(), Some(ChannelLayout::mono()));
        assert_eq!(demuxer.info().sample_format(), SampleFormat::S16);
        assert_eq!(demuxer.info().sample_rate(), 44_100);
        assert_eq!(demuxer.info().data_size(), 0);
        assert_eq!(demuxer.info().samples_per_channel(), 0);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_output_round_trips_through_demuxer() {
        let mut muxer = WavMuxer::new_pcm_s16le(1, 44_100).unwrap();
        muxer
            .write_packet(&Packet::new(vec![1, 0, 2, 0], 0))
            .unwrap();
        muxer.write_packet(&Packet::new(vec![3, 0], 0)).unwrap();
        let bytes = muxer.finish().unwrap();

        let mut demuxer = WavDemuxer::open(&bytes).unwrap();
        let packet = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(demuxer.info().channels(), 1);
        assert_eq!(demuxer.info().sample_format(), SampleFormat::S16);
        assert_eq!(demuxer.info().sample_rate(), 44_100);
        assert_eq!(packet.data(), &[1, 0, 2, 0, 3, 0]);
        assert_eq!(packet.duration(), 3);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_validates_stream_parameters_and_packets() {
        assert!(WavMuxer::new_pcm_s16le(0, 48_000).is_err());
        assert!(WavMuxer::new_pcm_s16le(1, 0).is_err());
        assert!(WavMuxer::new_pcm_s16le(32_768, 48_000).is_err());
        assert!(WavMuxer::new_pcm_s16le(2, u32::MAX).is_err());

        let mut muxer = WavMuxer::new_pcm_s16le(2, 48_000).unwrap();
        assert!(muxer.write_packet(&Packet::new(vec![0, 0], 1)).is_err());
        muxer.write_packet(&Packet::new(vec![0, 0, 1], 0)).unwrap();
        assert_eq!(muxer.data_len(), 3);
        assert_eq!(muxer.packets(), 1);
    }

    #[test]
    fn muxer_finish_prevents_more_writes() {
        let mut muxer = WavMuxer::new_pcm_s16le(1, 48_000).unwrap();
        let packet = Packet::new(vec![0, 0], 0);

        muxer.write_packet(&packet).unwrap();
        muxer.finish().unwrap();
        let err = muxer.write_packet(&packet).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.data_len(), 2);
        assert_eq!(muxer.packets(), 1);
    }

    fn wav_bytes_with_unknown_chunk(channels: u16, sample_rate: u32, data: &[u8]) -> Vec<u8> {
        wav_bytes_inner(channels, sample_rate, 1, 16, data, b"JUNK\x03\0\0\0abc\0")
    }

    fn wav_bytes_with_duplicate_fmt_chunks(
        first_channels: u16,
        first_sample_rate: u32,
        second_channels: u16,
        second_sample_rate: u32,
        data: &[u8],
    ) -> Vec<u8> {
        let mut body = fmt_chunk(1, first_channels, first_sample_rate, 16);
        body.extend_from_slice(&fmt_chunk(1, second_channels, second_sample_rate, 16));
        body.extend_from_slice(b"data");
        body.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            body.push(0);
        }

        wav_bytes_with_body(body)
    }

    fn wav_bytes_with_duplicate_data_chunks(
        channels: u16,
        sample_rate: u32,
        first_data: &[u8],
        second_data: &[u8],
    ) -> Vec<u8> {
        let mut body = fmt_chunk(1, channels, sample_rate, 16);
        body.extend_from_slice(b"data");
        body.extend_from_slice(&(u32::try_from(first_data.len()).unwrap()).to_le_bytes());
        body.extend_from_slice(first_data);
        if first_data.len() % 2 == 1 {
            body.push(0);
        }

        body.extend_from_slice(b"data");
        body.extend_from_slice(&(u32::try_from(second_data.len()).unwrap()).to_le_bytes());
        body.extend_from_slice(second_data);
        if second_data.len() % 2 == 1 {
            body.push(0);
        }

        wav_bytes_with_body(body)
    }

    fn wav_without_data_chunk() -> Vec<u8> {
        wav_bytes_with_body(fmt_chunk(1, 1, 48_000, 16))
    }

    fn wav_bytes(channels: u16, sample_rate: u32, data: &[u8]) -> Vec<u8> {
        wav_bytes_inner(channels, sample_rate, 1, 16, data, &[])
    }

    fn wav_bytes_with_body(body: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(&body);
        out
    }

    fn wav_with_bad_wave_fourcc() -> Vec<u8> {
        let mut out = wav_bytes(1, 48_000, &[0, 0]);
        out[8..12].copy_from_slice(b"WEBM");
        out
    }

    fn wav_with_too_small_riff_size() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0_u32.to_le_bytes());
        out
    }

    fn wav_with_short_pcm_fmt_chunk() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&20_u32.to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&8_u32.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
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

    fn wav_with_extensible_non_pcm_subformat() -> Vec<u8> {
        let mut bytes = wav_bytes_extensible(1, 44_100, &[0x00, 0x00, 0x01, 0x00]);
        let guid_offset = 12 + 8 + (2 + 2 + 4 + 4 + 2 + 2 + 2 + 2 + 4);
        bytes[guid_offset..guid_offset + 16].copy_from_slice(&[
            0x02, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xAA, 0, 0x38, 0x9B, 0x71,
        ]);
        bytes
    }

    fn wav_with_short_extensible_fmt_chunk() -> Vec<u8> {
        let mut payload = wav_extensible_fmt_chunk(1, 44_100).split_off(8);
        payload.truncate(38);
        wav_with_fmt_payload(payload, &[0x00, 0x00])
    }

    fn wav_with_small_extensible_cb_size() -> Vec<u8> {
        let mut payload = wav_extensible_fmt_chunk(1, 44_100).split_off(8);
        payload[16..18].copy_from_slice(&21_u16.to_le_bytes());
        wav_with_fmt_payload(payload, &[0x00, 0x00])
    }

    fn wav_with_fmt_payload(fmt_payload: Vec<u8>, data: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(b"fmt ");
        body.extend_from_slice(&(u32::try_from(fmt_payload.len()).unwrap()).to_le_bytes());
        body.extend_from_slice(&fmt_payload);
        body.extend_from_slice(b"data");
        body.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_le_bytes());
        body.extend_from_slice(data);
        wav_bytes_with_body(body)
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

        wav_bytes_with_body(body)
    }

    fn wav_bytes_extensible(channels: u16, sample_rate: u32, data: &[u8]) -> Vec<u8> {
        let mut body = wav_extensible_fmt_chunk(channels, sample_rate);
        body.extend_from_slice(b"data");
        body.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            body.push(0);
        }

        wav_bytes_with_body(body)
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

    fn wav_extensible_fmt_chunk(channels: u16, sample_rate: u32) -> Vec<u8> {
        let block_align = channels * 2;
        let byte_rate = sample_rate * u32::from(block_align);

        let mut chunk = Vec::new();
        chunk.extend_from_slice(b"fmt ");
        chunk.extend_from_slice(&40_u32.to_le_bytes());
        chunk.extend_from_slice(&WAV_FORMAT_EXTENSIBLE_FORMAT_TAG.to_le_bytes());
        chunk.extend_from_slice(&channels.to_le_bytes());
        chunk.extend_from_slice(&sample_rate.to_le_bytes());
        chunk.extend_from_slice(&byte_rate.to_le_bytes());
        chunk.extend_from_slice(&block_align.to_le_bytes());
        chunk.extend_from_slice(&16_u16.to_le_bytes());
        chunk.extend_from_slice(&22_u16.to_le_bytes());
        chunk.extend_from_slice(&16_u16.to_le_bytes());
        chunk.extend_from_slice(&3_u32.to_le_bytes());
        chunk.extend_from_slice(&WAV_FORMAT_EXTENSIBLE_PCM_GUID);
        chunk
    }
}
