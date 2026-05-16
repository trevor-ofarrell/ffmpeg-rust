use avutil::{AvError, AvErrorKind, AvResult, Packet, SideData};

const BYTES_PER_S16_SAMPLE: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmS16leInfo {
    sample_rate: u32,
    channels: u16,
    packet_samples: usize,
    bytes_per_sample_frame: usize,
    packet_size: usize,
    total_samples_per_channel: usize,
}

impl PcmS16leInfo {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn packet_samples(&self) -> usize {
        self.packet_samples
    }

    pub fn bytes_per_sample_frame(&self) -> usize {
        self.bytes_per_sample_frame
    }

    pub fn packet_size(&self) -> usize {
        self.packet_size
    }

    pub fn total_samples_per_channel(&self) -> usize {
        self.total_samples_per_channel
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
        if packet_samples == 0 {
            return Err(AvError::invalid_argument(
                "pcm_s16le packet samples must be non-zero",
            ));
        }

        let bytes_per_sample_frame = usize::from(channels)
            .checked_mul(BYTES_PER_S16_SAMPLE)
            .ok_or_else(|| AvError::invalid_argument("pcm_s16le sample frame size overflow"))?;
        let packet_size = bytes_per_sample_frame
            .checked_mul(packet_samples)
            .ok_or_else(|| AvError::invalid_argument("pcm_s16le packet size overflow"))?;

        if input.len() % bytes_per_sample_frame != 0 {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "pcm_s16le input ends with a partial sample frame",
            ));
        }
        let total_samples_per_channel = input.len() / bytes_per_sample_frame;

        Ok(Self {
            info: PcmS16leInfo {
                sample_rate,
                channels,
                packet_samples,
                bytes_per_sample_frame,
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
        let samples = packet_len / self.info.bytes_per_sample_frame;
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
            self.info.channels.to_string().into_bytes(),
        )?);
        self.position = end;
        self.next_pts = self
            .next_pts
            .checked_add(duration)
            .ok_or_else(|| AvError::invalid_data("pcm_s16le PTS overflow"))?;
        Ok(Some(packet))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_interleaved_stereo_packets_with_sample_timing() {
        let input = vec![
            0, 0, 1, 0, 2, 0, 3, 0, //
            4, 0, 5, 0, 6, 0, 7, 0,
        ];
        let mut demuxer = PcmS16leDemuxer::open(&input, 48_000, 2, 2).unwrap();

        assert_eq!(demuxer.info().sample_rate(), 48_000);
        assert_eq!(demuxer.info().channels(), 2);
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
    fn rejects_invalid_parameters_and_partial_sample_frames() {
        assert!(PcmS16leDemuxer::open(&[0, 0], 0, 2, 1).is_err());
        assert!(PcmS16leDemuxer::open(&[0, 0], 48_000, 0, 1).is_err());
        assert!(PcmS16leDemuxer::open(&[0, 0], 48_000, 1, 0).is_err());

        let err = PcmS16leDemuxer::open(&[0, 0, 1], 48_000, 1, 1).unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
        let err = PcmS16leDemuxer::open(&[0, 0, 1, 0, 2, 0], 48_000, 2, 1).unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
    }
}
