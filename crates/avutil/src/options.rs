use crate::{
    color::{find_named_color, parse_color, RgbaColor},
    dict::{Dictionary, MatchMode, SetMode},
    pixel::PixelFormat,
    samplefmt::SampleFormat,
    AvError, AvErrorCode, AvErrorKind, AvResult, ChannelLayoutSpec, Rational,
};

#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Bool(bool),
    Int(i64),
    Duration(i64),
    ImageSize { width: i32, height: i32 },
    PixelFormat(Option<PixelFormat>),
    SampleFormat(Option<SampleFormat>),
    ChannelLayout(ChannelLayoutSpec),
    VideoRate(Rational),
    Color(RgbaColor),
    Binary(Vec<u8>),
    Dictionary(Dictionary),
    Array(Vec<OptionValue>),
    Float(f64),
    Rational(Rational),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionKind {
    Bool,
    Int { min: i64, max: i64 },
    Duration { min: i64, max: i64 },
    ImageSize,
    PixelFormat { min: i32, max: i32 },
    SampleFormat { min: i32, max: i32 },
    ChannelLayout,
    VideoRate { min: Rational, max: Rational },
    Color,
    Binary,
    Dictionary,
    Array(OptionArrayKind),
    Float { min: f64, max: f64 },
    Rational { min: Rational, max: Rational },
    String { allow_empty: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionArrayKind {
    element: Box<OptionKind>,
    min_len: usize,
    max_len: Option<usize>,
    separator: char,
}

impl OptionArrayKind {
    pub fn new(
        element: OptionKind,
        min_len: usize,
        max_len: Option<usize>,
        separator: char,
    ) -> AvResult<Self> {
        let separator = if separator == '\0' { ',' } else { separator };
        if let Some(max_len) = max_len {
            if min_len > max_len {
                return Err(AvError::invalid_argument(
                    "array option minimum length must be <= maximum length",
                ));
            }
        }
        validate_array_separator(separator)?;
        validate_array_element_kind(&element)?;

        Ok(Self {
            element: Box::new(element),
            min_len,
            max_len,
            separator,
        })
    }

    pub fn element(&self) -> &OptionKind {
        &self.element
    }

    pub fn min_len(&self) -> usize {
        self.min_len
    }

    pub fn max_len(&self) -> Option<usize> {
        self.max_len
    }

    pub fn separator(&self) -> char {
        self.separator
    }
}

impl OptionKind {
    pub fn array(
        element: OptionKind,
        min_len: usize,
        max_len: Option<usize>,
        separator: char,
    ) -> AvResult<Self> {
        OptionArrayKind::new(element, min_len, max_len, separator).map(Self::Array)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionRange {
    min: OptionValue,
    max: OptionValue,
}

impl OptionRange {
    pub fn new(min: OptionValue, max: OptionValue) -> AvResult<Self> {
        match (&min, &max) {
            (OptionValue::Int(min), OptionValue::Int(max)) => {
                if min > max {
                    return Err(AvError::invalid_argument(
                        "integer option range min must be <= max",
                    ));
                }
            }
            (OptionValue::Duration(min), OptionValue::Duration(max)) => {
                if min > max {
                    return Err(AvError::invalid_argument(
                        "duration option range min must be <= max",
                    ));
                }
            }
            (OptionValue::Float(min), OptionValue::Float(max)) => {
                if !min.is_finite() || !max.is_finite() {
                    return Err(AvError::invalid_argument(
                        "float option range bounds must be finite",
                    ));
                }
                if min > max {
                    return Err(AvError::invalid_argument(
                        "float option range min must be <= max",
                    ));
                }
            }
            (OptionValue::Rational(min), OptionValue::Rational(max)) => {
                validate_rational_bound(*min, "range min")?;
                validate_rational_bound(*max, "range max")?;
                if min > max {
                    return Err(AvError::invalid_argument(
                        "rational option range min must be <= max",
                    ));
                }
            }
            (OptionValue::VideoRate(min), OptionValue::VideoRate(max)) => {
                validate_video_rate_bound(*min, "range min")?;
                validate_video_rate_bound(*max, "range max")?;
                if min > max {
                    return Err(AvError::invalid_argument(
                        "video rate option range min must be <= max",
                    ));
                }
            }
            (OptionValue::PixelFormat(min), OptionValue::PixelFormat(max)) => {
                let min = pixel_format_avoption_index(*min)?;
                let max = pixel_format_avoption_index(*max)?;
                if min > max {
                    return Err(AvError::invalid_argument(
                        "pixel format option range min must be <= max",
                    ));
                }
            }
            (OptionValue::SampleFormat(min), OptionValue::SampleFormat(max)) => {
                let min = sample_format_avoption_index(*min);
                let max = sample_format_avoption_index(*max);
                if min > max {
                    return Err(AvError::invalid_argument(
                        "sample format option range min must be <= max",
                    ));
                }
            }
            _ => {
                return Err(AvError::invalid_argument(
                    "option range bounds must be matching numeric values",
                ));
            }
        }

        Ok(Self { min, max })
    }

    pub fn min(&self) -> &OptionValue {
        &self.min
    }

    pub fn max(&self) -> &OptionValue {
        &self.max
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvOptionRangeEntry {
    value_min: f64,
    value_max: f64,
    component_min: f64,
    component_max: f64,
    is_range: bool,
}

impl AvOptionRangeEntry {
    pub fn value_min(&self) -> f64 {
        self.value_min
    }

    pub fn value_max(&self) -> f64 {
        self.value_max
    }

    pub fn component_min(&self) -> f64 {
        self.component_min
    }

    pub fn component_max(&self) -> f64 {
        self.component_max
    }

    pub fn is_range(&self) -> bool {
        self.is_range
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AvOptionRanges {
    ranges: Vec<AvOptionRangeEntry>,
    nb_components: usize,
}

impl AvOptionRanges {
    pub fn ranges(&self) -> &[AvOptionRangeEntry] {
        &self.ranges
    }

    pub fn nb_ranges(&self) -> usize {
        self.ranges.len()
    }

    pub fn nb_components(&self) -> usize {
        self.nb_components
    }

    fn one(range: AvOptionRangeEntry) -> Self {
        Self {
            ranges: vec![range],
            nb_components: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionConstant {
    name: String,
    unit: String,
    value: OptionValue,
    help: String,
    flags: OptionFlags,
}

impl OptionConstant {
    pub fn new(
        unit: impl Into<String>,
        name: impl Into<String>,
        value: OptionValue,
        help: impl Into<String>,
    ) -> AvResult<Self> {
        Self::new_with_flags(unit, name, value, help, OptionFlags::empty())
    }

    pub fn new_with_flags(
        unit: impl Into<String>,
        name: impl Into<String>,
        value: OptionValue,
        help: impl Into<String>,
        flags: OptionFlags,
    ) -> AvResult<Self> {
        let unit = validate_unit(&unit.into())?;
        let name = validate_name(&name.into())?;
        let help = validate_help(&help.into())?;

        Ok(Self {
            name,
            unit,
            value,
            help,
            flags: OptionFlags::from_bits_truncate(flags.bits()),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn unit(&self) -> &str {
        &self.unit
    }

    pub fn value(&self) -> &OptionValue {
        &self.value
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn flags(&self) -> OptionFlags {
        self.flags
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OptionFlags {
    bits: u32,
}

impl OptionFlags {
    pub const ENCODING_PARAM: Self = Self { bits: 1 << 0 };
    pub const DECODING_PARAM: Self = Self { bits: 1 << 1 };
    pub const AUDIO_PARAM: Self = Self { bits: 1 << 3 };
    pub const VIDEO_PARAM: Self = Self { bits: 1 << 4 };
    pub const SUBTITLE_PARAM: Self = Self { bits: 1 << 5 };
    pub const EXPORT: Self = Self { bits: 1 << 6 };
    pub const READONLY: Self = Self { bits: 1 << 7 };
    pub const BSF_PARAM: Self = Self { bits: 1 << 8 };
    pub const RUNTIME_PARAM: Self = Self { bits: 1 << 15 };
    pub const FILTERING_PARAM: Self = Self { bits: 1 << 16 };
    pub const DEPRECATED: Self = Self { bits: 1 << 17 };
    pub const CHILD_CONSTS: Self = Self { bits: 1 << 18 };

    const KNOWN_BITS: u32 = Self::ENCODING_PARAM.bits
        | Self::DECODING_PARAM.bits
        | Self::AUDIO_PARAM.bits
        | Self::VIDEO_PARAM.bits
        | Self::SUBTITLE_PARAM.bits
        | Self::EXPORT.bits
        | Self::READONLY.bits
        | Self::BSF_PARAM.bits
        | Self::RUNTIME_PARAM.bits
        | Self::FILTERING_PARAM.bits
        | Self::DEPRECATED.bits
        | Self::CHILD_CONSTS.bits;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn all() -> Self {
        Self {
            bits: Self::KNOWN_BITS,
        }
    }

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self {
            bits: bits & Self::KNOWN_BITS,
        }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.bits & other.bits) != 0
    }

    pub fn insert(&mut self, other: Self) {
        self.bits |= other.bits;
        self.bits &= Self::KNOWN_BITS;
    }

    pub fn remove(&mut self, other: Self) {
        self.bits &= !other.bits;
    }

    pub fn set(&mut self, other: Self, enabled: bool) {
        if enabled {
            self.insert(other);
        } else {
            self.remove(other);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OptionSearchFlags {
    bits: u32,
}

impl OptionSearchFlags {
    pub const CHILDREN: Self = Self { bits: 1 << 0 };
    pub const FAKE_OBJ: Self = Self { bits: 1 << 1 };
    pub const ARRAY_REPLACE: Self = Self { bits: 1 << 3 };

    const KNOWN_BITS: u32 = Self::CHILDREN.bits | Self::FAKE_OBJ.bits | Self::ARRAY_REPLACE.bits;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self {
            bits: bits & Self::KNOWN_BITS,
        }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.bits & other.bits) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OptionSerializeFlags {
    bits: u32,
}

impl OptionSerializeFlags {
    pub const SKIP_DEFAULTS: Self = Self { bits: 1 << 0 };
    pub const OPT_FLAGS_EXACT: Self = Self { bits: 1 << 1 };
    pub const SEARCH_CHILDREN: Self = Self { bits: 1 << 2 };

    const KNOWN_BITS: u32 =
        Self::SKIP_DEFAULTS.bits | Self::OPT_FLAGS_EXACT.bits | Self::SEARCH_CHILDREN.bits;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn all() -> Self {
        Self {
            bits: Self::KNOWN_BITS,
        }
    }

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self {
            bits: bits & Self::KNOWN_BITS,
        }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.bits & other.bits) != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OptionQuery {
    name: Option<String>,
    unit: Option<String>,
    required_flags: OptionFlags,
    rejected_flags: OptionFlags,
    search_children: bool,
}

impl OptionQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exported() -> Self {
        Self::new().require_flags(OptionFlags::EXPORT)
    }

    pub fn writable() -> Self {
        Self::new().reject_flags(OptionFlags::READONLY)
    }

    pub fn with_name(mut self, name: impl Into<String>) -> AvResult<Self> {
        self.name = Some(validate_name(&name.into())?);
        Ok(self)
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> AvResult<Self> {
        self.unit = Some(validate_unit(&unit.into())?);
        Ok(self)
    }

    pub fn require_flags(mut self, flags: OptionFlags) -> Self {
        self.required_flags = OptionFlags::from_bits_truncate(flags.bits());
        self
    }

    pub fn reject_flags(mut self, flags: OptionFlags) -> Self {
        self.rejected_flags = OptionFlags::from_bits_truncate(flags.bits());
        self
    }

    pub fn include_children(mut self, include: bool) -> Self {
        self.search_children = include;
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    pub fn required_flags(&self) -> OptionFlags {
        self.required_flags
    }

    pub fn rejected_flags(&self) -> OptionFlags {
        self.rejected_flags
    }

    pub fn searches_children(&self) -> bool {
        self.search_children
    }

    fn matches(&self, definition: &OptionDefinition) -> bool {
        if let Some(name) = &self.name {
            if !ascii_eq_ignore_case(definition.name(), name) {
                return false;
            }
        }

        if let Some(unit) = &self.unit {
            match definition.unit() {
                Some(definition_unit) if ascii_eq_ignore_case(definition_unit, unit) => {}
                _ => return false,
            }
        }

        if !definition.flags().contains(self.required_flags) {
            return false;
        }

        if !self.rejected_flags.is_empty() && definition.flags().intersects(self.rejected_flags) {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionDefinition {
    name: String,
    help: String,
    kind: OptionKind,
    default: OptionValue,
    flags: OptionFlags,
    unit: Option<String>,
}

impl OptionDefinition {
    pub fn new(
        name: impl Into<String>,
        kind: OptionKind,
        default: OptionValue,
        help: impl Into<String>,
    ) -> AvResult<Self> {
        Self::new_with_flags(name, kind, default, help, OptionFlags::empty())
    }

    pub fn new_with_flags(
        name: impl Into<String>,
        kind: OptionKind,
        default: OptionValue,
        help: impl Into<String>,
        flags: OptionFlags,
    ) -> AvResult<Self> {
        Self::new_with_flags_and_unit(name, kind, default, help, flags, None::<String>)
    }

    pub fn new_with_unit(
        name: impl Into<String>,
        kind: OptionKind,
        default: OptionValue,
        help: impl Into<String>,
        unit: impl Into<String>,
    ) -> AvResult<Self> {
        Self::new_with_flags_and_unit(name, kind, default, help, OptionFlags::empty(), Some(unit))
    }

    pub fn new_with_flags_and_unit(
        name: impl Into<String>,
        kind: OptionKind,
        default: OptionValue,
        help: impl Into<String>,
        flags: OptionFlags,
        unit: Option<impl Into<String>>,
    ) -> AvResult<Self> {
        let name = validate_name(&name.into())?;
        let help = validate_help(&help.into())?;
        let unit = unit.map(|unit| validate_unit(&unit.into())).transpose()?;
        validate_kind(&kind)?;
        validate_value_for_kind(&kind, &default)?;

        Ok(Self {
            name,
            help,
            kind,
            default,
            flags: OptionFlags::from_bits_truncate(flags.bits()),
            unit,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn kind(&self) -> &OptionKind {
        &self.kind
    }

    pub fn default(&self) -> &OptionValue {
        &self.default
    }

    pub fn flags(&self) -> OptionFlags {
        self.flags
    }

    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    pub fn range(&self) -> Option<OptionRange> {
        range_for_kind(&self.kind)
    }

    pub fn parse_value(&self, raw: &str) -> AvResult<OptionValue> {
        let parsed = match self.kind {
            OptionKind::Bool => OptionValue::Bool(parse_bool(raw)?),
            OptionKind::Int { .. } => OptionValue::Int(parse_int(raw)?),
            OptionKind::Duration { .. } => OptionValue::Duration(parse_duration(raw)?),
            OptionKind::ImageSize => {
                let (width, height) = parse_image_size(raw)?;
                OptionValue::ImageSize { width, height }
            }
            OptionKind::PixelFormat { min, max } => {
                OptionValue::PixelFormat(parse_pixel_format(raw, min, max)?)
            }
            OptionKind::SampleFormat { min, max } => {
                OptionValue::SampleFormat(parse_sample_format(raw, min, max)?)
            }
            OptionKind::ChannelLayout => OptionValue::ChannelLayout(parse_channel_layout(raw)?),
            OptionKind::VideoRate { .. } => OptionValue::VideoRate(parse_video_rate(raw)?),
            OptionKind::Color => OptionValue::Color(parse_color(raw)?),
            OptionKind::Binary => OptionValue::Binary(parse_binary(raw)?),
            OptionKind::Dictionary => OptionValue::Dictionary(parse_dictionary(raw)?),
            OptionKind::Array(ref array) => OptionValue::Array(parse_option_array(raw, array)?),
            OptionKind::Float { .. } => OptionValue::Float(parse_float(raw)?),
            OptionKind::Rational { .. } => OptionValue::Rational(parse_rational(raw)?),
            OptionKind::String { .. } => OptionValue::String(raw.to_owned()),
        };

        validate_value_for_kind(&self.kind, &parsed)?;
        Ok(parsed)
    }

    pub fn validate_value(&self, value: &OptionValue) -> AvResult<()> {
        validate_value_for_kind(&self.kind, value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionMatch<'a> {
    child_name: Option<&'a str>,
    definition: &'a OptionDefinition,
}

impl<'a> OptionMatch<'a> {
    fn root(definition: &'a OptionDefinition) -> Self {
        Self {
            child_name: None,
            definition,
        }
    }

    fn child(child_name: &'a str, definition: &'a OptionDefinition) -> Self {
        Self {
            child_name: Some(child_name),
            definition,
        }
    }

    pub fn child_name(&self) -> Option<&'a str> {
        self.child_name
    }

    pub fn definition(&self) -> &'a OptionDefinition {
        self.definition
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionEntry<'a> {
    Definition(&'a OptionDefinition),
    Constant(&'a OptionConstant),
}

impl<'a> OptionEntry<'a> {
    pub fn name(&self) -> &'a str {
        match self {
            Self::Definition(definition) => definition.name(),
            Self::Constant(constant) => constant.name(),
        }
    }

    pub fn unit(&self) -> Option<&'a str> {
        match self {
            Self::Definition(definition) => definition.unit(),
            Self::Constant(constant) => Some(constant.unit()),
        }
    }

    pub fn flags(&self) -> OptionFlags {
        match self {
            Self::Definition(definition) => definition.flags(),
            Self::Constant(constant) => constant.flags(),
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Constant(_))
    }

    pub fn definition(&self) -> Option<&'a OptionDefinition> {
        match self {
            Self::Definition(definition) => Some(definition),
            Self::Constant(_) => None,
        }
    }

    pub fn constant(&self) -> Option<&'a OptionConstant> {
        match self {
            Self::Definition(_) => None,
            Self::Constant(constant) => Some(constant),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionEntryMatch<'a> {
    child_name: Option<&'a str>,
    entry: OptionEntry<'a>,
}

impl<'a> OptionEntryMatch<'a> {
    fn root(entry: OptionEntry<'a>) -> Self {
        Self {
            child_name: None,
            entry,
        }
    }

    fn child(child_name: &'a str, entry: OptionEntry<'a>) -> Self {
        Self {
            child_name: Some(child_name),
            entry,
        }
    }

    pub fn child_name(&self) -> Option<&'a str> {
        self.child_name
    }

    pub fn entry(&self) -> OptionEntry<'a> {
        self.entry
    }

    pub fn name(&self) -> &'a str {
        self.entry.name()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionChild {
    name: String,
    help: String,
    options: OptionSet,
}

impl OptionChild {
    pub fn new(
        name: impl Into<String>,
        options: OptionSet,
        help: impl Into<String>,
    ) -> AvResult<Self> {
        let name = validate_name(&name.into())?;
        let help = validate_help(&help.into())?;

        Ok(Self {
            name,
            help,
            options,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn options(&self) -> &OptionSet {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut OptionSet {
        &mut self.options
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OptionEntryKey {
    Definition { name: String },
    Constant { unit: String, name: String },
}

impl OptionEntryKey {
    fn definition(name: &str) -> Self {
        Self::Definition {
            name: name.to_owned(),
        }
    }

    fn constant(unit: &str, name: &str) -> Self {
        Self::Constant {
            unit: unit.to_owned(),
            name: name.to_owned(),
        }
    }

    fn matches_definition(&self, name: &str) -> bool {
        matches!(self, Self::Definition { name: key_name } if ascii_eq_ignore_case(key_name, name))
    }

    fn matches_constant(&self, unit: &str, name: &str) -> bool {
        matches!(
            self,
            Self::Constant {
                unit: key_unit,
                name: key_name
            } if ascii_eq_ignore_case(key_unit, unit) && ascii_eq_ignore_case(key_name, name)
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OptionSet {
    definitions: Vec<OptionDefinition>,
    values: Vec<OptionValue>,
    constants: Vec<OptionConstant>,
    entries: Vec<OptionEntryKey>,
    children: Vec<OptionChild>,
}

impl OptionSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn definitions(&self) -> &[OptionDefinition] {
        &self.definitions
    }

    pub fn constants(&self) -> &[OptionConstant] {
        &self.constants
    }

    pub fn children(&self) -> &[OptionChild] {
        &self.children
    }

    pub fn constants_for_unit<'a>(
        &'a self,
        unit: &'a str,
    ) -> impl Iterator<Item = &'a OptionConstant> + 'a {
        self.constants
            .iter()
            .filter(move |constant| ascii_eq_ignore_case(constant.unit(), unit))
    }

    pub fn define(&mut self, definition: OptionDefinition) -> AvResult<()> {
        let current = definition.default().clone();
        self.define_with_current_value(definition, current)
    }

    pub fn define_with_current_value(
        &mut self,
        definition: OptionDefinition,
        current: OptionValue,
    ) -> AvResult<()> {
        if self.find_index(definition.name()).is_some() {
            return Err(AvError::invalid_argument(format!(
                "duplicate option `{}`",
                definition.name()
            )));
        }
        definition.validate_value(&current)?;

        let entry_key = OptionEntryKey::definition(definition.name());
        self.values.push(current);
        self.definitions.push(definition);
        self.entries.push(entry_key);
        Ok(())
    }

    pub fn define_constant(&mut self, constant: OptionConstant) -> AvResult<()> {
        if self
            .find_constant_index(constant.unit(), constant.name())
            .is_some()
        {
            return Err(AvError::invalid_argument(format!(
                "duplicate option constant `{}` for unit `{}`",
                constant.name(),
                constant.unit()
            )));
        }

        let entry_key = OptionEntryKey::constant(constant.unit(), constant.name());
        self.constants.push(constant);
        self.entries.push(entry_key);
        Ok(())
    }

    pub fn define_child(&mut self, child: OptionChild) -> AvResult<()> {
        if self.find_child_index(child.name()).is_some() {
            return Err(AvError::invalid_argument(format!(
                "duplicate option child `{}`",
                child.name()
            )));
        }

        self.children.push(child);
        Ok(())
    }

    pub fn remove_definition(&mut self, name: &str) -> AvResult<(OptionDefinition, OptionValue)> {
        let index = self.option_index(name)?;
        let definition = self.definitions.remove(index);
        let value = self.values.remove(index);
        self.entries
            .retain(|entry| !entry.matches_definition(definition.name()));
        Ok((definition, value))
    }

    pub fn remove_constant(&mut self, unit: &str, name: &str) -> AvResult<OptionConstant> {
        let index = self.find_constant_index(unit, name).ok_or_else(|| {
            AvError::new(
                AvErrorKind::NotFound,
                format!("unknown option constant `{name}` for unit `{unit}`"),
            )
        })?;
        let constant = self.constants.remove(index);
        self.entries
            .retain(|entry| !entry.matches_constant(constant.unit(), constant.name()));
        Ok(constant)
    }

    pub fn remove_child(&mut self, name: &str) -> AvResult<OptionChild> {
        let index = self.find_child_index(name).ok_or_else(|| {
            AvError::new(
                AvErrorKind::NotFound,
                format!("unknown option child `{name}`"),
            )
        })?;
        Ok(self.children.remove(index))
    }

    pub fn definition(&self, name: &str) -> Option<&OptionDefinition> {
        self.find_index(name).map(|index| &self.definitions[index])
    }

    pub fn definitions_matching<'a>(&'a self, query: &OptionQuery) -> Vec<OptionMatch<'a>> {
        let mut matches = Vec::new();

        for definition in &self.definitions {
            if query.matches(definition) {
                matches.push(OptionMatch::root(definition));
            }
        }

        if query.searches_children() {
            for child in &self.children {
                for definition in child.options().definitions() {
                    if query.matches(definition) {
                        matches.push(OptionMatch::child(child.name(), definition));
                    }
                }
            }
        }

        matches
    }

    pub fn first_definition_matching<'a>(&'a self, query: &OptionQuery) -> Option<OptionMatch<'a>> {
        self.definitions_matching(query).into_iter().next()
    }

    pub fn avoption_entries(&self) -> Vec<OptionEntryMatch<'_>> {
        self.entries
            .iter()
            .filter_map(|key| self.entry_for_key(key).map(OptionEntryMatch::root))
            .collect()
    }

    pub fn find_avoption(
        &self,
        name: &str,
        unit: Option<&str>,
        opt_flags: OptionFlags,
        search_flags: OptionSearchFlags,
    ) -> Option<OptionEntryMatch<'_>> {
        let opt_flags = OptionFlags::from_bits_truncate(opt_flags.bits());
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(found) = child.options().find_root_avoption(name, unit, opt_flags) {
                    return Some(OptionEntryMatch::child(child.name(), found));
                }
            }
        }

        self.find_root_avoption(name, unit, opt_flags)
            .map(OptionEntryMatch::root)
    }

    pub fn child(&self, name: &str) -> Option<&OptionChild> {
        self.find_child_index(name)
            .map(|index| &self.children[index])
    }

    pub fn child_mut(&mut self, name: &str) -> Option<&mut OptionChild> {
        self.find_child_index(name)
            .map(|index| &mut self.children[index])
    }

    pub fn get(&self, name: &str) -> Option<&OptionValue> {
        self.find_index(name).map(|index| &self.values[index])
    }

    pub fn get_avoption_string(&self, name: &str) -> AvResult<String> {
        let index = self.avoption_index(name)?;
        Ok(format_avoption_value_for_kind(
            self.definitions[index].kind(),
            &self.values[index],
        ))
    }

    pub fn get_avoption_string_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<String> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return Ok(format_avoption_value_for_kind(
                        child.options.definitions[index].kind(),
                        &child.options.values[index],
                    ));
                }
            }
        }

        self.get_avoption_string(name)
    }

    pub fn get_child_option(&self, child_name: &str, option_name: &str) -> AvResult<&OptionValue> {
        let child = self.child_by_name(child_name)?;
        let index = child.options.option_index(option_name)?;
        Ok(&child.options.values[index])
    }

    pub fn range(&self, name: &str) -> AvResult<Option<OptionRange>> {
        let index = self.option_index(name)?;
        Ok(self.definitions[index].range())
    }

    pub fn query_avoption_ranges(&self, name: &str) -> AvResult<AvOptionRanges> {
        let index = self.avoption_query_ranges_index(name)?;
        if matches!(self.definitions[index].kind(), OptionKind::ChannelLayout) {
            return Err(AvError::with_code(
                AvErrorKind::Unsupported,
                AvErrorCode::ENOSYS,
                format!(
                    "AVOption `{}` does not expose query ranges",
                    self.definitions[index].name()
                ),
            ));
        }
        if matches!(
            self.definitions[index].kind(),
            OptionKind::Binary | OptionKind::Dictionary | OptionKind::Array(_)
        ) {
            return Err(AvError::with_code(
                AvErrorKind::Unsupported,
                AvErrorCode::ENOSYS,
                format!(
                    "AVOption `{}` does not expose query ranges",
                    self.definitions[index].name()
                ),
            ));
        }
        Ok(avoption_ranges_for_kind(self.definitions[index].kind()))
    }

    pub fn child_range(
        &self,
        child_name: &str,
        option_name: &str,
    ) -> AvResult<Option<OptionRange>> {
        let child = self.child_by_name(child_name)?;
        child.options.range(option_name)
    }

    pub fn set(&mut self, name: &str, value: OptionValue) -> AvResult<()> {
        let index = self.option_index(name)?;
        self.ensure_writable(index)?;
        self.definitions[index].validate_value(&value)?;
        self.values[index] = value;
        Ok(())
    }

    pub fn set_from_str(&mut self, name: &str, raw: &str) -> AvResult<()> {
        let index = self.option_index(name)?;
        self.ensure_writable(index)?;
        let value = self.parse_value(index, raw)?;
        self.values[index] = value;
        Ok(())
    }

    pub fn set_avoption_from_str(&mut self, name: &str, raw: &str) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.ensure_writable(index)?;
        if matches!(self.definitions[index].kind(), OptionKind::Color) {
            let current = match self.values[index] {
                OptionValue::Color(color) => color,
                _ => {
                    return Err(AvError::invalid_argument(
                        "color AVOption storage does not match definition",
                    ))
                }
            };
            let (color, result) = parse_avoption_color_value(raw, current);
            self.values[index] = OptionValue::Color(color);
            return result;
        }
        if matches!(self.definitions[index].kind(), OptionKind::ChannelLayout) {
            match self.parse_avoption_value(index, raw) {
                Ok(value) => {
                    self.values[index] = value;
                    return Ok(());
                }
                Err(err) => {
                    self.values[index] = OptionValue::ChannelLayout(ChannelLayoutSpec::empty());
                    return Err(err);
                }
            }
        }
        if matches!(self.definitions[index].kind(), OptionKind::Binary) {
            match self.parse_avoption_value(index, raw) {
                Ok(value) => {
                    self.values[index] = value;
                    return Ok(());
                }
                Err(err) => {
                    self.values[index] = OptionValue::Binary(Vec::new());
                    return Err(err);
                }
            }
        }
        if let OptionKind::Array(array) = self.definitions[index].kind() {
            let values = parse_avoption_array(raw, array, self.definitions[index].name())?;
            if values.len() < array.min_len() {
                self.values[index] = OptionValue::Array(Vec::new());
                return Err(AvError::with_code(
                    AvErrorKind::InvalidArgument,
                    AvErrorCode::EINVAL,
                    format!(
                        "Cannot assign fewer than {} elements to array option {}",
                        array.min_len(),
                        self.definitions[index].name()
                    ),
                ));
            }
            self.values[index] = OptionValue::Array(values);
            return Ok(());
        }
        let value = self.parse_avoption_value(index, raw)?;
        self.values[index] = value;
        Ok(())
    }

    pub fn set_avoption_from_str_with_flags(
        &mut self,
        name: &str,
        raw: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if child.options.find_exact_index(name).is_some() {
                    return child.options.set_avoption_from_str(name, raw);
                }
            }
        }

        self.set_avoption_from_str(name, raw)
    }

    pub fn set_avoption_int(&mut self, name: &str, value: i64) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_number_at_index(index, AvOptionNumericInput::Int(value))
    }

    pub fn set_avoption_int_with_flags(
        &mut self,
        name: &str,
        value: i64,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        self.set_avoption_number_with_flags(name, AvOptionNumericInput::Int(value), search_flags)
    }

    pub fn set_avoption_double(&mut self, name: &str, value: f64) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_number_at_index(index, AvOptionNumericInput::Double(value))
    }

    pub fn set_avoption_double_with_flags(
        &mut self,
        name: &str,
        value: f64,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        self.set_avoption_number_with_flags(name, AvOptionNumericInput::Double(value), search_flags)
    }

    pub fn set_avoption_q(&mut self, name: &str, value: Rational) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_number_at_index(index, AvOptionNumericInput::Rational(value))
    }

    pub fn set_avoption_q_with_flags(
        &mut self,
        name: &str,
        value: Rational,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        self.set_avoption_number_with_flags(
            name,
            AvOptionNumericInput::Rational(value),
            search_flags,
        )
    }

    pub fn set_avoption_image_size(&mut self, name: &str, width: i32, height: i32) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_image_size_at_index(index, width, height)
    }

    pub fn set_avoption_image_size_with_flags(
        &mut self,
        name: &str,
        width: i32,
        height: i32,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .set_avoption_image_size_at_index(index, width, height);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_image_size_at_index(index, width, height)
    }

    pub fn set_avoption_pixel_format(
        &mut self,
        name: &str,
        value: Option<PixelFormat>,
    ) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_pixel_format_at_index(index, value)
    }

    pub fn set_avoption_pixel_format_with_flags(
        &mut self,
        name: &str,
        value: Option<PixelFormat>,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .set_avoption_pixel_format_at_index(index, value);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_pixel_format_at_index(index, value)
    }

    pub fn set_avoption_sample_format(
        &mut self,
        name: &str,
        value: Option<SampleFormat>,
    ) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_sample_format_at_index(index, value)
    }

    pub fn set_avoption_sample_format_with_flags(
        &mut self,
        name: &str,
        value: Option<SampleFormat>,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .set_avoption_sample_format_at_index(index, value);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_sample_format_at_index(index, value)
    }

    pub fn set_avoption_channel_layout(
        &mut self,
        name: &str,
        value: ChannelLayoutSpec,
    ) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_channel_layout_at_index(index, value)
    }

    pub fn set_avoption_channel_layout_with_flags(
        &mut self,
        name: &str,
        value: ChannelLayoutSpec,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .set_avoption_channel_layout_at_index(index, value.clone());
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_channel_layout_at_index(index, value)
    }

    pub fn set_avoption_video_rate(&mut self, name: &str, value: Rational) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_video_rate_at_index(index, value)
    }

    pub fn set_avoption_video_rate_with_flags(
        &mut self,
        name: &str,
        value: Rational,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.set_avoption_video_rate_at_index(index, value);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_video_rate_at_index(index, value)
    }

    pub fn set_avoption_binary(&mut self, name: &str, value: &[u8]) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_binary_at_index(index, value)
    }

    pub fn set_avoption_binary_with_flags(
        &mut self,
        name: &str,
        value: &[u8],
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.set_avoption_binary_at_index(index, value);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_binary_at_index(index, value)
    }

    pub fn set_avoption_dictionary(&mut self, name: &str, value: &Dictionary) -> AvResult<()> {
        let index = self.avoption_index(name)?;
        self.set_avoption_dictionary_at_index(index, value)
    }

    pub fn set_avoption_dictionary_with_flags(
        &mut self,
        name: &str,
        value: &Dictionary,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.set_avoption_dictionary_at_index(index, value);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_dictionary_at_index(index, value)
    }

    pub fn get_avoption_int(&self, name: &str) -> AvResult<i64> {
        let index = self.avoption_index(name)?;
        self.get_avoption_number_at_index(index)?.to_int()
    }

    pub fn get_avoption_int_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<i64> {
        self.get_avoption_number_with_flags(name, search_flags)?
            .to_int()
    }

    pub fn get_avoption_double(&self, name: &str) -> AvResult<f64> {
        let index = self.avoption_index(name)?;
        self.get_avoption_number_at_index(index)?.to_double()
    }

    pub fn get_avoption_double_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<f64> {
        self.get_avoption_number_with_flags(name, search_flags)?
            .to_double()
    }

    pub fn get_avoption_q(&self, name: &str) -> AvResult<Rational> {
        let index = self.avoption_index(name)?;
        self.get_avoption_number_at_index(index)?.to_rational()
    }

    pub fn get_avoption_q_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<Rational> {
        self.get_avoption_number_with_flags(name, search_flags)?
            .to_rational()
    }

    pub fn get_avoption_image_size(&self, name: &str) -> AvResult<(i32, i32)> {
        let index = self.avoption_index(name)?;
        self.get_avoption_image_size_at_index(index)
    }

    pub fn get_avoption_image_size_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<(i32, i32)> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_image_size_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_image_size_at_index(index)
    }

    pub fn get_avoption_pixel_format(&self, name: &str) -> AvResult<Option<PixelFormat>> {
        let index = self.avoption_index(name)?;
        self.get_avoption_pixel_format_at_index(index)
    }

    pub fn get_avoption_pixel_format_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<Option<PixelFormat>> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_pixel_format_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_pixel_format_at_index(index)
    }

    pub fn get_avoption_sample_format(&self, name: &str) -> AvResult<Option<SampleFormat>> {
        let index = self.avoption_index(name)?;
        self.get_avoption_sample_format_at_index(index)
    }

    pub fn get_avoption_sample_format_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<Option<SampleFormat>> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_sample_format_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_sample_format_at_index(index)
    }

    pub fn get_avoption_channel_layout(&self, name: &str) -> AvResult<ChannelLayoutSpec> {
        let index = self.avoption_index(name)?;
        self.get_avoption_channel_layout_at_index(index)
    }

    pub fn get_avoption_channel_layout_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<ChannelLayoutSpec> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_channel_layout_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_channel_layout_at_index(index)
    }

    pub fn get_avoption_video_rate(&self, name: &str) -> AvResult<Rational> {
        self.get_avoption_q(name)
    }

    pub fn get_avoption_video_rate_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<Rational> {
        self.get_avoption_q_with_flags(name, search_flags)
    }

    pub fn get_avoption_binary(&self, name: &str) -> AvResult<Vec<u8>> {
        let index = self.avoption_index(name)?;
        self.get_avoption_binary_at_index(index)
    }

    pub fn get_avoption_binary_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<Vec<u8>> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_binary_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_binary_at_index(index)
    }

    pub fn get_avoption_dictionary(&self, name: &str) -> AvResult<Dictionary> {
        let index = self.avoption_index(name)?;
        self.get_avoption_dictionary_at_index(index)
    }

    pub fn get_avoption_dictionary_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<Dictionary> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_dictionary_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_dictionary_at_index(index)
    }

    pub fn get_avoption_array_size(&self, name: &str) -> AvResult<usize> {
        let index = self.avoption_index(name)?;
        self.get_avoption_array_size_at_index(index)
    }

    pub fn get_avoption_array_size_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<usize> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_array_size_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_array_size_at_index(index)
    }

    pub fn get_avoption_array(
        &self,
        name: &str,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<OptionValue>> {
        let index = self.avoption_index(name)?;
        self.get_avoption_array_at_index(index, start_elem, nb_elems)
    }

    pub fn get_avoption_array_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<OptionValue>> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .get_avoption_array_at_index(index, start_elem, nb_elems);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_array_at_index(index, start_elem, nb_elems)
    }

    pub fn get_avoption_array_strings(
        &self,
        name: &str,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<String>> {
        let index = self.avoption_index(name)?;
        self.get_avoption_array_strings_at_index(index, start_elem, nb_elems)
    }

    pub fn get_avoption_array_strings_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<String>> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .get_avoption_array_strings_at_index(index, start_elem, nb_elems);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_array_strings_at_index(index, start_elem, nb_elems)
    }

    pub fn get_avoption_array_doubles(
        &self,
        name: &str,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<f64>> {
        let index = self.avoption_index(name)?;
        self.get_avoption_array_doubles_at_index(index, start_elem, nb_elems)
    }

    pub fn get_avoption_array_doubles_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<f64>> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .get_avoption_array_doubles_at_index(index, start_elem, nb_elems);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_array_doubles_at_index(index, start_elem, nb_elems)
    }

    pub fn get_avoption_array_rationals(
        &self,
        name: &str,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<Rational>> {
        let index = self.avoption_index(name)?;
        self.get_avoption_array_rationals_at_index(index, start_elem, nb_elems)
    }

    pub fn get_avoption_array_rationals_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<Rational>> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .get_avoption_array_rationals_at_index(index, start_elem, nb_elems);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_array_rationals_at_index(index, start_elem, nb_elems)
    }

    pub fn set_avoption_array(
        &mut self,
        name: &str,
        start_elem: usize,
        values: &[OptionValue],
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.set_avoption_array_at_index(
                        index,
                        start_elem,
                        values,
                        search_flags,
                    );
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_array_at_index(index, start_elem, values, search_flags)
    }

    pub fn remove_avoption_array(
        &mut self,
        name: &str,
        start_elem: usize,
        nb_elems: usize,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child
                        .options
                        .remove_avoption_array_at_index(index, start_elem, nb_elems);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.remove_avoption_array_at_index(index, start_elem, nb_elems)
    }

    pub fn set_avoptions_from_dict(
        &mut self,
        options: &mut Dictionary,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let original_entries: Vec<_> = options
            .entries()
            .iter()
            .map(|entry| (entry.key().to_owned(), entry.value().to_owned()))
            .collect();
        let mut remaining = Dictionary::new();

        for (key, value) in &original_entries {
            match self.set_avoption_from_str_with_flags(key, value, search_flags) {
                Ok(()) => {}
                Err(err) if err.code() == Some(AvErrorCode::OPTION_NOT_FOUND) => {
                    remaining.set_with_mode(
                        key.clone(),
                        value.clone(),
                        MatchMode::CaseSensitive,
                        SetMode::AllowMultiple,
                    )?;
                }
                Err(err) => return Err(err),
            }
        }

        *options = remaining;
        Ok(())
    }

    pub fn set_avoptions_from_string(
        &mut self,
        opts: &str,
        shorthand: &[&str],
        key_val_sep: &str,
        pairs_sep: &str,
    ) -> AvResult<usize> {
        validate_avoption_string_separators(key_val_sep, pairs_sep)?;

        let mut rest = opts;
        let mut shorthand_index = 0usize;
        let mut shorthand_available = !shorthand.is_empty();
        let mut count = 0usize;

        while !rest.is_empty() {
            let parsed = parse_avoption_string_pair(
                rest,
                key_val_sep,
                pairs_sep,
                shorthand_available && shorthand_index < shorthand.len(),
            )?;
            rest = parsed.rest;
            if !rest.is_empty() {
                let separator_len = rest
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .expect("non-empty rest has a separator");
                rest = &rest[separator_len..];
            }

            let (key, value) = match parsed.key {
                Some(key) => {
                    shorthand_available = false;
                    (key, parsed.value)
                }
                None => {
                    let key = shorthand
                        .get(shorthand_index)
                        .copied()
                        .ok_or_else(|| invalid_avoption_string(opts))?;
                    if !is_valid_avoption_string_key(key) {
                        return Err(invalid_avoption_string(opts));
                    }
                    shorthand_index += 1;
                    (key.to_owned(), parsed.value)
                }
            };

            self.set_avoption_from_str(&key, &value)?;
            count += 1;
        }

        Ok(count)
    }

    pub fn serialize_avoptions(
        &self,
        opt_flags: OptionFlags,
        serialize_flags: OptionSerializeFlags,
        key_val_sep: char,
        pairs_sep: char,
    ) -> AvResult<String> {
        validate_avoption_serialize_separators(key_val_sep, pairs_sep)?;

        let opt_flags = OptionFlags::from_bits_truncate(opt_flags.bits());
        let serialize_flags = OptionSerializeFlags::from_bits_truncate(serialize_flags.bits());
        let mut fields = Vec::new();

        self.collect_serialized_avoptions(
            opt_flags,
            serialize_flags,
            key_val_sep,
            pairs_sep,
            &mut fields,
        );

        let mut output = String::new();
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                output.push(pairs_sep);
            }
            output.push_str(field);
        }

        Ok(output)
    }

    pub fn copy_avoptions_from(&mut self, source: &OptionSet) -> AvResult<()> {
        if !self.has_matching_avoption_schema(source) {
            return Err(AvError::invalid_argument(
                "source and destination AVOption classes differ",
            ));
        }

        for (definition, value) in source.definitions.iter().zip(&source.values) {
            definition.validate_value(value)?;
        }

        self.values = source.values.clone();
        Ok(())
    }

    fn collect_serialized_avoptions(
        &self,
        opt_flags: OptionFlags,
        serialize_flags: OptionSerializeFlags,
        key_val_sep: char,
        pairs_sep: char,
        fields: &mut Vec<String>,
    ) {
        if serialize_flags.contains(OptionSerializeFlags::SEARCH_CHILDREN) {
            for child in &self.children {
                child.options().collect_serialized_avoptions(
                    opt_flags,
                    serialize_flags,
                    key_val_sep,
                    pairs_sep,
                    fields,
                );
            }
        }

        for (definition, value) in self.definitions.iter().zip(&self.values) {
            if !definition_matches_serialize_flags(definition, opt_flags, serialize_flags) {
                continue;
            }

            if serialize_flags.contains(OptionSerializeFlags::SKIP_DEFAULTS)
                && value == definition.default()
            {
                continue;
            }

            let mut field = String::new();
            push_avoption_serialize_escaped(&mut field, definition.name(), key_val_sep, pairs_sep);
            field.push(key_val_sep);
            push_avoption_serialize_escaped(
                &mut field,
                &format_avoption_value_for_kind(definition.kind(), value),
                key_val_sep,
                pairs_sep,
            );
            fields.push(field);
        }
    }

    fn set_avoption_number_with_flags(
        &mut self,
        name: &str,
        value: AvOptionNumericInput,
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &mut self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.set_avoption_number_at_index(index, value);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.set_avoption_number_at_index(index, value)
    }

    fn set_avoption_number_at_index(
        &mut self,
        index: usize,
        input: AvOptionNumericInput,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        let value = avoption_value_from_numeric(
            self.definitions[index].kind(),
            self.definitions[index].name(),
            input,
        )?;
        self.values[index] = value;
        Ok(())
    }

    fn set_avoption_image_size_at_index(
        &mut self,
        index: usize,
        width: i32,
        height: i32,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        if !matches!(self.definitions[index].kind(), OptionKind::ImageSize) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not an image size",
                self.definitions[index].name()
            )));
        }
        if width < 0 || height < 0 {
            return Err(AvError::invalid_argument(format!(
                "invalid negative image size {width}x{height}"
            )));
        }

        self.values[index] = OptionValue::ImageSize { width, height };
        Ok(())
    }

    fn set_avoption_pixel_format_at_index(
        &mut self,
        index: usize,
        value: Option<PixelFormat>,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        if !matches!(
            self.definitions[index].kind(),
            OptionKind::PixelFormat { .. }
        ) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a pixel format",
                self.definitions[index].name()
            )));
        }
        let value = avoption_value_from_numeric(
            self.definitions[index].kind(),
            self.definitions[index].name(),
            AvOptionNumericInput::Int(i64::from(pixel_format_avoption_index(value)?)),
        )?;
        self.values[index] = value;
        Ok(())
    }

    fn set_avoption_sample_format_at_index(
        &mut self,
        index: usize,
        value: Option<SampleFormat>,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        if !matches!(
            self.definitions[index].kind(),
            OptionKind::SampleFormat { .. }
        ) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a sample format",
                self.definitions[index].name()
            )));
        }
        let value = avoption_value_from_numeric(
            self.definitions[index].kind(),
            self.definitions[index].name(),
            AvOptionNumericInput::Int(i64::from(sample_format_avoption_index(value))),
        )?;
        self.values[index] = value;
        Ok(())
    }

    fn set_avoption_channel_layout_at_index(
        &mut self,
        index: usize,
        value: ChannelLayoutSpec,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        if !matches!(self.definitions[index].kind(), OptionKind::ChannelLayout) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a channel layout",
                self.definitions[index].name()
            )));
        }
        self.values[index] = OptionValue::ChannelLayout(value);
        Ok(())
    }

    fn set_avoption_video_rate_at_index(&mut self, index: usize, value: Rational) -> AvResult<()> {
        self.ensure_writable(index)?;
        if !matches!(self.definitions[index].kind(), OptionKind::VideoRate { .. }) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a video rate",
                self.definitions[index].name()
            )));
        }
        let value = avoption_value_from_numeric(
            self.definitions[index].kind(),
            self.definitions[index].name(),
            AvOptionNumericInput::Rational(value),
        )?;
        self.values[index] = value;
        Ok(())
    }

    fn set_avoption_binary_at_index(&mut self, index: usize, value: &[u8]) -> AvResult<()> {
        self.ensure_writable(index)?;
        if !matches!(self.definitions[index].kind(), OptionKind::Binary) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not binary",
                self.definitions[index].name()
            )));
        }
        self.values[index] = OptionValue::Binary(value.to_vec());
        Ok(())
    }

    fn set_avoption_dictionary_at_index(
        &mut self,
        index: usize,
        value: &Dictionary,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        if !matches!(self.definitions[index].kind(), OptionKind::Dictionary) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a dictionary",
                self.definitions[index].name()
            )));
        }
        self.values[index] = OptionValue::Dictionary(value.clone());
        Ok(())
    }

    fn get_avoption_number_with_flags(
        &self,
        name: &str,
        search_flags: OptionSearchFlags,
    ) -> AvResult<AvOptionNumberParts> {
        let search_flags = OptionSearchFlags::from_bits_truncate(search_flags.bits());
        if search_flags.contains(OptionSearchFlags::FAKE_OBJ) {
            return Err(avoption_not_found_error(name));
        }

        if search_flags.contains(OptionSearchFlags::CHILDREN) {
            for child in &self.children {
                if let Some(index) = child.options.find_exact_index(name) {
                    return child.options.get_avoption_number_at_index(index);
                }
            }
        }

        let index = self.avoption_index(name)?;
        self.get_avoption_number_at_index(index)
    }

    fn get_avoption_number_at_index(&self, index: usize) -> AvResult<AvOptionNumberParts> {
        avoption_number_parts(self.definitions[index].name(), &self.values[index])
    }

    fn get_avoption_image_size_at_index(&self, index: usize) -> AvResult<(i32, i32)> {
        if !matches!(self.definitions[index].kind(), OptionKind::ImageSize) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not an image size",
                self.definitions[index].name()
            )));
        }
        match self.values[index] {
            OptionValue::ImageSize { width, height } => Ok((width, height)),
            _ => Err(AvError::invalid_argument(format!(
                "AVOption `{}` storage is not an image size",
                self.definitions[index].name()
            ))),
        }
    }

    fn get_avoption_pixel_format_at_index(&self, index: usize) -> AvResult<Option<PixelFormat>> {
        if !matches!(
            self.definitions[index].kind(),
            OptionKind::PixelFormat { .. }
        ) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a pixel format",
                self.definitions[index].name()
            )));
        }
        match self.values[index] {
            OptionValue::PixelFormat(value) => Ok(value),
            _ => Err(AvError::invalid_argument(format!(
                "AVOption `{}` storage is not a pixel format",
                self.definitions[index].name()
            ))),
        }
    }

    fn get_avoption_sample_format_at_index(&self, index: usize) -> AvResult<Option<SampleFormat>> {
        if !matches!(
            self.definitions[index].kind(),
            OptionKind::SampleFormat { .. }
        ) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a sample format",
                self.definitions[index].name()
            )));
        }
        match self.values[index] {
            OptionValue::SampleFormat(value) => Ok(value),
            _ => Err(AvError::invalid_argument(format!(
                "AVOption `{}` storage is not a sample format",
                self.definitions[index].name()
            ))),
        }
    }

    fn get_avoption_channel_layout_at_index(&self, index: usize) -> AvResult<ChannelLayoutSpec> {
        if !matches!(self.definitions[index].kind(), OptionKind::ChannelLayout) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a channel layout",
                self.definitions[index].name()
            )));
        }
        match &self.values[index] {
            OptionValue::ChannelLayout(value) => Ok(value.clone()),
            _ => Err(AvError::invalid_argument(format!(
                "AVOption `{}` storage is not a channel layout",
                self.definitions[index].name()
            ))),
        }
    }

    fn get_avoption_binary_at_index(&self, index: usize) -> AvResult<Vec<u8>> {
        if !matches!(self.definitions[index].kind(), OptionKind::Binary) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not binary",
                self.definitions[index].name()
            )));
        }
        match &self.values[index] {
            OptionValue::Binary(value) => Ok(value.clone()),
            _ => Err(AvError::invalid_argument(format!(
                "AVOption `{}` storage is not binary",
                self.definitions[index].name()
            ))),
        }
    }

    fn get_avoption_dictionary_at_index(&self, index: usize) -> AvResult<Dictionary> {
        if !matches!(self.definitions[index].kind(), OptionKind::Dictionary) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not a dictionary",
                self.definitions[index].name()
            )));
        }
        match &self.values[index] {
            OptionValue::Dictionary(value) => Ok(value.clone()),
            _ => Err(AvError::invalid_argument(format!(
                "AVOption `{}` storage is not a dictionary",
                self.definitions[index].name()
            ))),
        }
    }

    fn get_avoption_array_size_at_index(&self, index: usize) -> AvResult<usize> {
        if !matches!(self.definitions[index].kind(), OptionKind::Array(_)) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not an array",
                self.definitions[index].name()
            )));
        }
        match &self.values[index] {
            OptionValue::Array(values) => Ok(values.len()),
            _ => Err(AvError::invalid_argument(format!(
                "AVOption `{}` storage is not an array",
                self.definitions[index].name()
            ))),
        }
    }

    fn get_avoption_array_at_index(
        &self,
        index: usize,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<OptionValue>> {
        if !matches!(self.definitions[index].kind(), OptionKind::Array(_)) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not an array",
                self.definitions[index].name()
            )));
        }
        let values = match &self.values[index] {
            OptionValue::Array(values) => values,
            _ => {
                return Err(AvError::invalid_argument(format!(
                    "AVOption `{}` storage is not an array",
                    self.definitions[index].name()
                )))
            }
        };
        if start_elem >= values.len() || values.len().saturating_sub(start_elem) < nb_elems {
            return Err(AvError::invalid_argument(format!(
                "array AVOption `{}` range is outside the current array",
                self.definitions[index].name()
            )));
        }

        Ok(values[start_elem..start_elem + nb_elems].to_vec())
    }

    fn get_avoption_array_strings_at_index(
        &self,
        index: usize,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<String>> {
        if !matches!(self.definitions[index].kind(), OptionKind::Array(_)) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not an array",
                self.definitions[index].name()
            )));
        }
        let values = match &self.values[index] {
            OptionValue::Array(values) => values,
            _ => {
                return Err(AvError::invalid_argument(format!(
                    "AVOption `{}` storage is not an array",
                    self.definitions[index].name()
                )))
            }
        };
        if start_elem >= values.len() || values.len().saturating_sub(start_elem) < nb_elems {
            return Err(AvError::invalid_argument(format!(
                "array AVOption `{}` range is outside the current array",
                self.definitions[index].name()
            )));
        }

        Ok(values[start_elem..start_elem + nb_elems]
            .iter()
            .map(format_avoption_value)
            .collect())
    }

    fn get_avoption_array_numbers_at_index(
        &self,
        index: usize,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<AvOptionNumberParts>> {
        if !matches!(self.definitions[index].kind(), OptionKind::Array(_)) {
            return Err(AvError::invalid_argument(format!(
                "AVOption `{}` is not an array",
                self.definitions[index].name()
            )));
        }
        let values = match &self.values[index] {
            OptionValue::Array(values) => values,
            _ => {
                return Err(AvError::invalid_argument(format!(
                    "AVOption `{}` storage is not an array",
                    self.definitions[index].name()
                )))
            }
        };
        if start_elem >= values.len() || values.len().saturating_sub(start_elem) < nb_elems {
            return Err(AvError::invalid_argument(format!(
                "array AVOption `{}` range is outside the current array",
                self.definitions[index].name()
            )));
        }

        values[start_elem..start_elem + nb_elems]
            .iter()
            .map(|value| avoption_number_parts(self.definitions[index].name(), value))
            .collect()
    }

    fn get_avoption_array_doubles_at_index(
        &self,
        index: usize,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<f64>> {
        self.get_avoption_array_numbers_at_index(index, start_elem, nb_elems)?
            .into_iter()
            .map(AvOptionNumberParts::to_double)
            .collect()
    }

    fn get_avoption_array_rationals_at_index(
        &self,
        index: usize,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<Vec<Rational>> {
        self.get_avoption_array_numbers_at_index(index, start_elem, nb_elems)?
            .into_iter()
            .map(AvOptionNumberParts::to_rational)
            .collect()
    }

    fn set_avoption_array_at_index(
        &mut self,
        index: usize,
        start_elem: usize,
        values: &[OptionValue],
        search_flags: OptionSearchFlags,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        let array = match self.definitions[index].kind() {
            OptionKind::Array(array) => array,
            _ => {
                return Err(AvError::invalid_argument(format!(
                    "AVOption `{}` is not an array",
                    self.definitions[index].name()
                )))
            }
        };
        let current = match &self.values[index] {
            OptionValue::Array(values) => values,
            _ => {
                return Err(AvError::invalid_argument(format!(
                    "AVOption `{}` storage is not an array",
                    self.definitions[index].name()
                )))
            }
        };
        if start_elem > current.len() {
            return Err(AvError::invalid_argument(format!(
                "array AVOption `{}` insertion point is outside the current array",
                self.definitions[index].name()
            )));
        }
        let coerced_values: Vec<_> = values
            .iter()
            .map(|value| {
                coerce_avoption_array_element(
                    array.element(),
                    self.definitions[index].name(),
                    value,
                )
            })
            .collect::<AvResult<_>>()?;

        let replacing = search_flags.contains(OptionSearchFlags::ARRAY_REPLACE);
        let new_len = if replacing {
            start_elem
                .checked_add(coerced_values.len())
                .map(|end| end.max(current.len()))
        } else {
            current.len().checked_add(coerced_values.len())
        }
        .ok_or_else(|| {
            AvError::invalid_argument(format!(
                "array AVOption `{}` size overflow",
                self.definitions[index].name()
            ))
        })?;
        validate_array_len(array, new_len, self.definitions[index].name())?;

        let mut updated = current.clone();
        if replacing {
            if start_elem + coerced_values.len() > updated.len() {
                updated.resize(
                    start_elem + coerced_values.len(),
                    OptionValue::String(String::new()),
                );
            }
            for (offset, value) in coerced_values.into_iter().enumerate() {
                updated[start_elem + offset] = value;
            }
        } else {
            updated.splice(start_elem..start_elem, coerced_values);
        }
        self.values[index] = OptionValue::Array(updated);
        Ok(())
    }

    fn remove_avoption_array_at_index(
        &mut self,
        index: usize,
        start_elem: usize,
        nb_elems: usize,
    ) -> AvResult<()> {
        self.ensure_writable(index)?;
        let array = match self.definitions[index].kind() {
            OptionKind::Array(array) => array,
            _ => {
                return Err(AvError::invalid_argument(format!(
                    "AVOption `{}` is not an array",
                    self.definitions[index].name()
                )))
            }
        };
        let current = match &self.values[index] {
            OptionValue::Array(values) => values,
            _ => {
                return Err(AvError::invalid_argument(format!(
                    "AVOption `{}` storage is not an array",
                    self.definitions[index].name()
                )))
            }
        };
        if start_elem > current.len() || current.len().saturating_sub(start_elem) < nb_elems {
            return Err(AvError::invalid_argument(format!(
                "array AVOption `{}` removal range is outside the current array",
                self.definitions[index].name()
            )));
        }
        let new_len = current.len() - nb_elems;
        validate_array_len(array, new_len, self.definitions[index].name())?;

        let mut updated = current.clone();
        updated.drain(start_elem..start_elem + nb_elems);
        self.values[index] = OptionValue::Array(updated);
        Ok(())
    }

    pub fn set_child(
        &mut self,
        child_name: &str,
        option_name: &str,
        value: OptionValue,
    ) -> AvResult<()> {
        let child = self.child_by_name_mut(child_name)?;
        child.options.set(option_name, value)
    }

    pub fn set_child_from_str(
        &mut self,
        child_name: &str,
        option_name: &str,
        raw: &str,
    ) -> AvResult<()> {
        let child = self.child_by_name_mut(child_name)?;
        child.options.set_from_str(option_name, raw)
    }

    fn parse_value(&self, index: usize, raw: &str) -> AvResult<OptionValue> {
        if let Some(unit) = self.definitions[index].unit() {
            if let Some(constant) = self.find_constant(unit, raw) {
                self.definitions[index].validate_value(constant.value())?;
                return Ok(constant.value().clone());
            }
        }

        self.definitions[index].parse_value(raw)
    }

    fn parse_avoption_value(&self, index: usize, raw: &str) -> AvResult<OptionValue> {
        if !matches!(
            self.definitions[index].kind(),
            OptionKind::Duration { .. }
                | OptionKind::VideoRate { .. }
                | OptionKind::Color
                | OptionKind::ChannelLayout
                | OptionKind::Binary
                | OptionKind::Dictionary
                | OptionKind::Array(_)
        ) {
            if let Some(unit) = self.definitions[index].unit() {
                if let Some(constant) = self.find_exact_constant(unit, raw) {
                    self.definitions[index].validate_value(constant.value())?;
                    return Ok(constant.value().clone());
                }
            }
        }

        match self.definitions[index].kind() {
            OptionKind::Bool
            | OptionKind::String { .. }
            | OptionKind::Duration { .. }
            | OptionKind::ImageSize
            | OptionKind::PixelFormat { .. }
            | OptionKind::SampleFormat { .. }
            | OptionKind::ChannelLayout
            | OptionKind::VideoRate { .. }
            | OptionKind::Color
            | OptionKind::Dictionary
            | OptionKind::Array(_) => {
                if matches!(self.definitions[index].kind(), OptionKind::Duration { .. }) {
                    let duration = parse_duration(raw)?;
                    return avoption_value_from_numeric(
                        self.definitions[index].kind(),
                        self.definitions[index].name(),
                        AvOptionNumericInput::Int(duration),
                    );
                }
                if matches!(self.definitions[index].kind(), OptionKind::VideoRate { .. }) {
                    let rate = parse_video_rate(raw)?;
                    return avoption_value_from_numeric(
                        self.definitions[index].kind(),
                        self.definitions[index].name(),
                        AvOptionNumericInput::Rational(rate),
                    );
                }
                if let OptionKind::Array(array) = self.definitions[index].kind() {
                    return Ok(OptionValue::Array(parse_avoption_array(
                        raw,
                        array,
                        self.definitions[index].name(),
                    )?));
                }
                self.definitions[index].parse_value(raw)
            }
            OptionKind::Binary => self.definitions[index].parse_value(raw),
            OptionKind::Int { .. } | OptionKind::Float { .. } | OptionKind::Rational { .. } => {
                if matches!(self.definitions[index].kind(), OptionKind::Rational { .. }) {
                    if let Some(rational) = parse_avoption_exact_rational_literal(raw)? {
                        return avoption_value_from_numeric(
                            self.definitions[index].kind(),
                            self.definitions[index].name(),
                            AvOptionNumericInput::Rational(rational),
                        );
                    }
                }

                let constants = self.avoption_expression_constants(index)?;
                let value = parse_avoption_numeric_expression(raw, &constants)?;
                avoption_value_from_numeric(
                    self.definitions[index].kind(),
                    self.definitions[index].name(),
                    AvOptionNumericInput::Double(value),
                )
            }
        }
    }

    fn avoption_expression_constants(
        &self,
        index: usize,
    ) -> AvResult<Vec<AvOptionExpressionConstant>> {
        let definition = &self.definitions[index];
        let mut constants = Vec::new();

        if let Some(unit) = definition.unit() {
            for constant in &self.constants {
                if constant.unit() == unit {
                    constants.push(AvOptionExpressionConstant {
                        name: constant.name().to_owned(),
                        value: avoption_number_parts(constant.name(), constant.value())?
                            .to_double()?,
                    });
                }
            }
        }

        constants.push(AvOptionExpressionConstant {
            name: "default".to_owned(),
            value: avoption_number_parts(definition.name(), definition.default())?.to_double()?,
        });
        constants.push(AvOptionExpressionConstant {
            name: "max".to_owned(),
            value: avoption_kind_max(definition.kind())?,
        });
        constants.push(AvOptionExpressionConstant {
            name: "min".to_owned(),
            value: avoption_kind_min(definition.kind())?,
        });
        constants.push(AvOptionExpressionConstant {
            name: "none".to_owned(),
            value: 0.0,
        });
        constants.push(AvOptionExpressionConstant {
            name: "all".to_owned(),
            value: -1.0,
        });

        Ok(constants)
    }

    fn option_index(&self, name: &str) -> AvResult<usize> {
        self.find_index(name)
            .ok_or_else(|| AvError::new(AvErrorKind::NotFound, format!("unknown option `{name}`")))
    }

    fn avoption_index(&self, name: &str) -> AvResult<usize> {
        self.find_exact_index(name)
            .ok_or_else(|| avoption_not_found_error(name))
    }

    fn avoption_query_ranges_index(&self, name: &str) -> AvResult<usize> {
        self.find_exact_index(name).ok_or_else(|| {
            AvError::with_code(
                AvErrorKind::NotFound,
                AvErrorCode::ENOMEM,
                format!("unknown AVOption range `{name}`"),
            )
        })
    }

    fn child_by_name(&self, name: &str) -> AvResult<&OptionChild> {
        self.child(name).ok_or_else(|| {
            AvError::new(
                AvErrorKind::NotFound,
                format!("unknown option child `{name}`"),
            )
        })
    }

    fn child_by_name_mut(&mut self, name: &str) -> AvResult<&mut OptionChild> {
        self.child_mut(name).ok_or_else(|| {
            AvError::new(
                AvErrorKind::NotFound,
                format!("unknown option child `{name}`"),
            )
        })
    }

    fn find_index(&self, name: &str) -> Option<usize> {
        self.definitions
            .iter()
            .position(|definition| ascii_eq_ignore_case(definition.name(), name))
    }

    fn find_exact_index(&self, name: &str) -> Option<usize> {
        self.definitions
            .iter()
            .position(|definition| definition.name() == name)
    }

    fn find_constant(&self, unit: &str, name: &str) -> Option<&OptionConstant> {
        self.find_constant_index(unit, name)
            .map(|index| &self.constants[index])
    }

    fn find_exact_constant(&self, unit: &str, name: &str) -> Option<&OptionConstant> {
        self.find_exact_constant_index(unit, name)
            .map(|index| &self.constants[index])
    }

    fn find_constant_index(&self, unit: &str, name: &str) -> Option<usize> {
        self.constants.iter().position(|constant| {
            ascii_eq_ignore_case(constant.unit(), unit)
                && ascii_eq_ignore_case(constant.name(), name)
        })
    }

    fn find_exact_constant_index(&self, unit: &str, name: &str) -> Option<usize> {
        self.constants
            .iter()
            .position(|constant| constant.unit() == unit && constant.name() == name)
    }

    fn entry_for_key(&self, key: &OptionEntryKey) -> Option<OptionEntry<'_>> {
        match key {
            OptionEntryKey::Definition { name } => self
                .definitions
                .iter()
                .find(|definition| ascii_eq_ignore_case(definition.name(), name))
                .map(OptionEntry::Definition),
            OptionEntryKey::Constant { unit, name } => self
                .constants
                .iter()
                .find(|constant| {
                    ascii_eq_ignore_case(constant.unit(), unit)
                        && ascii_eq_ignore_case(constant.name(), name)
                })
                .map(OptionEntry::Constant),
        }
    }

    fn find_root_avoption(
        &self,
        name: &str,
        unit: Option<&str>,
        opt_flags: OptionFlags,
    ) -> Option<OptionEntry<'_>> {
        for key in &self.entries {
            let entry = self.entry_for_key(key)?;
            if !entry.flags().contains(opt_flags) {
                continue;
            }

            match (unit, entry) {
                (None, OptionEntry::Definition(definition)) if definition.name() == name => {
                    return Some(entry);
                }
                (Some(unit), OptionEntry::Constant(constant))
                    if constant.name() == name && constant.unit() == unit =>
                {
                    return Some(entry);
                }
                _ => {}
            }
        }

        None
    }

    fn find_child_index(&self, name: &str) -> Option<usize> {
        self.children
            .iter()
            .position(|child| ascii_eq_ignore_case(child.name(), name))
    }

    fn has_matching_avoption_schema(&self, source: &OptionSet) -> bool {
        self.definitions == source.definitions
            && self.constants == source.constants
            && self.entries == source.entries
    }

    fn ensure_writable(&self, index: usize) -> AvResult<()> {
        if self.definitions[index]
            .flags()
            .contains(OptionFlags::READONLY)
        {
            return Err(AvError::invalid_argument(format!(
                "option `{}` is read-only",
                self.definitions[index].name()
            )));
        }

        Ok(())
    }
}

