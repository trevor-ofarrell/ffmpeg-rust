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
    TopCenter,
    TopFrontLeft,
    TopFrontCenter,
    TopFrontRight,
    TopBackLeft,
    TopBackCenter,
    TopBackRight,
    StereoLeft,
    StereoRight,
    WideLeft,
    WideRight,
    SurroundDirectLeft,
    SurroundDirectRight,
    LowFrequency2,
    TopSideLeft,
    TopSideRight,
    BottomFrontCenter,
    BottomFrontLeft,
    BottomFrontRight,
    SideSurroundLeft,
    SideSurroundRight,
    TopSurroundLeft,
    TopSurroundRight,
    BinauralLeft,
    BinauralRight,
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
        Self::TopCenter,
        Self::TopFrontLeft,
        Self::TopFrontCenter,
        Self::TopFrontRight,
        Self::TopBackLeft,
        Self::TopBackCenter,
        Self::TopBackRight,
        Self::StereoLeft,
        Self::StereoRight,
        Self::WideLeft,
        Self::WideRight,
        Self::SurroundDirectLeft,
        Self::SurroundDirectRight,
        Self::LowFrequency2,
        Self::TopSideLeft,
        Self::TopSideRight,
        Self::BottomFrontCenter,
        Self::BottomFrontLeft,
        Self::BottomFrontRight,
        Self::SideSurroundLeft,
        Self::SideSurroundRight,
        Self::TopSurroundLeft,
        Self::TopSurroundRight,
        Self::BinauralLeft,
        Self::BinauralRight,
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
            Self::TopCenter => "TC",
            Self::TopFrontLeft => "TFL",
            Self::TopFrontCenter => "TFC",
            Self::TopFrontRight => "TFR",
            Self::TopBackLeft => "TBL",
            Self::TopBackCenter => "TBC",
            Self::TopBackRight => "TBR",
            Self::StereoLeft => "DL",
            Self::StereoRight => "DR",
            Self::WideLeft => "WL",
            Self::WideRight => "WR",
            Self::SurroundDirectLeft => "SDL",
            Self::SurroundDirectRight => "SDR",
            Self::LowFrequency2 => "LFE2",
            Self::TopSideLeft => "TSL",
            Self::TopSideRight => "TSR",
            Self::BottomFrontCenter => "BFC",
            Self::BottomFrontLeft => "BFL",
            Self::BottomFrontRight => "BFR",
            Self::SideSurroundLeft => "SSL",
            Self::SideSurroundRight => "SSR",
            Self::TopSurroundLeft => "TTL",
            Self::TopSurroundRight => "TTR",
            Self::BinauralLeft => "BIL",
            Self::BinauralRight => "BIR",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|channel| channel.name().eq_ignore_ascii_case(name))
    }

    pub fn from_raw_id(raw_id: i32) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|channel| channel.raw_id() == raw_id)
    }

    pub fn raw_id(self) -> i32 {
        match self {
            Self::FrontLeft => 0,
            Self::FrontRight => 1,
            Self::FrontCenter => 2,
            Self::LowFrequency => 3,
            Self::BackLeft => 4,
            Self::BackRight => 5,
            Self::FrontLeftOfCenter => 6,
            Self::FrontRightOfCenter => 7,
            Self::BackCenter => 8,
            Self::SideLeft => 9,
            Self::SideRight => 10,
            Self::TopCenter => 11,
            Self::TopFrontLeft => 12,
            Self::TopFrontCenter => 13,
            Self::TopFrontRight => 14,
            Self::TopBackLeft => 15,
            Self::TopBackCenter => 16,
            Self::TopBackRight => 17,
            Self::StereoLeft => 29,
            Self::StereoRight => 30,
            Self::WideLeft => 31,
            Self::WideRight => 32,
            Self::SurroundDirectLeft => 33,
            Self::SurroundDirectRight => 34,
            Self::LowFrequency2 => 35,
            Self::TopSideLeft => 36,
            Self::TopSideRight => 37,
            Self::BottomFrontCenter => 38,
            Self::BottomFrontLeft => 39,
            Self::BottomFrontRight => 40,
            Self::SideSurroundLeft => 41,
            Self::SideSurroundRight => 42,
            Self::TopSurroundLeft => 43,
            Self::TopSurroundRight => 44,
            Self::BinauralLeft => 61,
            Self::BinauralRight => 62,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::FrontLeft => "front left",
            Self::FrontRight => "front right",
            Self::FrontCenter => "front center",
            Self::LowFrequency => "low frequency",
            Self::BackLeft => "back left",
            Self::BackRight => "back right",
            Self::FrontLeftOfCenter => "front left-of-center",
            Self::FrontRightOfCenter => "front right-of-center",
            Self::BackCenter => "back center",
            Self::SideLeft => "side left",
            Self::SideRight => "side right",
            Self::TopCenter => "top center",
            Self::TopFrontLeft => "top front left",
            Self::TopFrontCenter => "top front center",
            Self::TopFrontRight => "top front right",
            Self::TopBackLeft => "top back left",
            Self::TopBackCenter => "top back center",
            Self::TopBackRight => "top back right",
            Self::StereoLeft => "downmix left",
            Self::StereoRight => "downmix right",
            Self::WideLeft => "wide left",
            Self::WideRight => "wide right",
            Self::SurroundDirectLeft => "surround direct left",
            Self::SurroundDirectRight => "surround direct right",
            Self::LowFrequency2 => "low frequency 2",
            Self::TopSideLeft => "top side left",
            Self::TopSideRight => "top side right",
            Self::BottomFrontCenter => "bottom front center",
            Self::BottomFrontLeft => "bottom front left",
            Self::BottomFrontRight => "bottom front right",
            Self::SideSurroundLeft => "side surround left",
            Self::SideSurroundRight => "side surround right",
            Self::TopSurroundLeft => "top surround left",
            Self::TopSurroundRight => "top surround right",
            Self::BinauralLeft => "binaural left",
            Self::BinauralRight => "binaural right",
        }
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
            Self::TopCenter => 1 << 11,
            Self::TopFrontLeft => 1 << 12,
            Self::TopFrontCenter => 1 << 13,
            Self::TopFrontRight => 1 << 14,
            Self::TopBackLeft => 1 << 15,
            Self::TopBackCenter => 1 << 16,
            Self::TopBackRight => 1 << 17,
            Self::StereoLeft => 1 << 29,
            Self::StereoRight => 1 << 30,
            Self::WideLeft => 1 << 31,
            Self::WideRight => 1 << 32,
            Self::SurroundDirectLeft => 1 << 33,
            Self::SurroundDirectRight => 1 << 34,
            Self::LowFrequency2 => 1 << 35,
            Self::TopSideLeft => 1 << 36,
            Self::TopSideRight => 1 << 37,
            Self::BottomFrontCenter => 1 << 38,
            Self::BottomFrontLeft => 1 << 39,
            Self::BottomFrontRight => 1 << 40,
            Self::SideSurroundLeft => 1 << 41,
            Self::SideSurroundRight => 1 << 42,
            Self::TopSurroundLeft => 1 << 43,
            Self::TopSurroundRight => 1 << 44,
            Self::BinauralLeft => 1 << 61,
            Self::BinauralRight => 1 << 62,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelId {
    None,
    Native(Channel),
    Unused,
    Unknown,
    Ambisonic(u16),
    User(i32),
}

impl ChannelId {
    pub const NONE_RAW: i32 = -1;
    pub const UNUSED_RAW: i32 = 0x200;
    pub const UNKNOWN_RAW: i32 = 0x300;
    pub const AMBISONIC_BASE_RAW: i32 = 0x400;
    pub const AMBISONIC_END_RAW: i32 = 0x7ff;
    pub const MAX_AMBISONIC_ACN: u16 = 1023;

    pub fn from_raw(raw_id: i32) -> Self {
        if raw_id == Self::NONE_RAW {
            Self::None
        } else if let Some(channel) = Channel::from_raw_id(raw_id) {
            Self::Native(channel)
        } else if raw_id == Self::UNUSED_RAW {
            Self::Unused
        } else if raw_id == Self::UNKNOWN_RAW {
            Self::Unknown
        } else if (Self::AMBISONIC_BASE_RAW..=Self::AMBISONIC_END_RAW).contains(&raw_id) {
            Self::Ambisonic((raw_id - Self::AMBISONIC_BASE_RAW) as u16)
        } else {
            Self::User(raw_id)
        }
    }

    pub fn from_canonical_name(name: &str) -> Option<Self> {
        if name == "NONE" {
            return Some(Self::None);
        }
        if name == "UNSD" {
            return Some(Self::Unused);
        }
        if name == "UNK" {
            return Some(Self::Unknown);
        }
        if let Some(acn) = name.strip_prefix("AMBI").and_then(parse_ambisonic_acn) {
            return Some(Self::Ambisonic(acn));
        }
        if let Some(raw_id) = name.strip_prefix("USR").and_then(parse_user_channel_id) {
            return match Self::from_raw(raw_id) {
                Self::User(raw_id) => Some(Self::User(raw_id)),
                _ => None,
            };
        }
        Channel::ALL
            .iter()
            .copied()
            .find(|channel| channel.name() == name)
            .map(Self::Native)
    }

    pub fn raw_id(self) -> i32 {
        match self {
            Self::None => Self::NONE_RAW,
            Self::Native(channel) => channel.raw_id(),
            Self::Unused => Self::UNUSED_RAW,
            Self::Unknown => Self::UNKNOWN_RAW,
            Self::Ambisonic(acn) => Self::AMBISONIC_BASE_RAW + i32::from(acn),
            Self::User(raw_id) => raw_id,
        }
    }

    pub fn name(self) -> String {
        match self {
            Self::None => String::from("NONE"),
            Self::Native(channel) => String::from(channel.name()),
            Self::Unused => String::from("UNSD"),
            Self::Unknown => String::from("UNK"),
            Self::Ambisonic(acn) => format!("AMBI{acn}"),
            Self::User(raw_id) => format!("USR{raw_id}"),
        }
    }

    pub fn description(self) -> String {
        match self {
            Self::None => String::from("none"),
            Self::Native(channel) => String::from(channel.description()),
            Self::Unused => String::from("unused"),
            Self::Unknown => String::from("unknown"),
            Self::Ambisonic(acn) => format!("ambisonic ACN {acn}"),
            Self::User(raw_id) => format!("user {raw_id}"),
        }
    }

    pub fn native(self) -> Option<Channel> {
        match self {
            Self::Native(channel) => Some(channel),
            _ => None,
        }
    }

    pub fn is_valid_raw_id(self) -> bool {
        match self {
            Self::Ambisonic(acn) => acn <= Self::MAX_AMBISONIC_ACN,
            _ => true,
        }
    }
}

fn parse_ambisonic_acn(value: &str) -> Option<u16> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let acn = value.parse::<u16>().ok()?;
    (acn <= ChannelId::MAX_AMBISONIC_ACN).then_some(acn)
}

