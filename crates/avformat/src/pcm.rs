use crate::AudioStreamParameters;
use avutil::{AvError, AvResult, ChannelLayout, Packet, SampleFormat, SideData};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmS16leInfo {
    audio: AudioStreamParameters,
    packet_samples: usize,
    packet_size: usize,
    total_samples_per_channel: usize,
}

impl PcmS16leInfo {
    pub fn sample_rate(&self) -> u32 {
        self.audio.sample_rate()
    }

    pub fn channels(&self) -> u16 {
        self.audio.channels()
    }

    pub fn channel_layout(&self) -> Option<ChannelLayout> {
        self.audio.channel_layout()
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.audio.sample_format()
    }

    pub fn packet_samples(&self) -> usize {
        self.packet_samples
    }

    pub fn bytes_per_sample_frame(&self) -> usize {
        self.audio.bytes_per_sample_frame()
    }

    pub fn packet_size(&self) -> usize {
        self.packet_size
    }

    pub fn total_samples_per_channel(&self) -> usize {
        self.total_samples_per_channel
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmS16leMuxerInfo {
    audio: AudioStreamParameters,
    samples_per_channel: usize,
}

impl PcmS16leMuxerInfo {
    pub fn sample_rate(&self) -> u32 {
        self.audio.sample_rate()
    }

    pub fn channels(&self) -> u16 {
        self.audio.channels()
    }

    pub fn channel_layout(&self) -> Option<ChannelLayout> {
        self.audio.channel_layout()
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.audio.sample_format()
    }

    pub fn bytes_per_sample_frame(&self) -> usize {
        self.audio.bytes_per_sample_frame()
    }

    pub fn samples_per_channel(&self) -> usize {
        self.samples_per_channel
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmS16leDemuxer<'a> {
    info: PcmS16leInfo,
    input: &'a [u8],
    position: usize,
    next_pts: i64,
}

impl<'a> PcmS16leDemuxer<'a> {
    pub fn open(
        input: &'a [u8],
        sample_rate: u32,
        channels: u16,
        packet_samples: usize,
    ) -> AvResult<Self> {
        let audio = AudioStreamParameters::with_context(
            sample_rate,
            channels,
            SampleFormat::S16,
            "pcm_s16le",
        )?;
        if packet_samples == 0 {
            return Err(AvError::invalid_argument(
                "pcm_s16le packet samples must be non-zero",
            ));
        }

        let packet_size = audio
            .bytes_per_sample_frame()
            .checked_mul(packet_samples)
            .ok_or_else(|| AvError::invalid_argument("pcm_s16le packet size overflow"))?;

        let total_samples_per_channel =
            complete_sample_frames_in_bytes(input.len(), audio.bytes_per_sample_frame());

        Ok(Self {
            info: PcmS16leInfo {
                audio,
                packet_samples,
                packet_size,
                total_samples_per_channel,
            },
            input,
            position: 0,
            next_pts: 0,
        })
    }

    pub fn info(&self) -> &PcmS16leInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        if self.position == self.input.len() {
            return Ok(None);
        }

        let remaining = self.input.len() - self.position;
        let packet_len = remaining.min(self.info.packet_size);
        let samples =
            complete_sample_frames_in_bytes(packet_len, self.info.bytes_per_sample_frame());
        let duration = i64::try_from(samples)
            .map_err(|_| AvError::invalid_data("pcm_s16le packet duration does not fit i64"))?;
        let end = self
            .position
            .checked_add(packet_len)
            .ok_or_else(|| AvError::invalid_data("pcm_s16le packet end overflow"))?;

        let mut packet = Packet::new(self.input[self.position..end].to_vec(), 0);
        packet.set_pts(Some(self.next_pts));
        packet.set_dts(Some(self.next_pts));
        packet.set_duration(duration)?;
        packet.push_side_data(SideData::new("pcm_sample_fmt", b"s16".to_vec())?);
        packet.push_side_data(SideData::new(
            "pcm_channels",
            self.info.channels().to_string().into_bytes(),
        )?);
        self.position = end;
        self.next_pts = self
            .next_pts
            .checked_add(duration)
            .ok_or_else(|| AvError::invalid_data("pcm_s16le PTS overflow"))?;
        Ok(Some(packet))
    }
}

fn complete_sample_frames_in_bytes(byte_len: usize, bytes_per_sample_frame: usize) -> usize {
    byte_len / bytes_per_sample_frame
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmS16leMuxer {
    info: PcmS16leMuxerInfo,
    data: Vec<u8>,
    packets: u64,
    finished: bool,
}

impl PcmS16leMuxer {
    pub fn new(sample_rate: u32, channels: u16) -> AvResult<Self> {
        let audio = AudioStreamParameters::with_context(
            sample_rate,
            channels,
            SampleFormat::S16,
            "pcm_s16le",
        )?;

        Ok(Self {
            info: PcmS16leMuxerInfo {
                audio,
                samples_per_channel: 0,
            },
            data: Vec::new(),
            packets: 0,
            finished: false,
        })
    }

    pub fn info(&self) -> &PcmS16leMuxerInfo {
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
                "cannot write packet after pcm_s16le muxer is finished",
            ));
        }
        if packet.stream_index() != 0 {
            return Err(AvError::invalid_argument(format!(
                "pcm_s16le muxer only accepts stream 0, got stream {}",
                packet.stream_index()
            )));
        }
        let packet_samples = complete_sample_frames_in_bytes(
            packet.data().len(),
            self.info.audio.bytes_per_sample_frame(),
        );

        let new_len = self
            .data
            .len()
            .checked_add(packet.data().len())
            .ok_or_else(|| AvError::invalid_argument("pcm_s16le data size overflow"))?;
        let new_samples = self
            .info
            .samples_per_channel
            .checked_add(packet_samples)
            .ok_or_else(|| AvError::invalid_argument("pcm_s16le sample count overflow"))?;
        let new_packets = self
            .packets
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("pcm_s16le packet count overflow"))?;

