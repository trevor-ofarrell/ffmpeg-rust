use crate::{AvError, AvErrorCode, AvErrorKind, AvResult, Rational};

#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Rational(Rational),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionKind {
    Bool,
    Int { min: i64, max: i64 },
    Float { min: f64, max: f64 },
    Rational { min: Rational, max: Rational },
    String { allow_empty: bool },
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

    const KNOWN_BITS: u32 = Self::CHILDREN.bits | Self::FAKE_OBJ.bits;

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
        if self.find_index(definition.name()).is_some() {
            return Err(AvError::invalid_argument(format!(
                "duplicate option `{}`",
                definition.name()
            )));
        }

        let entry_key = OptionEntryKey::definition(definition.name());
        self.values.push(definition.default().clone());
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

    pub fn get_child_option(&self, child_name: &str, option_name: &str) -> AvResult<&OptionValue> {
        let child = self.child_by_name(child_name)?;
        let index = child.options.option_index(option_name)?;
        Ok(&child.options.values[index])
    }

    pub fn range(&self, name: &str) -> AvResult<Option<OptionRange>> {
        let index = self.option_index(name)?;
        Ok(self.definitions[index].range())
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
        let value = self.parse_avoption_value(index, raw)?;
        self.values[index] = value;
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
        if let Some(unit) = self.definitions[index].unit() {
            if let Some(constant) = self.find_exact_constant(unit, raw) {
                self.definitions[index].validate_value(constant.value())?;
                return Ok(constant.value().clone());
            }
        }

        self.definitions[index].parse_value(raw)
    }

    fn option_index(&self, name: &str) -> AvResult<usize> {
        self.find_index(name)
            .ok_or_else(|| AvError::new(AvErrorKind::NotFound, format!("unknown option `{name}`")))
    }

    fn avoption_index(&self, name: &str) -> AvResult<usize> {
        self.find_exact_index(name).ok_or_else(|| {
            AvError::with_code(
                AvErrorKind::NotFound,
                AvErrorCode::OPTION_NOT_FOUND,
                format!("unknown AVOption `{name}`"),
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
    }
}

fn range_for_kind(kind: &OptionKind) -> Option<OptionRange> {
    match *kind {
        OptionKind::Int { min, max } => Some(OptionRange {
            min: OptionValue::Int(min),
            max: OptionValue::Int(max),
        }),
        OptionKind::Float { min, max } => Some(OptionRange {
            min: OptionValue::Float(min),
            max: OptionValue::Float(max),
        }),
        OptionKind::Rational { min, max } => Some(OptionRange {
            min: OptionValue::Rational(min),
            max: OptionValue::Rational(max),
        }),
        OptionKind::Bool | OptionKind::String { .. } => None,
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

        let truncated = OptionSearchFlags::from_bits_truncate(u32::MAX);

        assert!(truncated.contains(OptionSearchFlags::CHILDREN));
        assert!(truncated.contains(OptionSearchFlags::FAKE_OBJ));
        assert!(truncated.intersects(OptionSearchFlags::CHILDREN));
        assert_eq!(
            truncated.bits(),
            OptionSearchFlags::CHILDREN.bits() | OptionSearchFlags::FAKE_OBJ.bits()
        );
        assert_eq!(OptionSearchFlags::empty().bits(), 0);
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
}