fn parse_user_channel_id(value: &str) -> Option<i32> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelCustom {
    id: ChannelId,
    name: String,
}

impl ChannelCustom {
    pub const NAME_STORAGE_BYTES: usize = 16;
    pub const MAX_NAME_BYTES: usize = Self::NAME_STORAGE_BYTES - 1;

    pub fn new(id: ChannelId, name: impl AsRef<str>) -> AvResult<Self> {
        if id == ChannelId::None {
            return Err(AvError::invalid_argument(
                "custom channel id cannot be NONE",
            ));
        }
        if !id.is_valid_raw_id() {
            return Err(AvError::invalid_argument(
                "custom channel id is outside FFmpeg's valid raw channel range",
            ));
        }
        let name = name.as_ref();
        if name.contains('\0') {
            return Err(AvError::invalid_argument(
                "custom channel name contains NUL byte",
            ));
        }
        if name.len() > Self::MAX_NAME_BYTES {
            return Err(AvError::invalid_argument(format!(
                "custom channel name exceeds {} bytes",
                Self::MAX_NAME_BYTES
            )));
        }
        Ok(Self {
            id,
            name: name.to_owned(),
        })
    }

    pub fn unknown() -> Self {
        Self {
            id: ChannelId::Unknown,
            name: String::new(),
        }
    }

    pub fn id(&self) -> ChannelId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn has_name(&self) -> bool {
        !self.name.is_empty()
    }