fn avoption_not_found_error(name: &str) -> AvError {
    AvError::with_code(
        AvErrorKind::NotFound,
        AvErrorCode::OPTION_NOT_FOUND,
        format!("unknown AVOption `{name}`"),
    )
}

fn validate_name(name: &str) -> AvResult<String> {
    if name.is_empty() {
        return Err(AvError::invalid_argument("option name must not be empty"));
    }

    if name.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "option name must not contain NUL",
        ));
    }

    Ok(name.to_owned())
}

fn validate_unit(unit: &str) -> AvResult<String> {
    if unit.is_empty() {
        return Err(AvError::invalid_argument("option unit must not be empty"));
    }

    if unit.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "option unit must not contain NUL",
        ));
    }

    Ok(unit.to_owned())
}

fn validate_help(help: &str) -> AvResult<String> {
    if help.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "option help must not contain NUL",
        ));
    }

    Ok(help.to_owned())
}

fn parse_avoption_color_value(raw: &str, current: RgbaColor) -> (RgbaColor, AvResult<()>) {
    let mut rgba = current.rgba();
    let (color_text, alpha_text) = raw.split_once('@').unwrap_or((raw, ""));
    let has_alpha = raw.contains('@');
    let (color_text, forced_hex) = if let Some(hex) = color_text.strip_prefix('#') {
        (hex, true)
    } else if let Some(hex) = color_text.strip_prefix("0x") {
        (hex, true)
    } else {
        (color_text, false)
    };

    rgba[3] = 0xFF;

    if color_text.eq_ignore_ascii_case("random") || color_text.eq_ignore_ascii_case("bikeshed") {
        return (
            RgbaColor::from_rgba(rgba),
            Err(AvError::unsupported(
                "random av_parse_color colors require a nondeterministic seed",
            )),
        );
    }

    if forced_hex || color_text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        match parse_avoption_hex_color(color_text) {
            Ok(parsed) => rgba = parsed,
            Err(err) => return (RgbaColor::from_rgba(rgba), Err(err)),
        }
    } else if let Some(named) = find_named_color(color_text) {
        let rgb = named.rgb();
        rgba[0] = rgb[0];
        rgba[1] = rgb[1];
        rgba[2] = rgb[2];
    } else {
        return (
            RgbaColor::from_rgba(rgba),
            Err(AvError::invalid_argument(format!(
                "unknown color name `{color_text}`"
            ))),
        );
    }

    if has_alpha {
        match parse_avoption_color_alpha(alpha_text) {
            Ok(alpha) => rgba[3] = alpha,
            Err(err) => return (RgbaColor::from_rgba(rgba), Err(err)),
        }
    }

    (RgbaColor::from_rgba(rgba), Ok(()))
}