        self.data.reserve(new_len - self.data.len());
        self.data.extend_from_slice(packet.data());
        self.info.samples_per_channel = new_samples;
        self.packets = new_packets;
        Ok(())
    }

    pub fn render(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn finish(&mut self) -> Vec<u8> {
        self.finished = true;
        self.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::AvErrorKind;

    #[test]
    fn slices_interleaved_stereo_packets_with_sample_timing() {
        let input = vec![
            0, 0, 1, 0, 2, 0, 3, 0, //
            4, 0, 5, 0, 6, 0, 7, 0,
        ];
        let mut demuxer = PcmS16leDemuxer::open(&input, 48_000, 2, 2).unwrap();

        assert_eq!(demuxer.info().sample_rate(), 48_000);
        assert_eq!(demuxer.info().channels(), 2);
        assert_eq!(
            demuxer.info().channel_layout(),
            Some(ChannelLayout::stereo())
        );
        assert_eq!(demuxer.info().sample_format(), SampleFormat::S16);
        assert_eq!(demuxer.info().packet_samples(), 2);
        assert_eq!(demuxer.info().bytes_per_sample_frame(), 4);
        assert_eq!(demuxer.info().packet_size(), 8);
        assert_eq!(demuxer.info().total_samples_per_channel(), 4);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data(), &[0, 0, 1, 0, 2, 0, 3, 0]);
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.duration(), 2);
        assert_eq!(first.side_data()[0].kind(), "pcm_sample_fmt");
        assert_eq!(first.side_data()[0].data(), b"s16");
        assert_eq!(first.side_data()[1].kind(), "pcm_channels");
        assert_eq!(first.side_data()[1].data(), b"2");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), &[4, 0, 5, 0, 6, 0, 7, 0]);
        assert_eq!(second.pts(), Some(2));
        assert_eq!(second.duration(), 2);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn emits_short_final_packet_on_sample_frame_boundary() {
        let input = vec![0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0];
        let mut demuxer = PcmS16leDemuxer::open(&input, 44_100, 1, 4).unwrap();

        assert_eq!(demuxer.info().channel_layout(), Some(ChannelLayout::mono()));
        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data().len(), 8);
        assert_eq!(first.duration(), 4);
        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), &[4, 0, 5, 0]);
        assert_eq!(second.pts(), Some(4));
        assert_eq!(second.duration(), 2);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn accepts_empty_input_as_zero_packets_when_parameters_are_valid() {
        let mut demuxer = PcmS16leDemuxer::open(&[], 48_000, 2, 1024).unwrap();

        assert_eq!(demuxer.info().total_samples_per_channel(), 0);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn derives_three_channel_layout_and_truncates_partial_packet_duration() {
        let input = (0_u8..14).collect::<Vec<_>>();
        let mut demuxer = PcmS16leDemuxer::open(&input, 48_000, 3, 1024).unwrap();

        assert_eq!(demuxer.info().channels(), 3);
        assert_eq!(
            demuxer.info().channel_layout(),
            Some(ChannelLayout::two_one())
        );
        assert_eq!(demuxer.info().bytes_per_sample_frame(), 6);
        assert_eq!(demuxer.info().total_samples_per_channel(), 2);

        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), input.as_slice());
        assert_eq!(packet.pts(), Some(0));
        assert_eq!(packet.dts(), Some(0));
        assert_eq!(packet.duration(), 2);
        assert_eq!(packet.side_data()[1].kind(), "pcm_channels");
        assert_eq!(packet.side_data()[1].data(), b"3");
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_parameters() {
        assert!(PcmS16leDemuxer::open(&[0, 0], 0, 2, 1).is_err());
        assert!(PcmS16leDemuxer::open(&[0, 0], 48_000, 0, 1).is_err());
        assert!(PcmS16leDemuxer::open(&[0, 0], 48_000, 1, 0).is_err());
    }

    #[test]
    fn accepts_partial_final_sample_frame_like_raw_ffmpeg_demuxing() {
        let input = [0, 1, 2, 3, 4, 5];
        let mut demuxer = PcmS16leDemuxer::open(&input, 48_000, 2, 2).unwrap();

        assert_eq!(demuxer.info().bytes_per_sample_frame(), 4);
        assert_eq!(demuxer.info().total_samples_per_channel(), 1);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data(), &[0, 1, 2, 3, 4, 5]);
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.duration(), 1);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn truncates_partial_packet_duration_to_complete_sample_frames() {
        let input = [0, 1, 2, 3, 4, 5];
        let mut demuxer = PcmS16leDemuxer::open(&input, 48_000, 2, 1).unwrap();

        let full = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(full.data(), &[0, 1, 2, 3]);
        assert_eq!(full.duration(), 1);

        let partial = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(partial.data(), &[4, 5]);
        assert_eq!(partial.pts(), Some(1));
        assert_eq!(partial.dts(), Some(1));
        assert_eq!(partial.duration(), 0);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn packetizes_multiple_packets_and_tracks_timestamps_with_odd_tail() {
        let input = vec![0_u8; 8_193];
        let mut demuxer = PcmS16leDemuxer::open(&input, 48_000, 2, 1024).unwrap();

        assert_eq!(demuxer.info().packet_samples(), 1024);
        assert_eq!(demuxer.info().packet_size(), 4_096);
        assert_eq!(demuxer.info().total_samples_per_channel(), 2_048);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data().len(), 4_096);
        assert_eq!(first.duration(), 1_024);
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data().len(), 4_096);
        assert_eq!(second.duration(), 1_024);
        assert_eq!(second.pts(), Some(1_024));
        assert_eq!(second.dts(), Some(1_024));

        let third = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(third.data().len(), 1);
        assert_eq!(third.duration(), 0);
        assert_eq!(third.pts(), Some(2_048));
        assert_eq!(third.dts(), Some(2_048));
        assert!(third.side_data()[1].data().eq(b"2"));

        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_concatenates_stream_zero_packets_and_tracks_samples() {
        let mut muxer = PcmS16leMuxer::new(48_000, 2).unwrap();
        let first = Packet::new(vec![0, 0, 1, 0, 2, 0, 3, 0], 0);
        let second = Packet::new(vec![4, 0, 5, 0], 0);

        muxer.write_packet(&first).unwrap();
        muxer.write_packet(&second).unwrap();

        assert_eq!(muxer.info().sample_rate(), 48_000);
        assert_eq!(muxer.info().channels(), 2);
        assert_eq!(muxer.info().channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(muxer.info().sample_format(), SampleFormat::S16);
        assert_eq!(muxer.info().bytes_per_sample_frame(), 4);
        assert_eq!(muxer.info().samples_per_channel(), 3);
        assert_eq!(muxer.packets(), 2);
        assert_eq!(muxer.data_len(), 12);
        assert_eq!(muxer.render(), vec![0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0]);
    }

    #[test]
    fn muxer_counts_empty_packets_without_changing_output() {
        let mut muxer = PcmS16leMuxer::new(44_100, 1).unwrap();

        muxer.write_packet(&Packet::new(Vec::new(), 0)).unwrap();

        assert_eq!(muxer.info().channel_layout(), Some(ChannelLayout::mono()));
        assert_eq!(muxer.packets(), 1);
        assert_eq!(muxer.data_len(), 0);
        assert_eq!(muxer.info().samples_per_channel(), 0);
        assert!(muxer.render().is_empty());
    }

    #[test]
    fn muxer_rejects_invalid_stream_parameters_and_packets() {
        assert!(PcmS16leMuxer::new(0, 2).is_err());
        assert!(PcmS16leMuxer::new(48_000, 0).is_err());

        let mut muxer = PcmS16leMuxer::new(48_000, 2).unwrap();
        let wrong_stream = muxer.write_packet(&Packet::new(vec![0, 0, 1, 0], 1));
        assert_eq!(
            wrong_stream.unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        let mut wrong_index_muxer = PcmS16leMuxer::new(48_000, 2).unwrap();
        assert!(wrong_index_muxer
            .write_packet(&Packet::new(vec![0, 0, 1, 0], 1))
            .is_err());
        let mut partial_frame_muxer = PcmS16leMuxer::new(48_000, 2).unwrap();
        partial_frame_muxer
            .write_packet(&Packet::new(vec![0, 0], 0))
            .unwrap();

        assert_eq!(muxer.packets(), 0);
        assert_eq!(muxer.data_len(), 0);
        assert_eq!(muxer.info().samples_per_channel(), 0);
    }

    #[test]
    fn muxer_accepts_partial_sample_frame_packets_without_padding() {
        let mut muxer = PcmS16leMuxer::new(48_000, 2).unwrap();

        let packet = Packet::new(vec![0, 0, 1, 0, 2], 0);
        muxer.write_packet(&packet).unwrap();

        assert_eq!(muxer.info().sample_rate(), 48_000);
        assert_eq!(muxer.info().channels(), 2);
        assert_eq!(muxer.info().bytes_per_sample_frame(), 4);
        assert_eq!(muxer.packets(), 1);
        assert_eq!(muxer.info().samples_per_channel(), 1);
        assert_eq!(muxer.data_len(), 5);
        assert_eq!(muxer.render(), packet.data().to_vec());
        assert!(!muxer.render().is_empty());
        assert_eq!(muxer.render().len() % 4, 1);
    }

    #[test]
    fn muxer_finish_prevents_more_writes() {
        let mut muxer = PcmS16leMuxer::new(48_000, 1).unwrap();
        let packet = Packet::new(vec![0, 0, 1, 0], 0);

        muxer.write_packet(&packet).unwrap();
        let output = muxer.finish();
        let err = muxer.write_packet(&packet).unwrap_err();

        assert!(muxer.is_finished());
        assert_eq!(output, vec![0, 0, 1, 0]);
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.packets(), 1);
        assert_eq!(muxer.info().samples_per_channel(), 2);
    }
}
