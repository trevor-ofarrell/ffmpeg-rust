use avutil::{AudioFrame, AvError, AvResult, Frame, Packet, SampleFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmS16leDecoder {
    sample_rate: u32,
    channels: u16,
}

impl PcmS16leDecoder {
    pub const BYTES_PER_SAMPLE: usize = 2;

    pub fn new(sample_rate: u32, channels: u16) -> AvResult<Self> {
        if sample_rate == 0 {
            return Err(AvError::invalid_argument(
                "pcm_s16le sample rate must be non-zero",
            ));
        }

        if channels == 0 {
            return Err(AvError::invalid_argument(
                "pcm_s16le channel count must be non-zero",
            ));
        }

        Ok(Self {
            sample_rate,
            channels,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn bytes_per_sample_frame(&self) -> usize {
        usize::from(self.channels) * Self::BYTES_PER_SAMPLE
    }

    pub fn samples_per_channel(&self, byte_len: usize) -> AvResult<usize> {
        let frame_size = self.bytes_per_sample_frame();
        if byte_len % frame_size != 0 {
            return Err(AvError::invalid_data(format!(
                "pcm_s16le packet has {byte_len} bytes, not divisible by {frame_size}"
            )));
        }

        Ok(byte_len / frame_size)
    }

    pub fn decode_packet(&self, packet: &Packet) -> AvResult<Frame> {
        let samples_per_channel = self.samples_per_channel(packet.data().len())?;
        let audio = AudioFrame::new(
            self.sample_rate,
            self.channels,
            SampleFormat::S16,
            samples_per_channel,
            vec![packet.data().to_vec()],
        )?;
        let mut frame = Frame::audio(audio);
        frame.set_pts(packet.pts());
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{AvErrorKind, ChannelLayout, FrameData};

    #[test]
    fn decodes_stereo_s16le_packet_to_packed_audio_frame() {
        let decoder = PcmS16leDecoder::new(48_000, 2).unwrap();
        let mut packet = Packet::new(vec![0, 0, 1, 0, 2, 0, 3, 0], 0);
        packet.set_pts(Some(1024));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(frame.pts(), Some(1024));
        match frame.data() {
            FrameData::Audio(audio) => {
                assert_eq!(audio.sample_rate(), 48_000);
                assert_eq!(audio.channels(), 2);
                assert_eq!(audio.channel_layout(), Some(ChannelLayout::stereo()));
                assert_eq!(audio.sample_format(), SampleFormat::S16);
                assert_eq!(audio.sample_format_name(), "s16");
                assert_eq!(audio.samples_per_channel(), 2);
                assert_eq!(audio.planes(), &[vec![0, 0, 1, 0, 2, 0, 3, 0]]);
            }
            FrameData::Video(_) => panic!("expected audio frame"),
            FrameData::Empty => panic!("expected audio frame"),
        }
    }

    #[test]
    fn accepts_empty_packets_as_zero_samples() {
        let decoder = PcmS16leDecoder::new(44_100, 1).unwrap();
        let frame = decoder.decode_packet(&Packet::new(Vec::new(), 0)).unwrap();

        match frame.data() {
            FrameData::Audio(audio) => {
                assert_eq!(audio.channel_layout(), Some(ChannelLayout::mono()));
                assert_eq!(audio.samples_per_channel(), 0);
                assert_eq!(audio.planes(), &[Vec::<u8>::new()]);
            }
            FrameData::Video(_) => panic!("expected audio frame"),
            FrameData::Empty => panic!("expected audio frame"),
        }
    }

    #[test]
    fn validates_constructor_arguments() {
        assert_eq!(
            PcmS16leDecoder::new(0, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            PcmS16leDecoder::new(48_000, 0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rejects_packets_that_split_sample_frames() {
        let decoder = PcmS16leDecoder::new(48_000, 2).unwrap();
        let packet = Packet::new(vec![0, 1, 2], 0);

        let err = decoder.decode_packet(&packet).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidData);
    }

    #[test]
    fn computes_sample_counts_for_mono_and_multichannel_packets() {
        let mono = PcmS16leDecoder::new(48_000, 1).unwrap();
        let surround = PcmS16leDecoder::new(48_000, 6).unwrap();

        assert_eq!(mono.bytes_per_sample_frame(), 2);
        assert_eq!(mono.samples_per_channel(8).unwrap(), 4);
        assert_eq!(surround.bytes_per_sample_frame(), 12);
        assert_eq!(surround.samples_per_channel(24).unwrap(), 2);
    }
}