fn parse_avoption_hex_color(hex: &str) -> AvResult<[u8; 4]> {
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AvError::invalid_argument(
            "expected color hex value in RRGGBB or RRGGBBAA form",
        ));
    }

    let mut value = u32::from_str_radix(hex, 16)
        .map_err(|_| AvError::invalid_argument("invalid color hex value"))?;
    let alpha = if hex.len() == 8 {
        let alpha = value as u8;
        value >>= 8;
        alpha
    } else {
        0xFF
    };

    Ok([(value >> 16) as u8, (value >> 8) as u8, value as u8, alpha])
}

fn parse_avoption_color_alpha(alpha: &str) -> AvResult<u8> {
    if let Some(hex) = alpha.strip_prefix("0x") {
        if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AvError::invalid_argument("invalid hexadecimal alpha value"));
        }
        let value = u32::from_str_radix(hex, 16)
            .map_err(|_| AvError::invalid_argument("invalid hexadecimal alpha value"))?;
        return u8::try_from(value)
            .map_err(|_| AvError::invalid_argument("hexadecimal alpha value out of range"));
    }

    let normalized = alpha
        .parse::<f64>()
        .map_err(|_| AvError::invalid_argument("invalid alpha value"))?;
    if !normalized.is_finite() || !(0.0..=1.0).contains(&normalized) {
        return Err(AvError::invalid_argument(
            "alpha value must be a finite value between 0 and 1",
        ));
    }

    Ok((255.0 * normalized).trunc() as u8)
}

fn parse_pixel_format(raw: &str, min: i32, max: i32) -> AvResult<Option<PixelFormat>> {
    if raw == "none" {
        return validate_pixel_format_index(None, min, max).map(|_| None);
    }

    if let Some(format) = PixelFormat::from_name(raw) {
        validate_pixel_format_index(Some(format), min, max)?;
        return Ok(Some(format));
    }

    let Some(index) = parse_c_auto_i32(raw) else {
        return Err(AvError::invalid_argument(format!(
            "unable to parse pixel format option value `{raw}`"
        )));
    };
    if index < 0 {
        return Err(AvError::invalid_argument(format!(
            "unable to parse pixel format option value `{raw}`"
        )));
    }
    if index < min || index > max {
        return Err(avoption_range_error(
            "pixel format",
            f64::from(index),
            f64::from(min),
            f64::from(max),
        ));
    }

    pixel_format_from_avoption_index(index)
}

const AV_SAMPLE_FMT_NB: i32 = 12;

fn parse_sample_format(raw: &str, min: i32, max: i32) -> AvResult<Option<SampleFormat>> {
    if raw == "none" {
        return validate_sample_format_index(None, min, max).map(|_| None);
    }

    if let Some(format) = SampleFormat::from_name(raw) {
        validate_sample_format_index(Some(format), min, max)?;
        return Ok(Some(format));
    }

    let Some(index) = parse_c_auto_i32(raw) else {
        return Err(AvError::invalid_argument(format!(
            "unable to parse sample format option value `{raw}`"
        )));
    };
    if !(0..AV_SAMPLE_FMT_NB).contains(&index) {
        return Err(AvError::invalid_argument(format!(
            "unable to parse sample format option value `{raw}`"
        )));
    }
    if index < min || index > max {
        return Err(avoption_range_error(
            "sample format",
            f64::from(index),
            f64::from(min),
            f64::from(max),
        ));
    }

    sample_format_from_avoption_index(index)
}

fn parse_channel_layout(raw: &str) -> AvResult<ChannelLayoutSpec> {
    ChannelLayoutSpec::parse(raw).map_err(|err| {
        AvError::with_code(
            AvErrorKind::InvalidArgument,
            AvErrorCode::EINVAL,
            err.to_string(),
        )
    })
}

fn parse_binary(raw: &str) -> AvResult<Vec<u8>> {
    let bytes = raw.as_bytes();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() % 2 != 0 {
        return Err(AvError::invalid_argument(
            "binary AVOption hex string must have an even length",
        ));
    }

    let mut parsed = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0]).ok_or_else(|| {
            AvError::invalid_argument("binary AVOption hex string contains a non-hex digit")
        })?;
        let low = hex_nibble(pair[1]).ok_or_else(|| {
            AvError::invalid_argument("binary AVOption hex string contains a non-hex digit")
        })?;
        parsed.push((high << 4) | low);
    }

    Ok(parsed)
}

fn parse_dictionary(raw: &str) -> AvResult<Dictionary> {
    let mut dict = Dictionary::new();
    let mut rest = raw;

    while !rest.is_empty() {
        let (key, key_rest) = parse_avoption_token(rest, "=");
        if key.is_empty() || !key_rest.starts_with('=') {
            return Err(AvError::with_code(
                AvErrorKind::InvalidArgument,
                AvErrorCode::EINVAL,
                "dictionary AVOption entry is missing a key/value separator",
            ));
        }

        let value_start = &key_rest[1..];
        let (value, value_rest) = parse_avoption_token(value_start, ":");
        if value.is_empty() {
            return Err(AvError::with_code(
                AvErrorKind::InvalidArgument,
                AvErrorCode::EINVAL,
                "dictionary AVOption entry has an empty value",
            ));
        }

        dict.set_with_mode(key, value, MatchMode::CaseInsensitive, SetMode::Overwrite)?;
        rest = if value_rest.is_empty() {
            value_rest
        } else {
            &value_rest[1..]
        };
    }

    Ok(dict)
}

fn parse_option_array(raw: &str, array: &OptionArrayKind) -> AvResult<Vec<OptionValue>> {
    let values = parse_array_elements(raw, array, "array option", |token| {
        parse_scalar_option_value_for_kind(array.element(), token)
    })?;
    validate_array_len(array, values.len(), "array option")?;
    Ok(values)
}

fn parse_avoption_array(
    raw: &str,
    array: &OptionArrayKind,
    name: &str,
) -> AvResult<Vec<OptionValue>> {
    parse_array_elements(raw, array, name, |token| {
        parse_scalar_avoption_value_for_kind(array.element(), name, token)
    })
}

fn parse_array_elements<F>(
    raw: &str,
    array: &OptionArrayKind,
    name: &str,
    mut parse_element: F,
) -> AvResult<Vec<OptionValue>>
where
    F: FnMut(&str) -> AvResult<OptionValue>,
{
    let mut values = Vec::new();
    if raw.is_empty() {
        return Ok(values);
    }

    let separator = array.separator();
    let mut token = String::new();
    let mut chars = raw.chars().peekable();
    let mut last_was_separator = false;

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(escaped) = chars.next() {
                token.push(escaped);
            } else {
                token.push(ch);
            }
            last_was_separator = false;
            continue;
        }

        if ch == separator {
            push_parsed_array_element(&mut values, &token, array, name, &mut parse_element)?;
            token.clear();
            last_was_separator = true;
            continue;
        }

        token.push(ch);
        last_was_separator = false;
    }

    if !last_was_separator {
        push_parsed_array_element(&mut values, &token, array, name, &mut parse_element)?;
    }

    Ok(values)
}

fn push_parsed_array_element<F>(
    values: &mut Vec<OptionValue>,
    token: &str,
    array: &OptionArrayKind,
    name: &str,
    parse_element: &mut F,
) -> AvResult<()>
where
    F: FnMut(&str) -> AvResult<OptionValue>,
{
    if array
        .max_len()
        .is_some_and(|max_len| values.len() >= max_len)
    {
        return Err(AvError::with_code(
            AvErrorKind::InvalidArgument,
            AvErrorCode::EINVAL,
            format!(
                "Cannot assign more than {} elements to array option {}",
                array.max_len().expect("checked above"),
                name
            ),
        ));
    }

    let value = parse_element(token)?;
    validate_value_for_kind(array.element(), &value)?;
    values.push(value);
    Ok(())
}

fn parse_scalar_option_value_for_kind(kind: &OptionKind, raw: &str) -> AvResult<OptionValue> {
    match kind {
        OptionKind::Bool => Ok(OptionValue::Bool(parse_bool(raw)?)),
        OptionKind::Int { .. } => Ok(OptionValue::Int(parse_int(raw)?)),
        OptionKind::Duration { .. } => Ok(OptionValue::Duration(parse_duration(raw)?)),
        OptionKind::ImageSize => {
            let (width, height) = parse_image_size(raw)?;
            Ok(OptionValue::ImageSize { width, height })
        }
        OptionKind::PixelFormat { min, max } => Ok(OptionValue::PixelFormat(parse_pixel_format(
            raw, *min, *max,
        )?)),
        OptionKind::SampleFormat { min, max } => Ok(OptionValue::SampleFormat(
            parse_sample_format(raw, *min, *max)?,
        )),
        OptionKind::ChannelLayout => Ok(OptionValue::ChannelLayout(parse_channel_layout(raw)?)),
        OptionKind::VideoRate { .. } => Ok(OptionValue::VideoRate(parse_video_rate(raw)?)),
        OptionKind::Color => Ok(OptionValue::Color(parse_color(raw)?)),
        OptionKind::Binary => Ok(OptionValue::Binary(parse_binary(raw)?)),
        OptionKind::Dictionary => Ok(OptionValue::Dictionary(parse_dictionary(raw)?)),
        OptionKind::Array(_) => Err(AvError::invalid_argument(
            "nested AVOption arrays are not supported",
        )),
        OptionKind::Float { .. } => Ok(OptionValue::Float(parse_float(raw)?)),
        OptionKind::Rational { .. } => Ok(OptionValue::Rational(parse_rational(raw)?)),
        OptionKind::String { .. } => Ok(OptionValue::String(raw.to_owned())),
    }
}

fn parse_scalar_avoption_value_for_kind(
    kind: &OptionKind,
    name: &str,
    raw: &str,
) -> AvResult<OptionValue> {
    match kind {
        OptionKind::Duration { .. } => {
            avoption_value_from_numeric(kind, name, AvOptionNumericInput::Int(parse_duration(raw)?))
        }
        OptionKind::VideoRate { .. } => avoption_value_from_numeric(
            kind,
            name,
            AvOptionNumericInput::Rational(parse_video_rate(raw)?),
        ),
        OptionKind::Int { .. } | OptionKind::Float { .. } | OptionKind::Rational { .. } => {
            if matches!(kind, OptionKind::Rational { .. }) {
                if let Some(rational) = parse_avoption_exact_rational_literal(raw)? {
                    return avoption_value_from_numeric(
                        kind,
                        name,
                        AvOptionNumericInput::Rational(rational),
                    );
                }
            }
            avoption_value_from_numeric(
                kind,
                name,
                AvOptionNumericInput::Double(parse_avoption_numeric_expression(raw, &[])?),
            )
        }
        _ => parse_scalar_option_value_for_kind(kind, raw),
    }
}

fn coerce_avoption_array_element(
    kind: &OptionKind,
    name: &str,
    value: &OptionValue,
) -> AvResult<OptionValue> {
    if validate_value_for_kind(kind, value).is_ok() {
        return Ok(value.clone());
    }

    if let OptionValue::String(raw) = value {
        let parsed = parse_scalar_avoption_value_for_kind(kind, name, raw)?;
        validate_value_for_kind(kind, &parsed)?;
        return Ok(parsed);
    }

    if let Some(input) = avoption_numeric_input_from_value(value)? {
        let parsed = avoption_value_from_numeric(kind, name, input)?;
        validate_value_for_kind(kind, &parsed)?;
        return Ok(parsed);
    }

    validate_value_for_kind(kind, value)?;
    unreachable!("validate_value_for_kind returned Ok above")
}