    fn describe(&self) -> String {
        let mut described = self.id.name();
        if self.has_name() {
            described.push('@');
            described.push_str(&self.name);
        }
        described
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomChannelLayout {
    channels: Vec<ChannelCustom>,
}

impl CustomChannelLayout {
    pub fn new(channels: Vec<ChannelCustom>) -> AvResult<Self> {
        if channels.is_empty() {
            return Err(AvError::invalid_argument(
                "custom channel layout must have at least one channel",
            ));
        }
        if channels.len() > usize::from(u16::MAX) {
            return Err(AvError::invalid_argument(
                "custom channel layout has too many channels",
            ));
        }
        for channel in &channels {
            if channel.id() == ChannelId::None {
                return Err(AvError::invalid_argument(
                    "custom channel layout contains NONE channel",
                ));
            }
            if !channel.id().is_valid_raw_id() {
                return Err(AvError::invalid_argument(
                    "custom channel layout contains invalid raw channel id",
                ));
            }
        }
        Ok(Self { channels })
    }

    pub fn unknown(channel_count: u16) -> AvResult<Self> {
        if channel_count == 0 {
            return Err(AvError::invalid_argument(
                "custom channel layout must have at least one channel",
            ));
        }
        Self::new(vec![ChannelCustom::unknown(); usize::from(channel_count)])
    }

    pub fn channel_count(&self) -> u16 {
        u16::try_from(self.channels.len()).expect("custom channel layout count fits u16")
    }

    pub fn channels(&self) -> &[ChannelCustom] {
        &self.channels
    }

    pub fn channel_from_index(&self, index: usize) -> Option<ChannelId> {
        self.channels.get(index).map(ChannelCustom::id)
    }

    pub fn channel_from_string(&self, name: &str) -> Option<ChannelId> {
        let index = self.index_from_string(name).ok()?;
        self.channel_from_index(index)
    }

    pub fn has_custom_names(&self) -> bool {
        self.channels.iter().any(ChannelCustom::has_name)
    }

    pub fn canonical_native_mask(&self) -> AvResult<u64> {
        let mut mask = 0u64;
        for channel in &self.channels {
            let native = channel.id().native().ok_or_else(|| {
                AvError::invalid_argument("custom layout cannot be represented as native mask")
            })?;
            let channel_mask = native.mask();
            if mask >= channel_mask {
                return Err(AvError::invalid_argument(
                    "custom layout native channels are not in canonical order",
                ));
            }
            mask |= channel_mask;
        }
        Ok(mask)
    }

    pub fn canonical_native_layout(&self) -> Option<ChannelLayout> {
        if self.has_custom_names() {
            return None;
        }
        let mask = self.canonical_native_mask().ok()?;
        ChannelLayout::from_channel_mask(mask)
    }

    pub fn subset_native_mask(&self, mask: u64) -> u64 {
        Channel::ALL
            .iter()
            .copied()
            .filter(|channel| mask & channel.mask() != 0)
            .filter(|channel| {
                self.channels
                    .iter()
                    .any(|custom| custom.id() == ChannelId::Native(*channel))
            })
            .fold(0u64, |subset, channel| subset | channel.mask())
    }

    pub fn is_equivalent_to_custom(&self, other: &Self) -> bool {
        self.channel_count() == other.channel_count()
            && self
                .channels
                .iter()
                .zip(other.channels.iter())
                .all(|(left, right)| left.id() == right.id())
    }

    pub fn is_equivalent_to_native(&self, other: ChannelLayout) -> bool {
        self.channel_count() == other.channel_count()
            && self
                .channels
                .iter()
                .zip(other.channels())
                .all(|(left, right)| left.id() == ChannelId::Native(*right))
    }

    pub fn ambisonic_order(&self) -> AvResult<u16> {
        let mut highest_ambi_index = None;
        let mut previous_was_non_ambisonic = false;

        for (index, channel) in self.channels.iter().enumerate() {
            match channel.id() {
                ChannelId::Ambisonic(acn) => {
                    if previous_was_non_ambisonic {
                        return Err(AvError::invalid_argument(
                            "ambisonic channel follows non-ambisonic channel",
                        ));
                    }
                    if usize::from(acn) != index {
                        return Err(AvError::invalid_argument(
                            "ambisonic channel is not in default ACN order",
                        ));
                    }
                    highest_ambi_index = Some(index);
                }
                _ => {
                    previous_was_non_ambisonic = true;
                }
            }
        }

        let highest_ambi_index = highest_ambi_index
            .ok_or_else(|| AvError::invalid_argument("custom layout has no ambisonic channels"))?;
        let mut order = 0usize;
        while (order + 1) * (order + 1) <= highest_ambi_index {
            order += 1;
        }
        if (order + 1) * (order + 1) != highest_ambi_index + 1 {
            return Err(AvError::invalid_argument(
                "custom layout has incomplete ambisonic order",
            ));
        }
        u16::try_from(order)
            .map_err(|_| AvError::invalid_argument("ambisonic order is outside supported range"))
    }

    fn ambisonic_channel_count(order: u16) -> usize {
        let side = usize::from(order) + 1;
        side * side
    }

    fn describe_ambisonic(&self) -> AvResult<String> {
        let order = self.ambisonic_order()?;
        let mut description = format!("ambisonic {order}");
        let ambisonic_channel_count = Self::ambisonic_channel_count(order);

        if ambisonic_channel_count < self.channels.len() {
            let extra =
                CustomChannelLayout::new(self.channels[ambisonic_channel_count..].to_vec())?;
            description.push('+');
            description.push_str(&extra.describe());
        }

        Ok(description)
    }

    pub fn index_from_channel(&self, id: ChannelId) -> AvResult<usize> {
        if id == ChannelId::None || !id.is_valid_raw_id() {
            return Err(AvError::invalid_argument("invalid channel id lookup"));
        }
        self.channels
            .iter()
            .position(|channel| channel.id() == id)
            .ok_or_else(|| AvError::invalid_argument("channel is not present in custom layout"))
    }

    pub fn index_from_string(&self, name: &str) -> AvResult<usize> {
        if name.is_empty() {
            return Err(AvError::invalid_argument("empty channel lookup"));
        }
        if name.contains('\0') {
            return Err(AvError::invalid_argument(
                "channel lookup contains NUL byte",
            ));
        }

        if let Some((id_part, custom_name)) = name.split_once('@') {
            if !custom_name.is_empty() {
                let id = if id_part.is_empty() {
                    None
                } else {
                    Some(ChannelId::from_canonical_name(id_part).ok_or_else(|| {
                        AvError::invalid_argument(format!("unknown channel id {id_part:?}"))
                    })?)
                };
                if let Some(index) = self.channels.iter().position(|channel| {
                    channel.name() == custom_name
                        && match id {
                            Some(id) => channel.id() == id,
                            None => true,
                        }
                }) {
                    return Ok(index);
                }
            }
            return Err(AvError::invalid_argument(format!(
                "channel name {name:?} is not present in custom layout"
            )));
        }

        let id = ChannelId::from_canonical_name(name)
            .ok_or_else(|| AvError::invalid_argument(format!("unknown channel id {name:?}")))?;
        self.index_from_channel(id)
    }

    pub fn describe(&self) -> String {
        if let Ok(description) = self.describe_ambisonic() {
            return description;
        }
        if let Some(layout) = self.canonical_native_layout() {
            return layout.name().to_owned();
        }

        let mut description = format!("{} channels (", self.channel_count());
        for (index, channel) in self.channels.iter().enumerate() {
            if index != 0 {
                description.push('+');
            }
            description.push_str(&channel.describe());
        }
        description.push(')');
        description
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

    pub fn twenty_two_two() -> Self {
        Self::new_static(
            "22.2",
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
                Channel::BackCenter,
                Channel::LowFrequency2,
                Channel::TopFrontCenter,
                Channel::TopCenter,
                Channel::TopBackCenter,
                Channel::BottomFrontCenter,
                Channel::BottomFrontLeft,
                Channel::BottomFrontRight,
            ],
        )
    }

    pub fn hexadecagonal() -> Self {
        Self::new_static(
            "hexadecagonal",
            &[
                Channel::FrontLeft,
                Channel::FrontRight,
                Channel::FrontCenter,
                Channel::BackLeft,
                Channel::BackRight,
                Channel::BackCenter,
                Channel::SideLeft,
                Channel::SideRight,
                Channel::WideLeft,
                Channel::WideRight,
                Channel::TopBackLeft,
                Channel::TopBackRight,
                Channel::TopBackCenter,
                Channel::TopFrontCenter,
                Channel::TopFrontLeft,
                Channel::TopFrontRight,
            ],
        )
    }

    pub fn binaural() -> Self {
        Self::new_static("binaural", &[Channel::BinauralLeft, Channel::BinauralRight])
    }

    pub fn downmix() -> Self {
        Self::new_static("downmix", &[Channel::StereoLeft, Channel::StereoRight])
    }

    pub fn known_layouts() -> [Self; 39] {
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
            Self::hexadecagonal(),
            Self::binaural(),
            Self::downmix(),
            Self::twenty_two_two(),
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
            "hexadecagonal" => Some(Self::hexadecagonal()),
            "binaural" => Some(Self::binaural()),
            "downmix" => Some(Self::downmix()),
            "22.2" => Some(Self::twenty_two_two()),
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
        Self::known_layouts()
            .into_iter()
            .find(|layout| layout.channel_count() == channels)
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

    pub fn subset_mask(self, mask: u64) -> u64 {
        self.channel_mask() & mask
    }

    pub fn channel_from_index(self, index: usize) -> Option<ChannelId> {
        let mut remaining = index;
        let mask = self.channel_mask();
        for channel in Channel::ALL.iter().copied() {
            if mask & channel.mask() == 0 {
                continue;
            }
            if remaining == 0 {
                return Some(ChannelId::Native(channel));
            }
            remaining -= 1;
        }
        None
    }

    pub fn index_from_channel(self, id: ChannelId) -> AvResult<usize> {
        let ChannelId::Native(channel) = id else {
            return Err(AvError::invalid_argument(
                "channel is not present in native layout",
            ));
        };
        let channel_mask = channel.mask();
        let mask = self.channel_mask();
        if mask & channel_mask == 0 {
            return Err(AvError::invalid_argument(
                "channel is not present in native layout",
            ));
        }

        Ok((mask & (channel_mask - 1)).count_ones() as usize)
    }

    pub fn index_from_string(self, name: &str) -> AvResult<usize> {
        if name.is_empty() {
            return Err(AvError::invalid_argument("empty channel lookup"));
        }
        if name.contains('\0') {
            return Err(AvError::invalid_argument(
                "channel lookup contains NUL byte",
            ));
        }

        let id = ChannelId::from_canonical_name(name)
            .ok_or_else(|| AvError::invalid_argument(format!("unknown channel id {name:?}")))?;
        self.index_from_channel(id)
    }

    pub fn channel_from_string(self, name: &str) -> Option<ChannelId> {
        let index = self.index_from_string(name).ok()?;
        self.channel_from_index(index)
    }

    pub fn is_equivalent_to(self, other: Self) -> bool {
        self.channel_count() == other.channel_count() && self.channel_mask() == other.channel_mask()
    }

    pub fn is_equivalent_to_custom(self, other: &CustomChannelLayout) -> bool {
        other.is_equivalent_to_native(self)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnspecifiedChannelLayout {
    channels: u16,
}

impl UnspecifiedChannelLayout {
    pub fn new(channels: u16) -> AvResult<Self> {
        if channels == 0 {
            return Err(AvError::invalid_argument(
                "unspecified channel layout must have at least one channel",
            ));
        }
        Ok(Self { channels })
    }

    pub fn channel_count(self) -> u16 {
        self.channels
    }

    pub fn describe(self) -> String {
        format!("{} channels", self.channels)
    }

    pub fn subset_mask(self, _mask: u64) -> u64 {
        0
    }

    pub fn channel_from_index(self, _index: usize) -> Option<ChannelId> {
        None
    }

    pub fn validate_channel_count(self, channels: u16) -> AvResult<()> {
        if self.channels != channels {
            return Err(AvError::invalid_argument(format!(
                "unspecified channel layout has {} channels, got {channels}",
                self.channels
            )));
        }
        Ok(())
    }

    pub fn is_equivalent_to_unspecified(self, other: Self) -> bool {
        self.channels == other.channels
    }

    pub fn is_equivalent_to_native(self, _other: ChannelLayout) -> bool {
        false
    }

    pub fn is_equivalent_to_custom(self, _other: &CustomChannelLayout) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelLayoutSpec {
    Native(ChannelLayout),
    Unspecified(UnspecifiedChannelLayout),
}

impl ChannelLayoutSpec {
    pub fn native(layout: ChannelLayout) -> Self {
        Self::Native(layout)
    }

    pub fn unspecified(channels: u16) -> AvResult<Self> {
        Ok(Self::Unspecified(UnspecifiedChannelLayout::new(channels)?))
    }

    pub fn default_for_count(channels: u16) -> AvResult<Self> {
        if let Some(layout) = ChannelLayout::default_for_count(channels) {
            return Ok(Self::Native(layout));
        }
        Self::unspecified(channels)
    }

    pub fn parse(value: &str) -> AvResult<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AvError::invalid_argument("empty channel layout"));
        }
        if trimmed.contains('\0') {
            return Err(AvError::invalid_argument(
                "channel layout contains NUL byte",
            ));
        }

        if let Some(layout) = Self::parse_described_channel_list(trimmed)? {
            return Ok(layout);
        }

        if let Some(channels) = parse_count_suffix(trimmed, "c") {
            let Some(layout) = ChannelLayout::default_for_count(channels) else {
                return Err(AvError::invalid_argument(format!(
                    "channel count {channels} has no native default layout"
                )));
            };
            return Ok(Self::Native(layout));
        }

        if let Some(channels) =
            parse_count_suffix(trimmed, "C").or_else(|| parse_count_suffix(trimmed, " channels"))
        {
            return Self::unspecified(channels);
        }

        Ok(Self::Native(ChannelLayout::parse(trimmed)?))
    }

    fn parse_described_channel_list(value: &str) -> AvResult<Option<Self>> {
        let Some((count_text, rest)) = value.split_once(" channels (") else {
            return Ok(None);
        };
        let Some(channel_list) = rest.strip_suffix(')') else {
            return Err(AvError::invalid_argument(format!(
                "unterminated channel layout list {value:?}"
            )));
        };
        let channels = parse_positive_channel_count(count_text).ok_or_else(|| {
            AvError::invalid_argument(format!("invalid channel count {count_text:?}"))
        })?;
        let layout = ChannelLayout::parse(channel_list)?;
        layout.validate_channel_count(channels)?;
        Ok(Some(Self::Native(layout)))
    }

    pub fn as_native(self) -> Option<ChannelLayout> {
        match self {
            Self::Native(layout) => Some(layout),
            Self::Unspecified(_) => None,
        }
    }

    pub fn as_unspecified(self) -> Option<UnspecifiedChannelLayout> {
        match self {
            Self::Native(_) => None,
            Self::Unspecified(layout) => Some(layout),
        }
    }

    pub fn is_unspecified(self) -> bool {
        matches!(self, Self::Unspecified(_))
    }

    pub fn channel_count(self) -> u16 {
        match self {
            Self::Native(layout) => layout.channel_count(),
            Self::Unspecified(layout) => layout.channel_count(),
        }
    }

    pub fn describe(self) -> String {
        match self {
            Self::Native(layout) => layout.name().to_owned(),
            Self::Unspecified(layout) => layout.describe(),
        }
    }

    pub fn subset_mask(self, mask: u64) -> u64 {
        match self {
            Self::Native(layout) => layout.subset_mask(mask),
            Self::Unspecified(layout) => layout.subset_mask(mask),
        }
    }

    pub fn channel_from_index(self, index: usize) -> Option<ChannelId> {
        match self {
            Self::Native(layout) => layout.channel_from_index(index),
            Self::Unspecified(layout) => layout.channel_from_index(index),
        }
    }

    pub fn validate_channel_count(self, channels: u16) -> AvResult<()> {
        match self {
            Self::Native(layout) => layout.validate_channel_count(channels),
            Self::Unspecified(layout) => layout.validate_channel_count(channels),
        }
    }

    pub fn is_equivalent_to(self, other: Self) -> bool {
        match (self, other) {
            (Self::Native(left), Self::Native(right)) => left.is_equivalent_to(right),
            (Self::Unspecified(left), Self::Unspecified(right)) => {
                left.is_equivalent_to_unspecified(right)
            }
            (Self::Native(_), Self::Unspecified(_)) | (Self::Unspecified(_), Self::Native(_)) => {
                false
            }
        }
    }

    pub fn is_equivalent_to_native(self, other: ChannelLayout) -> bool {
        match self {
            Self::Native(layout) => layout.is_equivalent_to(other),
            Self::Unspecified(layout) => layout.is_equivalent_to_native(other),
        }
    }

    pub fn is_equivalent_to_custom(self, other: &CustomChannelLayout) -> bool {
        match self {
            Self::Native(layout) => layout.is_equivalent_to_custom(other),
            Self::Unspecified(layout) => layout.is_equivalent_to_custom(other),
        }
    }
}

impl From<ChannelLayout> for ChannelLayoutSpec {
    fn from(layout: ChannelLayout) -> Self {
        Self::Native(layout)
    }
}

fn parse_count_suffix(value: &str, suffix: &str) -> Option<u16> {
    let count = value.strip_suffix(suffix)?;
    parse_positive_channel_count(count)
}

fn parse_positive_channel_count(value: &str) -> Option<u16> {
    if value.is_empty() {
        return None;
    }
    let channels = value.parse::<u16>().ok()?;
    (channels > 0).then_some(channels)
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
        assert_eq!(Channel::TopCenter.name(), "TC");
        assert_eq!(Channel::TopFrontLeft.name(), "TFL");
        assert_eq!(Channel::TopFrontCenter.name(), "TFC");
        assert_eq!(Channel::TopFrontRight.name(), "TFR");
        assert_eq!(Channel::TopBackLeft.name(), "TBL");
        assert_eq!(Channel::TopBackCenter.name(), "TBC");
        assert_eq!(Channel::TopBackRight.name(), "TBR");
        assert_eq!(Channel::StereoLeft.name(), "DL");
        assert_eq!(Channel::StereoRight.name(), "DR");
        assert_eq!(Channel::WideLeft.name(), "WL");
        assert_eq!(Channel::WideRight.name(), "WR");
        assert_eq!(Channel::SurroundDirectLeft.name(), "SDL");
        assert_eq!(Channel::SurroundDirectRight.name(), "SDR");
        assert_eq!(Channel::LowFrequency2.name(), "LFE2");
        assert_eq!(Channel::TopSideLeft.name(), "TSL");
        assert_eq!(Channel::TopSideRight.name(), "TSR");
        assert_eq!(Channel::BottomFrontCenter.name(), "BFC");
        assert_eq!(Channel::BottomFrontLeft.name(), "BFL");
        assert_eq!(Channel::BottomFrontRight.name(), "BFR");
        assert_eq!(Channel::SideSurroundLeft.name(), "SSL");
        assert_eq!(Channel::SideSurroundRight.name(), "SSR");
        assert_eq!(Channel::TopSurroundLeft.name(), "TTL");
        assert_eq!(Channel::TopSurroundRight.name(), "TTR");
        assert_eq!(Channel::BinauralLeft.name(), "BIL");
        assert_eq!(Channel::BinauralRight.name(), "BIR");

        assert_eq!(Channel::from_name("fl"), Some(Channel::FrontLeft));
        assert_eq!(Channel::from_name("LFE"), Some(Channel::LowFrequency));
        assert_eq!(Channel::from_name("flc"), Some(Channel::FrontLeftOfCenter));
        assert_eq!(Channel::from_name("FRC"), Some(Channel::FrontRightOfCenter));
        assert_eq!(Channel::from_name("bc"), Some(Channel::BackCenter));
        assert_eq!(Channel::from_name("tc"), Some(Channel::TopCenter));
        assert_eq!(Channel::from_name("tfl"), Some(Channel::TopFrontLeft));
        assert_eq!(Channel::from_name("TFC"), Some(Channel::TopFrontCenter));
        assert_eq!(Channel::from_name("TFR"), Some(Channel::TopFrontRight));
        assert_eq!(Channel::from_name("tbl"), Some(Channel::TopBackLeft));
        assert_eq!(Channel::from_name("TBC"), Some(Channel::TopBackCenter));
        assert_eq!(Channel::from_name("TBR"), Some(Channel::TopBackRight));
        assert_eq!(Channel::from_name("dl"), Some(Channel::StereoLeft));
        assert_eq!(Channel::from_name("DR"), Some(Channel::StereoRight));
        assert_eq!(Channel::from_name("wl"), Some(Channel::WideLeft));
        assert_eq!(Channel::from_name("WR"), Some(Channel::WideRight));
        assert_eq!(Channel::from_name("sdl"), Some(Channel::SurroundDirectLeft));
        assert_eq!(
            Channel::from_name("SDR"),
            Some(Channel::SurroundDirectRight)
        );
        assert_eq!(Channel::from_name("lfe2"), Some(Channel::LowFrequency2));
        assert_eq!(Channel::from_name("tsl"), Some(Channel::TopSideLeft));
        assert_eq!(Channel::from_name("TSR"), Some(Channel::TopSideRight));
        assert_eq!(Channel::from_name("bfc"), Some(Channel::BottomFrontCenter));
        assert_eq!(Channel::from_name("BFL"), Some(Channel::BottomFrontLeft));
        assert_eq!(Channel::from_name("bfr"), Some(Channel::BottomFrontRight));
        assert_eq!(Channel::from_name("ssl"), Some(Channel::SideSurroundLeft));
        assert_eq!(Channel::from_name("SSR"), Some(Channel::SideSurroundRight));
        assert_eq!(Channel::from_name("ttl"), Some(Channel::TopSurroundLeft));
        assert_eq!(Channel::from_name("TTR"), Some(Channel::TopSurroundRight));
        assert_eq!(Channel::from_name("bil"), Some(Channel::BinauralLeft));
        assert_eq!(Channel::from_name("BIR"), Some(Channel::BinauralRight));
        assert_eq!(Channel::from_name("unknown"), None);
        assert_eq!(Channel::FrontLeft.mask(), 1);
        assert_eq!(Channel::LowFrequency.mask(), 1 << 3);
        assert_eq!(Channel::FrontLeftOfCenter.mask(), 1 << 6);
        assert_eq!(Channel::FrontRightOfCenter.mask(), 1 << 7);
        assert_eq!(Channel::BackCenter.mask(), 1 << 8);
        assert_eq!(Channel::SideLeft.mask(), 1 << 9);
        assert_eq!(Channel::SideRight.mask(), 1 << 10);
        assert_eq!(Channel::TopCenter.mask(), 1 << 11);
        assert_eq!(Channel::TopFrontLeft.mask(), 1 << 12);
        assert_eq!(Channel::TopFrontCenter.mask(), 1 << 13);
        assert_eq!(Channel::TopFrontRight.mask(), 1 << 14);
        assert_eq!(Channel::TopBackLeft.mask(), 1 << 15);
        assert_eq!(Channel::TopBackCenter.mask(), 1 << 16);
        assert_eq!(Channel::TopBackRight.mask(), 1 << 17);
        assert_eq!(Channel::StereoLeft.mask(), 1 << 29);
        assert_eq!(Channel::StereoRight.mask(), 1 << 30);
        assert_eq!(Channel::WideLeft.mask(), 1 << 31);
        assert_eq!(Channel::WideRight.mask(), 1 << 32);
        assert_eq!(Channel::SurroundDirectLeft.mask(), 1 << 33);
        assert_eq!(Channel::SurroundDirectRight.mask(), 1 << 34);
        assert_eq!(Channel::LowFrequency2.mask(), 1 << 35);
        assert_eq!(Channel::TopSideLeft.mask(), 1 << 36);
        assert_eq!(Channel::TopSideRight.mask(), 1 << 37);
        assert_eq!(Channel::BottomFrontCenter.mask(), 1 << 38);
        assert_eq!(Channel::BottomFrontLeft.mask(), 1 << 39);
        assert_eq!(Channel::BottomFrontRight.mask(), 1 << 40);
        assert_eq!(Channel::SideSurroundLeft.mask(), 1 << 41);
        assert_eq!(Channel::SideSurroundRight.mask(), 1 << 42);
        assert_eq!(Channel::TopSurroundLeft.mask(), 1 << 43);
        assert_eq!(Channel::TopSurroundRight.mask(), 1 << 44);
        assert_eq!(Channel::BinauralLeft.mask(), 1 << 61);
        assert_eq!(Channel::BinauralRight.mask(), 1 << 62);
    }

    #[test]
    fn channel_ids_report_ffmpeg_raw_names_and_descriptions() {
        assert_eq!(Channel::FrontLeft.raw_id(), 0);
        assert_eq!(Channel::BinauralRight.raw_id(), 62);
        assert_eq!(Channel::from_raw_id(33), Some(Channel::SurroundDirectLeft));
        assert_eq!(Channel::from_raw_id(45), None);
        assert_eq!(
            Channel::TopSurroundRight.description(),
            "top surround right"
        );

        let front_left = ChannelId::from_raw(0);
        assert_eq!(front_left, ChannelId::Native(Channel::FrontLeft));
        assert_eq!(front_left.raw_id(), 0);
        assert_eq!(front_left.name(), "FL");
        assert_eq!(front_left.description(), "front left");
        assert_eq!(front_left.native(), Some(Channel::FrontLeft));
        assert_eq!(
            ChannelId::from_canonical_name("FL"),
            Some(ChannelId::Native(Channel::FrontLeft))
        );
        assert_eq!(ChannelId::from_canonical_name("fl"), None);

        assert_eq!(ChannelId::from_raw(-1), ChannelId::None);
        assert_eq!(ChannelId::None.name(), "NONE");
        assert_eq!(ChannelId::None.description(), "none");
        assert_eq!(
            ChannelId::from_canonical_name("NONE"),
            Some(ChannelId::None)
        );

        assert_eq!(ChannelId::from_raw(0x200), ChannelId::Unused);
        assert_eq!(ChannelId::Unused.name(), "UNSD");
        assert_eq!(ChannelId::Unused.description(), "unused");
        assert_eq!(
            ChannelId::from_canonical_name("UNSD"),
            Some(ChannelId::Unused)
        );
        assert_eq!(ChannelId::from_canonical_name("USR512"), None);

        assert_eq!(ChannelId::from_raw(0x300), ChannelId::Unknown);
        assert_eq!(ChannelId::Unknown.name(), "UNK");
        assert_eq!(ChannelId::Unknown.description(), "unknown");
        assert_eq!(
            ChannelId::from_canonical_name("UNK"),
            Some(ChannelId::Unknown)
        );

        let ambisonic = ChannelId::from_raw(0x400);
        assert_eq!(ambisonic, ChannelId::Ambisonic(0));
        assert_eq!(ambisonic.name(), "AMBI0");
        assert_eq!(ambisonic.description(), "ambisonic ACN 0");
        assert_eq!(ambisonic.raw_id(), 0x400);
        assert_eq!(
            ChannelId::from_canonical_name("AMBI1023"),
            Some(ChannelId::Ambisonic(1023))
        );
        assert_eq!(ChannelId::from_raw(0x7ff), ChannelId::Ambisonic(1023));
        assert_eq!(ChannelId::from_canonical_name("AMBI1024"), None);
        assert_eq!(ChannelId::from_canonical_name("AMBI"), None);

        let user = ChannelId::from_raw(0x800);
        assert_eq!(user, ChannelId::User(0x800));
        assert_eq!(user.name(), "USR2048");
        assert_eq!(user.description(), "user 2048");
        assert_eq!(user.raw_id(), 0x800);
        assert_eq!(
            ChannelId::from_canonical_name("USR2048"),
            Some(ChannelId::User(0x800))
        );
        assert_eq!(ChannelId::from_raw(-2).name(), "USR-2");
        assert_eq!(ChannelId::from_canonical_name("USR-2"), None);
    }

    #[test]
    fn custom_channel_layouts_model_ffmpeg_map_entries() {
        let unknown = CustomChannelLayout::unknown(3).unwrap();
        assert_eq!(unknown.channel_count(), 3);
        assert_eq!(unknown.channel_from_index(0), Some(ChannelId::Unknown));
        assert_eq!(unknown.channel_from_index(2), Some(ChannelId::Unknown));
        assert_eq!(unknown.channel_from_index(3), None);
        assert_eq!(unknown.index_from_channel(ChannelId::Unknown).unwrap(), 0);
        assert!(!unknown.has_custom_names());
        assert!(unknown.canonical_native_mask().is_err());
        assert_eq!(unknown.canonical_native_layout(), None);
        assert_eq!(unknown.describe(), "3 channels (UNK+UNK+UNK)");

        let layout = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "Left").unwrap(),
            ChannelCustom::new(ChannelId::Unknown, "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(2), "W").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "SecondLeft").unwrap(),
            ChannelCustom::new(ChannelId::Unused, "Gap").unwrap(),
            ChannelCustom::new(ChannelId::User(0x800), "Vendor").unwrap(),
        ])
        .unwrap();

        assert_eq!(ChannelCustom::NAME_STORAGE_BYTES, 16);
        assert_eq!(ChannelCustom::MAX_NAME_BYTES, 15);
        assert_eq!(layout.channel_count(), 6);
        assert!(layout.has_custom_names());
        assert_eq!(
            layout.channels()[0].id(),
            ChannelId::Native(Channel::FrontLeft)
        );
        assert_eq!(layout.channels()[0].name(), "Left");
        assert!(layout.channels()[0].has_name());
        assert_eq!(layout.channels()[1].name(), "");
        assert!(!layout.channels()[1].has_name());
        assert_eq!(layout.channel_from_index(2), Some(ChannelId::Ambisonic(2)));
        assert_eq!(
            layout
                .index_from_channel(ChannelId::Native(Channel::FrontLeft))
                .unwrap(),
            0
        );
        assert_eq!(
            layout.index_from_channel(ChannelId::Ambisonic(2)).unwrap(),
            2
        );
        assert_eq!(layout.index_from_channel(ChannelId::Unused).unwrap(), 4);
        assert_eq!(
            layout.index_from_channel(ChannelId::User(0x800)).unwrap(),
            5
        );
        assert_eq!(layout.index_from_string("FL").unwrap(), 0);
        assert_eq!(layout.index_from_string("AMBI2").unwrap(), 2);
        assert_eq!(layout.index_from_string("FL@Left").unwrap(), 0);
        assert_eq!(layout.index_from_string("@SecondLeft").unwrap(), 3);
        assert_eq!(layout.index_from_string("UNSD@Gap").unwrap(), 4);
        assert_eq!(layout.index_from_string("USR2048@Vendor").unwrap(), 5);
        assert_eq!(
            layout.channel_from_string("FL"),
            Some(ChannelId::Native(Channel::FrontLeft))
        );
        assert_eq!(
            layout.channel_from_string("@SecondLeft"),
            Some(ChannelId::Native(Channel::FrontLeft))
        );
        assert_eq!(
            layout.channel_from_string("AMBI2"),
            Some(ChannelId::Ambisonic(2))
        );
        assert_eq!(
            layout.channel_from_string("USR2048@Vendor"),
            Some(ChannelId::User(0x800))
        );
        assert_eq!(layout.channel_from_string("FR@Left"), None);
        assert_eq!(layout.channel_from_string(""), None);
        assert!(layout.canonical_native_mask().is_err());
        assert_eq!(layout.canonical_native_layout(), None);
        assert_eq!(
            layout.describe(),
            "6 channels (FL@Left+UNK+AMBI2@W+FL@SecondLeft+UNSD@Gap+USR2048@Vendor)"
        );

        for lookup in ["", "NOPE", "FL@", "FR@Left", "FL@Missing", "FL\0"] {
            let err = layout.index_from_string(lookup).unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        }
        for id in [
            ChannelId::None,
            ChannelId::Native(Channel::FrontRight),
            ChannelId::Ambisonic(1024),
        ] {
            let err = layout.index_from_channel(id).unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        }
    }

