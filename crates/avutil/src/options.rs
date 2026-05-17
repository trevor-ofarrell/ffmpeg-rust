use crate::{AvError, AvErrorKind, AvResult};

#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionKind {
    Bool,
    Int { min: i64, max: i64 },
    Float { min: f64, max: f64 },
    String { allow_empty: bool },
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

#[derive(Debug, Clone, PartialEq)]
pub struct OptionDefinition {
    name: String,
    help: String,
    kind: OptionKind,
    default: OptionValue,
    flags: OptionFlags,
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
        validate_name(&name.into()).and_then(|name| {
            validate_help(&help.into()).and_then(|help| {
                validate_kind(&kind)?;
                validate_value_for_kind(&kind, &default)?;
                Ok(Self {
                    name,
                    help,
                    kind,
                    default,
                    flags: OptionFlags::from_bits_truncate(flags.bits()),
                })
            })
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

    pub fn parse_value(&self, raw: &str) -> AvResult<OptionValue> {
        let parsed = match self.kind {
            OptionKind::Bool => OptionValue::Bool(parse_bool(raw)?),
            OptionKind::Int { .. } => OptionValue::Int(parse_int(raw)?),
            OptionKind::Float { .. } => OptionValue::Float(parse_float(raw)?),
            OptionKind::String { .. } => OptionValue::String(raw.to_owned()),
        };

        validate_value_for_kind(&self.kind, &parsed)?;
        Ok(parsed)
    }

    pub fn validate_value(&self, value: &OptionValue) -> AvResult<()> {
        validate_value_for_kind(&self.kind, value)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OptionSet {
    definitions: Vec<OptionDefinition>,
    values: Vec<OptionValue>,
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

    pub fn define(&mut self, definition: OptionDefinition) -> AvResult<()> {
        if self.find_index(definition.name()).is_some() {
            return Err(AvError::invalid_argument(format!(
                "duplicate option `{}`",
                definition.name()
            )));
        }

        self.values.push(definition.default().clone());
        self.definitions.push(definition);
        Ok(())
    }

    pub fn definition(&self, name: &str) -> Option<&OptionDefinition> {
        self.find_index(name).map(|index| &self.definitions[index])
    }

    pub fn get(&self, name: &str) -> Option<&OptionValue> {
        self.find_index(name).map(|index| &self.values[index])
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
        let value = self.definitions[index].parse_value(raw)?;
        self.values[index] = value;
        Ok(())
    }

    fn option_index(&self, name: &str) -> AvResult<usize> {
        self.find_index(name)
            .ok_or_else(|| AvError::new(AvErrorKind::NotFound, format!("unknown option `{name}`")))
    }

    fn find_index(&self, name: &str) -> Option<usize> {
        self.definitions
            .iter()
            .position(|definition| ascii_eq_ignore_case(definition.name(), name))
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

fn validate_help(help: &str) -> AvResult<String> {
    if help.as_bytes().contains(&0) {
        return Err(AvError::invalid_argument(
            "option help must not contain NUL",
        ));
    }

    Ok(help.to_owned())
}

fn validate_kind(kind: &OptionKind) -> AvResult<()> {
    match *kind {
        OptionKind::Bool | OptionKind::String { .. } => Ok(()),
        OptionKind::Int { min, max } => {
            if min > max {
                return Err(AvError::invalid_argument(
                    "integer option min must be <= max",
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
    }
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
    fn set_from_str_parses_supported_value_types() {
        let mut options = sample_options();

        options.set_from_str("threads", "8").unwrap();
        options.set_from_str("bitexact", "yes").unwrap();
        options.set_from_str("quality", "0.75").unwrap();
        options.set_from_str("metadata", "title=clip").unwrap();

        assert_eq!(options.get("threads"), Some(&OptionValue::Int(8)));
        assert_eq!(options.get("bitexact"), Some(&OptionValue::Bool(true)));
        assert_eq!(options.get("quality"), Some(&OptionValue::Float(0.75)));
        assert_eq!(
            options.get("metadata"),
            Some(&OptionValue::String("title=clip".to_string()))
        );
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
        assert!(options.set_from_str("threads", "0").is_err());
        assert!(options.set_from_str("quality", "inf").is_err());
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
        options
            .define(
                OptionDefinition::new_with_flags(
                    "exported",
                    OptionKind::Int { min: 0, max: 8 },
                    OptionValue::Int(4),
                    "read-only exported value",
                    OptionFlags::from_bits_truncate(
                        OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
                    ),
                )
                .unwrap(),
            )
            .unwrap();

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
    }
}