fn avoption_numeric_input_from_value(
    value: &OptionValue,
) -> AvResult<Option<AvOptionNumericInput>> {
    match value {
        OptionValue::Bool(value) => Ok(Some(AvOptionNumericInput::Int(i64::from(*value)))),
        OptionValue::Int(value) | OptionValue::Duration(value) => {
            Ok(Some(AvOptionNumericInput::Int(*value)))
        }
        OptionValue::PixelFormat(value) => Ok(Some(AvOptionNumericInput::Int(i64::from(
            pixel_format_avoption_index(*value)?,
        )))),
        OptionValue::SampleFormat(value) => Ok(Some(AvOptionNumericInput::Int(i64::from(
            sample_format_avoption_index(*value),
        )))),
        OptionValue::Float(value) => Ok(Some(AvOptionNumericInput::Double(*value))),
        OptionValue::Rational(value) => Ok(Some(AvOptionNumericInput::Rational(*value))),
        OptionValue::String(_)
        | OptionValue::ImageSize { .. }
        | OptionValue::ChannelLayout(_)
        | OptionValue::VideoRate(_)
        | OptionValue::Color(_)
        | OptionValue::Binary(_)
        | OptionValue::Dictionary(_)
        | OptionValue::Array(_) => Ok(None),
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_c_auto_i32(raw: &str) -> Option<i32> {
    let mut text = raw.trim_start();
    let mut sign = 1i64;
    if let Some(rest) = text.strip_prefix('+') {
        text = rest;
    } else if let Some(rest) = text.strip_prefix('-') {
        text = rest;
        sign = -1;
    }

    let (base, digits) = if let Some(rest) = text.strip_prefix("0x") {
        (16, rest)
    } else if let Some(rest) = text.strip_prefix("0X") {
        (16, rest)
    } else if text.len() > 1 {
        match text.strip_prefix('0') {
            Some(rest) if !rest.is_empty() => (8, rest),
            _ => (10, text),
        }
    } else {
        (10, text)
    };

    if digits.is_empty() {
        return None;
    }

    let valid = match base {
        8 => digits.bytes().all(|byte| matches!(byte, b'0'..=b'7')),
        10 => digits.bytes().all(|byte| byte.is_ascii_digit()),
        16 => digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
        _ => false,
    };
    if !valid {
        return None;
    }

    let value = i64::from_str_radix(digits, base).ok()?.checked_mul(sign)?;
    i32::try_from(value).ok()
}

fn validate_pixel_format_index(value: Option<PixelFormat>, min: i32, max: i32) -> AvResult<()> {
    let index = pixel_format_avoption_index(value)?;
    if index < min || index > max {
        return Err(avoption_range_error(
            "pixel format",
            f64::from(index),
            f64::from(min),
            f64::from(max),
        ));
    }
    Ok(())
}

fn pixel_format_from_avoption_index(index: i32) -> AvResult<Option<PixelFormat>> {
    let format = match index {
        -1 => return Ok(None),
        0 => PixelFormat::Yuv420p,
        1 => PixelFormat::Yuyv422,
        2 => PixelFormat::Rgb24,
        3 => PixelFormat::Bgr24,
        4 => PixelFormat::Yuv422p,
        5 => PixelFormat::Yuv444p,
        6 => PixelFormat::Yuv410p,
        7 => PixelFormat::Yuv411p,
        8 => PixelFormat::Gray8,
        9 => PixelFormat::MonoWhite,
        10 => PixelFormat::MonoBlack,
        11 => PixelFormat::Pal8,
        12 => PixelFormat::YuvJ420p,
        13 => PixelFormat::YuvJ422p,
        14 => PixelFormat::YuvJ444p,
        15 => PixelFormat::Uyvy422,
        16 => PixelFormat::Uyyvyy411,
        17 => PixelFormat::Bgr8,
        18 => PixelFormat::Bgr4,
        19 => PixelFormat::Bgr4Byte,
        20 => PixelFormat::Rgb8,
        21 => PixelFormat::Rgb4,
        22 => PixelFormat::Rgb4Byte,
        23 => PixelFormat::Nv12,
        24 => PixelFormat::Nv21,
        _ => {
            return Err(AvError::invalid_argument(format!(
                "unsupported bounded FFmpeg pixel format index {index}"
            )))
        }
    };
    Ok(Some(format))
}

fn pixel_format_avoption_index(value: Option<PixelFormat>) -> AvResult<i32> {
    match value {
        None => Ok(-1),
        Some(PixelFormat::Yuv420p) => Ok(0),
        Some(PixelFormat::Yuyv422) => Ok(1),
        Some(PixelFormat::Rgb24) => Ok(2),
        Some(PixelFormat::Bgr24) => Ok(3),
        Some(PixelFormat::Yuv422p) => Ok(4),
        Some(PixelFormat::Yuv444p) => Ok(5),
        Some(PixelFormat::Yuv410p) => Ok(6),
        Some(PixelFormat::Yuv411p) => Ok(7),
        Some(PixelFormat::Gray8) => Ok(8),
        Some(PixelFormat::MonoWhite) => Ok(9),
        Some(PixelFormat::MonoBlack) => Ok(10),
        Some(PixelFormat::Pal8) => Ok(11),
        Some(PixelFormat::YuvJ420p) => Ok(12),
        Some(PixelFormat::YuvJ422p) => Ok(13),
        Some(PixelFormat::YuvJ444p) => Ok(14),
        Some(PixelFormat::Uyvy422) => Ok(15),
        Some(PixelFormat::Uyyvyy411) => Ok(16),
        Some(PixelFormat::Bgr8) => Ok(17),
        Some(PixelFormat::Bgr4) => Ok(18),
        Some(PixelFormat::Bgr4Byte) => Ok(19),
        Some(PixelFormat::Rgb8) => Ok(20),
        Some(PixelFormat::Rgb4) => Ok(21),
        Some(PixelFormat::Rgb4Byte) => Ok(22),
        Some(PixelFormat::Nv12) => Ok(23),
        Some(PixelFormat::Nv21) => Ok(24),
        Some(format) => Err(AvError::unsupported(format!(
            "pixel format `{}` is outside the bounded AVOption index model",
            format.name()
        ))),
    }
}

fn validate_sample_format_index(value: Option<SampleFormat>, min: i32, max: i32) -> AvResult<()> {
    let index = sample_format_avoption_index(value);
    if index < min || index > max {
        return Err(avoption_range_error(
            "sample format",
            f64::from(index),
            f64::from(min),
            f64::from(max),
        ));
    }
    Ok(())
}

fn sample_format_from_avoption_index(index: i32) -> AvResult<Option<SampleFormat>> {
    let format = match index {
        -1 => return Ok(None),
        0 => SampleFormat::U8,
        1 => SampleFormat::S16,
        2 => SampleFormat::S32,
        3 => SampleFormat::Flt,
        4 => SampleFormat::Dbl,
        5 => SampleFormat::U8P,
        6 => SampleFormat::S16P,
        7 => SampleFormat::S32P,
        8 => SampleFormat::FltP,
        9 => SampleFormat::DblP,
        10 => SampleFormat::S64,
        11 => SampleFormat::S64P,
        _ => {
            return Err(AvError::invalid_argument(format!(
                "unsupported FFmpeg sample format index {index}"
            )))
        }
    };
    Ok(Some(format))
}

fn sample_format_avoption_index(value: Option<SampleFormat>) -> i32 {
    match value {
        None => -1,
        Some(SampleFormat::U8) => 0,
        Some(SampleFormat::S16) => 1,
        Some(SampleFormat::S32) => 2,
        Some(SampleFormat::Flt) => 3,
        Some(SampleFormat::Dbl) => 4,
        Some(SampleFormat::U8P) => 5,
        Some(SampleFormat::S16P) => 6,
        Some(SampleFormat::S32P) => 7,
        Some(SampleFormat::FltP) => 8,
        Some(SampleFormat::DblP) => 9,
        Some(SampleFormat::S64) => 10,
        Some(SampleFormat::S64P) => 11,
    }
}

fn validate_kind(kind: &OptionKind) -> AvResult<()> {
    match *kind {
        OptionKind::Bool
        | OptionKind::ImageSize
        | OptionKind::ChannelLayout
        | OptionKind::Color
        | OptionKind::Binary
        | OptionKind::Dictionary
        | OptionKind::String { .. } => Ok(()),
        OptionKind::Array(ref array) => {
            validate_array_element_kind(array.element())?;
            validate_array_separator(array.separator())?;
            if let Some(max_len) = array.max_len() {
                if array.min_len() > max_len {
                    return Err(AvError::invalid_argument(
                        "array option minimum length must be <= maximum length",
                    ));
                }
            }
            Ok(())
        }
        OptionKind::PixelFormat { min, max } => {
            if min > max {
                return Err(AvError::invalid_argument(
                    "pixel format option min must be <= max",
                ));
            }
            if min < -1 {
                return Err(AvError::invalid_argument(
                    "pixel format option min must be >= AV_PIX_FMT_NONE",
                ));
            }
            Ok(())
        }
        OptionKind::SampleFormat { min, max } => {
            if min > max {
                return Err(AvError::invalid_argument(
                    "sample format option min must be <= max",
                ));
            }
            if min < -1 {
                return Err(AvError::invalid_argument(
                    "sample format option min must be >= AV_SAMPLE_FMT_NONE",
                ));
            }
            if max >= AV_SAMPLE_FMT_NB {
                return Err(AvError::invalid_argument(
                    "sample format option max must be < AV_SAMPLE_FMT_NB",
                ));
            }
            Ok(())
        }
        OptionKind::Int { min, max } => {
            if min > max {
                return Err(AvError::invalid_argument(
                    "integer option min must be <= max",
                ));
            }
            Ok(())
        }
        OptionKind::Duration { min, max } => {
            if min > max {
                return Err(AvError::invalid_argument(
                    "duration option min must be <= max",
                ));
            }
            Ok(())
        }
        OptionKind::Float { min, max } => {
            if !min.is_finite() || !max.is_finite() {
                return Err(AvError::invalid_argument(
                    "float option range must be finite",
                ));
            }
            if min > max {
                return Err(AvError::invalid_argument("float option min must be <= max"));
            }
            Ok(())
        }
        OptionKind::Rational { min, max } => {
            validate_rational_bound(min, "option min")?;
            validate_rational_bound(max, "option max")?;
            if min > max {
                return Err(AvError::invalid_argument(
                    "rational option min must be <= max",
                ));
            }
            Ok(())
        }
        OptionKind::VideoRate { min, max } => {
            validate_video_rate_bound(min, "option min")?;
            validate_video_rate_bound(max, "option max")?;
            if min > max {
                return Err(AvError::invalid_argument(
                    "video rate option min must be <= max",
                ));
            }
            Ok(())
        }
    }
}

fn validate_array_element_kind(kind: &OptionKind) -> AvResult<()> {
    if matches!(kind, OptionKind::Array(_)) {
        return Err(AvError::invalid_argument(
            "nested array AVOptions are not supported",
        ));
    }
    validate_kind(kind)
}

fn validate_array_separator(separator: char) -> AvResult<()> {
    if !separator.is_ascii()
        || !(separator.is_ascii_graphic() || separator == ' ')
        || separator.is_ascii_alphanumeric()
        || separator == '\\'
    {
        return Err(AvError::invalid_argument(
            "array option separator must be printable ASCII, non-alphanumeric, and not backslash",
        ));
    }
    Ok(())
}

fn validate_array_len(array: &OptionArrayKind, len: usize, name: &str) -> AvResult<()> {
    if len < array.min_len() {
        return Err(AvError::with_code(
            AvErrorKind::InvalidArgument,
            AvErrorCode::EINVAL,
            format!(
                "Cannot assign fewer than {} elements to array option {}",
                array.min_len(),
                name
            ),
        ));
    }
    if let Some(max_len) = array.max_len() {
        if len > max_len {
            return Err(AvError::with_code(
                AvErrorKind::InvalidArgument,
                AvErrorCode::EINVAL,
                format!("Cannot assign more than {max_len} elements to array option {name}"),
            ));
        }
    }
    Ok(())
}

fn range_for_kind(kind: &OptionKind) -> Option<OptionRange> {
    match *kind {
        OptionKind::Int { min, max } => Some(OptionRange {
            min: OptionValue::Int(min),
            max: OptionValue::Int(max),
        }),
        OptionKind::Duration { min, max } => Some(OptionRange {
            min: OptionValue::Duration(min),
            max: OptionValue::Duration(max),
        }),
        OptionKind::Float { min, max } => Some(OptionRange {
            min: OptionValue::Float(min),
            max: OptionValue::Float(max),
        }),
        OptionKind::Rational { min, max } => Some(OptionRange {
            min: OptionValue::Rational(min),
            max: OptionValue::Rational(max),
        }),
        OptionKind::VideoRate { min, max } => Some(OptionRange {
            min: OptionValue::VideoRate(min),
            max: OptionValue::VideoRate(max),
        }),
        OptionKind::PixelFormat { .. }
        | OptionKind::SampleFormat { .. }
        | OptionKind::ChannelLayout
        | OptionKind::Binary
        | OptionKind::Dictionary
        | OptionKind::Array(_) => None,
        OptionKind::Bool
        | OptionKind::ImageSize
        | OptionKind::Color
        | OptionKind::String { .. } => None,
    }
}

fn avoption_ranges_for_kind(kind: &OptionKind) -> AvOptionRanges {
    let mut range = AvOptionRangeEntry {
        value_min: 0.0,
        value_max: 0.0,
        component_min: 0.0,
        component_max: 0.0,
        is_range: true,
    };

    match *kind {
        OptionKind::Bool => {
            range.value_min = 0.0;
            range.value_max = 1.0;
        }
        OptionKind::Int { min, max } => {
            range.value_min = min as f64;
            range.value_max = max as f64;
        }
        OptionKind::Duration { min, max } => {
            range.value_min = min as f64;
            range.value_max = max as f64;
        }
        OptionKind::ImageSize => {
            range.value_min = 0.0;
            range.value_max = f64::from(i32::MAX / 8);
            range.component_min = 0.0;
            range.component_max = f64::from(i32::MAX / 128 / 8);
        }
        OptionKind::PixelFormat { min, max } => {
            range.value_min = f64::from(min);
            range.value_max = f64::from(max);
        }
        OptionKind::SampleFormat { min, max } => {
            range.value_min = f64::from(min);
            range.value_max = f64::from(max);
        }
        OptionKind::ChannelLayout => {}
        OptionKind::Binary => {}
        OptionKind::Dictionary => {}
        OptionKind::Array(_) => {}
        OptionKind::VideoRate { .. } => {
            range.value_min = 1.0;
            range.value_max = i32::MAX as f64;
            range.component_min = 1.0;
            range.component_max = i32::MAX as f64;
        }
        OptionKind::Color => {}
        OptionKind::Float { min, max } => {
            range.value_min = min;
            range.value_max = max;
        }
        OptionKind::Rational { min, max } => {
            range.value_min = min.to_f64();
            range.value_max = max.to_f64();
            range.component_min = i32::MIN as f64;
            range.component_max = i32::MAX as f64;
        }
        OptionKind::String { .. } => {
            range.value_min = -1.0;
            range.value_max = i32::MAX as f64;
            range.component_min = 0.0;
            range.component_max = 0x10ffff as f64;
        }
    }

    AvOptionRanges::one(range)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AvOptionNumericInput {
    Int(i64),
    Double(f64),
    Rational(Rational),
}

impl AvOptionNumericInput {
    fn value(self) -> AvResult<f64> {
        match self {
            Self::Int(value) => Ok(value as f64),
            Self::Double(value) => {
                if value.is_finite() {
                    Ok(value)
                } else {
                    Err(AvError::invalid_argument(
                        "numeric AVOption value must be finite",
                    ))
                }
            }
            Self::Rational(value) => {
                validate_rational_bound(value, "numeric AVOption rational")?;
                Ok(value.to_f64())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AvOptionNumberParts {
    num: f64,
    den: i32,
    intnum: i64,
}

impl AvOptionNumberParts {
    fn to_double(self) -> AvResult<f64> {
        if self.den == 0 {
            return Err(AvError::invalid_argument(
                "numeric AVOption denominator must not be zero",
            ));
        }
        Ok(self.num * self.intnum as f64 / f64::from(self.den))
    }

    fn to_int(self) -> AvResult<i64> {
        if self.den == 0 {
            return Err(AvError::invalid_argument(
                "numeric AVOption denominator must not be zero",
            ));
        }

        if self.num == f64::from(self.den) {
            return Ok(self.intnum);
        }

        trunc_f64_to_i64(self.num * self.intnum as f64 / f64::from(self.den))
    }

    fn to_rational(self) -> AvResult<Rational> {
        if self.den == 0 {
            return Err(AvError::invalid_argument(
                "numeric AVOption denominator must not be zero",
            ));
        }

        if self.num == 1.0 {
            let num = match i32::try_from(self.intnum) {
                Ok(num) => num,
                Err(_) => return Rational::from_f64_limited(self.to_double()?, 1 << 24),
            };
            return Rational::new(num, self.den);
        }

        Rational::from_f64_limited(self.to_double()?, 1 << 24)
    }
}

fn avoption_number_parts(name: &str, value: &OptionValue) -> AvResult<AvOptionNumberParts> {
    match value {
        OptionValue::Bool(value) => Ok(AvOptionNumberParts {
            num: 1.0,
            den: 1,
            intnum: i64::from(*value),
        }),
        OptionValue::Int(value) => Ok(AvOptionNumberParts {
            num: 1.0,
            den: 1,
            intnum: *value,
        }),
        OptionValue::Duration(value) => Ok(AvOptionNumberParts {
            num: 1.0,
            den: 1,
            intnum: *value,
        }),
        OptionValue::ImageSize { .. } => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
        OptionValue::PixelFormat(value) => Ok(AvOptionNumberParts {
            num: 1.0,
            den: 1,
            intnum: i64::from(pixel_format_avoption_index(*value)?),
        }),
        OptionValue::SampleFormat(value) => Ok(AvOptionNumberParts {
            num: 1.0,
            den: 1,
            intnum: i64::from(sample_format_avoption_index(*value)),
        }),
        OptionValue::ChannelLayout(_) => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
        OptionValue::VideoRate(_) => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
        OptionValue::Color(_) => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
        OptionValue::Binary(_) => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
        OptionValue::Dictionary(_) => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
        OptionValue::Array(_) => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
        OptionValue::Float(value) => Ok(AvOptionNumberParts {
            num: *value,
            den: 1,
            intnum: 1,
        }),
        OptionValue::Rational(value) => {
            validate_rational_bound(*value, "numeric AVOption rational")?;
            Ok(AvOptionNumberParts {
                num: 1.0,
                den: value.den(),
                intnum: i64::from(value.num()),
            })
        }
        OptionValue::String(_) => Err(AvError::invalid_argument(format!(
            "AVOption `{name}` is not numeric"
        ))),
    }
}

fn avoption_value_from_numeric(
    kind: &OptionKind,
    name: &str,
    input: AvOptionNumericInput,
) -> AvResult<OptionValue> {
    let value = input.value()?;

    match *kind {
        OptionKind::Bool => {
            avoption_check_numeric_range(name, value, 0.0, 1.0)?;
            Ok(OptionValue::Bool(round_f64_ties_even_to_i64(value)? != 0))
        }
        OptionKind::Int { min, max } => {
            avoption_check_numeric_range(name, value, min as f64, max as f64)?;
            Ok(OptionValue::Int(round_f64_ties_even_to_i64(value)?))
        }
        OptionKind::Duration { min, max } => {
            avoption_check_numeric_range(name, value, min as f64, max as f64)?;
            Ok(OptionValue::Duration(round_f64_ties_even_to_i64(value)?))
        }
        OptionKind::ImageSize => {
            if value == 0.0 {
                Err(AvError::invalid_argument(format!(
                    "AVOption `{name}` is not numeric"
                )))
            } else {
                Err(avoption_range_error(name, value, 0.0, 0.0))
            }
        }
        OptionKind::PixelFormat { min, max } => {
            avoption_check_numeric_range(name, value, f64::from(min), f64::from(max))?;
            let index = round_f64_ties_even_to_i64(value)?;
            let index = i32::try_from(index).map_err(|_| {
                AvError::invalid_argument("numeric AVOption pixel format out of range")
            })?;
            Ok(OptionValue::PixelFormat(pixel_format_from_avoption_index(
                index,
            )?))
        }
        OptionKind::SampleFormat { min, max } => {
            avoption_check_numeric_range(name, value, f64::from(min), f64::from(max))?;
            let index = round_f64_ties_even_to_i64(value)?;
            let index = i32::try_from(index).map_err(|_| {
                AvError::invalid_argument("numeric AVOption sample format out of range")
            })?;
            Ok(OptionValue::SampleFormat(
                sample_format_from_avoption_index(index)?,
            ))
        }
        OptionKind::ChannelLayout => {
            if value == 0.0 {
                Err(AvError::invalid_argument(format!(
                    "AVOption `{name}` is not numeric"
                )))
            } else {
                Err(avoption_range_error(name, value, 0.0, 0.0))
            }
        }
        OptionKind::VideoRate { min, max } => {
            avoption_check_numeric_range(name, value, min.to_f64(), max.to_f64())?;
            let rational = rational_from_avoption_numeric_input(input)?;
            validate_video_rate_bound(rational, "numeric AVOption video rate")?;
            Ok(OptionValue::VideoRate(rational))
        }
        OptionKind::Color => {
            if value == 0.0 {
                Err(AvError::invalid_argument(format!(
                    "AVOption `{name}` is not numeric"
                )))
            } else {
                Err(avoption_range_error(name, value, 0.0, 0.0))
            }
        }
        OptionKind::Binary => {
            if value == 0.0 {
                Err(AvError::invalid_argument(format!(
                    "AVOption `{name}` is not numeric"
                )))
            } else {
                Err(avoption_range_error(name, value, 0.0, 0.0))
            }
        }
        OptionKind::Dictionary => {
            if value == 0.0 {
                Err(AvError::invalid_argument(format!(
                    "AVOption `{name}` is not numeric"
                )))
            } else {
                Err(avoption_range_error(name, value, 0.0, 0.0))
            }
        }
        OptionKind::Array(_) => {
            if value == 0.0 {
                Err(AvError::invalid_argument(format!(
                    "AVOption `{name}` is not numeric"
                )))
            } else {
                Err(avoption_range_error(name, value, 0.0, 0.0))
            }
        }
        OptionKind::Float { min, max } => {
            avoption_check_numeric_range(name, value, min, max)?;
            Ok(OptionValue::Float(value))
        }
        OptionKind::Rational { min, max } => {
            avoption_check_numeric_range(name, value, min.to_f64(), max.to_f64())?;
            let rational = match input {
                AvOptionNumericInput::Int(value) => {
                    let value = i32::try_from(value).map_err(|_| {
                        AvError::invalid_argument(
                            "numeric AVOption rational numerator out of range",
                        )
                    })?;
                    Rational::new(value, 1)?
                }
                AvOptionNumericInput::Double(value) => Rational::from_f64_limited(value, 1 << 24)?,
                AvOptionNumericInput::Rational(value) => {
                    validate_rational_bound(value, "numeric AVOption rational")?;
                    value
                }
            };
            validate_rational_bound(rational, "numeric AVOption rational")?;
            Ok(OptionValue::Rational(rational))
        }
        OptionKind::String { .. } => {
            if value == 0.0 {
                Err(AvError::invalid_argument(format!(
                    "AVOption `{name}` is not numeric"
                )))
            } else {
                Err(avoption_range_error(name, value, 0.0, 0.0))
            }
        }
    }
}

fn rational_from_avoption_numeric_input(input: AvOptionNumericInput) -> AvResult<Rational> {
    match input {
        AvOptionNumericInput::Int(value) => {
            let value = i32::try_from(value).map_err(|_| {
                AvError::invalid_argument("numeric AVOption rational numerator out of range")
            })?;
            Rational::new(value, 1)
        }
        AvOptionNumericInput::Double(value) => {
            let mut rational = Rational::from_f64_limited(value, 1 << 24)?;
            if (rational.num() == 0 || rational.den() == 0) && value != 0.0 {
                rational = Rational::from_f64_limited(value, i32::MAX)?;
            }
            Ok(rational)
        }
        AvOptionNumericInput::Rational(value) => {
            validate_rational_bound(value, "numeric AVOption rational")?;
            Ok(value)
        }
    }
}

fn avoption_check_numeric_range(name: &str, value: f64, min: f64, max: f64) -> AvResult<()> {
    if !value.is_finite() {
        return Err(AvError::invalid_argument(
            "numeric AVOption value must be finite",
        ));
    }

    if value < min || value > max {
        return Err(avoption_range_error(name, value, min, max));
    }

    Ok(())
}

fn avoption_range_error(name: &str, value: f64, min: f64, max: f64) -> AvError {
    AvError::with_code(
        AvErrorKind::InvalidArgument,
        AvErrorCode::from_posix_errno(34),
        format!("AVOption `{name}` value {value} outside range {min}..={max}"),
    )
}

fn round_f64_ties_even_to_i64(value: f64) -> AvResult<i64> {
    if !value.is_finite() || value < i64::MIN as f64 || value > i64::MAX as f64 {
        return Err(AvError::invalid_argument(
            "numeric AVOption integer value out of range",
        ));
    }

    let floor = value.floor();
    let fraction = value - floor;
    let rounded = if fraction < 0.5 {
        floor
    } else if fraction > 0.5 {
        floor + 1.0
    } else if (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    };

    Ok(rounded as i64)
}

fn trunc_f64_to_i64(value: f64) -> AvResult<i64> {
    if !value.is_finite() || value < i64::MIN as f64 || value > i64::MAX as f64 {
        return Err(AvError::invalid_argument(
            "numeric AVOption integer value out of range",
        ));
    }

    Ok(value.trunc() as i64)
}

fn validate_value_for_kind(kind: &OptionKind, value: &OptionValue) -> AvResult<()> {
    match (kind, value) {
        (OptionKind::Bool, OptionValue::Bool(_)) => Ok(()),
        (OptionKind::Int { min, max }, OptionValue::Int(value)) => {
            if value < min || value > max {
                return Err(AvError::invalid_argument(format!(
                    "integer option value {value} outside range {min}..={max}"
                )));
            }
            Ok(())
        }
        (OptionKind::Duration { min, max }, OptionValue::Duration(value)) => {
            if value < min || value > max {
                return Err(AvError::invalid_argument(format!(
                    "duration option value {value} outside range {min}..={max}"
                )));
            }
            Ok(())
        }
        (OptionKind::ImageSize, OptionValue::ImageSize { width, height }) => {
            if *width < 0 || *height < 0 {
                return Err(AvError::invalid_argument(format!(
                    "image size option value {width}x{height} must not be negative"
                )));
            }
            Ok(())
        }
        (OptionKind::PixelFormat { min, max }, OptionValue::PixelFormat(value)) => {
            validate_pixel_format_index(*value, *min, *max)
        }
        (OptionKind::SampleFormat { min, max }, OptionValue::SampleFormat(value)) => {
            validate_sample_format_index(*value, *min, *max)
        }
        (OptionKind::ChannelLayout, OptionValue::ChannelLayout(_)) => Ok(()),
        (OptionKind::VideoRate { min, max }, OptionValue::VideoRate(value)) => {
            validate_video_rate_bound(*value, "video rate option value")?;
            if value < min || value > max {
                return Err(AvError::invalid_argument(format!(
                    "video rate option value {value} outside range {min}..={max}"
                )));
            }
            Ok(())
        }
        (OptionKind::Color, OptionValue::Color(_)) => Ok(()),
        (OptionKind::Binary, OptionValue::Binary(_)) => Ok(()),
        (OptionKind::Dictionary, OptionValue::Dictionary(_)) => Ok(()),
        (OptionKind::Array(array), OptionValue::Array(values)) => {
            validate_array_len(array, values.len(), "array option")?;
            for value in values {
                validate_value_for_kind(array.element(), value)?;
            }
            Ok(())
        }
        (OptionKind::Float { min, max }, OptionValue::Float(value)) => {
            if !value.is_finite() {
                return Err(AvError::invalid_argument(
                    "float option value must be finite",
                ));
            }
            if value < min || value > max {
                return Err(AvError::invalid_argument(format!(
                    "float option value {value} outside range {min}..={max}"
                )));
            }
            Ok(())
        }
        (OptionKind::Rational { min, max }, OptionValue::Rational(value)) => {
            validate_rational_bound(*value, "rational option value")?;
            if value < min || value > max {
                return Err(AvError::invalid_argument(format!(
                    "rational option value {value} outside range {min}..={max}"
                )));
            }
            Ok(())
        }
        (OptionKind::String { allow_empty }, OptionValue::String(value)) => {
            if !allow_empty && value.is_empty() {
                return Err(AvError::invalid_argument(
                    "string option value must not be empty",
                ));
            }
            if value.as_bytes().contains(&0) {
                return Err(AvError::invalid_argument(
                    "string option value must not contain NUL",
                ));
            }
            Ok(())
        }
        _ => Err(AvError::invalid_argument(
            "option value type does not match option kind",
        )),
    }
}

fn validate_rational_bound(value: Rational, label: &str) -> AvResult<()> {
    if value.den() <= 0 {
        return Err(AvError::invalid_argument(format!(
            "{label} must have a positive denominator"
        )));
    }
    Ok(())
}

fn validate_video_rate_bound(value: Rational, label: &str) -> AvResult<()> {
    validate_rational_bound(value, label)?;
    if value.num() <= 0 {
        return Err(AvError::invalid_argument(format!(
            "{label} must have a positive numerator"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
struct AvOptionExpressionConstant {
    name: String,
    value: f64,
}

fn avoption_kind_min(kind: &OptionKind) -> AvResult<f64> {
    match *kind {
        OptionKind::Bool => Ok(0.0),
        OptionKind::Int { min, .. } => Ok(min as f64),
        OptionKind::Duration { min, .. } => Ok(min as f64),
        OptionKind::ImageSize => Err(AvError::invalid_argument(
            "image size AVOption does not have a scalar numeric minimum",
        )),
        OptionKind::PixelFormat { min, .. } => Ok(f64::from(min)),
        OptionKind::SampleFormat { min, .. } => Ok(f64::from(min)),
        OptionKind::ChannelLayout => Err(AvError::invalid_argument(
            "channel layout AVOption does not have a numeric minimum",
        )),
        OptionKind::VideoRate { min, .. } => Ok(min.to_f64()),
        OptionKind::Color => Err(AvError::invalid_argument(
            "color AVOption does not have a numeric minimum",
        )),
        OptionKind::Binary => Err(AvError::invalid_argument(
            "binary AVOption does not have a numeric minimum",
        )),
        OptionKind::Dictionary => Err(AvError::invalid_argument(
            "dictionary AVOption does not have a numeric minimum",
        )),
        OptionKind::Array(_) => Err(AvError::invalid_argument(
            "array AVOption does not have a numeric minimum",
        )),
        OptionKind::Float { min, .. } => Ok(min),
        OptionKind::Rational { min, .. } => Ok(min.to_f64()),
        OptionKind::String { .. } => Err(AvError::invalid_argument(
            "string AVOption does not have a numeric minimum",
        )),
    }
}

fn avoption_kind_max(kind: &OptionKind) -> AvResult<f64> {
    match *kind {
        OptionKind::Bool => Ok(1.0),
        OptionKind::Int { max, .. } => Ok(max as f64),
        OptionKind::Duration { max, .. } => Ok(max as f64),
        OptionKind::ImageSize => Err(AvError::invalid_argument(
            "image size AVOption does not have a scalar numeric maximum",
        )),
        OptionKind::PixelFormat { max, .. } => Ok(f64::from(max)),
        OptionKind::SampleFormat { max, .. } => Ok(f64::from(max)),
        OptionKind::ChannelLayout => Err(AvError::invalid_argument(
            "channel layout AVOption does not have a numeric maximum",
        )),
        OptionKind::VideoRate { max, .. } => Ok(max.to_f64()),
        OptionKind::Color => Err(AvError::invalid_argument(
            "color AVOption does not have a numeric maximum",
        )),
        OptionKind::Binary => Err(AvError::invalid_argument(
            "binary AVOption does not have a numeric maximum",
        )),
        OptionKind::Dictionary => Err(AvError::invalid_argument(
            "dictionary AVOption does not have a numeric maximum",
        )),
        OptionKind::Array(_) => Err(AvError::invalid_argument(
            "array AVOption does not have a numeric maximum",
        )),
        OptionKind::Float { max, .. } => Ok(max),
        OptionKind::Rational { max, .. } => Ok(max.to_f64()),
        OptionKind::String { .. } => Err(AvError::invalid_argument(
            "string AVOption does not have a numeric maximum",
        )),
    }
}

fn parse_avoption_exact_rational_literal(raw: &str) -> AvResult<Option<Rational>> {
    let raw = raw.trim_start();
    let Some((num, pos)) = parse_signed_decimal_i32(raw, 0) else {
        return Ok(None);
    };
    let Some(separator) = raw.as_bytes().get(pos).copied() else {
        return Ok(None);
    };
    if separator != b'/' && separator != b':' {
        return Ok(None);
    }

    let Some((den, pos)) = parse_signed_decimal_i32(raw, pos + 1) else {
        return Ok(None);
    };
    if pos != raw.len() {
        return Ok(None);
    }

    Rational::new(num, den)
        .map(Some)
        .map_err(|_| AvError::invalid_argument(format!("invalid rational option value `{raw}`")))
}

fn parse_signed_decimal_i32(raw: &str, start: usize) -> Option<(i32, usize)> {
    let bytes = raw.as_bytes();
    let mut pos = start;
    if matches!(bytes.get(pos), Some(b'+') | Some(b'-')) {
        pos += 1;
    }

    let digits_start = pos;
    while matches!(bytes.get(pos), Some(b'0'..=b'9')) {
        pos += 1;
    }
    if pos == digits_start {
        return None;
    }

    raw[start..pos]
        .parse::<i32>()
        .ok()
        .map(|value| (value, pos))
}

fn parse_avoption_numeric_expression(
    raw: &str,
    constants: &[AvOptionExpressionConstant],
) -> AvResult<f64> {
    let source: String = raw.chars().filter(|ch| !ch.is_ascii_whitespace()).collect();
    if source.is_empty() {
        return Err(AvError::invalid_argument(
            "empty numeric AVOption expression",
        ));
    }

    let mut parser = AvOptionExpressionParser {
        source: &source,
        pos: 0,
        constants,
    };
    let value = parser.parse_expression()?;
    if parser.pos != parser.source.len() {
        return Err(AvError::invalid_argument(format!(
            "invalid numeric AVOption expression `{raw}`"
        )));
    }
    if value.is_nan() {
        return Err(AvError::invalid_argument(format!(
            "invalid numeric AVOption expression `{raw}`"
        )));
    }

    Ok(value)
}

struct AvOptionExpressionParser<'a> {
    source: &'a str,
    pos: usize,
    constants: &'a [AvOptionExpressionConstant],
}

impl AvOptionExpressionParser<'_> {
    fn parse_expression(&mut self) -> AvResult<f64> {
        let mut value = self.parse_sum()?;
        while self.consume_ascii(b';') {
            value = self.parse_sum()?;
        }
        Ok(value)
    }

    fn parse_sum(&mut self) -> AvResult<f64> {
        let mut value = self.parse_product()?;
        loop {
            if self.consume_ascii(b'+') {
                value += self.parse_product()?;
            } else if self.consume_ascii(b'-') {
                value -= self.parse_product()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_product(&mut self) -> AvResult<f64> {
        let mut value = self.parse_power()?;
        loop {
            if self.consume_ascii(b'*') {
                value *= self.parse_power()?;
            } else if self.consume_ascii(b'/') {
                value /= self.parse_power()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_power(&mut self) -> AvResult<f64> {
        let mut value = self.parse_unary()?;
        while self.consume_ascii(b'^') {
            value = value.powf(self.parse_unary()?);
        }
        Ok(value)
    }

    fn parse_unary(&mut self) -> AvResult<f64> {
        if self.consume_ascii(b'+') {
            return self.parse_unary();
        }
        if self.consume_ascii(b'-') {
            return Ok(-self.parse_unary()?);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> AvResult<f64> {
        if self.consume_ascii(b'(') {
            let value = self.parse_expression()?;
            if !self.consume_ascii(b')') {
                return Err(AvError::invalid_argument(
                    "unclosed numeric AVOption expression group",
                ));
            }
            return Ok(value);
        }

        if let Some(value) = self.parse_number()? {
            return Ok(value);
        }
        if let Some(value) = self.parse_constant() {
            return Ok(value);
        }

        Err(AvError::invalid_argument(format!(
            "invalid numeric AVOption expression `{}`",
            self.source
        )))
    }

    fn parse_number(&mut self) -> AvResult<Option<f64>> {
        let start = self.pos;
        let mut value = if self.remaining().starts_with("0x") || self.remaining().starts_with("0X")
        {
            self.pos += 2;
            let digits_start = self.pos;
            while self
                .peek_ascii()
                .is_some_and(|byte| byte.is_ascii_hexdigit())
            {
                self.pos += 1;
            }
            if self.pos == digits_start {
                self.pos = start;
                return Ok(None);
            }
            u128::from_str_radix(&self.source[digits_start..self.pos], 16)
                .map(|value| value as f64)
                .map_err(|_| AvError::invalid_argument("invalid hexadecimal AVOption number"))?
        } else {
            let mut has_digits = false;
            while matches!(self.peek_ascii(), Some(b'0'..=b'9')) {
                has_digits = true;
                self.pos += 1;
            }
            if self.consume_ascii(b'.') {
                while matches!(self.peek_ascii(), Some(b'0'..=b'9')) {
                    has_digits = true;
                    self.pos += 1;
                }
            }
            if !has_digits {
                self.pos = start;
                return Ok(None);
            }

            let exponent_start = self.pos;
            if matches!(self.peek_ascii(), Some(b'e') | Some(b'E')) {
                self.pos += 1;
                if matches!(self.peek_ascii(), Some(b'+') | Some(b'-')) {
                    self.pos += 1;
                }
                let digits_start = self.pos;
                while matches!(self.peek_ascii(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
                if self.pos == digits_start {
                    self.pos = exponent_start;
                }
            }

            self.source[start..self.pos]
                .parse::<f64>()
                .map_err(|_| AvError::invalid_argument("invalid AVOption number"))?
        };

        self.apply_number_suffixes(&mut value);
        Ok(Some(value))
    }

    fn apply_number_suffixes(&mut self, value: &mut f64) {
        if self.remaining().starts_with("dB") {
            *value = 10.0_f64.powf(*value / 20.0);
            self.pos += 2;
        } else if let Some(prefix) = self.peek_char().and_then(avoption_si_prefix) {
            if self.remaining_after_char().starts_with('i') {
                *value *= prefix.binary;
                self.pos += 2;
            } else {
                *value *= prefix.decimal;
                self.pos += 1;
            }
        }

        if self.consume_ascii(b'B') {
            *value *= 8.0;
        }
    }

    fn parse_constant(&mut self) -> Option<f64> {
        for constant in self.constants {
            if self.identifier_matches(&constant.name) {
                self.pos += constant.name.len();
                return Some(constant.value);
            }
        }

        for (name, value) in [
            ("E", std::f64::consts::E),
            ("PI", std::f64::consts::PI),
            ("PHI", 1.618_033_988_749_895_f64),
            ("QP2LAMBDA", 118.0),
        ] {
            if self.identifier_matches(name) {
                self.pos += name.len();
                return Some(value);
            }
        }

        None
    }

    fn identifier_matches(&self, prefix: &str) -> bool {
        let remaining = self.remaining();
        if !remaining.starts_with(prefix) {
            return false;
        }
        match remaining.as_bytes().get(prefix.len()) {
            Some(byte) => !is_avoption_identifier_char(*byte),
            None => true,
        }
    }

    fn remaining(&self) -> &str {
        &self.source[self.pos..]
    }

    fn remaining_after_char(&self) -> &str {
        let Some(ch) = self.peek_char() else {
            return "";
        };
        &self.source[self.pos + ch.len_utf8()..]
    }

    fn peek_ascii(&self) -> Option<u8> {
        self.source.as_bytes().get(self.pos).copied()
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn consume_ascii(&mut self, byte: u8) -> bool {
        if self.peek_ascii() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AvOptionSiPrefix {
    binary: f64,
    decimal: f64,
}

fn avoption_si_prefix(ch: char) -> Option<AvOptionSiPrefix> {
    let prefix = match ch {
        'y' => AvOptionSiPrefix {
            binary: 8.271_806_125_530_277e-25,
            decimal: 1e-24,
        },
        'z' => AvOptionSiPrefix {
            binary: 8.470_329_472_543_003e-22,
            decimal: 1e-21,
        },
        'a' => AvOptionSiPrefix {
            binary: 8.673_617_379_884_036e-19,
            decimal: 1e-18,
        },
        'f' => AvOptionSiPrefix {
            binary: 8.881_784_197_001_252e-16,
            decimal: 1e-15,
        },
        'p' => AvOptionSiPrefix {
            binary: 9.094_947_017_729_282e-13,
            decimal: 1e-12,
        },
        'n' => AvOptionSiPrefix {
            binary: 9.313_225_746_154_785e-10,
            decimal: 1e-9,
        },
        'u' => AvOptionSiPrefix {
            binary: 9.536_743_164_062_5e-7,
            decimal: 1e-6,
        },
        'm' => AvOptionSiPrefix {
            binary: 9.765_625e-4,
            decimal: 1e-3,
        },
        'c' => AvOptionSiPrefix {
            binary: 9.843_133_202_303_695e-3,
            decimal: 1e-2,
        },
        'd' => AvOptionSiPrefix {
            binary: 9.921_256_574_801_246e-2,
            decimal: 1e-1,
        },
        'h' => AvOptionSiPrefix {
            binary: 1.015_936_673_259_648e2,
            decimal: 1e2,
        },
        'k' | 'K' => AvOptionSiPrefix {
            binary: 1.024e3,
            decimal: 1e3,
        },
        'M' => AvOptionSiPrefix {
            binary: 1.048_576e6,
            decimal: 1e6,
        },
        'G' => AvOptionSiPrefix {
            binary: 1.073_741_824e9,
            decimal: 1e9,
        },
        'T' => AvOptionSiPrefix {
            binary: 1.099_511_627_776e12,
            decimal: 1e12,
        },
        'P' => AvOptionSiPrefix {
            binary: 1.125_899_906_842_624e15,
            decimal: 1e15,
        },
        'E' => AvOptionSiPrefix {
            binary: 1.152_921_504_606_847e18,
            decimal: 1e18,
        },
        'Z' => AvOptionSiPrefix {
            binary: 1.180_591_620_717_411_3e21,
            decimal: 1e21,
        },
        'Y' => AvOptionSiPrefix {
            binary: 1.208_925_819_614_629_2e24,
            decimal: 1e24,
        },
        _ => return None,
    };
    Some(prefix)
}

fn is_avoption_identifier_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn parse_bool(raw: &str) -> AvResult<bool> {
    match raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "y" => Ok(true),
        "0" | "false" | "no" | "off" | "n" => Ok(false),
        _ => Err(AvError::invalid_argument(format!(
            "invalid boolean option value `{raw}`"
        ))),
    }
}

fn parse_int(raw: &str) -> AvResult<i64> {
    raw.parse::<i64>()
        .map_err(|_| AvError::invalid_argument(format!("invalid integer option value `{raw}`")))
}

fn parse_float(raw: &str) -> AvResult<f64> {
    raw.parse::<f64>()
        .map_err(|_| AvError::invalid_argument(format!("invalid float option value `{raw}`")))
}

fn parse_image_size(raw: &str) -> AvResult<(i32, i32)> {
    if raw == "none" {
        return Ok((0, 0));
    }

    for (name, width, height) in VIDEO_SIZE_ABBREVIATIONS {
        if raw == *name {
            return Ok((*width, *height));
        }
    }

    let (width, mut pos) = parse_image_size_decimal(raw, 0)?;
    if pos < raw.len() {
        if raw[pos..]
            .chars()
            .next()
            .expect("separator exists")
            .len_utf8()
            != 1
        {
            return Err(image_size_parse_error(raw));
        }
        pos += 1;
    }
    let (height, pos) = parse_image_size_decimal(raw, pos)?;
    if pos != raw.len() || width <= 0 || height <= 0 {
        return Err(image_size_parse_error(raw));
    }

    Ok((width, height))
}

fn parse_image_size_decimal(raw: &str, start: usize) -> AvResult<(i32, usize)> {
    let bytes = raw.as_bytes();
    let mut pos = start;
    while matches!(bytes.get(pos), Some(byte) if byte.is_ascii_whitespace()) {
        pos += 1;
    }
    if matches!(bytes.get(pos), Some(b'+') | Some(b'-')) {
        pos += 1;
    }
    let digits_start = pos;
    while matches!(bytes.get(pos), Some(b'0'..=b'9')) {
        pos += 1;
    }
    if pos == digits_start {
        return Err(image_size_parse_error(raw));
    }

    raw[start..pos]
        .trim_start()
        .parse::<i32>()
        .map(|value| (value, pos))
        .map_err(|_| image_size_parse_error(raw))
}

const VIDEO_SIZE_ABBREVIATIONS: &[(&str, i32, i32)] = &[
    ("ntsc", 720, 480),
    ("pal", 720, 576),
    ("qntsc", 352, 240),
    ("qpal", 352, 288),
    ("sntsc", 640, 480),
    ("spal", 768, 576),
    ("film", 352, 240),
    ("ntsc-film", 352, 240),
    ("sqcif", 128, 96),
    ("qcif", 176, 144),
    ("cif", 352, 288),
    ("4cif", 704, 576),
    ("16cif", 1408, 1152),
    ("qqvga", 160, 120),
    ("qvga", 320, 240),
    ("vga", 640, 480),
    ("svga", 800, 600),
    ("xga", 1024, 768),
    ("uxga", 1600, 1200),
    ("qxga", 2048, 1536),
    ("sxga", 1280, 1024),
    ("qsxga", 2560, 2048),
    ("hsxga", 5120, 4096),
    ("wvga", 852, 480),
    ("wxga", 1366, 768),
    ("wsxga", 1600, 1024),
    ("wuxga", 1920, 1200),
    ("woxga", 2560, 1600),
    ("wqhd", 2560, 1440),
    ("wqsxga", 3200, 2048),
    ("wquxga", 3840, 2400),
    ("whsxga", 6400, 4096),
    ("whuxga", 7680, 4800),
    ("cga", 320, 200),
    ("ega", 640, 350),
    ("hd480", 852, 480),
    ("hd720", 1280, 720),
    ("hd1080", 1920, 1080),
    ("quadhd", 2560, 1440),
    ("2k", 2048, 1080),
    ("2kdci", 2048, 1080),
    ("2kflat", 1998, 1080),
    ("2kscope", 2048, 858),
    ("4k", 4096, 2160),
    ("4kdci", 4096, 2160),
    ("4kflat", 3996, 2160),
    ("4kscope", 4096, 1716),
    ("nhd", 640, 360),
    ("hqvga", 240, 160),
    ("wqvga", 400, 240),
    ("fwqvga", 432, 240),
    ("hvga", 480, 320),
    ("qhd", 960, 540),
    ("uhd2160", 3840, 2160),
    ("uhd4320", 7680, 4320),
];

const VIDEO_RATE_ABBREVIATIONS: &[(&str, Rational)] = &[
    ("ntsc", Rational::from_raw(30000, 1001)),
    ("pal", Rational::from_raw(25, 1)),
    ("qntsc", Rational::from_raw(30000, 1001)),
    ("qpal", Rational::from_raw(25, 1)),
    ("sntsc", Rational::from_raw(30000, 1001)),
    ("spal", Rational::from_raw(25, 1)),
    ("film", Rational::from_raw(24, 1)),
    ("ntsc-film", Rational::from_raw(24000, 1001)),
];

fn image_size_parse_error(raw: &str) -> AvError {
    AvError::invalid_argument(format!("invalid image size option value `{raw}`"))
}

fn parse_video_rate(raw: &str) -> AvResult<Rational> {
    for (name, rate) in VIDEO_RATE_ABBREVIATIONS {
        if raw == *name {
            return Ok(*rate);
        }
    }

    let mut rate = parse_video_rate_with_max(raw, 1_001_000)?;
    if rate.num() == 0 || rate.den() == 0 {
        rate = parse_video_rate_with_max(raw, i32::MAX)?;
    }
    if rate.num() <= 0 || rate.den() <= 0 {
        return Err(video_rate_parse_error(raw));
    }
    Ok(rate)
}

fn parse_video_rate_with_max(raw: &str, max: i32) -> AvResult<Rational> {
    if let Some((num, den)) = parse_video_rate_colon_pair(raw) {
        let (rate, _) = Rational::reduce_i64(i64::from(num), i64::from(den), max)
            .map_err(|_| video_rate_parse_error(raw))?;
        return Ok(rate);
    }

    let value = parse_avoption_numeric_expression(raw, &[])?;
    Rational::from_f64_limited(value, max).map_err(|_| video_rate_parse_error(raw))
}

fn parse_video_rate_colon_pair(raw: &str) -> Option<(i32, i32)> {
    let raw = raw.trim_start();
    let (num, pos) = parse_signed_decimal_i32(raw, 0)?;
    if raw.as_bytes().get(pos).copied()? != b':' {
        return None;
    }
    let (den, pos) = parse_signed_decimal_i32(raw, pos + 1)?;
    if pos == raw.len() {
        Some((num, den))
    } else {
        None
    }
}

fn video_rate_parse_error(raw: &str) -> AvError {
    AvError::with_code(
        AvErrorKind::InvalidArgument,
        AvErrorCode::EINVAL,
        format!("invalid video rate option value `{raw}`"),
    )
}

fn parse_duration(raw: &str) -> AvResult<i64> {
    const USECS_PER_SEC: i128 = 1_000_000;

    if raw.is_empty() {
        return Err(duration_parse_error(raw));
    }

    let (negative, rest) = if let Some(rest) = raw.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = raw.strip_prefix('+') {
        (false, rest)
    } else {
        (false, raw)
    };
    if rest.is_empty() {
        return Err(duration_parse_error(raw));
    }

    let parts = rest.split(':').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(duration_parse_error(raw));
    }

    let (seconds, micros, suffix) = match parts.as_slice() {
        [seconds] => parse_duration_seconds(seconds, raw)?,
        [minutes, seconds] => {
            let minutes = parse_duration_component(minutes, raw)?;
            let (seconds, micros, suffix) = parse_duration_seconds(seconds, raw)?;
            let seconds = minutes
                .checked_mul(60)
                .and_then(|minutes| minutes.checked_add(seconds))
                .ok_or_else(|| duration_range_error(raw))?;
            (seconds, micros, suffix)
        }
        [hours, minutes, seconds] => {
            let hours = parse_duration_component(hours, raw)?;
            let minutes = parse_duration_component(minutes, raw)?;
            let (seconds, micros, suffix) = parse_duration_seconds(seconds, raw)?;
            let seconds = hours
                .checked_mul(3600)
                .and_then(|hours| minutes.checked_mul(60).and_then(|m| hours.checked_add(m)))
                .and_then(|total| total.checked_add(seconds))
                .ok_or_else(|| duration_range_error(raw))?;
            (seconds, micros, suffix)
        }
        _ => unreachable!("duration part count checked above"),
    };

    let (scale, micros) = match suffix {
        "" | "s" => (USECS_PER_SEC, micros),
        "ms" => (1_000, micros / 1_000),
        "us" => (1, 0),
        _ => return Err(duration_parse_error(raw)),
    };

    let total = seconds
        .checked_mul(scale)
        .and_then(|seconds| seconds.checked_add(micros))
        .ok_or_else(|| duration_range_error(raw))?;
    let signed = if negative {
        total
            .checked_neg()
            .ok_or_else(|| duration_range_error(raw))?
    } else {
        total
    };
    i64::try_from(signed).map_err(|_| duration_range_error(raw))
}

fn parse_duration_component(part: &str, raw: &str) -> AvResult<i128> {
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(duration_parse_error(raw));
    }
    parse_duration_decimal(part, raw)
}

fn parse_duration_seconds<'a>(part: &'a str, raw: &str) -> AvResult<(i128, i128, &'a str)> {
    let bytes = part.as_bytes();
    let mut pos = 0usize;
    while matches!(bytes.get(pos), Some(b'0'..=b'9')) {
        pos += 1;
    }
    if pos == 0 {
        return Err(duration_parse_error(raw));
    }
    let seconds = parse_duration_decimal(&part[..pos], raw)?;

    let mut micros = 0i128;
    if matches!(bytes.get(pos), Some(b'.')) {
        pos += 1;
        let mut digits = 0usize;
        while let Some(byte @ b'0'..=b'9') = bytes.get(pos).copied() {
            if digits < 6 {
                micros = micros
                    .checked_mul(10)
                    .and_then(|value| value.checked_add(i128::from(byte - b'0')))
                    .ok_or_else(|| duration_range_error(raw))?;
                digits += 1;
            }
            pos += 1;
        }
        while digits < 6 {
            micros = micros
                .checked_mul(10)
                .ok_or_else(|| duration_range_error(raw))?;
            digits += 1;
        }
    }

    Ok((seconds, micros, &part[pos..]))
}

fn parse_duration_decimal(digits: &str, raw: &str) -> AvResult<i128> {
    let mut value = 0i128;
    for byte in digits.bytes() {
        if !byte.is_ascii_digit() {
            return Err(duration_parse_error(raw));
        }
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(i128::from(byte - b'0')))
            .ok_or_else(|| duration_range_error(raw))?;
    }
    Ok(value)
}

fn duration_parse_error(raw: &str) -> AvError {
    AvError::with_code(
        AvErrorKind::InvalidArgument,
        AvErrorCode::EINVAL,
        format!("invalid duration option value `{raw}`"),
    )
}

fn duration_range_error(raw: &str) -> AvError {
    AvError::with_code(
        AvErrorKind::InvalidArgument,
        AvErrorCode::from_posix_errno(34),
        format!("duration option value `{raw}` out of range"),
    )
}

fn format_duration(value: i64) -> String {
    if value == i64::MAX {
        return "INT64_MAX".to_owned();
    }
    if value == i64::MIN {
        return "INT64_MIN".to_owned();
    }

    let mut duration = value;
    let mut output = String::new();
    if duration < 0 {
        output.push('-');
        duration = -duration;
    }

    let seconds = duration / 1_000_000;
    let micros = duration % 1_000_000;
    if duration > 3_600_000_000 {
        output.push_str(&format!(
            "{}:{:02}:{:02}.{:06}",
            seconds / 3600,
            (seconds / 60) % 60,
            seconds % 60,
            micros
        ));
    } else if duration > 60_000_000 {
        output.push_str(&format!(
            "{}:{:02}.{:06}",
            seconds / 60,
            seconds % 60,
            micros
        ));
    } else {
        output.push_str(&format!("{}.{:06}", seconds, micros));
    }

    while output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
    output
}

fn parse_rational(raw: &str) -> AvResult<Rational> {
    let (num, den) = if let Some((num, den)) = raw.split_once('/') {
        (
            parse_rational_part(num, raw)?,
            parse_rational_part(den, raw)?,
        )
    } else {
        (parse_rational_part(raw, raw)?, 1)
    };

    Rational::new(num, den)
        .map_err(|_| AvError::invalid_argument(format!("invalid rational option value `{raw}`")))
}

fn parse_rational_part(part: &str, raw: &str) -> AvResult<i32> {
    if part.is_empty() {
        return Err(AvError::invalid_argument(format!(
            "invalid rational option value `{raw}`"
        )));
    }

    part.parse::<i32>()
        .map_err(|_| AvError::invalid_argument(format!("invalid rational option value `{raw}`")))
}

struct ParsedAvOptionStringPair<'a> {
    key: Option<String>,
    value: String,
    rest: &'a str,
}

fn parse_avoption_string_pair<'a>(
    opts: &'a str,
    key_val_sep: &str,
    pairs_sep: &str,
    implicit_key_allowed: bool,
) -> AvResult<ParsedAvOptionStringPair<'a>> {
    if let Some((key, value_start)) = parse_avoption_string_key(opts, key_val_sep) {
        let (value, rest) = parse_avoption_token(value_start, pairs_sep);
        return Ok(ParsedAvOptionStringPair {
            key: Some(key.to_owned()),
            value,
            rest,
        });
    }

    if !implicit_key_allowed {
        return Err(invalid_avoption_string(opts));
    }

    let (value, rest) = parse_avoption_token(opts, pairs_sep);
    Ok(ParsedAvOptionStringPair {
        key: None,
        value,
        rest,
    })
}

fn parse_avoption_string_key<'a>(opts: &'a str, key_val_sep: &str) -> Option<(&'a str, &'a str)> {
    let key_start = skip_ffmpeg_token_whitespace(opts);
    let mut key_end = key_start.len();
    for (index, ch) in key_start.char_indices() {
        if !is_avoption_string_key_char(ch) {
            key_end = index;
            break;
        }
        key_end = index + ch.len_utf8();
    }

    let (key, after_key) = key_start.split_at(key_end);
    let after_key = skip_ffmpeg_token_whitespace(after_key);
    let separator = after_key.chars().next()?;
    if !key_val_sep.contains(separator) {
        return None;
    }

    Some((key, &after_key[separator.len_utf8()..]))
}

fn parse_avoption_token<'a>(opts: &'a str, terms: &str) -> (String, &'a str) {
    let mut rest = skip_ffmpeg_token_whitespace(opts);
    let mut output = String::new();
    let mut protected_len = 0usize;

    while let Some(ch) = rest.chars().next() {
        if terms.contains(ch) {
            break;
        }

        rest = &rest[ch.len_utf8()..];
        if ch == '\\' {
            if let Some(escaped) = rest.chars().next() {
                output.push(escaped);
                rest = &rest[escaped.len_utf8()..];
                protected_len = output.len();
            } else {
                output.push(ch);
            }
        } else if ch == '\'' {
            let mut closed = false;
            while let Some(quoted) = rest.chars().next() {
                rest = &rest[quoted.len_utf8()..];
                if quoted == '\'' {
                    protected_len = output.len();
                    closed = true;
                    break;
                }
                output.push(quoted);
            }
            if !closed {
                break;
            }
        } else {
            output.push(ch);
        }
    }

    while output.len() > protected_len
        && output
            .chars()
            .last()
            .is_some_and(is_ffmpeg_token_whitespace)
    {
        output.pop();
    }

    (output, rest)
}

fn skip_ffmpeg_token_whitespace(text: &str) -> &str {
    text.trim_start_matches(is_ffmpeg_token_whitespace)
}

fn is_ffmpeg_token_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\n' | '\t' | '\r')
}

fn validate_avoption_string_separators(key_val_sep: &str, pairs_sep: &str) -> AvResult<()> {
    if key_val_sep.is_empty()
        || pairs_sep.is_empty()
        || key_val_sep
            .chars()
            .any(|ch| ch == '\0' || pairs_sep.contains(ch) || is_avoption_string_key_char(ch))
        || pairs_sep
            .chars()
            .any(|ch| ch == '\0' || is_avoption_string_key_char(ch))
    {
        return Err(AvError::invalid_argument(
            "invalid AVOption string separators",
        ));
    }
    Ok(())
}

fn validate_avoption_serialize_separators(key_val_sep: char, pairs_sep: char) -> AvResult<()> {
    if key_val_sep == '\0'
        || pairs_sep == '\0'
        || key_val_sep == pairs_sep
        || key_val_sep == '\\'
        || pairs_sep == '\\'
    {
        return Err(AvError::with_code(
            AvErrorKind::InvalidArgument,
            AvErrorCode::EINVAL,
            "invalid AVOption serialize separators",
        ));
    }
    Ok(())
}

fn is_valid_avoption_string_key(key: &str) -> bool {
    !key.is_empty() && key.chars().all(is_avoption_string_key_char)
}

fn is_avoption_string_key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.' | '/' | '_')
}

fn invalid_avoption_string(text: &str) -> AvError {
    AvError::invalid_argument(format!("invalid AVOption string near `{text}`"))
}

fn format_avoption_value_for_kind(kind: &OptionKind, value: &OptionValue) -> String {
    if let (OptionKind::Array(array), OptionValue::Array(values)) = (kind, value) {
        return format_avoption_array(values, array.separator());
    }
    format_avoption_value(value)
}

fn format_avoption_array(values: &[OptionValue], separator: char) -> String {
    let mut output = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(separator);
        }
        push_avoption_array_escaped(&mut output, &format_avoption_value(value), separator);
    }
    output
}

fn push_avoption_array_escaped(output: &mut String, value: &str, separator: char) {
    for ch in value.chars() {
        if ch == '\\' || ch == separator {
            output.push('\\');
        }
        output.push(ch);
    }
}

fn format_avoption_value(value: &OptionValue) -> String {
    match value {
        OptionValue::Bool(false) => "false".to_owned(),
        OptionValue::Bool(true) => "true".to_owned(),
        OptionValue::Int(value) => value.to_string(),
        OptionValue::Duration(value) => format_duration(*value),
        OptionValue::ImageSize { width, height } => format!("{width}x{height}"),
        OptionValue::PixelFormat(None) => "none".to_owned(),
        OptionValue::PixelFormat(Some(value)) => value.name().to_owned(),
        OptionValue::SampleFormat(None) => "none".to_owned(),
        OptionValue::SampleFormat(Some(value)) => value.name().to_owned(),
        OptionValue::ChannelLayout(value) => value.describe(),
        OptionValue::VideoRate(value) => format!("{}/{}", value.num(), value.den()),
        OptionValue::Color(value) => {
            let rgba = value.rgba();
            format!(
                "0x{:02x}{:02x}{:02x}{:02x}",
                rgba[0], rgba[1], rgba[2], rgba[3]
            )
        }
        OptionValue::Binary(value) => {
            let mut formatted = String::with_capacity(value.len() * 2);
            for byte in value {
                use std::fmt::Write as _;
                let _ = write!(&mut formatted, "{byte:02X}");
            }
            formatted
        }
        OptionValue::Dictionary(value) => value
            .to_pairs_string('=', ':')
            .expect("dictionary AVOption values use valid separators"),
        OptionValue::Array(values) => format_avoption_array(values, ','),
        OptionValue::Float(value) => format!("{value:.6}"),
        OptionValue::Rational(value) => format!("{}/{}", value.num(), value.den()),
        OptionValue::String(value) => value.clone(),
    }
}

fn definition_matches_serialize_flags(
    definition: &OptionDefinition,
    opt_flags: OptionFlags,
    serialize_flags: OptionSerializeFlags,
) -> bool {
    if serialize_flags.contains(OptionSerializeFlags::OPT_FLAGS_EXACT) {
        definition.flags() == opt_flags
    } else {
        definition.flags().contains(opt_flags)
    }
}

fn push_avoption_serialize_escaped(
    output: &mut String,
    value: &str,
    key_val_sep: char,
    pairs_sep: char,
) {
    for ch in value.chars() {
        if ch == '\\' || ch == key_val_sep || ch == pairs_sep {
            output.push('\\');
        }
        output.push(ch);
    }
}

fn ascii_eq_ignore_case(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .as_bytes()
            .iter()
            .zip(right.as_bytes())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_validate_ranges_names_and_defaults() {
        assert!(OptionDefinition::new("", OptionKind::Bool, OptionValue::Bool(false), "").is_err());
        assert!(
            OptionDefinition::new("bad\0name", OptionKind::Bool, OptionValue::Bool(false), "")
                .is_err()
        );
        assert!(OptionDefinition::new(
            "threads",
            OptionKind::Int { min: 8, max: 1 },
            OptionValue::Int(1),
            ""
        )
        .is_err());
        assert!(OptionDefinition::new(
            "quality",
            OptionKind::Float {
                min: f64::NAN,
                max: 1.0
            },
            OptionValue::Float(0.5),
            ""
        )
        .is_err());
        assert!(OptionDefinition::new(
            "threads",
            OptionKind::Int { min: 1, max: 8 },
            OptionValue::Int(0),
            ""
        )
        .is_err());
        assert!(OptionDefinition::new(
            "aspect_ratio",
            OptionKind::Rational {
                min: Rational::ONE,
                max: Rational::ZERO,
            },
            OptionValue::Rational(Rational::ONE),
            ""
        )
        .is_err());
        assert!(OptionDefinition::new(
            "aspect_ratio",
            OptionKind::Rational {
                min: Rational::ZERO,
                max: Rational::new(16, 9).unwrap(),
            },
            OptionValue::Rational(Rational::from_raw(1, 0)),
            ""
        )
        .is_err());
    }

    #[test]
    fn option_flags_match_ffmpeg_bits_and_truncate_unknown_bits() {
        assert_eq!(OptionFlags::ENCODING_PARAM.bits(), 1 << 0);
        assert_eq!(OptionFlags::DECODING_PARAM.bits(), 1 << 1);
        assert_eq!(OptionFlags::AUDIO_PARAM.bits(), 1 << 3);
        assert_eq!(OptionFlags::VIDEO_PARAM.bits(), 1 << 4);
        assert_eq!(OptionFlags::SUBTITLE_PARAM.bits(), 1 << 5);
        assert_eq!(OptionFlags::EXPORT.bits(), 1 << 6);
        assert_eq!(OptionFlags::READONLY.bits(), 1 << 7);
        assert_eq!(OptionFlags::BSF_PARAM.bits(), 1 << 8);
        assert_eq!(OptionFlags::RUNTIME_PARAM.bits(), 1 << 15);
        assert_eq!(OptionFlags::FILTERING_PARAM.bits(), 1 << 16);
        assert_eq!(OptionFlags::DEPRECATED.bits(), 1 << 17);
        assert_eq!(OptionFlags::CHILD_CONSTS.bits(), 1 << 18);
        assert!(OptionFlags::empty().is_empty());

        let truncated = OptionFlags::from_bits_truncate(u32::MAX);

        assert_eq!(truncated, OptionFlags::all());
        assert_eq!(truncated.bits() & !OptionFlags::all().bits(), 0);
        assert!(truncated.contains(OptionFlags::ENCODING_PARAM));
        assert!(truncated.contains(OptionFlags::CHILD_CONSTS));
        assert!(truncated.intersects(OptionFlags::EXPORT));
        assert!(!OptionFlags::VIDEO_PARAM.intersects(OptionFlags::AUDIO_PARAM));
    }

    #[test]
    fn option_search_flags_match_ffmpeg_bits_and_truncate_unknown_bits() {
        assert_eq!(OptionSearchFlags::CHILDREN.bits(), 1 << 0);
        assert_eq!(OptionSearchFlags::FAKE_OBJ.bits(), 1 << 1);
        assert_eq!(OptionSearchFlags::ARRAY_REPLACE.bits(), 1 << 3);

        let truncated = OptionSearchFlags::from_bits_truncate(u32::MAX);

        assert!(truncated.contains(OptionSearchFlags::CHILDREN));
        assert!(truncated.contains(OptionSearchFlags::FAKE_OBJ));
        assert!(truncated.contains(OptionSearchFlags::ARRAY_REPLACE));
        assert!(truncated.intersects(OptionSearchFlags::CHILDREN));
        assert_eq!(
            truncated.bits(),
            OptionSearchFlags::CHILDREN.bits()
                | OptionSearchFlags::FAKE_OBJ.bits()
                | OptionSearchFlags::ARRAY_REPLACE.bits()
        );
        assert_eq!(OptionSearchFlags::empty().bits(), 0);
    }

    #[test]
    fn option_serialize_flags_match_ffmpeg_bits_and_truncate_unknown_bits() {
        assert_eq!(OptionSerializeFlags::SKIP_DEFAULTS.bits(), 1 << 0);
        assert_eq!(OptionSerializeFlags::OPT_FLAGS_EXACT.bits(), 1 << 1);
        assert_eq!(OptionSerializeFlags::SEARCH_CHILDREN.bits(), 1 << 2);

        let truncated = OptionSerializeFlags::from_bits_truncate(u32::MAX);

        assert_eq!(truncated, OptionSerializeFlags::all());
        assert!(truncated.contains(OptionSerializeFlags::SKIP_DEFAULTS));
        assert!(truncated.contains(OptionSerializeFlags::OPT_FLAGS_EXACT));
        assert!(truncated.contains(OptionSerializeFlags::SEARCH_CHILDREN));
        assert!(truncated.intersects(OptionSerializeFlags::SEARCH_CHILDREN));
        assert_eq!(OptionSerializeFlags::empty().bits(), 0);
    }

    #[test]
    fn definitions_store_flags_with_option_metadata() {
        let flags = OptionFlags::from_bits_truncate(
            OptionFlags::ENCODING_PARAM.bits() | OptionFlags::VIDEO_PARAM.bits(),
        );
        let definition = OptionDefinition::new_with_flags(
            "profile",
            OptionKind::String { allow_empty: false },
            OptionValue::String("main".to_owned()),
            "encoding profile",
            flags,
        )
        .unwrap();

        assert_eq!(definition.flags(), flags);
        assert!(definition.flags().contains(OptionFlags::ENCODING_PARAM));
        assert!(definition.flags().contains(OptionFlags::VIDEO_PARAM));
        assert!(!definition.flags().contains(OptionFlags::DECODING_PARAM));
    }

    #[test]
    fn definitions_store_units_for_named_constants() {
        let definition = OptionDefinition::new_with_flags_and_unit(
            "preset_level",
            OptionKind::Int { min: 0, max: 10 },
            OptionValue::Int(0),
            "preset level",
            OptionFlags::ENCODING_PARAM,
            Some("preset"),
        )
        .unwrap();

        assert_eq!(definition.unit(), Some("preset"));
        assert!(definition.flags().contains(OptionFlags::ENCODING_PARAM));
        assert!(OptionDefinition::new_with_unit(
            "bad",
            OptionKind::Int { min: 0, max: 1 },
            OptionValue::Int(0),
            "",
            "",
        )
        .is_err());
        assert!(OptionDefinition::new_with_unit(
            "bad",
            OptionKind::Int { min: 0, max: 1 },
            OptionValue::Int(0),
            "",
            "bad\0unit",
        )
        .is_err());
    }

    #[test]
    fn option_ranges_validate_and_expose_numeric_bounds() {
        assert!(OptionRange::new(OptionValue::Int(8), OptionValue::Int(1)).is_err());
        assert!(OptionRange::new(OptionValue::Duration(8), OptionValue::Duration(1)).is_err());
        assert!(OptionRange::new(OptionValue::Float(f64::NAN), OptionValue::Float(1.0)).is_err());
        assert!(OptionRange::new(
            OptionValue::Rational(Rational::ONE),
            OptionValue::Rational(Rational::ZERO)
        )
        .is_err());
        assert!(OptionRange::new(
            OptionValue::Rational(Rational::from_raw(1, 0)),
            OptionValue::Rational(Rational::ONE)
        )
        .is_err());
        assert!(OptionRange::new(OptionValue::Bool(false), OptionValue::Bool(true)).is_err());

        let options = sample_options();
        let threads = options.range("threads").unwrap().unwrap();
        let quality = options.range("quality").unwrap().unwrap();
        let aspect = options.range("aspect_ratio").unwrap().unwrap();

        assert_eq!(threads.min(), &OptionValue::Int(1));
        assert_eq!(threads.max(), &OptionValue::Int(64));
        assert_eq!(quality.min(), &OptionValue::Float(0.0));
        assert_eq!(quality.max(), &OptionValue::Float(1.0));
        assert_eq!(aspect.min(), &OptionValue::Rational(Rational::ONE));
        assert_eq!(
            aspect.max(),
            &OptionValue::Rational(Rational::new(16, 9).unwrap())
        );
        assert_eq!(options.range("bitexact").unwrap(), None);
        assert_eq!(options.range("metadata").unwrap(), None);
        assert_eq!(
            options.range("missing").unwrap_err().kind(),
            AvErrorKind::NotFound
        );
    }

    #[test]
    fn duration_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "timeout",
                    OptionKind::Duration {
                        min: 0,
                        max: 7_200_000_000,
                    },
                    OptionValue::Duration(0),
                    "timeout",
                )
                .unwrap(),
            )
            .unwrap();

        let range = options.range("timeout").unwrap().unwrap();
        assert_eq!(range.min(), &OptionValue::Duration(0));
        assert_eq!(range.max(), &OptionValue::Duration(7_200_000_000));
        let av_ranges = options.query_avoption_ranges("timeout").unwrap();
        assert_eq!(av_ranges.nb_ranges(), 1);
        assert_eq!(av_ranges.ranges()[0].value_min(), 0.0);
        assert_eq!(av_ranges.ranges()[0].value_max(), 7_200_000_000.0);
        assert_eq!(options.get_avoption_string("timeout").unwrap(), "0");

        options.set_avoption_from_str("timeout", "1.5").unwrap();
        assert_eq!(
            options.get("timeout"),
            Some(&OptionValue::Duration(1_500_000))
        );
        assert_eq!(options.get_avoption_string("timeout").unwrap(), "1.5");

        options
            .set_avoption_from_str("timeout", "00:01:02.250")
            .unwrap();
        assert_eq!(
            options.get("timeout"),
            Some(&OptionValue::Duration(62_250_000))
        );
        assert_eq!(options.get_avoption_string("timeout").unwrap(), "1:02.25");

        options.set_avoption_from_str("timeout", "1500ms").unwrap();
        assert_eq!(
            options.get("timeout"),
            Some(&OptionValue::Duration(1_500_000))
        );
        assert_eq!(options.get_avoption_string("timeout").unwrap(), "1.5");

        options.set_avoption_from_str("timeout", "42us").unwrap();
        assert_eq!(options.get("timeout"), Some(&OptionValue::Duration(42)));
        assert_eq!(options.get_avoption_string("timeout").unwrap(), "0.000042");

        options.set_avoption_int("timeout", 90_500_000).unwrap();
        assert_eq!(options.get_avoption_int("timeout").unwrap(), 90_500_000);
        assert_eq!(
            options.get_avoption_double("timeout").unwrap(),
            90_500_000.0
        );
        assert_eq!(
            options.get_avoption_q("timeout").unwrap(),
            Rational::new(90_500_000, 1).unwrap()
        );
        assert_eq!(options.get_avoption_string("timeout").unwrap(), "1:30.5");

        let before_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("timeout", "bad")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options
                .set_avoption_from_str("timeout", "-1")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(options, before_errors);
    }

    #[test]
    fn image_size_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "size",
                    OptionKind::ImageSize,
                    OptionValue::ImageSize {
                        width: 320,
                        height: 240,
                    },
                    "image size",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("size").unwrap(), None);
        let av_ranges = options.query_avoption_ranges("size").unwrap();
        assert_eq!(av_ranges.nb_ranges(), 1);
        assert_eq!(av_ranges.ranges()[0].value_min(), 0.0);
        assert_eq!(av_ranges.ranges()[0].value_max(), f64::from(i32::MAX / 8));
        assert_eq!(av_ranges.ranges()[0].component_min(), 0.0);
        assert_eq!(
            av_ranges.ranges()[0].component_max(),
            f64::from(i32::MAX / 128 / 8)
        );
        assert_eq!(options.get_avoption_string("size").unwrap(), "320x240");

        options.set_avoption_from_str("size", "640x480").unwrap();
        assert_eq!(
            options.get("size"),
            Some(&OptionValue::ImageSize {
                width: 640,
                height: 480
            })
        );
        assert_eq!(options.get_avoption_image_size("size").unwrap(), (640, 480));
        assert_eq!(options.get_avoption_string("size").unwrap(), "640x480");

        options.set_avoption_from_str("size", "hd720").unwrap();
        assert_eq!(
            options.get("size"),
            Some(&OptionValue::ImageSize {
                width: 1280,
                height: 720
            })
        );
        assert_eq!(options.get_avoption_string("size").unwrap(), "1280x720");

        options.set_avoption_from_str("size", "none").unwrap();
        assert_eq!(
            options.get("size"),
            Some(&OptionValue::ImageSize {
                width: 0,
                height: 0
            })
        );
        assert_eq!(options.get_avoption_string("size").unwrap(), "0x0");

        options.set_avoption_image_size("size", 800, 600).unwrap();
        assert_eq!(options.get_avoption_image_size("size").unwrap(), (800, 600));
        assert_eq!(options.get_avoption_string("size").unwrap(), "800x600");

        let before_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("size", "bad")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options
                .set_avoption_from_str("size", "0x480")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options
                .set_avoption_image_size("size", -1, 480)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options.set_avoption_int("size", 10).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options.get_avoption_int("size").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
    }

    #[test]
    fn pixel_format_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "pix_fmt",
                    OptionKind::PixelFormat { min: -1, max: 24 },
                    OptionValue::PixelFormat(Some(PixelFormat::Yuv420p)),
                    "pixel format",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "scalar",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(4),
                    "scalar",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("pix_fmt").unwrap(), None);
        let av_ranges = options.query_avoption_ranges("pix_fmt").unwrap();
        assert_eq!(av_ranges.nb_ranges(), 1);
        assert_eq!(av_ranges.ranges()[0].value_min(), -1.0);
        assert_eq!(av_ranges.ranges()[0].value_max(), 24.0);
        assert_eq!(av_ranges.ranges()[0].component_min(), 0.0);
        assert_eq!(av_ranges.ranges()[0].component_max(), 0.0);
        assert_eq!(
            options.get_avoption_pixel_format("pix_fmt").unwrap(),
            Some(PixelFormat::Yuv420p)
        );
        assert_eq!(options.get_avoption_string("pix_fmt").unwrap(), "yuv420p");
        assert_eq!(options.get_avoption_int("pix_fmt").unwrap(), 0);

        options.set_avoption_from_str("pix_fmt", "rgb24").unwrap();
        assert_eq!(
            options.get("pix_fmt"),
            Some(&OptionValue::PixelFormat(Some(PixelFormat::Rgb24)))
        );
        assert_eq!(options.get_avoption_int("pix_fmt").unwrap(), 2);

        options.set_avoption_from_str("pix_fmt", "gray").unwrap();
        assert_eq!(
            options.get("pix_fmt"),
            Some(&OptionValue::PixelFormat(Some(PixelFormat::Gray8)))
        );
        assert_eq!(options.get_avoption_string("pix_fmt").unwrap(), "gray");

        options.set_avoption_from_str("pix_fmt", "none").unwrap();
        assert_eq!(
            options.get("pix_fmt"),
            Some(&OptionValue::PixelFormat(None))
        );
        assert_eq!(options.get_avoption_string("pix_fmt").unwrap(), "none");
        assert_eq!(options.get_avoption_int("pix_fmt").unwrap(), -1);

        options.set_avoption_from_str("pix_fmt", "0x3").unwrap();
        assert_eq!(
            options.get("pix_fmt"),
            Some(&OptionValue::PixelFormat(Some(PixelFormat::Bgr24)))
        );
        assert_eq!(options.get_avoption_string("pix_fmt").unwrap(), "bgr24");

        let before_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("pix_fmt", "bad")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options
                .set_avoption_from_str("pix_fmt", "25")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(options, before_errors);

        options
            .set_avoption_pixel_format("pix_fmt", Some(PixelFormat::Rgb24))
            .unwrap();
        assert_eq!(
            options.get_avoption_pixel_format("pix_fmt").unwrap(),
            Some(PixelFormat::Rgb24)
        );
        options.set_avoption_pixel_format("pix_fmt", None).unwrap();
        assert_eq!(options.get_avoption_pixel_format("pix_fmt").unwrap(), None);
        options.set_avoption_int("pix_fmt", 3).unwrap();
        assert_eq!(
            options.get_avoption_pixel_format("pix_fmt").unwrap(),
            Some(PixelFormat::Bgr24)
        );
        assert_eq!(options.get_avoption_double("pix_fmt").unwrap(), 3.0);
        assert_eq!(
            options.get_avoption_q("pix_fmt").unwrap(),
            Rational::new(3, 1).unwrap()
        );
        let before_typed_errors = options.clone();
        assert_eq!(
            options.set_avoption_int("pix_fmt", 25).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(options, before_typed_errors);
        assert_eq!(
            options
                .set_avoption_pixel_format("scalar", Some(PixelFormat::Rgb24))
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .get_avoption_pixel_format("scalar")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_typed_errors);
    }

    #[test]
    fn sample_format_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "sample_fmt",
                    OptionKind::SampleFormat { min: -1, max: 11 },
                    OptionValue::SampleFormat(Some(SampleFormat::S16)),
                    "sample format",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "scalar",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(4),
                    "scalar",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("sample_fmt").unwrap(), None);
        let av_ranges = options.query_avoption_ranges("sample_fmt").unwrap();
        assert_eq!(av_ranges.nb_ranges(), 1);
        assert_eq!(av_ranges.ranges()[0].value_min(), -1.0);
        assert_eq!(av_ranges.ranges()[0].value_max(), 11.0);
        assert_eq!(av_ranges.ranges()[0].component_min(), 0.0);
        assert_eq!(av_ranges.ranges()[0].component_max(), 0.0);
        assert_eq!(
            options.get_avoption_sample_format("sample_fmt").unwrap(),
            Some(SampleFormat::S16)
        );
        assert_eq!(options.get_avoption_string("sample_fmt").unwrap(), "s16");
        assert_eq!(options.get_avoption_int("sample_fmt").unwrap(), 1);

        options.set_avoption_from_str("sample_fmt", "fltp").unwrap();
        assert_eq!(
            options.get("sample_fmt"),
            Some(&OptionValue::SampleFormat(Some(SampleFormat::FltP)))
        );
        assert_eq!(options.get_avoption_int("sample_fmt").unwrap(), 8);

        options.set_avoption_from_str("sample_fmt", "none").unwrap();
        assert_eq!(
            options.get("sample_fmt"),
            Some(&OptionValue::SampleFormat(None))
        );
        assert_eq!(options.get_avoption_string("sample_fmt").unwrap(), "none");
        assert_eq!(options.get_avoption_int("sample_fmt").unwrap(), -1);

        options.set_avoption_from_str("sample_fmt", "0x4").unwrap();
        assert_eq!(
            options.get("sample_fmt"),
            Some(&OptionValue::SampleFormat(Some(SampleFormat::Dbl)))
        );
        assert_eq!(options.get_avoption_string("sample_fmt").unwrap(), "dbl");

        let before_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("sample_fmt", "bad")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options
                .set_avoption_from_str("sample_fmt", "12")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);

        options
            .set_avoption_sample_format("sample_fmt", Some(SampleFormat::S32P))
            .unwrap();
        assert_eq!(
            options.get_avoption_sample_format("sample_fmt").unwrap(),
            Some(SampleFormat::S32P)
        );
        options
            .set_avoption_sample_format("sample_fmt", None)
            .unwrap();
        assert_eq!(
            options.get_avoption_sample_format("sample_fmt").unwrap(),
            None
        );
        options.set_avoption_int("sample_fmt", 10).unwrap();
        assert_eq!(
            options.get_avoption_sample_format("sample_fmt").unwrap(),
            Some(SampleFormat::S64)
        );
        assert_eq!(options.get_avoption_double("sample_fmt").unwrap(), 10.0);
        assert_eq!(
            options.get_avoption_q("sample_fmt").unwrap(),
            Rational::new(10, 1).unwrap()
        );
        assert_eq!(options.get_avoption_string("sample_fmt").unwrap(), "s64");

        let before_typed_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_int("sample_fmt", 12)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(options, before_typed_errors);
        assert_eq!(
            options
                .set_avoption_sample_format("scalar", Some(SampleFormat::S16))
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .get_avoption_sample_format("scalar")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_typed_errors);
    }

    #[test]
    fn channel_layout_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "layout",
                    OptionKind::ChannelLayout,
                    OptionValue::ChannelLayout(ChannelLayoutSpec::native(
                        crate::ChannelLayout::stereo(),
                    )),
                    "channel layout",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "scalar",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(4),
                    "scalar",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("layout").unwrap(), None);
        assert_eq!(
            options.query_avoption_ranges("layout").unwrap_err().code(),
            Some(AvErrorCode::ENOSYS)
        );
        assert_eq!(
            options.get_avoption_channel_layout("layout").unwrap(),
            ChannelLayoutSpec::native(crate::ChannelLayout::stereo())
        );
        assert_eq!(options.get_avoption_string("layout").unwrap(), "stereo");
        assert_eq!(
            options.get_avoption_int("layout").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );

        options.set_avoption_from_str("layout", "mono").unwrap();
        assert_eq!(
            options.get("layout"),
            Some(&OptionValue::ChannelLayout(ChannelLayoutSpec::native(
                crate::ChannelLayout::mono()
            )))
        );
        assert_eq!(options.get_avoption_string("layout").unwrap(), "mono");

        options.set_avoption_from_str("layout", "5.1").unwrap();
        assert_eq!(options.get_avoption_string("layout").unwrap(), "5.1");

        options.set_avoption_from_str("layout", "2C").unwrap();
        assert_eq!(
            options
                .get_avoption_channel_layout("layout")
                .unwrap()
                .describe(),
            "2 channels"
        );
        assert_eq!(options.get_avoption_string("layout").unwrap(), "2 channels");

        assert_eq!(
            options
                .set_avoption_from_str("layout", "bad")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options.get_avoption_string("layout").unwrap(), "0 channels");

        options
            .set_avoption_channel_layout(
                "layout",
                ChannelLayoutSpec::native(crate::ChannelLayout::mono()),
            )
            .unwrap();
        assert_eq!(options.get_avoption_string("layout").unwrap(), "mono");

        let before_typed_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_channel_layout(
                    "scalar",
                    ChannelLayoutSpec::native(crate::ChannelLayout::stereo()),
                )
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.set_avoption_int("layout", 2).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options.set_avoption_int("layout", 0).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_q("layout").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .get_avoption_channel_layout("scalar")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_typed_errors);
    }

    #[test]
    fn binary_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "blob",
                    OptionKind::Binary,
                    OptionValue::Binary(vec![0x00, 0x01, 0xAA, 0xFF]),
                    "binary data",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "scalar",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(4),
                    "scalar",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("blob").unwrap(), None);
        assert_eq!(
            options.query_avoption_ranges("blob").unwrap_err().code(),
            Some(AvErrorCode::ENOSYS)
        );
        assert_eq!(
            options.get("blob"),
            Some(&OptionValue::Binary(vec![0x00, 0x01, 0xAA, 0xFF]))
        );
        assert_eq!(
            options.get_avoption_binary("blob").unwrap(),
            [0x00, 0x01, 0xAA, 0xFF]
        );
        assert_eq!(options.get_avoption_string("blob").unwrap(), "0001AAFF");
        assert_eq!(
            options.get_avoption_int("blob").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );

        options.set_avoption_from_str("blob", "0f10Aa").unwrap();
        assert_eq!(
            options.get("blob"),
            Some(&OptionValue::Binary(vec![0x0F, 0x10, 0xAA]))
        );
        assert_eq!(options.get_avoption_string("blob").unwrap(), "0F10AA");

        options.set_avoption_from_str("blob", "").unwrap();
        assert_eq!(
            options.get_avoption_binary("blob").unwrap(),
            Vec::<u8>::new()
        );
        assert_eq!(options.get_avoption_string("blob").unwrap(), "");

        options.set_avoption_from_str("blob", "deAd").unwrap();
        assert_eq!(options.get_avoption_string("blob").unwrap(), "DEAD");
        assert_eq!(
            options
                .set_avoption_from_str("blob", "abc")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_binary("blob").unwrap(),
            Vec::<u8>::new()
        );
        assert_eq!(options.get_avoption_string("blob").unwrap(), "");

        options.set_avoption_from_str("blob", "beef").unwrap();
        assert_eq!(
            options
                .set_avoption_from_str("blob", "0g")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_binary("blob").unwrap(),
            Vec::<u8>::new()
        );

        options.set_avoption_binary("blob", &[0xDE, 0xAD]).unwrap();
        assert_eq!(options.get_avoption_string("blob").unwrap(), "DEAD");
        options.set_avoption_binary("blob", &[]).unwrap();
        assert_eq!(options.get_avoption_string("blob").unwrap(), "");

        let before_typed_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_binary("scalar", &[1])
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.set_avoption_int("blob", 2).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options.set_avoption_int("blob", 0).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_q("blob").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_binary("scalar").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_typed_errors);
    }

    #[test]
    fn dictionary_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut default_dict = Dictionary::new();
        default_dict.set("title", "clip").unwrap();
        default_dict.set("note", "hello:world").unwrap();

        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "dict",
                    OptionKind::Dictionary,
                    OptionValue::Dictionary(default_dict.clone()),
                    "dictionary data",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "scalar",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(4),
                    "scalar",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("dict").unwrap(), None);
        assert_eq!(
            options.get("dict"),
            Some(&OptionValue::Dictionary(default_dict))
        );
        assert_eq!(
            options.get_avoption_string("dict").unwrap(),
            "title=clip:note=hello\\:world"
        );
        assert_eq!(
            options.query_avoption_ranges("dict").unwrap_err().code(),
            Some(AvErrorCode::ENOSYS)
        );

        options
            .set_avoption_from_str("dict", "artist=rust:comment='a:b'")
            .unwrap();
        let parsed = options.get_avoption_dictionary("dict").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get("artist"), Some("rust"));
        assert_eq!(parsed.get("comment"), Some("a:b"));
        assert_eq!(
            options.get_avoption_string("dict").unwrap(),
            "artist=rust:comment=a\\:b"
        );

        options.set_avoption_from_str("dict", "").unwrap();
        assert!(options.get_avoption_dictionary("dict").unwrap().is_empty());
        assert_eq!(options.get_avoption_string("dict").unwrap(), "");

        options.set_avoption_from_str("dict", "key=value").unwrap();
        let before_parse_error = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("dict", "missing-separator")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_parse_error);

        let mut typed = Dictionary::new();
        typed.set("typed", "one").unwrap();
        typed.set("note", "two:three").unwrap();
        options.set_avoption_dictionary("dict", &typed).unwrap();
        assert_eq!(
            options.get_avoption_string("dict").unwrap(),
            "typed=one:note=two\\:three"
        );
        typed.set("typed", "mutated").unwrap();
        assert_eq!(
            options
                .get_avoption_dictionary("dict")
                .unwrap()
                .get("typed"),
            Some("one")
        );

        let empty = Dictionary::new();
        options.set_avoption_dictionary("dict", &empty).unwrap();
        assert_eq!(options.get_avoption_string("dict").unwrap(), "");

        let before_typed_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_dictionary("scalar", &typed)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.set_avoption_int("dict", 2).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options.set_avoption_int("dict", 0).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_q("dict").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .get_avoption_dictionary("scalar")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_typed_errors);
    }

    #[test]
    fn array_options_parse_format_and_mutate_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "ints",
                    OptionKind::array(OptionKind::Int { min: 0, max: 10 }, 0, Some(4), ',')
                        .unwrap(),
                    OptionValue::Array(vec![OptionValue::Int(1), OptionValue::Int(2)]),
                    "integer array",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "words",
                    OptionKind::array(OptionKind::String { allow_empty: true }, 0, Some(3), '|')
                        .unwrap(),
                    OptionValue::Array(vec![
                        OptionValue::String("alpha".to_owned()),
                        OptionValue::String("beta|gamma".to_owned()),
                    ]),
                    "string array",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "required",
                    OptionKind::array(OptionKind::Int { min: 0, max: 10 }, 2, Some(3), ',')
                        .unwrap(),
                    OptionValue::Array(vec![OptionValue::Int(3), OptionValue::Int(4)]),
                    "required integer array",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "scalar",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(4),
                    "scalar",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("ints").unwrap(), None);
        assert_eq!(options.get_avoption_array_size("ints").unwrap(), 2);
        assert_eq!(
            options.get_avoption_array("ints", 0, 2).unwrap(),
            vec![OptionValue::Int(1), OptionValue::Int(2)]
        );
        assert_eq!(options.get_avoption_string("ints").unwrap(), "1,2");
        assert_eq!(
            options.get_avoption_string("words").unwrap(),
            "alpha|beta\\|gamma"
        );
        assert_eq!(
            options.query_avoption_ranges("ints").unwrap_err().code(),
            Some(AvErrorCode::ENOSYS)
        );

        options.set_avoption_from_str("ints", "3,4").unwrap();
        assert_eq!(
            options.get("ints"),
            Some(&OptionValue::Array(vec![
                OptionValue::Int(3),
                OptionValue::Int(4)
            ]))
        );
        assert_eq!(options.get_avoption_string("ints").unwrap(), "3,4");

        options
            .set_avoption_from_str("words", "left|right\\|inner|slash\\\\tail")
            .unwrap();
        assert_eq!(
            options.get("words"),
            Some(&OptionValue::Array(vec![
                OptionValue::String("left".to_owned()),
                OptionValue::String("right|inner".to_owned()),
                OptionValue::String("slash\\tail".to_owned()),
            ]))
        );
        assert_eq!(
            options.get_avoption_string("words").unwrap(),
            "left|right\\|inner|slash\\\\tail"
        );

        let before_parse_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("ints", "7,11")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options
                .set_avoption_from_str("words", "a|b|c|d")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_parse_errors);

        assert_eq!(
            options
                .set_avoption_from_str("required", "9")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get("required"),
            Some(&OptionValue::Array(Vec::new()))
        );

        options
            .set_avoption_array(
                "ints",
                1,
                &[OptionValue::Int(8)],
                OptionSearchFlags::empty(),
            )
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "3,8,4");
        options
            .set_avoption_array(
                "ints",
                1,
                &[OptionValue::Int(5)],
                OptionSearchFlags::ARRAY_REPLACE,
            )
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "3,5,4");
        options
            .remove_avoption_array("ints", 0, 1, OptionSearchFlags::empty())
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "5,4");
        options
            .set_avoption_array(
                "ints",
                1,
                &[OptionValue::String("6".to_owned())],
                OptionSearchFlags::empty(),
            )
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "5,6,4");
        assert_eq!(
            options.get_avoption_array_strings("ints", 0, 3).unwrap(),
            vec!["5".to_owned(), "6".to_owned(), "4".to_owned()]
        );
        options
            .set_avoption_array(
                "ints",
                2,
                &[OptionValue::String("9".to_owned())],
                OptionSearchFlags::ARRAY_REPLACE,
            )
            .unwrap();
        options
            .remove_avoption_array("ints", 0, 1, OptionSearchFlags::empty())
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "6,9");
        options.set_avoption_from_str("ints", "3,4").unwrap();
        options
            .set_avoption_array(
                "ints",
                1,
                &[OptionValue::Float(6.0)],
                OptionSearchFlags::empty(),
            )
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "3,6,4");
        options
            .set_avoption_array(
                "ints",
                2,
                &[OptionValue::Rational(Rational::new(9, 1).unwrap())],
                OptionSearchFlags::ARRAY_REPLACE,
            )
            .unwrap();
        options
            .remove_avoption_array("ints", 0, 1, OptionSearchFlags::empty())
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "6,9");
        assert_eq!(
            options.get_avoption_array_doubles("ints", 0, 2).unwrap(),
            vec![6.0, 9.0]
        );
        assert_eq!(
            options.get_avoption_array_rationals("ints", 0, 2).unwrap(),
            vec![Rational::new(6, 1).unwrap(), Rational::new(9, 1).unwrap()]
        );
        assert_eq!(
            options.get_avoption_array("ints", 2, 0).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .get_avoption_array_strings("ints", 2, 0)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .get_avoption_array_doubles("ints", 2, 0)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .get_avoption_array_rationals("ints", 2, 0)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        options
            .set_avoption_array("ints", 2, &[], OptionSearchFlags::empty())
            .unwrap();
        options
            .set_avoption_array("ints", 2, &[], OptionSearchFlags::ARRAY_REPLACE)
            .unwrap();
        options
            .remove_avoption_array("ints", 2, 0, OptionSearchFlags::empty())
            .unwrap();
        assert_eq!(options.get_avoption_string("ints").unwrap(), "6,9");

        options
            .set_avoption_from_str("words", "left|right\\|inner")
            .unwrap();
        options
            .set_avoption_array(
                "words",
                1,
                &[OptionValue::String("middle|pipe".to_owned())],
                OptionSearchFlags::empty(),
            )
            .unwrap();
        assert_eq!(
            options.get_avoption_string("words").unwrap(),
            "left|middle\\|pipe|right\\|inner"
        );
        options
            .set_avoption_array(
                "words",
                2,
                &[OptionValue::String("tail\\slash".to_owned())],
                OptionSearchFlags::ARRAY_REPLACE,
            )
            .unwrap();
        assert_eq!(
            options.get_avoption_array("words", 1, 2).unwrap(),
            vec![
                OptionValue::String("middle|pipe".to_owned()),
                OptionValue::String("tail\\slash".to_owned())
            ]
        );
        options
            .remove_avoption_array("words", 0, 1, OptionSearchFlags::empty())
            .unwrap();
        assert_eq!(
            options.get_avoption_string("words").unwrap(),
            "middle\\|pipe|tail\\\\slash"
        );

        let before_typed_errors = options.clone();
        assert_eq!(
            options
                .get_avoption_array_size("scalar")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_array("ints", 2, 1).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_array("ints", 3, 0).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .set_avoption_array(
                    "ints",
                    0,
                    &[OptionValue::String("bad".to_owned())],
                    OptionSearchFlags::empty()
                )
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .set_avoption_array(
                    "ints",
                    0,
                    &[OptionValue::Rational(Rational::new(11, 1).unwrap())],
                    OptionSearchFlags::empty()
                )
                .unwrap_err()
                .code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options
                .remove_avoption_array("ints", 0, 3, OptionSearchFlags::empty())
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_typed_errors);
    }

    #[test]
    fn video_rate_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "rate",
                    OptionKind::VideoRate {
                        min: Rational::ONE,
                        max: Rational::new(120, 1).unwrap(),
                    },
                    OptionValue::VideoRate(Rational::new(25, 1).unwrap()),
                    "video rate",
                )
                .unwrap(),
            )
            .unwrap();

        let range = options.range("rate").unwrap().unwrap();
        assert_eq!(range.min(), &OptionValue::VideoRate(Rational::ONE));
        assert_eq!(
            range.max(),
            &OptionValue::VideoRate(Rational::new(120, 1).unwrap())
        );
        let av_ranges = options.query_avoption_ranges("rate").unwrap();
        assert_eq!(av_ranges.nb_ranges(), 1);
        assert_eq!(av_ranges.ranges()[0].value_min(), 1.0);
        assert_eq!(av_ranges.ranges()[0].value_max(), i32::MAX as f64);
        assert_eq!(av_ranges.ranges()[0].component_min(), 1.0);
        assert_eq!(av_ranges.ranges()[0].component_max(), i32::MAX as f64);
        assert_eq!(options.get_avoption_string("rate").unwrap(), "25/1");

        options.set_avoption_from_str("rate", "ntsc").unwrap();
        assert_eq!(
            options.get("rate"),
            Some(&OptionValue::VideoRate(Rational::new(30000, 1001).unwrap()))
        );
        assert_eq!(options.get_avoption_string("rate").unwrap(), "30000/1001");

        options.set_avoption_from_str("rate", "film").unwrap();
        assert_eq!(
            options.get("rate"),
            Some(&OptionValue::VideoRate(Rational::new(24, 1).unwrap()))
        );

        options.set_avoption_from_str("rate", "30000/1001").unwrap();
        assert_eq!(
            options.get("rate"),
            Some(&OptionValue::VideoRate(Rational::new(30000, 1001).unwrap()))
        );

        options
            .set_avoption_video_rate("rate", Rational::new(50, 1).unwrap())
            .unwrap();
        assert_eq!(options.get_avoption_string("rate").unwrap(), "50/1");
        options
            .set_avoption_q("rate", Rational::new(60, 1).unwrap())
            .unwrap();
        assert_eq!(options.get_avoption_string("rate").unwrap(), "60/1");

        let before_errors = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("rate", "bad")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options
                .set_avoption_from_str("rate", "0")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
        assert_eq!(
            options
                .set_avoption_video_rate("rate", Rational::ZERO)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options.get_avoption_video_rate("rate").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_q("rate").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_errors);
    }

    #[test]
    fn color_options_parse_format_and_query_like_bounded_ffmpeg_shape() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "color",
                    OptionKind::Color,
                    OptionValue::Color(RgbaColor::from_rgba([0xFF, 0x00, 0x00, 0xFF])),
                    "color",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.range("color").unwrap(), None);
        let av_ranges = options.query_avoption_ranges("color").unwrap();
        assert_eq!(av_ranges.nb_ranges(), 1);
        assert_eq!(av_ranges.ranges()[0].value_min(), 0.0);
        assert_eq!(av_ranges.ranges()[0].value_max(), 0.0);
        assert_eq!(av_ranges.ranges()[0].component_min(), 0.0);
        assert_eq!(av_ranges.ranges()[0].component_max(), 0.0);
        assert_eq!(options.get_avoption_string("color").unwrap(), "0xff0000ff");

        options.set_avoption_from_str("color", "Blue@0.5").unwrap();
        assert_eq!(
            options.get("color"),
            Some(&OptionValue::Color(RgbaColor::from_rgba([
                0x00, 0x00, 0xFF, 0x7F
            ])))
        );
        assert_eq!(options.get_avoption_string("color").unwrap(), "0x0000ff7f");

        options.set_avoption_from_str("color", "#112233").unwrap();
        assert_eq!(
            options.get("color"),
            Some(&OptionValue::Color(RgbaColor::from_rgba([
                0x11, 0x22, 0x33, 0xFF
            ])))
        );

        options
            .set_avoption_from_str("color", "0x11223344")
            .unwrap();
        assert_eq!(
            options.get("color"),
            Some(&OptionValue::Color(RgbaColor::from_rgba([
                0x11, 0x22, 0x33, 0x44
            ])))
        );
        assert_eq!(options.get_avoption_string("color").unwrap(), "0x11223344");

        let before_name_error = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("color", "not-a-color")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get("color"),
            Some(&OptionValue::Color(RgbaColor::from_rgba([
                0x11, 0x22, 0x33, 0xFF
            ])))
        );
        assert_eq!(
            options
                .set_avoption_from_str("color", "red@2")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get("color"),
            Some(&OptionValue::Color(RgbaColor::from_rgba([
                0xFF, 0x00, 0x00, 0xFF
            ])))
        );
        assert_ne!(options, before_name_error);

        let before_numeric_errors = options.clone();
        assert_eq!(
            options.set_avoption_int("color", 10).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(options, before_numeric_errors);
        assert_eq!(
            options.set_avoption_int("color", 0).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_numeric_errors);
        assert_eq!(
            options.get_avoption_int("color").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_q("color").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before_numeric_errors);
    }

    #[test]
    fn option_set_stores_defaults_and_preserves_order() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "threads",
                    OptionKind::Int { min: 1, max: 64 },
                    OptionValue::Int(1),
                    "worker count",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "bitexact",
                    OptionKind::Bool,
                    OptionValue::Bool(false),
                    "bit-exact output",
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(options.len(), 2);
        assert_eq!(options.definitions()[0].name(), "threads");
        assert_eq!(options.get("THREADS"), Some(&OptionValue::Int(1)));
        assert_eq!(options.get("bitexact"), Some(&OptionValue::Bool(false)));
    }

    #[test]
    fn set_from_str_resolves_unit_constants_case_insensitively() {
        let mut options = sample_options();

        options.set_from_str("preset_level", "FAST").unwrap();
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(2)));

        options.set_from_str("preset_level", "slow").unwrap();
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(8)));
        assert_eq!(options.constants_for_unit("PRESET").count(), 2);
    }

    #[test]
    fn unit_constants_are_scoped_and_preserve_state_on_errors() {
        let mut options = sample_options();

        options
            .define_constant(
                OptionConstant::new("mode", "fast", OptionValue::Int(1), "mode fast").unwrap(),
            )
            .unwrap();
        options
            .define_constant(
                OptionConstant::new(
                    "preset",
                    "not_an_int",
                    OptionValue::String("bad".to_owned()),
                    "wrong type",
                )
                .unwrap(),
            )
            .unwrap();

        assert!(options.set_from_str("threads", "fast").is_err());
        assert_eq!(options.get("threads"), Some(&OptionValue::Int(1)));
        assert!(options.set_from_str("preset_level", "not_an_int").is_err());
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(0)));
        assert_eq!(options.constants_for_unit("mode").count(), 1);
    }

    #[test]
    fn duplicate_unit_constants_are_rejected_case_insensitively() {
        let mut options = sample_options();

        let err = options
            .define_constant(
                OptionConstant::new("PRESET", "FAST", OptionValue::Int(4), "duplicate").unwrap(),
            )
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    }

    #[test]
    fn set_from_str_parses_supported_value_types() {
        let mut options = sample_options();

        options.set_from_str("threads", "8").unwrap();
        options.set_from_str("bitexact", "yes").unwrap();
        options.set_from_str("quality", "0.75").unwrap();
        options.set_from_str("aspect_ratio", "4/3").unwrap();
        options.set_from_str("metadata", "title=clip").unwrap();

        assert_eq!(options.get("threads"), Some(&OptionValue::Int(8)));
        assert_eq!(options.get("bitexact"), Some(&OptionValue::Bool(true)));
        assert_eq!(options.get("quality"), Some(&OptionValue::Float(0.75)));
        assert_eq!(
            options.get("aspect_ratio"),
            Some(&OptionValue::Rational(Rational::new(4, 3).unwrap()))
        );
        options.set_from_str("aspect_ratio", "1").unwrap();
        assert_eq!(
            options.get("aspect_ratio"),
            Some(&OptionValue::Rational(Rational::ONE))
        );
        assert_eq!(
            options.get("metadata"),
            Some(&OptionValue::String("title=clip".to_string()))
        );
    }

    #[test]
    fn set_avoption_from_str_uses_ffmpeg_exact_lookup_shape() {
        let mut options = sample_options();

        let missing_option = options.set_avoption_from_str("THREADS", "8").unwrap_err();
        assert_eq!(missing_option.kind(), AvErrorKind::NotFound);
        assert_eq!(missing_option.code(), Some(AvErrorCode::OPTION_NOT_FOUND));
        assert_eq!(options.get("threads"), Some(&OptionValue::Int(1)));

        options.set_avoption_from_str("threads", "8").unwrap();
        assert_eq!(options.get("THREADS"), Some(&OptionValue::Int(8)));

        options
            .set_avoption_from_str("preset_level", "fast")
            .unwrap();
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(2)));

        let before = options.clone();
        assert!(options
            .set_avoption_from_str("preset_level", "FAST")
            .is_err());
        assert_eq!(options, before);
        assert!(options.set_avoption_from_str("fast", "2").is_err());
        assert_eq!(options, before);

        options.set_from_str("preset_level", "FAST").unwrap();
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(2)));
    }

    #[test]
    fn set_avoption_from_str_parses_bounded_ffmpeg_numeric_expressions() {
        let mut options = sample_options();

        options.set_avoption_from_str("threads", " 2 * 3 ").unwrap();
        options.set_avoption_from_str("quality", "500m").unwrap();
        options
            .set_avoption_from_str("aspect_ratio", "1+1/2")
            .unwrap();
        options
            .set_avoption_from_str("preset_level", "slow+2")
            .unwrap();

        assert_eq!(options.get("threads"), Some(&OptionValue::Int(6)));
        assert_eq!(options.get("quality"), Some(&OptionValue::Float(0.5)));
        assert_eq!(
            options.get("aspect_ratio"),
            Some(&OptionValue::Rational(Rational::new(3, 2).unwrap()))
        );
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(10)));

        let before = options.clone();
        assert_eq!(
            options
                .set_avoption_from_str("threads", "1K")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(options, before);
        assert_eq!(
            options
                .set_avoption_from_str("quality", "2*")
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(options, before);
    }

    #[test]
    fn get_avoption_string_formats_values_like_bounded_ffmpeg_surface() {
        let mut options = sample_options();

        assert_eq!(options.get_avoption_string("threads").unwrap(), "1");
        assert_eq!(options.get_avoption_string("bitexact").unwrap(), "false");
        assert_eq!(options.get_avoption_string("quality").unwrap(), "0.500000");
        assert_eq!(options.get_avoption_string("aspect_ratio").unwrap(), "1/1");
        assert_eq!(options.get_avoption_string("metadata").unwrap(), "default");
        assert_eq!(options.get_avoption_string("preset_level").unwrap(), "0");

        let missing = options.get_avoption_string("THREADS").unwrap_err();
        assert_eq!(missing.kind(), AvErrorKind::NotFound);
        assert_eq!(missing.code(), Some(AvErrorCode::OPTION_NOT_FOUND));

        options.set_avoption_from_str("threads", "8").unwrap();
        options.set_avoption_from_str("bitexact", "yes").unwrap();
        options.set_avoption_from_str("quality", "0.75").unwrap();
        options
            .set_avoption_from_str("aspect_ratio", "4/3")
            .unwrap();
        options
            .set_avoption_from_str("metadata", "title=clip")
            .unwrap();
        options
            .set_avoption_from_str("preset_level", "slow")
            .unwrap();

        assert_eq!(options.get_avoption_string("threads").unwrap(), "8");
        assert_eq!(options.get_avoption_string("bitexact").unwrap(), "true");
        assert_eq!(options.get_avoption_string("quality").unwrap(), "0.750000");
        assert_eq!(options.get_avoption_string("aspect_ratio").unwrap(), "4/3");
        assert_eq!(
            options.get_avoption_string("metadata").unwrap(),
            "title=clip"
        );
        assert_eq!(options.get_avoption_string("preset_level").unwrap(), "8");
    }

    #[test]
    fn typed_avoption_get_set_matches_bounded_ffmpeg_shape() {
        let mut options = sample_options();
        options
            .define_with_current_value(
                OptionDefinition::new_with_flags(
                    "exported",
                    OptionKind::Int { min: 0, max: 8 },
                    OptionValue::Int(4),
                    "read-only exported value",
                    OptionFlags::READONLY,
                )
                .unwrap(),
                OptionValue::Int(0),
            )
            .unwrap();

        options.set_avoption_int("threads", 21).unwrap();
        options.set_avoption_int("bitexact", 1).unwrap();
        options.set_avoption_double("quality", 0.625).unwrap();
        options
            .set_avoption_q("aspect_ratio", Rational::new(3, 2).unwrap())
            .unwrap();
        options.set_avoption_int("preset_level", 6).unwrap();

        assert_eq!(options.get("threads"), Some(&OptionValue::Int(21)));
        assert_eq!(options.get("bitexact"), Some(&OptionValue::Bool(true)));
        assert_eq!(options.get("quality"), Some(&OptionValue::Float(0.625)));
        assert_eq!(
            options.get("aspect_ratio"),
            Some(&OptionValue::Rational(Rational::new(3, 2).unwrap()))
        );
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(6)));

        assert_eq!(options.get_avoption_int("threads").unwrap(), 21);
        assert_eq!(options.get_avoption_double("threads").unwrap(), 21.0);
        assert_eq!(
            options.get_avoption_q("threads").unwrap(),
            Rational::new(21, 1).unwrap()
        );
        assert_eq!(options.get_avoption_int("bitexact").unwrap(), 1);
        assert_eq!(options.get_avoption_double("quality").unwrap(), 0.625);
        assert_eq!(options.get_avoption_int("quality").unwrap(), 0);
        assert_eq!(
            options.get_avoption_q("aspect_ratio").unwrap(),
            Rational::new(3, 2).unwrap()
        );

        assert_eq!(
            options.set_avoption_int("metadata", 1).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options.set_avoption_int("threads", 128).unwrap_err().code(),
            Some(AvErrorCode::from_posix_errno(34))
        );
        assert_eq!(
            options.set_avoption_int("exported", 1).unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_int("metadata").unwrap_err().code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options.get_avoption_int("missing").unwrap_err().code(),
            Some(AvErrorCode::OPTION_NOT_FOUND)
        );

        let mut parent = sample_options();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "child_only",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(5),
                    "child-only value",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        parent
            .define_child(OptionChild::new("decoder", child_options, "").unwrap())
            .unwrap();

        parent
            .set_avoption_int_with_flags("threads", 9, OptionSearchFlags::CHILDREN)
            .unwrap();
        parent
            .set_avoption_int_with_flags("child_only", 7, OptionSearchFlags::CHILDREN)
            .unwrap();

        assert_eq!(parent.get_avoption_int("threads").unwrap(), 1);
        assert_eq!(
            parent
                .get_avoption_int_with_flags("threads", OptionSearchFlags::CHILDREN)
                .unwrap(),
            9
        );
        assert_eq!(
            parent
                .get_avoption_int_with_flags("child_only", OptionSearchFlags::CHILDREN)
                .unwrap(),
            7
        );
        assert_eq!(
            parent
                .set_avoption_int_with_flags("threads", 10, OptionSearchFlags::FAKE_OBJ)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::OPTION_NOT_FOUND)
        );
    }

    #[test]
    fn query_avoption_ranges_matches_bounded_ffmpeg_default_shape() {
        let options = sample_options();
        let threads = options.query_avoption_ranges("threads").unwrap();
        assert_eq!(threads.nb_ranges(), 1);
        assert_eq!(threads.nb_components(), 1);
        assert_eq!(threads.ranges()[0].value_min(), 1.0);
        assert_eq!(threads.ranges()[0].value_max(), 64.0);
        assert_eq!(threads.ranges()[0].component_min(), 0.0);
        assert_eq!(threads.ranges()[0].component_max(), 0.0);
        assert!(threads.ranges()[0].is_range());

        let bitexact = options.query_avoption_ranges("bitexact").unwrap();
        assert_eq!(bitexact.ranges()[0].value_min(), 0.0);
        assert_eq!(bitexact.ranges()[0].value_max(), 1.0);

        let metadata = options.query_avoption_ranges("metadata").unwrap();
        assert_eq!(metadata.ranges()[0].value_min(), -1.0);
        assert_eq!(metadata.ranges()[0].value_max(), i32::MAX as f64);
        assert_eq!(metadata.ranges()[0].component_min(), 0.0);
        assert_eq!(metadata.ranges()[0].component_max(), 0x10ffff as f64);

        let aspect = options.query_avoption_ranges("aspect_ratio").unwrap();
        assert_eq!(aspect.ranges()[0].value_min(), 1.0);
        assert_eq!(
            aspect.ranges()[0].value_max(),
            Rational::new(16, 9).unwrap().to_f64()
        );
        assert_eq!(aspect.ranges()[0].component_min(), i32::MIN as f64);
        assert_eq!(aspect.ranges()[0].component_max(), i32::MAX as f64);

        let missing = options.query_avoption_ranges("THREADS").unwrap_err();
        assert_eq!(missing.kind(), AvErrorKind::NotFound);
        assert_eq!(missing.code(), Some(AvErrorCode::ENOMEM));
    }

    #[test]
    fn rejects_unknown_options_type_mismatches_and_out_of_range_values() {
        let mut options = sample_options();

        assert_eq!(
            options
                .set("missing", OptionValue::Bool(true))
                .unwrap_err()
                .kind(),
            AvErrorKind::NotFound
        );
        assert!(options
            .set("threads", OptionValue::String("8".to_string()))
            .is_err());
        assert!(options
            .set("aspect_ratio", OptionValue::Float(1.0))
            .is_err());
        assert!(options
            .set(
                "aspect_ratio",
                OptionValue::Rational(Rational::new(2, 1).unwrap())
            )
            .is_err());
        assert!(options.set_from_str("threads", "0").is_err());
        assert!(options.set_from_str("quality", "inf").is_err());
        assert!(options.set_from_str("aspect_ratio", "1/0").is_err());
        assert!(options.set_from_str("aspect_ratio", "2/1").is_err());
        assert!(options.set_from_str("aspect_ratio", "bad/1").is_err());
        assert!(options.set_from_str("aspect_ratio", "1/").is_err());
        assert!(options.set_from_str("bitexact", "maybe").is_err());
    }

    #[test]
    fn duplicate_definitions_are_rejected_case_insensitively() {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "codec",
                    OptionKind::String { allow_empty: false },
                    OptionValue::String("copy".to_string()),
                    "",
                )
                .unwrap(),
            )
            .unwrap();

        let err = options
            .define(
                OptionDefinition::new(
                    "CODEC",
                    OptionKind::String { allow_empty: false },
                    OptionValue::String("rawvideo".to_string()),
                    "",
                )
                .unwrap(),
            )
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
    }

    #[test]
    fn string_options_enforce_empty_and_nul_rules() {
        let definition = OptionDefinition::new(
            "metadata",
            OptionKind::String { allow_empty: false },
            OptionValue::String("default".to_string()),
            "",
        )
        .unwrap();

        assert!(definition.parse_value("").is_err());
        assert!(definition.parse_value("bad\0value").is_err());
        assert_eq!(
            definition.parse_value("ok").unwrap(),
            OptionValue::String("ok".to_string())
        );
    }

    #[test]
    fn readonly_options_reject_mutation_without_changing_value() {
        let mut options = OptionSet::new();
        let definition = OptionDefinition::new_with_flags(
            "exported",
            OptionKind::Int { min: 0, max: 8 },
            OptionValue::Int(4),
            "read-only exported value",
            OptionFlags::from_bits_truncate(
                OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
            ),
        )
        .unwrap();
        options.define(definition).unwrap();

        let typed_err = options.set("exported", OptionValue::Int(5)).unwrap_err();
        let parsed_err = options.set_from_str("exported", "6").unwrap_err();

        assert_eq!(typed_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(parsed_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(options.get("exported"), Some(&OptionValue::Int(4)));
        assert!(options
            .definition("exported")
            .unwrap()
            .flags()
            .contains(OptionFlags::READONLY));
    }

    #[test]
    fn definitions_can_model_current_storage_distinct_from_declared_default() {
        let mut options = OptionSet::new();
        let definition = OptionDefinition::new_with_flags(
            "exported",
            OptionKind::Int { min: 0, max: 8 },
            OptionValue::Int(4),
            "read-only exported value",
            OptionFlags::from_bits_truncate(
                OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
            ),
        )
        .unwrap();

        options
            .define_with_current_value(definition, OptionValue::Int(0))
            .unwrap();

        assert_eq!(
            options.definition("exported").unwrap().default(),
            &OptionValue::Int(4)
        );
        assert_eq!(options.get("exported"), Some(&OptionValue::Int(0)));
        assert!(options.set_avoption_from_str("exported", "4").is_err());
        assert_eq!(
            options
                .serialize_avoptions(OptionFlags::EXPORT, OptionSerializeFlags::empty(), '=', ',',)
                .unwrap(),
            "exported=0"
        );
        assert_eq!(
            options
                .serialize_avoptions(
                    OptionFlags::empty(),
                    OptionSerializeFlags::SKIP_DEFAULTS,
                    '=',
                    ',',
                )
                .unwrap(),
            "exported=0"
        );
    }

    #[test]
    fn child_option_sets_register_independent_option_namespaces() {
        let mut parent = sample_options();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                )
                .unwrap(),
            )
            .unwrap();

        parent
            .define_child(
                OptionChild::new("encoder", child_options, "encoder private options").unwrap(),
            )
            .unwrap();

        let child = parent.child("ENCODER").unwrap();

        assert_eq!(parent.children().len(), 1);
        assert_eq!(child.name(), "encoder");
        assert_eq!(child.help(), "encoder private options");
        assert_eq!(parent.get("threads"), Some(&OptionValue::Int(1)));
        assert_eq!(child.options().get("threads"), Some(&OptionValue::Int(2)));
        assert_eq!(
            child.options().definition("THREADS").unwrap().help(),
            "child worker count"
        );
    }

    #[test]
    fn child_option_values_can_be_mutated_through_parent_without_touching_root() {
        let mut parent = sample_options();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_unit(
                    "preset_level",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(0),
                    "child preset level",
                    "preset",
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define_constant(
                OptionConstant::new("preset", "fast", OptionValue::Int(3), "child fast").unwrap(),
            )
            .unwrap();

        parent
            .define_child(OptionChild::new("encoder", child_options, "encoder options").unwrap())
            .unwrap();

        parent
            .set_child("ENCODER", "THREADS", OptionValue::Int(4))
            .unwrap();
        parent
            .set_child_from_str("encoder", "preset_level", "FAST")
            .unwrap();
        parent
            .child_mut("encoder")
            .unwrap()
            .options_mut()
            .set("threads", OptionValue::Int(6))
            .unwrap();

        assert_eq!(parent.get("threads"), Some(&OptionValue::Int(1)));
        assert_eq!(
            parent.get_child_option("encoder", "threads").unwrap(),
            &OptionValue::Int(6)
        );
        assert_eq!(
            parent.get_child_option("encoder", "preset_level").unwrap(),
            &OptionValue::Int(3)
        );
        let range = parent.child_range("encoder", "threads").unwrap().unwrap();
        assert_eq!(range.min(), &OptionValue::Int(1));
        assert_eq!(range.max(), &OptionValue::Int(16));
    }

    #[test]
    fn child_option_mutation_errors_preserve_existing_values() {
        let mut parent = sample_options();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "readonly",
                    OptionKind::Bool,
                    OptionValue::Bool(false),
                    "read-only child flag",
                    OptionFlags::READONLY,
                )
                .unwrap(),
            )
            .unwrap();
        parent
            .define_child(OptionChild::new("decoder", child_options, "").unwrap())
            .unwrap();

        let before = parent.clone();

        assert_eq!(
            parent
                .set_child("missing", "threads", OptionValue::Int(4))
                .unwrap_err()
                .kind(),
            AvErrorKind::NotFound
        );
        assert_eq!(
            parent
                .set_child("decoder", "missing", OptionValue::Int(4))
                .unwrap_err()
                .kind(),
            AvErrorKind::NotFound
        );
        assert_eq!(
            parent
                .get_child_option("missing", "threads")
                .unwrap_err()
                .kind(),
            AvErrorKind::NotFound
        );
        assert!(parent
            .set_child("decoder", "threads", OptionValue::Int(99))
            .is_err());
        assert!(parent
            .set_child_from_str("decoder", "threads", "not_an_int")
            .is_err());
        assert!(parent
            .set_child_from_str("decoder", "readonly", "yes")
            .is_err());

        assert_eq!(parent, before);
        assert_eq!(
            parent.get_child_option("decoder", "threads").unwrap(),
            &OptionValue::Int(2)
        );
    }

    #[test]
    fn duplicate_child_option_sets_are_rejected_case_insensitively() {
        let mut options = sample_options();
        options
            .define_child(OptionChild::new("decoder", OptionSet::new(), "").unwrap())
            .unwrap();

        let before = options.clone();
        let err = options
            .define_child(OptionChild::new("DECODER", OptionSet::new(), "duplicate").unwrap())
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(options, before);
    }

    #[test]
    fn option_set_removes_definitions_constants_and_children_case_insensitively() {
        let mut options = sample_options();
        options.set("threads", OptionValue::Int(8)).unwrap();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define_child(OptionChild::new("decoder", child_options, "decoder options").unwrap())
            .unwrap();

        let before_missing = options.clone();
        assert_eq!(
            options.remove_definition("missing").unwrap_err().kind(),
            AvErrorKind::NotFound
        );
        assert_eq!(options, before_missing);

        let (removed_definition, removed_value) = options.remove_definition("THREADS").unwrap();
        assert_eq!(removed_definition.name(), "threads");
        assert_eq!(removed_value, OptionValue::Int(8));
        assert_eq!(options.len(), before_missing.len() - 1);
        assert_eq!(options.definitions()[0].name(), "bitexact");
        assert!(options.definition("threads").is_none());
        assert!(options.get("threads").is_none());
        assert_eq!(
            options.get_child_option("decoder", "threads").unwrap(),
            &OptionValue::Int(2)
        );

        let before_missing_constant = options.clone();
        assert_eq!(
            options
                .remove_constant("preset", "missing")
                .unwrap_err()
                .kind(),
            AvErrorKind::NotFound
        );
        assert_eq!(options, before_missing_constant);

        let removed_constant = options.remove_constant("PRESET", "FAST").unwrap();
        assert_eq!(removed_constant.unit(), "preset");
        assert_eq!(removed_constant.name(), "fast");
        assert_eq!(
            options
                .constants_for_unit("preset")
                .map(OptionConstant::name)
                .collect::<Vec<_>>(),
            vec!["slow"]
        );
        assert!(options.set_from_str("preset_level", "fast").is_err());
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(0)));
        options.set_from_str("preset_level", "slow").unwrap();
        assert_eq!(options.get("preset_level"), Some(&OptionValue::Int(8)));

        let before_missing_child = options.clone();
        assert_eq!(
            options.remove_child("encoder").unwrap_err().kind(),
            AvErrorKind::NotFound
        );
        assert_eq!(options, before_missing_child);

        let removed_child = options.remove_child("DECODER").unwrap();
        assert_eq!(removed_child.name(), "decoder");
        assert!(options.child("decoder").is_none());
        assert_eq!(
            options
                .get_child_option("decoder", "threads")
                .unwrap_err()
                .kind(),
            AvErrorKind::NotFound
        );
    }

    #[test]
    fn invalid_child_option_set_metadata_is_rejected() {
        assert!(OptionChild::new("", OptionSet::new(), "").is_err());
        assert!(OptionChild::new("bad\0child", OptionSet::new(), "").is_err());
        assert!(OptionChild::new("child", OptionSet::new(), "bad\0help").is_err());
    }

    #[test]
    fn option_queries_filter_by_name_unit_flags_and_order() {
        let mut options = sample_options();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "exported",
                    OptionKind::Int { min: 0, max: 8 },
                    OptionValue::Int(4),
                    "exported value",
                    OptionFlags::from_bits_truncate(
                        OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "video_only",
                    OptionKind::Bool,
                    OptionValue::Bool(false),
                    "video option",
                    OptionFlags::VIDEO_PARAM,
                )
                .unwrap(),
            )
            .unwrap();

        let named = options
            .first_definition_matching(&OptionQuery::new().with_name("THREADS").unwrap())
            .unwrap();
        let unit_names: Vec<_> = options
            .definitions_matching(&OptionQuery::new().with_unit("PRESET").unwrap())
            .into_iter()
            .map(|found| found.definition().name())
            .collect();
        let exported_names: Vec<_> = options
            .definitions_matching(&OptionQuery::exported())
            .into_iter()
            .map(|found| found.definition().name())
            .collect();
        let writable_names: Vec<_> = options
            .definitions_matching(&OptionQuery::writable())
            .into_iter()
            .map(|found| found.definition().name())
            .collect();
        let video_names: Vec<_> = options
            .definitions_matching(&OptionQuery::new().require_flags(OptionFlags::VIDEO_PARAM))
            .into_iter()
            .map(|found| found.definition().name())
            .collect();

        assert_eq!(named.child_name(), None);
        assert_eq!(named.definition().name(), "threads");
        assert_eq!(unit_names, vec!["preset_level"]);
        assert_eq!(exported_names, vec!["exported"]);
        assert!(!writable_names.contains(&"exported"));
        assert!(writable_names.contains(&"threads"));
        assert_eq!(video_names, vec!["video_only"]);
    }

    #[test]
    fn option_queries_can_search_child_option_sets() {
        let mut parent = sample_options();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "secret",
                    OptionKind::String { allow_empty: false },
                    OptionValue::String("hidden".to_owned()),
                    "child private value",
                    OptionFlags::READONLY,
                )
                .unwrap(),
            )
            .unwrap();
        parent
            .define_child(OptionChild::new("decoder", child_options, "").unwrap())
            .unwrap();

        let root_only: Vec<_> = parent
            .definitions_matching(&OptionQuery::new().with_name("threads").unwrap())
            .into_iter()
            .map(|found| (found.child_name(), found.definition().name()))
            .collect();
        let with_children: Vec<_> = parent
            .definitions_matching(
                &OptionQuery::new()
                    .with_name("threads")
                    .unwrap()
                    .include_children(true),
            )
            .into_iter()
            .map(|found| (found.child_name(), found.definition().name()))
            .collect();
        let decoding: Vec<_> = parent
            .definitions_matching(
                &OptionQuery::new()
                    .require_flags(OptionFlags::DECODING_PARAM)
                    .include_children(true),
            )
            .into_iter()
            .map(|found| (found.child_name(), found.definition().name()))
            .collect();
        let writable_child: Vec<_> = parent
            .definitions_matching(
                &OptionQuery::writable()
                    .with_name("secret")
                    .unwrap()
                    .include_children(true),
            )
            .into_iter()
            .map(|found| (found.child_name(), found.definition().name()))
            .collect();

        assert_eq!(root_only, vec![(None, "threads")]);
        assert_eq!(
            with_children,
            vec![(None, "threads"), (Some("decoder"), "threads")]
        );
        assert_eq!(decoding, vec![(Some("decoder"), "threads")]);
        assert!(writable_child.is_empty());
    }

    #[test]
    fn avoption_entries_and_find_follow_ffmpeg_search_shape() {
        let mut options = sample_options();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "exported",
                    OptionKind::Int { min: 0, max: 8 },
                    OptionValue::Int(4),
                    "exported value",
                    OptionFlags::from_bits_truncate(
                        OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
                    ),
                )
                .unwrap(),
            )
            .unwrap();

        let entries: Vec<_> = options
            .avoption_entries()
            .into_iter()
            .map(|found| {
                (
                    found.child_name(),
                    found.name(),
                    found.entry().is_constant(),
                )
            })
            .collect();

        assert_eq!(
            entries,
            vec![
                (None, "threads", false),
                (None, "bitexact", false),
                (None, "quality", false),
                (None, "metadata", false),
                (None, "aspect_ratio", false),
                (None, "preset_level", false),
                (None, "fast", true),
                (None, "slow", true),
                (None, "exported", false),
            ]
        );

        assert_eq!(
            options
                .find_avoption(
                    "threads",
                    None,
                    OptionFlags::empty(),
                    OptionSearchFlags::empty()
                )
                .unwrap()
                .name(),
            "threads"
        );
        assert!(options
            .find_avoption(
                "THREADS",
                None,
                OptionFlags::empty(),
                OptionSearchFlags::empty()
            )
            .is_none());
        assert!(options
            .find_avoption(
                "fast",
                None,
                OptionFlags::empty(),
                OptionSearchFlags::empty()
            )
            .is_none());
        assert_eq!(
            options
                .find_avoption(
                    "fast",
                    Some("preset"),
                    OptionFlags::empty(),
                    OptionSearchFlags::empty()
                )
                .unwrap()
                .name(),
            "fast"
        );
        assert!(options
            .find_avoption(
                "FAST",
                Some("preset"),
                OptionFlags::empty(),
                OptionSearchFlags::empty()
            )
            .is_none());
        assert!(options
            .find_avoption(
                "slow",
                Some("PRESET"),
                OptionFlags::empty(),
                OptionSearchFlags::empty()
            )
            .is_none());
        assert_eq!(
            options
                .find_avoption(
                    "exported",
                    None,
                    OptionFlags::from_bits_truncate(
                        OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits()
                    ),
                    OptionSearchFlags::empty()
                )
                .unwrap()
                .name(),
            "exported"
        );
        assert!(options
            .find_avoption(
                "exported",
                None,
                OptionFlags::VIDEO_PARAM,
                OptionSearchFlags::empty()
            )
            .is_none());

        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define_child(OptionChild::new("decoder", child_options, "").unwrap())
            .unwrap();

        assert!(options
            .find_avoption(
                "threads",
                None,
                OptionFlags::DECODING_PARAM,
                OptionSearchFlags::empty()
            )
            .is_none());
        let child_found = options
            .find_avoption(
                "threads",
                None,
                OptionFlags::DECODING_PARAM,
                OptionSearchFlags::CHILDREN,
            )
            .unwrap();
        assert_eq!(child_found.child_name(), Some("decoder"));
        assert_eq!(child_found.name(), "threads");
    }

    #[test]
    fn avoption_get_set_with_search_flags_use_child_target_before_root() {
        let mut options = sample_options();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "child_only",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(5),
                    "child-only value",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "child_readonly",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(0),
                    "child read-only value",
                    OptionFlags::from_bits_truncate(
                        OptionFlags::DECODING_PARAM.bits() | OptionFlags::READONLY.bits(),
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define_child(OptionChild::new("decoder", child_options, "").unwrap())
            .unwrap();

        assert_eq!(
            options
                .get_avoption_string_with_flags("threads", OptionSearchFlags::CHILDREN)
                .unwrap(),
            "2"
        );
        assert_eq!(
            options
                .get_avoption_string_with_flags("child_only", OptionSearchFlags::CHILDREN)
                .unwrap(),
            "5"
        );
        assert_eq!(
            options
                .get_avoption_string_with_flags("child_only", OptionSearchFlags::empty())
                .unwrap_err()
                .code(),
            Some(AvErrorCode::OPTION_NOT_FOUND)
        );
        assert_eq!(
            options
                .get_avoption_string_with_flags("threads", OptionSearchFlags::FAKE_OBJ)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::OPTION_NOT_FOUND)
        );

        assert_eq!(
            options
                .set_avoption_from_str_with_flags("child_only", "7", OptionSearchFlags::empty())
                .unwrap_err()
                .code(),
            Some(AvErrorCode::OPTION_NOT_FOUND)
        );
        options
            .set_avoption_from_str_with_flags("child_only", "7", OptionSearchFlags::CHILDREN)
            .unwrap();
        options
            .set_avoption_from_str_with_flags("threads", "9", OptionSearchFlags::CHILDREN)
            .unwrap();
        assert_eq!(
            options
                .set_avoption_from_str_with_flags(
                    "child_readonly",
                    "4",
                    OptionSearchFlags::CHILDREN
                )
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert_eq!(
            options
                .set_avoption_from_str_with_flags("threads", "10", OptionSearchFlags::FAKE_OBJ)
                .unwrap_err()
                .code(),
            Some(AvErrorCode::OPTION_NOT_FOUND)
        );

        assert_eq!(options.get_avoption_string("threads").unwrap(), "1");
        assert_eq!(
            options.get_child_option("decoder", "threads").unwrap(),
            &OptionValue::Int(9)
        );
        assert_eq!(
            options.get_child_option("decoder", "child_only").unwrap(),
            &OptionValue::Int(7)
        );
        assert_eq!(
            options
                .get_child_option("decoder", "child_readonly")
                .unwrap(),
            &OptionValue::Int(0)
        );
    }

    #[test]
    fn set_avoptions_from_dict_matches_bounded_ffmpeg_remainder_shape() {
        let mut options = sample_options();
        let mut dict = Dictionary::new();
        dict.set_with_mode(
            "threads",
            "11",
            MatchMode::CaseSensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "unknown",
            "first",
            MatchMode::CaseSensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "bitexact",
            "true",
            MatchMode::CaseSensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "unknown",
            "second",
            MatchMode::CaseSensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
        dict.set_with_mode(
            "metadata",
            "from-dict",
            MatchMode::CaseSensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();

        options
            .set_avoptions_from_dict(&mut dict, OptionSearchFlags::empty())
            .unwrap();

        assert_eq!(options.get("threads"), Some(&OptionValue::Int(11)));
        assert_eq!(options.get("bitexact"), Some(&OptionValue::Bool(true)));
        assert_eq!(
            options.get("metadata"),
            Some(&OptionValue::String("from-dict".to_owned()))
        );
        assert_eq!(
            dictionary_pairs(&dict),
            vec![("unknown", "first"), ("unknown", "second")]
        );

        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "child_only",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(5),
                    "child-only value",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        let mut with_child = sample_options();
        with_child
            .define_child(OptionChild::new("decoder", child_options, "").unwrap())
            .unwrap();
        let mut child_dict = Dictionary::new();
        for (key, value) in [
            ("threads", "9"),
            ("child_only", "6"),
            ("quality", "0.25"),
            ("unknown", "value"),
        ] {
            child_dict
                .set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple)
                .unwrap();
        }

        with_child
            .set_avoptions_from_dict(&mut child_dict, OptionSearchFlags::CHILDREN)
            .unwrap();

        assert_eq!(with_child.get("threads"), Some(&OptionValue::Int(1)));
        assert_eq!(
            with_child.get_child_option("decoder", "threads").unwrap(),
            &OptionValue::Int(9)
        );
        assert_eq!(
            with_child
                .get_child_option("decoder", "child_only")
                .unwrap(),
            &OptionValue::Int(6)
        );
        assert_eq!(with_child.get("quality"), Some(&OptionValue::Float(0.25)));
        assert_eq!(dictionary_pairs(&child_dict), vec![("unknown", "value")]);

        let mut error_options = sample_options();
        let mut error_dict = Dictionary::new();
        for (key, value) in [
            ("threads", "13"),
            ("bitexact", "maybe"),
            ("unknown", "later"),
        ] {
            error_dict
                .set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple)
                .unwrap();
        }
        let original_error_dict = error_dict.clone();

        let err = error_options
            .set_avoptions_from_dict(&mut error_dict, OptionSearchFlags::empty())
            .unwrap_err();

        assert_eq!(err.code(), Some(AvErrorCode::EINVAL));
        assert_eq!(error_options.get("threads"), Some(&OptionValue::Int(13)));
        assert_eq!(
            error_options.get("bitexact"),
            Some(&OptionValue::Bool(false))
        );
        assert_eq!(error_dict, original_error_dict);
    }

    #[test]
    fn set_avoptions_from_string_matches_bounded_ffmpeg_shape() {
        let mut named = sample_options();
        assert_eq!(
            named
                .set_avoptions_from_string(
                    "threads=7:quality=0.25:metadata=from-string",
                    &[],
                    "=",
                    ":",
                )
                .unwrap(),
            3
        );
        assert_eq!(named.get("threads"), Some(&OptionValue::Int(7)));
        assert_eq!(named.get("quality"), Some(&OptionValue::Float(0.25)));
        assert_eq!(
            named.get("metadata"),
            Some(&OptionValue::String("from-string".to_owned()))
        );

        let mut shorthand = sample_options();
        assert_eq!(
            shorthand
                .set_avoptions_from_string(
                    " 9 : yes : metadata = shorthand ",
                    &["threads", "bitexact"],
                    "=",
                    ":",
                )
                .unwrap(),
            3
        );
        assert_eq!(shorthand.get("threads"), Some(&OptionValue::Int(9)));
        assert_eq!(shorthand.get("bitexact"), Some(&OptionValue::Bool(true)));
        assert_eq!(
            shorthand.get("metadata"),
            Some(&OptionValue::String("shorthand".to_owned()))
        );

        let mut after_named_error = sample_options();
        let err = after_named_error
            .set_avoptions_from_string("10:quality=0.75:no", &["threads", "bitexact"], "=", ":")
            .unwrap_err();
        assert_eq!(err.code(), Some(AvErrorCode::EINVAL));
        assert_eq!(
            after_named_error.get("threads"),
            Some(&OptionValue::Int(10))
        );
        assert_eq!(
            after_named_error.get("quality"),
            Some(&OptionValue::Float(0.75))
        );
        assert_eq!(
            after_named_error.get("bitexact"),
            Some(&OptionValue::Bool(false))
        );

        let mut set_error = sample_options();
        let err = set_error
            .set_avoptions_from_string("threads=11:bitexact=maybe", &[], "=", ":")
            .unwrap_err();
        assert_eq!(err.code(), Some(AvErrorCode::EINVAL));
        assert_eq!(set_error.get("threads"), Some(&OptionValue::Int(11)));
        assert_eq!(set_error.get("bitexact"), Some(&OptionValue::Bool(false)));

        let mut not_found = sample_options();
        let err = not_found
            .set_avoptions_from_string("threads=12:unknown=1", &[], "=", ":")
            .unwrap_err();
        assert_eq!(err.code(), Some(AvErrorCode::OPTION_NOT_FOUND));
        assert_eq!(not_found.get("threads"), Some(&OptionValue::Int(12)));

        let mut no_shorthand = sample_options();
        let err = no_shorthand
            .set_avoptions_from_string("12", &[], "=", ":")
            .unwrap_err();
        assert_eq!(err.code(), Some(AvErrorCode::EINVAL));
        assert_eq!(no_shorthand, sample_options());

        let mut escaped = sample_options();
        assert_eq!(
            escaped
                .set_avoptions_from_string(
                    "metadata=title\\:clip\\=one\\\\two:threads=14:preset_level=slow",
                    &[],
                    "=",
                    ":",
                )
                .unwrap(),
            3
        );
        assert_eq!(escaped.get("threads"), Some(&OptionValue::Int(14)));
        assert_eq!(
            escaped.get("metadata"),
            Some(&OptionValue::String("title:clip=one\\two".to_owned()))
        );
        assert_eq!(escaped.get("preset_level"), Some(&OptionValue::Int(8)));

        let mut quoted = sample_options();
        assert_eq!(
            quoted
                .set_avoptions_from_string(
                    "metadata=' title : clip = one ':threads=15",
                    &[],
                    "=",
                    ":",
                )
                .unwrap(),
            2
        );
        assert_eq!(quoted.get("threads"), Some(&OptionValue::Int(15)));
        assert_eq!(
            quoted.get("metadata"),
            Some(&OptionValue::String(" title : clip = one ".to_owned()))
        );
    }

    #[test]
    fn serialize_avoptions_matches_bounded_ffmpeg_shape() {
        let defaults = sample_flagged_options();
        assert_eq!(
            defaults
                .serialize_avoptions(
                    OptionFlags::empty(),
                    OptionSerializeFlags::empty(),
                    '=',
                    ',',
                )
                .unwrap(),
            "threads=1,bitexact=false,quality=0.500000,aspect_ratio=1/1,metadata=default,preset_level=0,exported=0"
        );
        assert_eq!(
            defaults
                .serialize_avoptions(
                    OptionFlags::ENCODING_PARAM,
                    OptionSerializeFlags::OPT_FLAGS_EXACT,
                    '=',
                    ',',
                )
                .unwrap(),
            "threads=1,bitexact=false,quality=0.500000,aspect_ratio=1/1,metadata=default,preset_level=0"
        );
        assert_eq!(
            defaults
                .serialize_avoptions(
                    OptionFlags::empty(),
                    OptionSerializeFlags::OPT_FLAGS_EXACT,
                    '=',
                    ',',
                )
                .unwrap(),
            ""
        );
        assert_eq!(
            defaults
                .serialize_avoptions(OptionFlags::EXPORT, OptionSerializeFlags::empty(), '=', ',',)
                .unwrap(),
            "exported=0"
        );

        let mut changed = sample_flagged_options();
        changed.set_avoption_from_str("threads", "8").unwrap();
        changed.set_avoption_from_str("bitexact", "true").unwrap();
        changed
            .set_avoption_from_str("metadata", "title=clip,segment\\one")
            .unwrap();
        changed
            .set_avoption_from_str("preset_level", "slow")
            .unwrap();
        assert_eq!(
            changed
                .serialize_avoptions(
                    OptionFlags::empty(),
                    OptionSerializeFlags::SKIP_DEFAULTS,
                    '=',
                    ',',
                )
                .unwrap(),
            "threads=8,bitexact=true,metadata=title\\=clip\\,segment\\\\one,preset_level=8,exported=0"
        );

        let with_child = sample_flagged_options_with_child();
        assert_eq!(
            with_child
                .serialize_avoptions(
                    OptionFlags::empty(),
                    OptionSerializeFlags::SEARCH_CHILDREN,
                    '=',
                    ',',
                )
                .unwrap(),
            "threads=2,child_only=5,child_readonly=0,threads=1,bitexact=false,quality=0.500000,aspect_ratio=1/1,metadata=default,preset_level=0,exported=0"
        );
        assert_eq!(
            with_child
                .serialize_avoptions(
                    OptionFlags::DECODING_PARAM,
                    OptionSerializeFlags::SEARCH_CHILDREN,
                    '=',
                    ',',
                )
                .unwrap(),
            "threads=2,child_only=5,child_readonly=0"
        );

        assert_eq!(
            defaults
                .serialize_avoptions(
                    OptionFlags::empty(),
                    OptionSerializeFlags::empty(),
                    '=',
                    '=',
                )
                .unwrap_err()
                .code(),
            Some(AvErrorCode::EINVAL)
        );
        assert!(defaults
            .serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::empty(),
                '\\',
                ',',
            )
            .is_err());
        assert!(defaults
            .serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::empty(),
                '=',
                '\0',
            )
            .is_err());
    }

    #[test]
    fn copy_avoptions_from_matches_bounded_ffmpeg_root_shape() {
        fn child_options() -> OptionSet {
            let mut child = OptionSet::new();
            child
                .define(
                    OptionDefinition::new_with_flags(
                        "threads",
                        OptionKind::Int { min: 1, max: 16 },
                        OptionValue::Int(2),
                        "child worker count",
                        OptionFlags::DECODING_PARAM,
                    )
                    .unwrap(),
                )
                .unwrap();
            child
                .define(
                    OptionDefinition::new_with_flags(
                        "child_only",
                        OptionKind::Int { min: 0, max: 10 },
                        OptionValue::Int(5),
                        "child-only value",
                        OptionFlags::DECODING_PARAM,
                    )
                    .unwrap(),
                )
                .unwrap();
            child
        }

        let mut source = sample_options();
        source
            .define_child(OptionChild::new("decoder", child_options(), "").unwrap())
            .unwrap();
        source.set_avoption_from_str("threads", "12").unwrap();
        source.set_avoption_from_str("bitexact", "true").unwrap();
        source.set_avoption_from_str("quality", "0.875").unwrap();
        source.set_avoption_from_str("aspect_ratio", "3/2").unwrap();
        source.set_avoption_from_str("metadata", "source").unwrap();
        source
            .set_avoption_from_str("preset_level", "slow")
            .unwrap();
        source
            .set_child_from_str("decoder", "threads", "11")
            .unwrap();
        source
            .set_child_from_str("decoder", "child_only", "6")
            .unwrap();

        let mut destination = sample_options();
        destination
            .define_child(OptionChild::new("decoder", child_options(), "").unwrap())
            .unwrap();
        destination.set_avoption_from_str("threads", "3").unwrap();
        destination
            .set_avoption_from_str("metadata", "destination")
            .unwrap();
        destination
            .set_child_from_str("decoder", "threads", "14")
            .unwrap();
        destination
            .set_child_from_str("decoder", "child_only", "4")
            .unwrap();

        destination.copy_avoptions_from(&source).unwrap();

        assert_eq!(destination.get("threads"), Some(&OptionValue::Int(12)));
        assert_eq!(destination.get("bitexact"), Some(&OptionValue::Bool(true)));
        assert_eq!(destination.get("quality"), Some(&OptionValue::Float(0.875)));
        assert_eq!(
            destination.get("aspect_ratio"),
            Some(&OptionValue::Rational(Rational::new(3, 2).unwrap()))
        );
        assert_eq!(
            destination.get("metadata"),
            Some(&OptionValue::String("source".to_owned()))
        );
        assert_eq!(destination.get("preset_level"), Some(&OptionValue::Int(8)));
        assert_eq!(
            destination.get_child_option("decoder", "threads").unwrap(),
            &OptionValue::Int(14)
        );
        assert_eq!(
            destination
                .get_child_option("decoder", "child_only")
                .unwrap(),
            &OptionValue::Int(4)
        );

        source
            .set_avoption_from_str("metadata", "mutated-source")
            .unwrap();
        assert_eq!(
            destination.get("metadata"),
            Some(&OptionValue::String("source".to_owned()))
        );

        let child_source = source.child("decoder").unwrap().options().clone();
        let mut child_destination = destination.child("decoder").unwrap().options().clone();
        child_destination
            .copy_avoptions_from(&child_source)
            .unwrap();
        assert_eq!(
            child_destination.get("threads"),
            Some(&OptionValue::Int(11))
        );
        assert_eq!(
            child_destination.get("child_only"),
            Some(&OptionValue::Int(6))
        );

        let mut mismatch = OptionSet::new();
        mismatch
            .define(
                OptionDefinition::new("other", OptionKind::Bool, OptionValue::Bool(false), "")
                    .unwrap(),
            )
            .unwrap();
        let before_mismatch = mismatch.clone();
        let err = mismatch.copy_avoptions_from(&source).unwrap_err();
        assert_eq!(err.code(), Some(AvErrorCode::EINVAL));
        assert_eq!(mismatch, before_mismatch);
    }

    #[test]
    fn option_queries_validate_name_and_unit_metadata() {
        assert!(OptionQuery::new().with_name("").is_err());
        assert!(OptionQuery::new().with_name("bad\0name").is_err());
        assert!(OptionQuery::new().with_unit("").is_err());
        assert!(OptionQuery::new().with_unit("bad\0unit").is_err());

        let query = OptionQuery::new()
            .require_flags(OptionFlags::from_bits_truncate(u32::MAX))
            .reject_flags(OptionFlags::from_bits_truncate(u32::MAX))
            .include_children(true);

        assert_eq!(query.required_flags(), OptionFlags::all());
        assert_eq!(query.rejected_flags(), OptionFlags::all());
        assert!(query.searches_children());
        assert_eq!(query.name(), None);
        assert_eq!(query.unit(), None);
    }

    fn sample_flagged_options() -> OptionSet {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "threads",
                    OptionKind::Int { min: 1, max: 64 },
                    OptionValue::Int(1),
                    "worker count",
                    OptionFlags::ENCODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "bitexact",
                    OptionKind::Bool,
                    OptionValue::Bool(false),
                    "bit-exact output",
                    OptionFlags::ENCODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "quality",
                    OptionKind::Float { min: 0.0, max: 1.0 },
                    OptionValue::Float(0.5),
                    "quality",
                    OptionFlags::ENCODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "aspect_ratio",
                    OptionKind::Rational {
                        min: Rational::ONE,
                        max: Rational::new(16, 9).unwrap(),
                    },
                    OptionValue::Rational(Rational::ONE),
                    "aspect ratio",
                    OptionFlags::ENCODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new_with_flags(
                    "metadata",
                    OptionKind::String { allow_empty: false },
                    OptionValue::String("default".to_string()),
                    "metadata",
                    OptionFlags::ENCODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new_with_flags_and_unit(
                    "preset_level",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(0),
                    "preset level",
                    OptionFlags::ENCODING_PARAM,
                    Some("PRESET"),
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define_constant(
                OptionConstant::new_with_flags(
                    "PRESET",
                    "fast",
                    OptionValue::Int(2),
                    "fast preset",
                    OptionFlags::ENCODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define_constant(
                OptionConstant::new_with_flags(
                    "PRESET",
                    "slow",
                    OptionValue::Int(8),
                    "slow preset",
                    OptionFlags::ENCODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        let exported = OptionDefinition::new_with_flags(
            "exported",
            OptionKind::Int { min: 0, max: 8 },
            OptionValue::Int(4),
            "read-only exported value",
            OptionFlags::from_bits_truncate(
                OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
            ),
        )
        .unwrap();
        options
            .define_with_current_value(exported, OptionValue::Int(0))
            .unwrap();
        options
    }

    fn sample_flagged_options_with_child() -> OptionSet {
        let mut options = sample_flagged_options();
        let mut child_options = OptionSet::new();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "threads",
                    OptionKind::Int { min: 1, max: 16 },
                    OptionValue::Int(2),
                    "child worker count",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "child_only",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(5),
                    "child-only value",
                    OptionFlags::DECODING_PARAM,
                )
                .unwrap(),
            )
            .unwrap();
        child_options
            .define(
                OptionDefinition::new_with_flags(
                    "child_readonly",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(0),
                    "child read-only value",
                    OptionFlags::from_bits_truncate(
                        OptionFlags::DECODING_PARAM.bits() | OptionFlags::READONLY.bits(),
                    ),
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define_child(OptionChild::new("decoder", child_options, "decoder options").unwrap())
            .unwrap();
        options
    }

    fn sample_options() -> OptionSet {
        let mut options = OptionSet::new();
        options
            .define(
                OptionDefinition::new(
                    "threads",
                    OptionKind::Int { min: 1, max: 64 },
                    OptionValue::Int(1),
                    "",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new("bitexact", OptionKind::Bool, OptionValue::Bool(false), "")
                    .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "quality",
                    OptionKind::Float { min: 0.0, max: 1.0 },
                    OptionValue::Float(0.5),
                    "",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "metadata",
                    OptionKind::String { allow_empty: false },
                    OptionValue::String("default".to_string()),
                    "",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new(
                    "aspect_ratio",
                    OptionKind::Rational {
                        min: Rational::ONE,
                        max: Rational::new(16, 9).unwrap(),
                    },
                    OptionValue::Rational(Rational::ONE),
                    "",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define(
                OptionDefinition::new_with_unit(
                    "preset_level",
                    OptionKind::Int { min: 0, max: 10 },
                    OptionValue::Int(0),
                    "preset level",
                    "preset",
                )
                .unwrap(),
            )
            .unwrap();
        options
            .define_constant(
                OptionConstant::new("preset", "fast", OptionValue::Int(2), "fast preset").unwrap(),
            )
            .unwrap();
        options
            .define_constant(
                OptionConstant::new("preset", "slow", OptionValue::Int(8), "slow preset").unwrap(),
            )
            .unwrap();
        options
    }

    fn dictionary_pairs(dict: &Dictionary) -> Vec<(&str, &str)> {
        dict.entries()
            .iter()
            .map(|entry| (entry.key(), entry.value()))
            .collect()
    }
}
