use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    FrontLeft,
    FrontRight,
    FrontCenter,
    LowFrequency,
    BackLeft,
    BackRight,
    FrontLeftOfCenter,
    FrontRightOfCenter,
    BackCenter,
    SideLeft,
    SideRight,
    TopFrontLeft,
    TopFrontRight,
    TopBackLeft,
    TopBackCenter,
    TopBackRight,
    LowFrequency2,
    TopSideLeft,
    TopSideRight,
}

impl Channel {
    pub const ALL: &'static [Self] = &[
        Self::FrontLeft,
        Self::FrontRight,
        Self::FrontCenter,
        Self::LowFrequency,
        Self::BackLeft,
        Self::BackRight,
        Self::FrontLeftOfCenter,
        Self::FrontRightOfCenter,
        Self::BackCenter,
        Self::SideLeft,
        Self::SideRight,
        Self::TopFrontLeft,
        Self::TopFrontRight,
        Self::TopBackLeft,
        Self::TopBackCenter,
        Self::TopBackRight,
        Self::LowFrequency2,
        Self::TopSideLeft,
        Self::TopSideRight,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::FrontLeft => "FL",
            Self::FrontRight => "FR",
            Self::FrontCenter => "FC",
            Self::LowFrequency => "LFE",
            Self::BackLeft => "BL",
            Self::BackRight => "BR",
            Self::FrontLeftOfCenter => "FLC",
            Self::FrontRightOfCenter => "FRC",
            Self::BackCenter => "BC",
            Self::SideLeft => "SL",
            Self::SideRight => "SR",
            Self::TopFrontLeft => "TFL",
            Self::TopFrontRight => "TFR",
            Self::TopBackLeft => "TBL",
            Self::TopBackCenter => "TBC",
            Self::TopBackRight => "TBR",
            Self::LowFrequency2 => "LFE2",
            Self::TopSideLeft => "TSL",
            Self::TopSideRight => "TSR",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|channel| channel.name().eq_ignore_ascii_case(name))
    }

    pub fn mask(self) -> u64 {
        match self {
            Self::FrontLeft => 1 << 0,
            Self::FrontRight => 1 << 1,
            Self::FrontCenter => 1 << 2,
            Self::LowFrequency => 1 << 3,
            Self::BackLeft => 1 << 4,
            Self::BackRight => 1 << 5,
            Self::FrontLeftOfCenter => 1 << 6,
            Self::FrontRightOfCenter => 1 << 7,
            Self::BackCenter => 1 << 8,
            Self::SideLeft => 1 << 9,
            Self::SideRight => 1 << 10,
            Self::TopFrontLeft => 1 << 12,
            Self::TopFrontRight => 1 << 14,
            Self::TopBackLeft => 1 << 15,
            Self::TopBackCenter => 1 << 16,
            Self::TopBackRight => 1 << 17,
            Self::LowFrequency2 => 1 << 35,
            Self::TopSideLeft => 1 << 36,
            Self::TopSideRight => 1 << 37,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelLayout {
    name: &'static str,
    channels: &'static [Channel],
}

impl ChannelLayout {
    pub const fn new_static(name: &'static str, channels: &'static [Channel]) -> Self {
        Self { name, channels }
    }

    pub fn mono() -> Self {
        Self::new_static("mono", &[Channel::FrontCenter])
    }

    pub fn stereo() -> Self {
        Self::new_static("stereo", &[Channel::FrontLeft, Channel::FrontRight])
    }

    pub fn two_one() -> Self {
        Self::new_static(
            "2.1",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::LowFrequency,
            ],
        )
    }

    pub fn three_zero() -> Self {
        Self::new_static(
            "3.0",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
            ],
        )
    }

    pub fn three_zero_back() -> Self {
        Self::new_static(
            "3.0(back)",
            &[Channel::FrontLeft, Channel::FrontRight, Channel::BackCenter],
        )
    }

    pub fn four_zero() -> Self {
        Self::new_static(
            "4.0",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::BackCenter,
            ],
        )
    }

    pub fn quad() -> Self {
        Self::new_static(
            "quad",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::BackLeft,
                Channel::BackRight,
            ],
        )
    }

    pub fn quad_side() -> Self {
        Self::new_static(
            "quad(side)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn three_one() -> Self {
        Self::new_static(
            "3.1",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
            ],
        )
    }

    pub fn five_zero() -> Self {
        Self::new_static(
            "5.0",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::BackLeft,
                Channel::BackRight,
            ],
        )
    }

    pub fn five_zero_side() -> Self {
        Self::new_static(
            "5.0(side)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn four_one() -> Self {
        Self::new_static(
            "4.1",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackCenter,
            ],
        )
    }

    pub fn five_one() -> Self {
        Self::new_static(
            "5.1",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
            ],
        )
    }

    pub fn five_one_side() -> Self {
        Self::new_static(
            "5.1(side)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn six_zero() -> Self {
        Self::new_static(
            "6.0",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::BackCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn six_zero_front() -> Self {
        Self::new_static(
            "6.0(front)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontLeftOfCenter,
                Channel::FrontRightOfCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn hexagonal() -> Self {
        Self::new_static(
            "hexagonal",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::BackCenter,
            ],
        )
    }

    pub fn six_one() -> Self {
        Self::new_static(
            "6.1",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn six_one_back() -> Self {
        Self::new_static(
            "6.1(back)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::BackCenter,
            ],
        )
    }

    pub fn six_one_front() -> Self {
        Self::new_static(
            "6.1(front)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::LowFrequency,
                Channel::FrontLeftOfCenter,
                Channel::FrontRightOfCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn seven_zero() -> Self {
        Self::new_static(
            "7.0",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn seven_zero_front() -> Self {
        Self::new_static(
            "7.0(front)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::FrontLeftOfCenter,
                Channel::FrontRightOfCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn seven_one() -> Self {
        Self::new_static(
            "7.1",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn seven_one_wide() -> Self {
        Self::new_static(
            "7.1(wide)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::FrontLeftOfCenter,
                Channel::FrontRightOfCenter,
            ],
        )
    }

    pub fn seven_one_wide_side() -> Self {
        Self::new_static(
            "7.1(wide-side)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::FrontLeftOfCenter,
                Channel::FrontRightOfCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn five_one_two() -> Self {
        Self::new_static(
            "5.1.2",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
            ],
        )
    }

    pub fn five_one_two_back() -> Self {
        Self::new_static(
            "5.1.2(back)",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
            ],
        )
    }

    pub fn octagonal() -> Self {
        Self::new_static(
            "octagonal",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::BackCenter,
                Channel::SideLeft,
                Channel::SideRight,
            ],
        )
    }

    pub fn cube() -> Self {
        Self::new_static(
            "cube",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
                Channel::TopBackLeft,
                Channel::TopBackRight,
            ],
        )
    }

    pub fn five_one_four() -> Self {
        Self::new_static(
            "5.1.4",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
                Channel::TopBackLeft,
                Channel::TopBackRight,
            ],
        )
    }

    pub fn seven_one_two() -> Self {
        Self::new_static(
            "7.1.2",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
            ],
        )
    }

    pub fn seven_one_four() -> Self {
        Self::new_static(
            "7.1.4",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
                Channel::TopBackLeft,
                Channel::TopBackRight,
            ],
        )
    }

    pub fn seven_two_three() -> Self {
        Self::new_static(
            "7.2.3",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
                Channel::TopBackCenter,
                Channel::LowFrequency2,
            ],
        )
    }

    pub fn nine_one_four() -> Self {
        Self::new_static(
            "9.1.4",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::FrontLeftOfCenter,
                Channel::FrontRightOfCenter,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
                Channel::TopBackLeft,
                Channel::TopBackRight,
            ],
        )
    }

    pub fn nine_one_six() -> Self {
        Self::new_static(
            "9.1.6",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::LowFrequency,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::FrontLeftOfCenter,
                Channel::FrontRightOfCenter,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
                Channel::TopBackLeft,
                Channel::TopBackRight,
                Channel::TopSideLeft,
                Channel::TopSideRight,
            ],
        )
    }

    pub fn known_layouts() -> [Self; 35] {
        [
            Self::mono(),
            Self::stereo(),
            Self::two_one(),
            Self::three_zero(),
            Self::three_zero_back(),
            Self::four_zero(),
            Self::quad(),
            Self::quad_side(),
            Self::three_one(),
            Self::five_zero(),
            Self::five_zero_side(),
            Self::four_one(),
            Self::five_one(),
            Self::five_one_side(),
            Self::six_zero(),
            Self::six_zero_front(),
            Self::hexagonal(),
            Self::six_one(),
            Self::six_one_back(),
            Self::six_one_front(),
            Self::seven_zero(),
            Self::seven_zero_front(),
            Self::seven_one(),
            Self::seven_one_wide(),
            Self::seven_one_wide_side(),
            Self::five_one_two(),
            Self::five_one_two_back(),
            Self::octagonal(),
            Self::cube(),
            Self::five_one_four(),
            Self::seven_one_two(),
            Self::seven_one_four(),
            Self::seven_two_three(),
            Self::nine_one_four(),
            Self::nine_one_six(),
        ]
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "mono" => Some(Self::mono()),
            "stereo" => Some(Self::stereo()),
            "2.1" => Some(Self::two_one()),
            "3.0" => Some(Self::three_zero()),
            "3.0(back)" => Some(Self::three_zero_back()),
            "4.0" => Some(Self::four_zero()),
            "quad" => Some(Self::quad()),
            "quad(side)" => Some(Self::quad_side()),
            "3.1" => Some(Self::three_one()),
            "5.0" => Some(Self::five_zero()),
            "5.0(side)" => Some(Self::five_zero_side()),
            "4.1" => Some(Self::four_one()),
            "5.1" => Some(Self::five_one()),
            "5.1(side)" => Some(Self::five_one_side()),
            "6.0" => Some(Self::six_zero()),
            "6.0(front)" => Some(Self::six_zero_front()),
            "hexagonal" => Some(Self::hexagonal()),
            "6.1" => Some(Self::six_one()),
            "6.1(back)" => Some(Self::six_one_back()),
            "6.1(front)" => Some(Self::six_one_front()),
            "7.0" => Some(Self::seven_zero()),
            "7.0(front)" => Some(Self::seven_zero_front()),
            "7.1" => Some(Self::seven_one()),
            "7.1(wide)" => Some(Self::seven_one_wide()),
            "7.1(wide-side)" => Some(Self::seven_one_wide_side()),
            "5.1.2" => Some(Self::five_one_two()),
            "5.1.2(back)" => Some(Self::five_one_two_back()),
            "octagonal" => Some(Self::octagonal()),
            "cube" => Some(Self::cube()),
            "5.1.4" => Some(Self::five_one_four()),
            "7.1.2" => Some(Self::seven_one_two()),
            "7.1.4" => Some(Self::seven_one_four()),
            "7.2.3" => Some(Self::seven_two_three()),
            "9.1.4" => Some(Self::nine_one_four()),
            "9.1.6" => Some(Self::nine_one_six()),
            _ => None,
        }
    }

    pub fn parse(name_or_channels: &str) -> AvResult<Self> {
        let trimmed = name_or_channels.trim();
        if trimmed.is_empty() {
            return Err(AvError::invalid_argument("empty channel layout"));
        }
        if trimmed.contains('\0') {
            return Err(AvError::invalid_argument(
                "channel layout contains NUL byte",
            ));
        }
        if let Some(layout) = Self::known_layouts()
            .into_iter()
            .find(|layout| layout.name().eq_ignore_ascii_case(trimmed))
        {
            return Ok(layout);
        }

        let mut channels = Vec::new();
        for token in trimmed.split('+') {
            let token = token.trim();
            if token.is_empty() {
                return Err(AvError::invalid_argument(format!(
                    "empty channel name in layout {name_or_channels:?}"
                )));
            }
            let channel = Channel::from_name(token).ok_or_else(|| {
                AvError::invalid_argument(format!("unknown channel name {token:?}"))
            })?;
            if channels.contains(&channel) {
                return Err(AvError::invalid_argument(format!(
                    "duplicate channel name {}",
                    channel.name()
                )));
            }
            channels.push(channel);
        }

        Self::from_channels(&channels).ok_or_else(|| {
            AvError::invalid_argument(format!(
                "unsupported channel layout expression {name_or_channels:?}"
            ))
        })
    }

    pub fn from_channels(channels: &[Channel]) -> Option<Self> {
        if channels.is_empty() {
            return None;
        }
        let mut mask = 0u64;
        for channel in channels {
            let channel_mask = channel.mask();
            if mask & channel_mask != 0 {
                return None;
            }
            mask |= channel_mask;
        }
        Self::from_channel_mask(mask)
    }

    pub fn from_channel_mask(mask: u64) -> Option<Self> {
        Self::known_layouts()
            .into_iter()
            .find(|layout| layout.channel_mask() == mask)
    }

    pub fn default_for_count(channels: u16) -> Option<Self> {
        match channels {
            1 => Some(Self::mono()),
            2 => Some(Self::stereo()),
            3 => Some(Self::two_one()),
            4 => Some(Self::four_zero()),
            5 => Some(Self::five_zero()),
            6 => Some(Self::five_one()),
            7 => Some(Self::six_one()),
            8 => Some(Self::seven_one()),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        self.name
    }

    pub fn channels(self) -> &'static [Channel] {
        self.channels
    }

    pub fn channel_mask(self) -> u64 {
        self.channels
            .iter()
            .fold(0u64, |mask, channel| mask | channel.mask())
    }

    pub fn channel_string(self) -> String {
        let mut output = String::new();
        for (index, channel) in self.channels.iter().enumerate() {
            if index != 0 {
                output.push('+');
            }
            output.push_str(channel.name());
        }
        output
    }

    pub fn channel_count(self) -> u16 {
        u16::try_from(self.channels.len()).expect("static channel layout count fits u16")
    }

    pub fn contains(self, channel: Channel) -> bool {
        self.channels.contains(&channel)
    }

    pub fn validate_channel_count(self, channels: u16) -> AvResult<()> {
        if self.channel_count() != channels {
            return Err(AvError::invalid_argument(format!(
                "{} channel layout has {} channels, got {channels}",
                self.name,
                self.channel_count()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn channels_report_ffmpeg_short_names() {
        assert_eq!(Channel::FrontLeft.name(), "FL");
        assert_eq!(Channel::FrontRight.name(), "FR");
        assert_eq!(Channel::FrontCenter.name(), "FC");
        assert_eq!(Channel::LowFrequency.name(), "LFE");
        assert_eq!(Channel::BackLeft.name(), "BL");
        assert_eq!(Channel::BackRight.name(), "BR");
        assert_eq!(Channel::FrontLeftOfCenter.name(), "FLC");
        assert_eq!(Channel::FrontRightOfCenter.name(), "FRC");
        assert_eq!(Channel::BackCenter.name(), "BC");
        assert_eq!(Channel::SideLeft.name(), "SL");
        assert_eq!(Channel::SideRight.name(), "SR");
        assert_eq!(Channel::TopFrontLeft.name(), "TFL");
        assert_eq!(Channel::TopFrontRight.name(), "TFR");
        assert_eq!(Channel::TopBackLeft.name(), "TBL");
        assert_eq!(Channel::TopBackCenter.name(), "TBC");
        assert_eq!(Channel::TopBackRight.name(), "TBR");
        assert_eq!(Channel::LowFrequency2.name(), "LFE2");
        assert_eq!(Channel::TopSideLeft.name(), "TSL");
        assert_eq!(Channel::TopSideRight.name(), "TSR");

        assert_eq!(Channel::from_name("fl"), Some(Channel::FrontLeft));
        assert_eq!(Channel::from_name("LFE"), Some(Channel::LowFrequency));
        assert_eq!(Channel::from_name("flc"), Some(Channel::FrontLeftOfCenter));
        assert_eq!(Channel::from_name("FRC"), Some(Channel::FrontRightOfCenter));
        assert_eq!(Channel::from_name("bc"), Some(Channel::BackCenter));
        assert_eq!(Channel::from_name("tfl"), Some(Channel::TopFrontLeft));
        assert_eq!(Channel::from_name("TFR"), Some(Channel::TopFrontRight));
        assert_eq!(Channel::from_name("tbl"), Some(Channel::TopBackLeft));
        assert_eq!(Channel::from_name("TBC"), Some(Channel::TopBackCenter));
        assert_eq!(Channel::from_name("TBR"), Some(Channel::TopBackRight));
        assert_eq!(Channel::from_name("lfe2"), Some(Channel::LowFrequency2));
        assert_eq!(Channel::from_name("tsl"), Some(Channel::TopSideLeft));
        assert_eq!(Channel::from_name("TSR"), Some(Channel::TopSideRight));
        assert_eq!(Channel::from_name("unknown"), None);
        assert_eq!(Channel::FrontLeft.mask(), 1);
        assert_eq!(Channel::LowFrequency.mask(), 1 << 3);
        assert_eq!(Channel::FrontLeftOfCenter.mask(), 1 << 6);
        assert_eq!(Channel::FrontRightOfCenter.mask(), 1 << 7);
        assert_eq!(Channel::BackCenter.mask(), 1 << 8);
        assert_eq!(Channel::SideLeft.mask(), 1 << 9);
        assert_eq!(Channel::TopFrontLeft.mask(), 1 << 12);
        assert_eq!(Channel::TopFrontRight.mask(), 1 << 14);
        assert_eq!(Channel::TopBackLeft.mask(), 1 << 15);
        assert_eq!(Channel::TopBackCenter.mask(), 1 << 16);
        assert_eq!(Channel::TopBackRight.mask(), 1 << 17);
        assert_eq!(Channel::LowFrequency2.mask(), 1 << 35);
        assert_eq!(Channel::TopSideLeft.mask(), 1 << 36);
        assert_eq!(Channel::TopSideRight.mask(), 1 << 37);
    }

    #[test]
    fn named_layouts_expose_ordered_channels_and_counts() {
        let stereo = ChannelLayout::stereo();
        assert_eq!(stereo.name(), "stereo");
        assert_eq!(stereo.channel_count(), 2);
        assert_eq!(
            stereo.channels(),
            &[Channel::FrontLeft, Channel::FrontRight]
        );
        assert!(stereo.contains(Channel::FrontLeft));
        assert!(!stereo.contains(Channel::LowFrequency));

        let two_one = ChannelLayout::two_one();
        assert_eq!(two_one.name(), "2.1");
        assert_eq!(two_one.channel_count(), 3);
        assert_eq!(
            two_one.channels(),
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::LowFrequency
            ]
        );

        let three_zero_back = ChannelLayout::three_zero_back();
        assert_eq!(three_zero_back.name(), "3.0(back)");
        assert_eq!(three_zero_back.channel_count(), 3);
        assert!(three_zero_back.contains(Channel::BackCenter));
        assert!(!three_zero_back.contains(Channel::FrontCenter));

        let four_zero = ChannelLayout::four_zero();
        assert_eq!(four_zero.name(), "4.0");
        assert_eq!(four_zero.channel_count(), 4);
        assert!(four_zero.contains(Channel::FrontCenter));
        assert!(four_zero.contains(Channel::BackCenter));

        let quad_side = ChannelLayout::quad_side();
        assert_eq!(quad_side.name(), "quad(side)");
        assert_eq!(quad_side.channel_count(), 4);
        assert!(quad_side.contains(Channel::SideLeft));
        assert!(!quad_side.contains(Channel::BackLeft));

        let three_one = ChannelLayout::three_one();
        assert_eq!(three_one.name(), "3.1");
        assert_eq!(three_one.channel_count(), 4);
        assert!(three_one.contains(Channel::LowFrequency));

        let five_zero = ChannelLayout::five_zero();
        assert_eq!(five_zero.name(), "5.0");
        assert_eq!(five_zero.channel_count(), 5);
        assert!(five_zero.contains(Channel::BackRight));
        assert!(!five_zero.contains(Channel::LowFrequency));

        let five_zero_side = ChannelLayout::five_zero_side();
        assert_eq!(five_zero_side.name(), "5.0(side)");
        assert_eq!(five_zero_side.channel_count(), 5);
        assert!(five_zero_side.contains(Channel::SideRight));
        assert!(!five_zero_side.contains(Channel::BackRight));

        let four_one = ChannelLayout::four_one();
        assert_eq!(four_one.name(), "4.1");
        assert_eq!(four_one.channel_count(), 5);
        assert!(four_one.contains(Channel::LowFrequency));
        assert!(four_one.contains(Channel::BackCenter));

        let surround = ChannelLayout::five_one();
        assert_eq!(surround.name(), "5.1");
        assert_eq!(surround.channel_count(), 6);
        assert!(surround.contains(Channel::LowFrequency));
        assert!(surround.contains(Channel::BackLeft));

        let six_zero = ChannelLayout::six_zero();
        assert_eq!(six_zero.name(), "6.0");
        assert_eq!(six_zero.channel_count(), 6);
        assert!(six_zero.contains(Channel::BackCenter));
        assert!(six_zero.contains(Channel::SideRight));
        assert!(!six_zero.contains(Channel::LowFrequency));

        let six_zero_front = ChannelLayout::six_zero_front();
        assert_eq!(six_zero_front.name(), "6.0(front)");
        assert_eq!(six_zero_front.channel_count(), 6);
        assert!(six_zero_front.contains(Channel::FrontLeftOfCenter));
        assert!(six_zero_front.contains(Channel::FrontRightOfCenter));
        assert!(!six_zero_front.contains(Channel::FrontCenter));

        let hexagonal = ChannelLayout::hexagonal();
        assert_eq!(hexagonal.name(), "hexagonal");
        assert_eq!(hexagonal.channel_count(), 6);
        assert!(hexagonal.contains(Channel::BackCenter));
        assert!(hexagonal.contains(Channel::BackLeft));
        assert!(!hexagonal.contains(Channel::SideLeft));

        let six_one = ChannelLayout::six_one();
        assert_eq!(six_one.name(), "6.1");
        assert_eq!(six_one.channel_count(), 7);
        assert!(six_one.contains(Channel::LowFrequency));
        assert!(six_one.contains(Channel::BackCenter));
        assert!(six_one.contains(Channel::SideRight));
        assert!(!six_one.contains(Channel::BackLeft));

        let six_one_back = ChannelLayout::six_one_back();
        assert_eq!(six_one_back.name(), "6.1(back)");
        assert_eq!(six_one_back.channel_count(), 7);
        assert!(six_one_back.contains(Channel::BackLeft));
        assert!(six_one_back.contains(Channel::BackCenter));
        assert!(!six_one_back.contains(Channel::SideLeft));

        let six_one_front = ChannelLayout::six_one_front();
        assert_eq!(six_one_front.name(), "6.1(front)");
        assert_eq!(six_one_front.channel_count(), 7);
        assert!(six_one_front.contains(Channel::LowFrequency));
        assert!(six_one_front.contains(Channel::FrontLeftOfCenter));
        assert!(!six_one_front.contains(Channel::FrontCenter));

        let seven_zero = ChannelLayout::seven_zero();
        assert_eq!(seven_zero.name(), "7.0");
        assert_eq!(seven_zero.channel_count(), 7);
        assert!(seven_zero.contains(Channel::BackLeft));
        assert!(seven_zero.contains(Channel::SideRight));
        assert!(!seven_zero.contains(Channel::LowFrequency));

        let seven_zero_front = ChannelLayout::seven_zero_front();
        assert_eq!(seven_zero_front.name(), "7.0(front)");
        assert_eq!(seven_zero_front.channel_count(), 7);
        assert!(seven_zero_front.contains(Channel::FrontLeftOfCenter));
        assert!(seven_zero_front.contains(Channel::FrontCenter));
        assert!(!seven_zero_front.contains(Channel::LowFrequency));

        let seven_one_wide = ChannelLayout::seven_one_wide();
        assert_eq!(seven_one_wide.name(), "7.1(wide)");
        assert_eq!(seven_one_wide.channel_count(), 8);
        assert!(seven_one_wide.contains(Channel::FrontLeftOfCenter));
        assert!(seven_one_wide.contains(Channel::BackLeft));
        assert!(!seven_one_wide.contains(Channel::SideLeft));

        let seven_one_wide_side = ChannelLayout::seven_one_wide_side();
        assert_eq!(seven_one_wide_side.name(), "7.1(wide-side)");
        assert_eq!(seven_one_wide_side.channel_count(), 8);
        assert!(seven_one_wide_side.contains(Channel::FrontRightOfCenter));
        assert!(seven_one_wide_side.contains(Channel::SideLeft));
        assert!(!seven_one_wide_side.contains(Channel::BackLeft));

        let five_one_two = ChannelLayout::five_one_two();
        assert_eq!(five_one_two.name(), "5.1.2");
        assert_eq!(five_one_two.channel_count(), 8);
        assert!(five_one_two.contains(Channel::TopFrontLeft));
        assert!(five_one_two.contains(Channel::TopFrontRight));
        assert!(five_one_two.contains(Channel::SideLeft));
        assert!(!five_one_two.contains(Channel::BackLeft));

        let five_one_two_back = ChannelLayout::five_one_two_back();
        assert_eq!(five_one_two_back.name(), "5.1.2(back)");
        assert_eq!(five_one_two_back.channel_count(), 8);
        assert!(five_one_two_back.contains(Channel::TopFrontLeft));
        assert!(five_one_two_back.contains(Channel::TopFrontRight));
        assert!(five_one_two_back.contains(Channel::BackLeft));
        assert!(!five_one_two_back.contains(Channel::SideLeft));

        let octagonal = ChannelLayout::octagonal();
        assert_eq!(octagonal.name(), "octagonal");
        assert_eq!(octagonal.channel_count(), 8);
        assert!(octagonal.contains(Channel::BackCenter));
        assert!(octagonal.contains(Channel::SideRight));
        assert!(!octagonal.contains(Channel::LowFrequency));

        let cube = ChannelLayout::cube();
        assert_eq!(cube.name(), "cube");
        assert_eq!(cube.channel_count(), 8);
        assert!(cube.contains(Channel::TopFrontLeft));
        assert!(cube.contains(Channel::TopBackRight));
        assert!(cube.contains(Channel::BackLeft));
        assert!(!cube.contains(Channel::FrontCenter));

        let five_one_four = ChannelLayout::five_one_four();
        assert_eq!(five_one_four.name(), "5.1.4");
        assert_eq!(five_one_four.channel_count(), 10);
        assert!(five_one_four.contains(Channel::SideRight));
        assert!(five_one_four.contains(Channel::TopBackLeft));
        assert!(!five_one_four.contains(Channel::BackLeft));

        let seven_one_two = ChannelLayout::seven_one_two();
        assert_eq!(seven_one_two.name(), "7.1.2");
        assert_eq!(seven_one_two.channel_count(), 10);
        assert!(seven_one_two.contains(Channel::BackLeft));
        assert!(seven_one_two.contains(Channel::SideLeft));
        assert!(seven_one_two.contains(Channel::TopFrontRight));
        assert!(!seven_one_two.contains(Channel::TopBackLeft));

        let seven_one_four = ChannelLayout::seven_one_four();
        assert_eq!(seven_one_four.name(), "7.1.4");
        assert_eq!(seven_one_four.channel_count(), 12);
        assert!(seven_one_four.contains(Channel::BackLeft));
        assert!(seven_one_four.contains(Channel::SideLeft));
        assert!(seven_one_four.contains(Channel::TopBackRight));

        let seven_two_three = ChannelLayout::seven_two_three();
        assert_eq!(seven_two_three.name(), "7.2.3");
        assert_eq!(seven_two_three.channel_count(), 12);
        assert!(seven_two_three.contains(Channel::BackRight));
        assert!(seven_two_three.contains(Channel::TopBackCenter));
        assert!(seven_two_three.contains(Channel::LowFrequency2));
        assert!(!seven_two_three.contains(Channel::TopBackLeft));

        let nine_one_four = ChannelLayout::nine_one_four();
        assert_eq!(nine_one_four.name(), "9.1.4");
        assert_eq!(nine_one_four.channel_count(), 14);
        assert!(nine_one_four.contains(Channel::FrontLeftOfCenter));
        assert!(nine_one_four.contains(Channel::FrontRightOfCenter));
        assert!(nine_one_four.contains(Channel::TopBackRight));
        assert!(!nine_one_four.contains(Channel::LowFrequency2));

        let nine_one_six = ChannelLayout::nine_one_six();
        assert_eq!(nine_one_six.name(), "9.1.6");
        assert_eq!(nine_one_six.channel_count(), 16);
        assert!(nine_one_six.contains(Channel::TopSideLeft));
        assert!(nine_one_six.contains(Channel::TopSideRight));
        assert!(nine_one_six.contains(Channel::TopBackRight));
        assert!(!nine_one_six.contains(Channel::LowFrequency2));
    }

    #[test]
    fn layout_masks_and_channel_strings_are_canonical() {
        let stereo_mask = Channel::FrontLeft.mask() | Channel::FrontRight.mask();
        assert_eq!(ChannelLayout::stereo().channel_mask(), stereo_mask);
        assert_eq!(
            ChannelLayout::from_channel_mask(stereo_mask),
            Some(ChannelLayout::stereo())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[Channel::FrontRight, Channel::FrontLeft]),
            Some(ChannelLayout::stereo())
        );
        assert_eq!(ChannelLayout::stereo().channel_string(), "FL+FR");
        assert_eq!(ChannelLayout::two_one().channel_string(), "FL+FR+LFE");
        assert_eq!(ChannelLayout::three_zero().channel_string(), "FL+FR+FC");
        assert_eq!(
            ChannelLayout::three_zero_back().channel_string(),
            "FL+FR+BC"
        );
        assert_eq!(ChannelLayout::four_zero().channel_string(), "FL+FR+FC+BC");
        assert_eq!(ChannelLayout::quad_side().channel_string(), "FL+FR+SL+SR");
        assert_eq!(ChannelLayout::three_one().channel_string(), "FL+FR+FC+LFE");
        assert_eq!(
            ChannelLayout::five_zero().channel_string(),
            "FL+FR+FC+BL+BR"
        );
        assert_eq!(
            ChannelLayout::five_zero_side().channel_string(),
            "FL+FR+FC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::four_one().channel_string(),
            "FL+FR+FC+LFE+BC"
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask() | Channel::FrontRight.mask() | Channel::BackCenter.mask()
            ),
            Some(ChannelLayout::three_zero_back())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::BackCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::three_zero_back())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
            ),
            Some(ChannelLayout::five_zero_side())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::SideRight,
                Channel::FrontCenter,
                Channel::SideLeft,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::five_zero_side())
        );
        assert_eq!(
            ChannelLayout::five_one_side().channel_string(),
            "FL+FR+FC+LFE+SL+SR"
        );
        assert_eq!(
            ChannelLayout::six_zero().channel_string(),
            "FL+FR+FC+BC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::six_zero_front().channel_string(),
            "FL+FR+FLC+FRC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::hexagonal().channel_string(),
            "FL+FR+FC+BL+BR+BC"
        );
        assert_eq!(
            ChannelLayout::six_one().channel_string(),
            "FL+FR+FC+LFE+BC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::six_one_back().channel_string(),
            "FL+FR+FC+LFE+BL+BR+BC"
        );
        assert_eq!(
            ChannelLayout::six_one_front().channel_string(),
            "FL+FR+LFE+FLC+FRC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::seven_zero().channel_string(),
            "FL+FR+FC+BL+BR+SL+SR"
        );
        assert_eq!(
            ChannelLayout::seven_zero_front().channel_string(),
            "FL+FR+FC+FLC+FRC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::seven_one_wide().channel_string(),
            "FL+FR+FC+LFE+BL+BR+FLC+FRC"
        );
        assert_eq!(
            ChannelLayout::seven_one_wide_side().channel_string(),
            "FL+FR+FC+LFE+FLC+FRC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::five_one_two().channel_string(),
            "FL+FR+FC+LFE+SL+SR+TFL+TFR"
        );
        assert_eq!(
            ChannelLayout::five_one_two_back().channel_string(),
            "FL+FR+FC+LFE+BL+BR+TFL+TFR"
        );
        assert_eq!(
            ChannelLayout::octagonal().channel_string(),
            "FL+FR+FC+BL+BR+BC+SL+SR"
        );
        assert_eq!(
            ChannelLayout::cube().channel_string(),
            "FL+FR+BL+BR+TFL+TFR+TBL+TBR"
        );
        assert_eq!(
            ChannelLayout::five_one_four().channel_string(),
            "FL+FR+FC+LFE+SL+SR+TFL+TFR+TBL+TBR"
        );
        assert_eq!(
            ChannelLayout::seven_one_two().channel_string(),
            "FL+FR+FC+LFE+BL+BR+SL+SR+TFL+TFR"
        );
        assert_eq!(
            ChannelLayout::seven_one_four().channel_string(),
            "FL+FR+FC+LFE+BL+BR+SL+SR+TFL+TFR+TBL+TBR"
        );
        assert_eq!(
            ChannelLayout::seven_two_three().channel_string(),
            "FL+FR+FC+LFE+BL+BR+SL+SR+TFL+TFR+TBC+LFE2"
        );
        assert_eq!(
            ChannelLayout::nine_one_four().channel_string(),
            "FL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFL+TFR+TBL+TBR"
        );
        assert_eq!(
            ChannelLayout::nine_one_six().channel_string(),
            "FL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFL+TFR+TBL+TBR+TSL+TSR"
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontLeftOfCenter.mask()
                    | Channel::FrontRightOfCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
            ),
            Some(ChannelLayout::six_zero_front())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::BackCenter,
                Channel::BackRight,
                Channel::FrontCenter,
                Channel::BackLeft,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::hexagonal())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::BackCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
            ),
            Some(ChannelLayout::six_one())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::BackLeft.mask()
                    | Channel::BackRight.mask()
                    | Channel::BackCenter.mask()
            ),
            Some(ChannelLayout::six_one_back())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::SideRight,
                Channel::FrontRight,
                Channel::SideLeft,
                Channel::FrontRightOfCenter,
                Channel::FrontLeft,
                Channel::LowFrequency,
                Channel::FrontLeftOfCenter,
            ]),
            Some(ChannelLayout::six_one_front())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::SideRight,
                Channel::BackRight,
                Channel::FrontCenter,
                Channel::SideLeft,
                Channel::BackLeft,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::seven_zero())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::FrontLeftOfCenter.mask()
                    | Channel::FrontRightOfCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
            ),
            Some(ChannelLayout::seven_zero_front())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::BackLeft.mask()
                    | Channel::BackRight.mask()
                    | Channel::FrontLeftOfCenter.mask()
                    | Channel::FrontRightOfCenter.mask()
            ),
            Some(ChannelLayout::seven_one_wide())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::SideRight,
                Channel::SideLeft,
                Channel::FrontRightOfCenter,
                Channel::FrontLeftOfCenter,
                Channel::LowFrequency,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::seven_one_wide_side())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
                    | Channel::TopFrontLeft.mask()
                    | Channel::TopFrontRight.mask()
            ),
            Some(ChannelLayout::five_one_two())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::LowFrequency,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::five_one_two_back())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::BackLeft.mask()
                    | Channel::BackRight.mask()
                    | Channel::BackCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
            ),
            Some(ChannelLayout::octagonal())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::TopBackRight,
                Channel::TopBackLeft,
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::cube())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
                    | Channel::TopFrontLeft.mask()
                    | Channel::TopFrontRight.mask()
                    | Channel::TopBackLeft.mask()
                    | Channel::TopBackRight.mask()
            ),
            Some(ChannelLayout::five_one_four())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::SideRight,
                Channel::SideLeft,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::LowFrequency,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::seven_one_two())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::TopBackRight,
                Channel::TopBackLeft,
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::SideRight,
                Channel::SideLeft,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::LowFrequency,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::seven_one_four())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::BackLeft.mask()
                    | Channel::BackRight.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
                    | Channel::TopFrontLeft.mask()
                    | Channel::TopFrontRight.mask()
                    | Channel::TopBackCenter.mask()
                    | Channel::LowFrequency2.mask()
            ),
            Some(ChannelLayout::seven_two_three())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::LowFrequency2,
                Channel::TopBackCenter,
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::SideRight,
                Channel::SideLeft,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::LowFrequency,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::seven_two_three())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::BackLeft.mask()
                    | Channel::BackRight.mask()
                    | Channel::FrontLeftOfCenter.mask()
                    | Channel::FrontRightOfCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
                    | Channel::TopFrontLeft.mask()
                    | Channel::TopFrontRight.mask()
                    | Channel::TopBackLeft.mask()
                    | Channel::TopBackRight.mask()
            ),
            Some(ChannelLayout::nine_one_four())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::TopBackRight,
                Channel::TopBackLeft,
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::SideRight,
                Channel::SideLeft,
                Channel::FrontRightOfCenter,
                Channel::FrontLeftOfCenter,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::LowFrequency,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::nine_one_four())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::LowFrequency.mask()
                    | Channel::BackLeft.mask()
                    | Channel::BackRight.mask()
                    | Channel::FrontLeftOfCenter.mask()
                    | Channel::FrontRightOfCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
                    | Channel::TopFrontLeft.mask()
                    | Channel::TopFrontRight.mask()
                    | Channel::TopBackLeft.mask()
                    | Channel::TopBackRight.mask()
                    | Channel::TopSideLeft.mask()
                    | Channel::TopSideRight.mask()
            ),
            Some(ChannelLayout::nine_one_six())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::TopSideRight,
                Channel::TopSideLeft,
                Channel::TopBackRight,
                Channel::TopBackLeft,
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::SideRight,
                Channel::SideLeft,
                Channel::FrontRightOfCenter,
                Channel::FrontLeftOfCenter,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::LowFrequency,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::nine_one_six())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(Channel::FrontLeft.mask() | Channel::BackRight.mask()),
            None
        );
        assert_eq!(
            ChannelLayout::from_channels(&[Channel::FrontLeft, Channel::FrontLeft]),
            None
        );
    }

    #[test]
    fn layout_name_and_default_count_lookup_are_narrow_and_explicit() {
        let known_names: Vec<_> = ChannelLayout::known_layouts()
            .into_iter()
            .map(ChannelLayout::name)
            .collect();
        assert_eq!(
            known_names,
            [
                "mono",
                "stereo",
                "2.1",
                "3.0",
                "3.0(back)",
                "4.0",
                "quad",
                "quad(side)",
                "3.1",
                "5.0",
                "5.0(side)",
                "4.1",
                "5.1",
                "5.1(side)",
                "6.0",
                "6.0(front)",
                "hexagonal",
                "6.1",
                "6.1(back)",
                "6.1(front)",
                "7.0",
                "7.0(front)",
                "7.1",
                "7.1(wide)",
                "7.1(wide-side)",
                "5.1.2",
                "5.1.2(back)",
                "octagonal",
                "cube",
                "5.1.4",
                "7.1.2",
                "7.1.4",
                "7.2.3",
                "9.1.4",
                "9.1.6",
            ]
        );

        assert_eq!(
            ChannelLayout::from_name("mono"),
            Some(ChannelLayout::mono())
        );
        assert_eq!(
            ChannelLayout::from_name("2.1"),
            Some(ChannelLayout::two_one())
        );
        assert_eq!(
            ChannelLayout::from_name("3.0"),
            Some(ChannelLayout::three_zero())
        );
        assert_eq!(
            ChannelLayout::from_name("3.0(back)"),
            Some(ChannelLayout::three_zero_back())
        );
        assert_eq!(
            ChannelLayout::from_name("4.0"),
            Some(ChannelLayout::four_zero())
        );
        assert_eq!(
            ChannelLayout::from_name("quad(side)"),
            Some(ChannelLayout::quad_side())
        );
        assert_eq!(
            ChannelLayout::from_name("3.1"),
            Some(ChannelLayout::three_one())
        );
        assert_eq!(
            ChannelLayout::from_name("5.0"),
            Some(ChannelLayout::five_zero())
        );
        assert_eq!(
            ChannelLayout::from_name("5.0(side)"),
            Some(ChannelLayout::five_zero_side())
        );
        assert_eq!(
            ChannelLayout::from_name("4.1"),
            Some(ChannelLayout::four_one())
        );
        assert_eq!(
            ChannelLayout::from_name("5.1(side)"),
            Some(ChannelLayout::five_one_side())
        );
        assert_eq!(
            ChannelLayout::from_name("6.0"),
            Some(ChannelLayout::six_zero())
        );
        assert_eq!(
            ChannelLayout::from_name("6.0(front)"),
            Some(ChannelLayout::six_zero_front())
        );
        assert_eq!(
            ChannelLayout::from_name("hexagonal"),
            Some(ChannelLayout::hexagonal())
        );
        assert_eq!(
            ChannelLayout::from_name("6.1"),
            Some(ChannelLayout::six_one())
        );
        assert_eq!(
            ChannelLayout::from_name("6.1(back)"),
            Some(ChannelLayout::six_one_back())
        );
        assert_eq!(
            ChannelLayout::from_name("6.1(front)"),
            Some(ChannelLayout::six_one_front())
        );
        assert_eq!(
            ChannelLayout::from_name("7.0"),
            Some(ChannelLayout::seven_zero())
        );
        assert_eq!(
            ChannelLayout::from_name("7.0(front)"),
            Some(ChannelLayout::seven_zero_front())
        );
        assert_eq!(
            ChannelLayout::from_name("7.1"),
            Some(ChannelLayout::seven_one())
        );
        assert_eq!(
            ChannelLayout::from_name("7.1(wide)"),
            Some(ChannelLayout::seven_one_wide())
        );
        assert_eq!(
            ChannelLayout::from_name("7.1(wide-side)"),
            Some(ChannelLayout::seven_one_wide_side())
        );
        assert_eq!(
            ChannelLayout::from_name("5.1.2"),
            Some(ChannelLayout::five_one_two())
        );
        assert_eq!(
            ChannelLayout::from_name("5.1.2(back)"),
            Some(ChannelLayout::five_one_two_back())
        );
        assert_eq!(
            ChannelLayout::from_name("octagonal"),
            Some(ChannelLayout::octagonal())
        );
        assert_eq!(
            ChannelLayout::from_name("cube"),
            Some(ChannelLayout::cube())
        );
        assert_eq!(
            ChannelLayout::from_name("5.1.4"),
            Some(ChannelLayout::five_one_four())
        );
        assert_eq!(
            ChannelLayout::from_name("7.1.2"),
            Some(ChannelLayout::seven_one_two())
        );
        assert_eq!(
            ChannelLayout::from_name("7.1.4"),
            Some(ChannelLayout::seven_one_four())
        );
        assert_eq!(
            ChannelLayout::from_name("7.2.3"),
            Some(ChannelLayout::seven_two_three())
        );
        assert_eq!(
            ChannelLayout::from_name("9.1.4"),
            Some(ChannelLayout::nine_one_four())
        );
        assert_eq!(
            ChannelLayout::from_name("9.1.6"),
            Some(ChannelLayout::nine_one_six())
        );
        assert_eq!(ChannelLayout::from_name("unknown"), None);

        assert_eq!(
            ChannelLayout::default_for_count(1),
            Some(ChannelLayout::mono())
        );
        assert_eq!(
            ChannelLayout::default_for_count(2),
            Some(ChannelLayout::stereo())
        );
        assert_eq!(
            ChannelLayout::default_for_count(3),
            Some(ChannelLayout::two_one())
        );
        assert_eq!(
            ChannelLayout::default_for_count(4),
            Some(ChannelLayout::four_zero())
        );
        assert_eq!(
            ChannelLayout::default_for_count(5),
            Some(ChannelLayout::five_zero())
        );
        assert_eq!(
            ChannelLayout::default_for_count(6),
            Some(ChannelLayout::five_one())
        );
        assert_eq!(
            ChannelLayout::default_for_count(7),
            Some(ChannelLayout::six_one())
        );
        assert_eq!(
            ChannelLayout::default_for_count(8),
            Some(ChannelLayout::seven_one())
        );
        assert_eq!(ChannelLayout::default_for_count(9), None);
        assert_eq!(ChannelLayout::default_for_count(10), None);
        assert_eq!(ChannelLayout::default_for_count(12), None);
        assert_eq!(ChannelLayout::default_for_count(14), None);
        assert_eq!(ChannelLayout::default_for_count(16), None);
    }

    #[test]
    fn layout_parser_accepts_named_and_channel_expressions() {
        assert_eq!(
            ChannelLayout::parse("stereo").unwrap(),
            ChannelLayout::stereo()
        );
        assert_eq!(
            ChannelLayout::parse("  STEREO  ").unwrap(),
            ChannelLayout::stereo()
        );
        assert_eq!(ChannelLayout::parse("FC").unwrap(), ChannelLayout::mono());
        assert_eq!(
            ChannelLayout::parse("fr + fl").unwrap(),
            ChannelLayout::stereo()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+LFE").unwrap(),
            ChannelLayout::two_one()
        );
        assert_eq!(
            ChannelLayout::parse("fl+fr+fc").unwrap(),
            ChannelLayout::three_zero()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+BC").unwrap(),
            ChannelLayout::three_zero_back()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+BC").unwrap(),
            ChannelLayout::four_zero()
        );
        assert_eq!(
            ChannelLayout::parse("SL+FR+SR+FL").unwrap(),
            ChannelLayout::quad_side()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE").unwrap(),
            ChannelLayout::three_one()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+BL+BR").unwrap(),
            ChannelLayout::five_zero()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+SL+SR").unwrap(),
            ChannelLayout::five_zero_side()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BC").unwrap(),
            ChannelLayout::four_one()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR").unwrap(),
            ChannelLayout::five_one()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+SL+SR").unwrap(),
            ChannelLayout::five_one_side()
        );
        assert_eq!(
            ChannelLayout::parse("FR+FC+FL+LFE+SR+SL").unwrap(),
            ChannelLayout::five_one_side()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+BC+SL+SR").unwrap(),
            ChannelLayout::six_zero()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FRC+SR+SL+FLC+FR").unwrap(),
            ChannelLayout::six_zero_front()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+BL+BR+BC").unwrap(),
            ChannelLayout::hexagonal()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BC+SL+SR").unwrap(),
            ChannelLayout::six_one()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR+BC").unwrap(),
            ChannelLayout::six_one_back()
        );
        assert_eq!(
            ChannelLayout::parse("FL+LFE+FRC+SR+SL+FLC+FR").unwrap(),
            ChannelLayout::six_one_front()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+BL+BR+SL+SR").unwrap(),
            ChannelLayout::seven_zero()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+FLC+FRC+SL+SR").unwrap(),
            ChannelLayout::seven_zero_front()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR+FLC+FRC").unwrap(),
            ChannelLayout::seven_one_wide()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+FLC+FRC+SL+SR").unwrap(),
            ChannelLayout::seven_one_wide_side()
        );
        assert_eq!(
            ChannelLayout::parse("5.1.2").unwrap(),
            ChannelLayout::five_one_two()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+SL+SR+TFL+TFR").unwrap(),
            ChannelLayout::five_one_two()
        );
        assert_eq!(
            ChannelLayout::parse("FL+TFR+FR+FC+TFL+LFE+BR+BL").unwrap(),
            ChannelLayout::five_one_two_back()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+BL+BR+BC+SL+SR").unwrap(),
            ChannelLayout::octagonal()
        );
        assert_eq!(ChannelLayout::parse("cube").unwrap(), ChannelLayout::cube());
        assert_eq!(
            ChannelLayout::parse("FL+TBR+FR+TFL+BR+TFR+TBL+BL").unwrap(),
            ChannelLayout::cube()
        );
        assert_eq!(
            ChannelLayout::parse("5.1.4").unwrap(),
            ChannelLayout::five_one_four()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+SL+SR+TFL+TFR+TBL+TBR").unwrap(),
            ChannelLayout::five_one_four()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR+SL+SR+TFL+TFR").unwrap(),
            ChannelLayout::seven_one_two()
        );
        assert_eq!(
            ChannelLayout::parse("FL+TBR+FR+TFL+BR+TFR+TBL+FC+LFE+BL+SL+SR").unwrap(),
            ChannelLayout::seven_one_four()
        );
        assert_eq!(
            ChannelLayout::parse("7.2.3").unwrap(),
            ChannelLayout::seven_two_three()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR+SL+SR+TFL+TFR+TBC+LFE2").unwrap(),
            ChannelLayout::seven_two_three()
        );
        assert_eq!(
            ChannelLayout::parse("LFE2+TFR+FL+TFL+FR+FC+TBC+LFE+BR+BL+SR+SL").unwrap(),
            ChannelLayout::seven_two_three()
        );
        assert_eq!(
            ChannelLayout::parse("9.1.4").unwrap(),
            ChannelLayout::nine_one_four()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFL+TFR+TBL+TBR").unwrap(),
            ChannelLayout::nine_one_four()
        );
        assert_eq!(
            ChannelLayout::parse("TBR+FL+TFL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFR+TBL").unwrap(),
            ChannelLayout::nine_one_four()
        );
        assert_eq!(
            ChannelLayout::parse("9.1.6").unwrap(),
            ChannelLayout::nine_one_six()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFL+TFR+TBL+TBR+TSL+TSR")
                .unwrap(),
            ChannelLayout::nine_one_six()
        );
        assert_eq!(
            ChannelLayout::parse("TSR+TBR+FL+TFL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFR+TBL+TSL",)
                .unwrap(),
            ChannelLayout::nine_one_six()
        );
    }

    #[test]
    fn layout_parser_rejects_invalid_or_unsupported_expressions() {
        for input in ["", "   ", "+", "FL+", "FL++FR", "FL\0FR", "FL+FL", "FL+BR"] {
            let err = ChannelLayout::parse(input).unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        }
        let err = ChannelLayout::parse("FL+unknown").unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    }

    #[test]
    fn layouts_validate_channel_counts() {
        assert!(ChannelLayout::stereo().validate_channel_count(2).is_ok());
        let err = ChannelLayout::stereo()
            .validate_channel_count(1)
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    }
}