    #[test]
    fn custom_channel_layouts_retype_to_native_only_when_lossless() {
        let stereo = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
        ])
        .unwrap();
        assert!(!stereo.has_custom_names());
        assert_eq!(
            stereo.canonical_native_mask().unwrap(),
            ChannelLayout::stereo().channel_mask()
        );
        assert_eq!(
            stereo.canonical_native_layout(),
            Some(ChannelLayout::stereo())
        );
        assert_eq!(stereo.describe(), "stereo");

        let named_stereo = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "Left").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "Right").unwrap(),
        ])
        .unwrap();
        assert!(named_stereo.has_custom_names());
        assert_eq!(
            named_stereo.canonical_native_mask().unwrap(),
            ChannelLayout::stereo().channel_mask()
        );
        assert_eq!(named_stereo.canonical_native_layout(), None);
        assert_eq!(named_stereo.describe(), "2 channels (FL@Left+FR@Right)");

        let out_of_order = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
        ])
        .unwrap();
        let err = out_of_order.canonical_native_mask().unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(out_of_order.canonical_native_layout(), None);
        assert_eq!(out_of_order.describe(), "2 channels (FR+FL)");

        let duplicate = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
        ])
        .unwrap();
        let err = duplicate.canonical_native_mask().unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);

        let non_native = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
            ChannelCustom::new(ChannelId::Unknown, "").unwrap(),
        ])
        .unwrap();
        let err = non_native.canonical_native_mask().unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    }

    #[test]
    fn custom_channel_layouts_detect_ambisonic_order() {
        let zeroth_order =
            CustomChannelLayout::new(vec![
                ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap()
            ])
            .unwrap();
        assert_eq!(zeroth_order.ambisonic_order().unwrap(), 0);
        assert_eq!(zeroth_order.describe(), "ambisonic 0");

        let named_zeroth_order =
            CustomChannelLayout::new(vec![
                ChannelCustom::new(ChannelId::Ambisonic(0), "W").unwrap()
            ])
            .unwrap();
        assert_eq!(named_zeroth_order.ambisonic_order().unwrap(), 0);
        assert_eq!(named_zeroth_order.describe(), "ambisonic 0");

        let first_order = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(2), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(3), "").unwrap(),
        ])
        .unwrap();
        assert_eq!(first_order.ambisonic_order().unwrap(), 1);
        assert_eq!(first_order.describe(), "ambisonic 1");

        let first_order_with_native_extra = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(2), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(3), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
        ])
        .unwrap();
        assert_eq!(first_order_with_native_extra.ambisonic_order().unwrap(), 1);
        assert_eq!(
            first_order_with_native_extra.describe(),
            "ambisonic 1+stereo"
        );

        let first_order_with_named_extra = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(2), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(3), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "Left").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "Right").unwrap(),
        ])
        .unwrap();
        assert_eq!(first_order_with_named_extra.ambisonic_order().unwrap(), 1);
        assert_eq!(
            first_order_with_named_extra.describe(),
            "ambisonic 1+2 channels (FL@Left+FR@Right)"
        );

        let first_order_with_custom_extra = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(2), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(3), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
        ])
        .unwrap();
        assert_eq!(first_order_with_custom_extra.ambisonic_order().unwrap(), 1);
        assert_eq!(
            first_order_with_custom_extra.describe(),
            "ambisonic 1+2 channels (FR+FL)"
        );
    }

    #[test]
    fn custom_channel_layouts_reject_invalid_ambisonic_orders() {
        let native_only = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
        ])
        .unwrap();
        assert_eq!(
            native_only.ambisonic_order().unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(native_only.describe(), "stereo");

        for invalid in [
            vec![
                ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap(),
                ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap(),
            ],
            vec![ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap()],
            vec![
                ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
                ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap(),
            ],
            vec![
                ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap(),
                ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
                ChannelCustom::new(ChannelId::Ambisonic(2), "").unwrap(),
            ],
        ] {
            let layout = CustomChannelLayout::new(invalid).unwrap();
            let err = layout.ambisonic_order().unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
            assert!(layout.describe().contains("channels ("));
        }
    }

    #[test]
    fn channel_layouts_compare_current_native_and_custom_subset() {
        let stereo = ChannelLayout::stereo();
        assert!(stereo.is_equivalent_to(ChannelLayout::stereo()));
        assert!(!stereo.is_equivalent_to(ChannelLayout::downmix()));
        assert!(!ChannelLayout::five_one().is_equivalent_to(ChannelLayout::five_one_side()));

        let custom_stereo = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
        ])
        .unwrap();
        let named_custom_stereo = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "Left").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "Right").unwrap(),
        ])
        .unwrap();
        assert!(custom_stereo.is_equivalent_to_native(stereo));
        assert!(stereo.is_equivalent_to_custom(&named_custom_stereo));
        assert!(named_custom_stereo.is_equivalent_to_custom(&custom_stereo));

        let reversed_custom_stereo = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
        ])
        .unwrap();
        assert!(!reversed_custom_stereo.is_equivalent_to_native(stereo));
        assert!(!reversed_custom_stereo.is_equivalent_to_custom(&custom_stereo));

        let unknown_pair = CustomChannelLayout::unknown(2).unwrap();
        let named_unknown_pair = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Unknown, "A").unwrap(),
            ChannelCustom::new(ChannelId::Unknown, "B").unwrap(),
        ])
        .unwrap();
        assert!(unknown_pair.is_equivalent_to_custom(&named_unknown_pair));
        assert!(!unknown_pair.is_equivalent_to_native(stereo));

        let ambisonic = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Ambisonic(0), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(1), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(2), "").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(3), "").unwrap(),
        ])
        .unwrap();
        let named_ambisonic = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Ambisonic(0), "W").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(1), "Y").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(2), "Z").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(3), "X").unwrap(),
        ])
        .unwrap();
        assert!(ambisonic.is_equivalent_to_custom(&named_ambisonic));
        assert!(!ambisonic.is_equivalent_to_native(ChannelLayout::four_zero()));
    }

    #[test]
    fn channel_layouts_report_source_shaped_native_subsets() {
        let stereo_mask = ChannelLayout::stereo().channel_mask();
        assert_eq!(
            ChannelLayout::five_one().subset_mask(stereo_mask),
            stereo_mask
        );
        assert_eq!(
            ChannelLayout::stereo()
                .subset_mask(Channel::FrontLeft.mask() | Channel::BackLeft.mask()),
            Channel::FrontLeft.mask()
        );
        assert_eq!(ChannelLayout::stereo().subset_mask(0), 0);

        let mixed_custom = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "Left").unwrap(),
            ChannelCustom::new(ChannelId::Unknown, "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "SecondLeft").unwrap(),
            ChannelCustom::new(ChannelId::Ambisonic(0), "W").unwrap(),
            ChannelCustom::new(ChannelId::User(2048), "Vendor").unwrap(),
        ])
        .unwrap();
        assert_eq!(
            mixed_custom.subset_native_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
            ),
            ChannelLayout::stereo().channel_mask()
        );

        let out_of_order_custom = CustomChannelLayout::new(vec![
            ChannelCustom::new(ChannelId::Native(Channel::FrontRight), "").unwrap(),
            ChannelCustom::new(ChannelId::Native(Channel::FrontLeft), "").unwrap(),
        ])
        .unwrap();
        assert_eq!(
            out_of_order_custom.subset_native_mask(stereo_mask),
            stereo_mask
        );
        assert_eq!(
            CustomChannelLayout::unknown(2)
                .unwrap()
                .subset_native_mask(stereo_mask),
            0
        );
    }

    #[test]
    fn channel_layouts_lookup_channels_in_source_order() {
        let stereo = ChannelLayout::stereo();
        assert_eq!(
            stereo.channel_from_index(0),
            Some(ChannelId::Native(Channel::FrontLeft))
        );
        assert_eq!(
            stereo.channel_from_index(1),
            Some(ChannelId::Native(Channel::FrontRight))
        );
        assert_eq!(stereo.channel_from_index(2), None);
        assert_eq!(
            stereo
                .index_from_channel(ChannelId::Native(Channel::FrontRight))
                .unwrap(),
            1
        );
        assert_eq!(stereo.index_from_string("FR").unwrap(), 1);
        assert_eq!(
            stereo.channel_from_string("FR"),
            Some(ChannelId::Native(Channel::FrontRight))
        );
        assert_eq!(stereo.channel_from_string("BR"), None);

        for id in [
            ChannelId::None,
            ChannelId::Unknown,
            ChannelId::Ambisonic(0),
            ChannelId::User(0x800),
            ChannelId::Native(Channel::BackLeft),
        ] {
            let err = stereo.index_from_channel(id).unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        }
        for lookup in ["", "BR", "AMBI0", "FL@Left", "NOPE", "FL\0"] {
            let err = stereo.index_from_string(lookup).unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        }

        let twenty_two_two = ChannelLayout::twenty_two_two();
        assert_eq!(
            twenty_two_two.channel_from_index(8),
            Some(ChannelId::Native(Channel::BackCenter))
        );
        assert_eq!(
            twenty_two_two
                .index_from_channel(ChannelId::Native(Channel::TopCenter))
                .unwrap(),
            11
        );
        assert_eq!(twenty_two_two.index_from_string("BFC").unwrap(), 21);
        assert_eq!(
            twenty_two_two.channel_from_index(21),
            Some(ChannelId::Native(Channel::BottomFrontCenter))
        );
        assert_eq!(
            twenty_two_two.channel_from_index(23),
            Some(ChannelId::Native(Channel::BottomFrontRight))
        );
        assert_eq!(twenty_two_two.channel_from_index(24), None);
    }

    #[test]
    fn custom_channel_layouts_reject_invalid_entries() {
        for invalid in [
            CustomChannelLayout::unknown(0),
            CustomChannelLayout::new(Vec::new()),
        ] {
            let err = invalid.unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        }

        let err = ChannelCustom::new(ChannelId::None, "").unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        let err = ChannelCustom::new(ChannelId::Ambisonic(1024), "").unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        let err = ChannelCustom::new(ChannelId::Unknown, "1234567890123456").unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        let err = ChannelCustom::new(ChannelId::Unknown, "bad\0name").unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);

        let max_name = "123456789012345";
        assert_eq!(
            ChannelCustom::new(ChannelId::Unknown, max_name)
                .unwrap()
                .name(),
            max_name
        );
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

        let hexadecagonal = ChannelLayout::hexadecagonal();
        assert_eq!(hexadecagonal.name(), "hexadecagonal");
        assert_eq!(hexadecagonal.channel_count(), 16);
        assert!(hexadecagonal.contains(Channel::WideLeft));
        assert!(hexadecagonal.contains(Channel::WideRight));
        assert!(hexadecagonal.contains(Channel::TopFrontCenter));
        assert!(!hexadecagonal.contains(Channel::LowFrequency));
        assert!(!hexadecagonal.contains(Channel::TopSideLeft));

        let binaural = ChannelLayout::binaural();
        assert_eq!(binaural.name(), "binaural");
        assert_eq!(binaural.channel_count(), 2);
        assert_eq!(
            binaural.channels(),
            &[Channel::BinauralLeft, Channel::BinauralRight]
        );
        assert!(!binaural.contains(Channel::FrontLeft));

        let downmix = ChannelLayout::downmix();
        assert_eq!(downmix.name(), "downmix");
        assert_eq!(downmix.channel_count(), 2);
        assert_eq!(
            downmix.channels(),
            &[Channel::StereoLeft, Channel::StereoRight]
        );
        assert!(!downmix.contains(Channel::FrontLeft));

        let twenty_two_two = ChannelLayout::twenty_two_two();
        assert_eq!(twenty_two_two.name(), "22.2");
        assert_eq!(twenty_two_two.channel_count(), 24);
        assert_eq!(
            twenty_two_two.channels(),
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
                Channel::BackCenter,
                Channel::LowFrequency2,
                Channel::TopFrontCenter,
                Channel::TopCenter,
                Channel::TopBackCenter,
                Channel::BottomFrontCenter,
                Channel::BottomFrontLeft,
                Channel::BottomFrontRight,
            ]
        );
        assert!(twenty_two_two.contains(Channel::TopCenter));
        assert!(twenty_two_two.contains(Channel::BottomFrontCenter));
        assert!(twenty_two_two.contains(Channel::BottomFrontLeft));
        assert!(twenty_two_two.contains(Channel::BottomFrontRight));
        assert!(!twenty_two_two.contains(Channel::BinauralLeft));
        assert!(!twenty_two_two.contains(Channel::StereoLeft));
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
            ChannelLayout::twenty_two_two().channel_string(),
            "FL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFL+TFR+TBL+TBR+TSL+TSR+BC+LFE2+TFC+TC+TBC+BFC+BFL+BFR"
        );
        assert_eq!(
            ChannelLayout::hexadecagonal().channel_string(),
            "FL+FR+FC+BL+BR+BC+SL+SR+WL+WR+TBL+TBR+TBC+TFC+TFL+TFR"
        );
        assert_eq!(ChannelLayout::binaural().channel_string(), "BIL+BIR");
        assert_eq!(ChannelLayout::downmix().channel_string(), "DL+DR");
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
            ChannelLayout::from_channel_mask(
                Channel::FrontLeft.mask()
                    | Channel::FrontRight.mask()
                    | Channel::FrontCenter.mask()
                    | Channel::BackLeft.mask()
                    | Channel::BackRight.mask()
                    | Channel::BackCenter.mask()
                    | Channel::SideLeft.mask()
                    | Channel::SideRight.mask()
                    | Channel::WideLeft.mask()
                    | Channel::WideRight.mask()
                    | Channel::TopBackLeft.mask()
                    | Channel::TopBackRight.mask()
                    | Channel::TopBackCenter.mask()
                    | Channel::TopFrontCenter.mask()
                    | Channel::TopFrontLeft.mask()
                    | Channel::TopFrontRight.mask()
            ),
            Some(ChannelLayout::hexadecagonal())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::TopFrontRight,
                Channel::TopFrontLeft,
                Channel::TopFrontCenter,
                Channel::TopBackCenter,
                Channel::TopBackRight,
                Channel::TopBackLeft,
                Channel::WideRight,
                Channel::WideLeft,
                Channel::SideRight,
                Channel::SideLeft,
                Channel::BackCenter,
                Channel::BackRight,
                Channel::BackLeft,
                Channel::FrontCenter,
                Channel::FrontRight,
                Channel::FrontLeft,
            ]),
            Some(ChannelLayout::hexadecagonal())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::BinauralLeft.mask() | Channel::BinauralRight.mask()
            ),
            Some(ChannelLayout::binaural())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[Channel::BinauralRight, Channel::BinauralLeft]),
            Some(ChannelLayout::binaural())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                Channel::StereoLeft.mask() | Channel::StereoRight.mask()
            ),
            Some(ChannelLayout::downmix())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[Channel::StereoRight, Channel::StereoLeft]),
            Some(ChannelLayout::downmix())
        );
        assert_eq!(
            ChannelLayout::from_channel_mask(
                ChannelLayout::nine_one_six().channel_mask()
                    | Channel::BackCenter.mask()
                    | Channel::LowFrequency2.mask()
                    | Channel::TopFrontCenter.mask()
                    | Channel::TopCenter.mask()
                    | Channel::TopBackCenter.mask()
                    | Channel::BottomFrontCenter.mask()
                    | Channel::BottomFrontLeft.mask()
                    | Channel::BottomFrontRight.mask()
            ),
            Some(ChannelLayout::twenty_two_two())
        );
        assert_eq!(
            ChannelLayout::from_channels(&[
                Channel::BottomFrontRight,
                Channel::BottomFrontLeft,
                Channel::BottomFrontCenter,
                Channel::TopBackCenter,
                Channel::TopCenter,
                Channel::TopFrontCenter,
                Channel::LowFrequency2,
                Channel::BackCenter,
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
            Some(ChannelLayout::twenty_two_two())
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
                "hexadecagonal",
                "binaural",
                "downmix",
                "22.2",
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
        assert_eq!(
            ChannelLayout::from_name("hexadecagonal"),
            Some(ChannelLayout::hexadecagonal())
        );
        assert_eq!(
            ChannelLayout::from_name("binaural"),
            Some(ChannelLayout::binaural())
        );
        assert_eq!(
            ChannelLayout::from_name("downmix"),
            Some(ChannelLayout::downmix())
        );
        assert_eq!(
            ChannelLayout::from_name("22.2"),
            Some(ChannelLayout::twenty_two_two())
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
        assert_eq!(
            ChannelLayout::default_for_count(10),
            Some(ChannelLayout::five_one_four())
        );
        assert_eq!(ChannelLayout::default_for_count(11), None);
        assert_eq!(
            ChannelLayout::default_for_count(12),
            Some(ChannelLayout::seven_one_four())
        );
        assert_eq!(ChannelLayout::default_for_count(13), None);
        assert_eq!(
            ChannelLayout::default_for_count(14),
            Some(ChannelLayout::nine_one_four())
        );
        assert_eq!(ChannelLayout::default_for_count(15), None);
        assert_eq!(
            ChannelLayout::default_for_count(16),
            Some(ChannelLayout::nine_one_six())
        );
        assert_eq!(ChannelLayout::default_for_count(17), None);
        assert_eq!(ChannelLayout::default_for_count(23), None);
        assert_eq!(
            ChannelLayout::default_for_count(24),
            Some(ChannelLayout::twenty_two_two())
        );
    }

    #[test]
    fn unspecified_layouts_model_ffmpeg_count_only_order() {
        let unspecified = UnspecifiedChannelLayout::new(9).unwrap();
        assert_eq!(unspecified.channel_count(), 9);
        assert_eq!(unspecified.describe(), "9 channels");
        assert_eq!(
            unspecified.subset_mask(ChannelLayout::stereo().channel_mask()),
            0
        );
        assert_eq!(unspecified.channel_from_index(0), None);
        assert!(unspecified.validate_channel_count(9).is_ok());
        assert_eq!(
            unspecified.validate_channel_count(8).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            UnspecifiedChannelLayout::new(0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        let default_unspecified = ChannelLayoutSpec::default_for_count(9).unwrap();
        assert_eq!(
            default_unspecified,
            ChannelLayoutSpec::Unspecified(unspecified)
        );
        assert!(default_unspecified.is_unspecified());
        assert_eq!(default_unspecified.as_native(), None);
        assert_eq!(default_unspecified.as_unspecified(), Some(unspecified));
        assert_eq!(default_unspecified.channel_count(), 9);
        assert_eq!(default_unspecified.describe(), "9 channels");
        assert_eq!(
            default_unspecified.subset_mask(ChannelLayout::stereo().channel_mask()),
            0
        );
        assert_eq!(default_unspecified.channel_from_index(0), None);
        assert!(default_unspecified.validate_channel_count(9).is_ok());
        assert_eq!(
            ChannelLayoutSpec::default_for_count(0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        let default_native = ChannelLayoutSpec::default_for_count(10).unwrap();
        assert_eq!(
            default_native,
            ChannelLayoutSpec::Native(ChannelLayout::five_one_four())
        );
        assert_eq!(
            default_native.as_native(),
            Some(ChannelLayout::five_one_four())
        );
        assert_eq!(default_native.describe(), "5.1.4");
        assert_eq!(default_native.channel_count(), 10);
        assert!(!default_native.is_unspecified());

        assert!(ChannelLayoutSpec::unspecified(9)
            .unwrap()
            .is_equivalent_to(ChannelLayoutSpec::unspecified(9).unwrap()));
        assert!(!ChannelLayoutSpec::unspecified(9)
            .unwrap()
            .is_equivalent_to(ChannelLayoutSpec::unspecified(8).unwrap()));
        assert!(!ChannelLayoutSpec::unspecified(2)
            .unwrap()
            .is_equivalent_to(ChannelLayoutSpec::Native(ChannelLayout::stereo())));
        assert!(ChannelLayoutSpec::Native(ChannelLayout::stereo())
            .is_equivalent_to_native(ChannelLayout::stereo()));
        assert!(!ChannelLayoutSpec::unspecified(2)
            .unwrap()
            .is_equivalent_to_native(ChannelLayout::stereo()));
        assert!(!ChannelLayoutSpec::unspecified(2)
            .unwrap()
            .is_equivalent_to_custom(&CustomChannelLayout::unknown(2).unwrap()));
    }

    #[test]
    fn layout_spec_parser_models_ffmpeg_count_suffixes() {
        assert_eq!(
            ChannelLayoutSpec::parse("stereo").unwrap(),
            ChannelLayoutSpec::Native(ChannelLayout::stereo())
        );
        assert_eq!(
            ChannelLayoutSpec::parse("FL+FR").unwrap(),
            ChannelLayoutSpec::Native(ChannelLayout::stereo())
        );
        assert_eq!(
            ChannelLayoutSpec::parse("2 channels (FL+FR)").unwrap(),
            ChannelLayoutSpec::Native(ChannelLayout::stereo())
        );
        assert_eq!(
            ChannelLayoutSpec::parse("10c").unwrap(),
            ChannelLayoutSpec::Native(ChannelLayout::five_one_four())
        );
        assert_eq!(
            ChannelLayoutSpec::parse("2c").unwrap(),
            ChannelLayoutSpec::Native(ChannelLayout::stereo())
        );
        assert_eq!(
            ChannelLayoutSpec::parse("2C").unwrap(),
            ChannelLayoutSpec::unspecified(2).unwrap()
        );
        assert_eq!(
            ChannelLayoutSpec::parse("2 channels").unwrap(),
            ChannelLayoutSpec::unspecified(2).unwrap()
        );
        assert_eq!(
            ChannelLayoutSpec::parse("9C").unwrap(),
            ChannelLayoutSpec::unspecified(9).unwrap()
        );
        assert_eq!(
            ChannelLayoutSpec::parse("9 channels").unwrap(),
            ChannelLayoutSpec::unspecified(9).unwrap()
        );

        for input in [
            "",
            "0C",
            "0 channels",
            "9c",
            "3 channels (FL+FR)",
            "2 channels (FL+FR",
            "2 channels ()",
            "2channels",
            "2 channels trailing",
            "2 channels (FL+FR) trailing",
            "FL\0FR",
        ] {
            let err = ChannelLayoutSpec::parse(input).unwrap_err();
            assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        }
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
        assert_eq!(
            ChannelLayout::parse("22.2").unwrap(),
            ChannelLayout::twenty_two_two()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFL+TFR+TBL+TBR+TSL+TSR+BC+LFE2+TFC+TC+TBC+BFC+BFL+BFR").unwrap(),
            ChannelLayout::twenty_two_two()
        );
        assert_eq!(
            ChannelLayout::parse("BFR+BFL+BFC+TBC+TC+TFC+LFE2+BC+TSR+TSL+TBR+TBL+TFR+TFL+SR+SL+FRC+FLC+BR+BL+LFE+FC+FR+FL").unwrap(),
            ChannelLayout::twenty_two_two()
        );
        assert_eq!(
            ChannelLayout::parse("hexadecagonal").unwrap(),
            ChannelLayout::hexadecagonal()
        );
        assert_eq!(
            ChannelLayout::parse("FL+FR+FC+BL+BR+BC+SL+SR+WL+WR+TBL+TBR+TBC+TFC+TFL+TFR").unwrap(),
            ChannelLayout::hexadecagonal()
        );
        assert_eq!(
            ChannelLayout::parse("TFR+TFL+TFC+TBC+TBR+TBL+WR+WL+SR+SL+BC+BR+BL+FC+FR+FL").unwrap(),
            ChannelLayout::hexadecagonal()
        );
        assert_eq!(
            ChannelLayout::parse("binaural").unwrap(),
            ChannelLayout::binaural()
        );
        assert_eq!(
            ChannelLayout::parse("BIR+BIL").unwrap(),
            ChannelLayout::binaural()
        );
        assert_eq!(
            ChannelLayout::parse("downmix").unwrap(),
            ChannelLayout::downmix()
        );
        assert_eq!(
            ChannelLayout::parse("dr+dl").unwrap(),
            ChannelLayout::downmix()
        );
    }

    #[test]
    fn layout_parser_rejects_invalid_or_unsupported_expressions() {
        for input in [
            "",
            "   ",
            "+",
            "FL+",
            "FL++FR",
            "FL\0FR",
            "FL+FL",
            "FL+BR",
            "SDL+SDR",
            "SSL+SSR",
            "TTL+TTR",
            "SDL+SDR+SSL+SSR+TTL+TTR",
        ] {
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
