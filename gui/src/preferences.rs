//! GUI-only, versioned preferences and bounded local persistence.
//!
//! This module has no daemon/client dependency. Imported values cannot schedule
//! acquisition, select an endpoint, grant access, or invoke hardware operations.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::{CString, OsStr};
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const MAX_PROFILE_BYTES: usize = 262_144;
const MAX_DEPTH: usize = 8;
const KIND: &str = "siliconwatch.gui";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    OledBlack,
    Dark,
    Light,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    #[default]
    Compact,
    Comfortable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnId {
    Sensor,
    Current,
    Minimum,
    Maximum,
    Average,
}
impl ColumnId {
    pub const ALL: [Self; 5] = [
        Self::Sensor,
        Self::Current,
        Self::Minimum,
        Self::Maximum,
        Self::Average,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sensor => "Sensor",
            Self::Current => "Current",
            Self::Minimum => "Minimum",
            Self::Maximum => "Maximum",
            Self::Average => "Average",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u32,
    pub profile_kind: String,
    #[serde(default)]
    pub behavior: Behavior,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub windows: Windows,
    #[serde(default)]
    pub table: Table,
    #[serde(default)]
    pub group_overrides: Vec<GroupOverride>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            profile_kind: KIND.into(),
            behavior: Behavior::default(),
            appearance: Appearance::default(),
            windows: Windows::default(),
            table: Table::default(),
            group_overrides: Vec::new(),
        }
    }
}
/// Optional schema-1 GUI behavior. Older profiles retain close-to-exit behavior.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Behavior {
    pub minimize_to_tray: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub theme: Theme,
    pub font_family: String,
    pub font_size_px: u32,
    pub temperature_unit: TemperatureUnit,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::OledBlack,
            font_family: "system-ui".into(),
            font_size_px: 14,
            temperature_unit: TemperatureUnit::Celsius,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Windows {
    pub main: MainWindow,
    pub details: DetailsWindow,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MainWindow {
    pub width_px: u32,
    pub height_px: u32,
    pub maximized: bool,
}
impl Default for MainWindow {
    fn default() -> Self {
        Self {
            width_px: 1120,
            height_px: 680,
            maximized: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetailsWindow {
    pub width_px: u32,
    pub height_px: u32,
}
impl Default for DetailsWindow {
    fn default() -> Self {
        Self {
            width_px: 960,
            height_px: 720,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Table {
    pub column_order: Vec<ColumnId>,
    pub column_widths_px: ColumnWidths,
    pub density: Density,
    pub row_separators: bool,
    pub alternate_rows: bool,
    pub default_expanded: bool,
}
impl Default for Table {
    fn default() -> Self {
        Self {
            column_order: ColumnId::ALL.to_vec(),
            column_widths_px: ColumnWidths::default(),
            density: Density::Compact,
            row_separators: true,
            alternate_rows: false,
            default_expanded: true,
        }
    }
}
// Deliberately no serde(default): a present width map must contain all five keys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnWidths {
    pub sensor: u32,
    pub current: u32,
    pub minimum: u32,
    pub maximum: u32,
    pub average: u32,
}
impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            sensor: 440,
            current: 152,
            minimum: 152,
            maximum: 152,
            average: 152,
        }
    }
}
impl ColumnWidths {
    pub fn get(&self, column: ColumnId) -> u32 {
        match column {
            ColumnId::Sensor => self.sensor,
            ColumnId::Current => self.current,
            ColumnId::Minimum => self.minimum,
            ColumnId::Maximum => self.maximum,
            ColumnId::Average => self.average,
        }
    }
    pub fn set(&mut self, column: ColumnId, width: u32) {
        match column {
            ColumnId::Sensor => self.sensor = width,
            ColumnId::Current => self.current = width,
            ColumnId::Minimum => self.minimum = width,
            ColumnId::Maximum => self.maximum = width,
            ColumnId::Average => self.average = width,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupOverride {
    pub expanded: bool,
    pub members: Vec<MemberSelector>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberSelector {
    pub identity_schema_version: u32,
    pub candidate_id: String,
    pub measurement: String,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferenceError {
    pub message: String,
}
impl PreferenceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
    fn io(action: &'static str, error: io::Error) -> Self {
        // Never format a caller's input bytes or OS filename into an error.
        Self::new(format!("{action}: {:?}", error.kind()))
    }
}
impl fmt::Display for PreferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for PreferenceError {}
type Result<T> = std::result::Result<T, PreferenceError>;
fn invalid(field: &'static str) -> PreferenceError {
    PreferenceError::new(format!("Invalid preference: {field}"))
}
fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(PreferenceError::new("Configuration operation cancelled"))
    } else {
        Ok(())
    }
}

impl Preferences {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(invalid("schema_version (only version 1 is supported)"));
        }
        if self.profile_kind != KIND {
            return Err(invalid("profile_kind"));
        }
        let font = &self.appearance.font_family;
        if font.is_empty()
            || font.len() > 128
            || font.trim() != font
            || ["url(", "local(", "var(", "expression("]
                .iter()
                .any(|prefix| font.to_ascii_lowercase().starts_with(prefix))
            || !font
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')'))
        {
            return Err(invalid("appearance.font_family"));
        }
        range(
            self.appearance.font_size_px,
            10,
            24,
            "appearance.font_size_px",
        )?;
        range(
            self.windows.main.width_px,
            760,
            4096,
            "windows.main.width_px",
        )?;
        range(
            self.windows.main.height_px,
            520,
            2160,
            "windows.main.height_px",
        )?;
        range(
            self.windows.details.width_px,
            640,
            4096,
            "windows.details.width_px",
        )?;
        range(
            self.windows.details.height_px,
            480,
            2160,
            "windows.details.height_px",
        )?;
        if self.table.column_order.len() != 5
            || self
                .table
                .column_order
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != 5
        {
            return Err(invalid("table.column_order"));
        }
        range(
            self.table.column_widths_px.sensor,
            240,
            1024,
            "table.column_widths_px.sensor",
        )?;
        for column in &ColumnId::ALL[1..] {
            range(
                self.table.column_widths_px.get(*column),
                112,
                512,
                "table.column_widths_px (numeric column)",
            )?;
        }
        if self.group_overrides.len() > 256 {
            return Err(invalid("group_overrides (maximum 256)"));
        }
        let mut all_selectors = BTreeSet::new();
        for group in &self.group_overrides {
            if group.members.is_empty() || group.members.len() > 16 {
                return Err(invalid("group_overrides.members (1–16 required)"));
            }
            for member in &group.members {
                member.validate()?;
                if !all_selectors.insert(member) {
                    return Err(invalid(
                        "group_overrides (duplicate or conflicting member selector)",
                    ));
                }
            }
        }
        Ok(())
    }
}
fn range(n: u32, low: u32, high: u32, field: &'static str) -> Result<()> {
    if (low..=high).contains(&n) {
        Ok(())
    } else {
        Err(invalid(field))
    }
}
impl MemberSelector {
    pub fn validate(&self) -> Result<()> {
        if self.identity_schema_version != 1
            || self.candidate_id.is_empty()
            || self.candidate_id.len() > 4096
        {
            return Err(invalid("group_overrides.members.identity"));
        }
        let prefix = match (self.measurement.as_str(), self.unit.as_str()) {
            ("temperature_input_millidegree_celsius", "°C") => "temp",
            ("fan_speed_input_rpm", "RPM") => "fan",
            ("voltage_input_millivolt", "V") => "in",
            _ => return Err(invalid("group_overrides.members.measurement/unit")),
        };
        let tuple: (String, u32, String, String, String, String) =
            serde_json::from_str(&self.candidate_id)
                .map_err(|_| invalid("group_overrides.members.candidate_id"))?;
        let canonical = serde_json::to_string(&tuple)
            .map_err(|_| invalid("group_overrides.members.candidate_id"))?;
        let channel_number = tuple
            .4
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix("_input"));
        if tuple.0 != "siliconwatch.hwmon-source"
            || tuple.1 != 1
            || tuple.5 != self.measurement
            || [&tuple.2, &tuple.3, &tuple.4]
                .iter()
                .any(|s| s.is_empty() || s.chars().any(char::is_control))
            || canonical != self.candidate_id
            || !channel_number
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(invalid(
                "group_overrides.members.candidate_id (inconsistent identity evidence)",
            ));
        }
        Ok(())
    }
}

pub fn parse_profile(bytes: &[u8]) -> Result<Preferences> {
    if bytes.len() > MAX_PROFILE_BYTES {
        return Err(PreferenceError::new("Configuration exceeds 256 KiB"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PreferenceError::new("Configuration must be UTF-8"))?;
    check_depth(text)?;
    let preferences: Preferences = toml::from_str(text).map_err(|e| {
        let offset = e.span().map_or(0, |s| s.start.min(text.len()));
        let prefix = &text.as_bytes()[..offset];
        let line = prefix.iter().filter(|b| **b == b'\n').count() + 1;
        let column = prefix.iter().rposition(|b| *b == b'\n').map_or(offset + 1, |i| offset - i);
        PreferenceError::new(format!("Invalid TOML or unknown/missing/incorrectly typed preference at line {line}, byte column {column}"))
    })?;
    preferences.validate()?;
    Ok(preferences)
}
pub fn serialize_profile(preferences: &Preferences) -> Result<Vec<u8>> {
    preferences.validate()?;
    let bytes = toml::to_string_pretty(preferences)
        .map_err(|_| PreferenceError::new("Unable to serialize preferences"))?
        .into_bytes();
    if parse_profile(&bytes)? != *preferences {
        return Err(PreferenceError::new(
            "Preference serialization did not round-trip",
        ));
    }
    Ok(bytes)
}

// This bounded structural pre-parser runs BEFORE toml allocates its document.
// It recognizes containers/keys while treating all strings/comments as opaque.
// The TOML decoder, not this scanner, decides syntax, duplicates, and semantics.
fn check_depth(text: &str) -> Result<()> {
    const ATOM: u8 = b'a';
    let bytes = text.as_bytes();
    let mut tokens = Vec::with_capacity(bytes.len().min(4096));
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b' ' | b'\t' | b'\r' => i += 1,
            quote @ (b'\'' | b'"') => {
                let multiline = bytes
                    .get(i..i + 3)
                    .is_some_and(|s| s == [quote, quote, quote]);
                i += if multiline { 3 } else { 1 };
                let mut closed = false;
                while i < bytes.len() {
                    if quote == b'"' && bytes[i] == b'\\' {
                        i = (i + 2).min(bytes.len());
                        continue;
                    }
                    if bytes[i] == quote {
                        let count = bytes[i..].iter().take_while(|b| **b == quote).count();
                        if !multiline {
                            i += 1;
                            closed = true;
                            break;
                        }
                        if count >= 3 {
                            i += count;
                            closed = true;
                            break;
                        }
                        i += count;
                    } else {
                        i += 1;
                    }
                }
                if !closed {
                    return Err(PreferenceError::new("Unterminated TOML string"));
                }
                tokens.push(ATOM);
            }
            c @ (b'[' | b']' | b'{' | b'}' | b'=' | b'.' | b',' | b'\n') => {
                tokens.push(c);
                i += 1;
            }
            _ => {
                tokens.push(ATOM);
                while i < bytes.len()
                    && !matches!(
                        bytes[i],
                        b' ' | b'\t'
                            | b'\r'
                            | b'\n'
                            | b'#'
                            | b'\''
                            | b'"'
                            | b'['
                            | b']'
                            | b'{'
                            | b'}'
                            | b'='
                            | b'.'
                            | b','
                    )
                {
                    i += 1;
                }
            }
        }
    }
    struct Guard<'a> {
        tokens: &'a [u8],
        i: usize,
    }
    impl Guard<'_> {
        fn peek(&self) -> Option<u8> {
            self.tokens.get(self.i).copied()
        }
        fn whitespace(&mut self) {
            while self.peek() == Some(b'\n') {
                self.i += 1;
            }
        }
        fn depth(depth: usize) -> Result<()> {
            if depth > MAX_DEPTH {
                Err(PreferenceError::new(
                    "Configuration structural nesting exceeds 8",
                ))
            } else {
                Ok(())
            }
        }
        fn key(&mut self, end: u8) -> Result<usize> {
            let mut segments = 1;
            while let Some(t) = self.peek() {
                if t == end {
                    self.i += 1;
                    return Ok(segments);
                }
                if t == b'.' {
                    segments += 1;
                    Self::depth(segments)?;
                } else if t != b'a' {
                    return Err(PreferenceError::new("Invalid TOML key or table header"));
                }
                self.i += 1;
            }
            Err(PreferenceError::new("Incomplete TOML key or table header"))
        }
        fn value(&mut self, depth: usize) -> Result<()> {
            Self::depth(depth)?;
            match self.peek() {
                Some(b'[') => {
                    self.i += 1;
                    loop {
                        self.whitespace();
                        if self.peek() == Some(b']') {
                            self.i += 1;
                            return Ok(());
                        }
                        self.value(depth + 1)?;
                        self.whitespace();
                        match self.peek() {
                            Some(b',') => self.i += 1,
                            Some(b']') => {}
                            _ => return Err(PreferenceError::new("Invalid TOML array")),
                        }
                    }
                }
                Some(b'{') => {
                    self.i += 1;
                    loop {
                        self.whitespace();
                        if self.peek() == Some(b'}') {
                            self.i += 1;
                            return Ok(());
                        }
                        let segments = self.key(b'=')?;
                        self.value(depth + segments)?;
                        self.whitespace();
                        match self.peek() {
                            Some(b',') => self.i += 1,
                            Some(b'}') => {}
                            _ => return Err(PreferenceError::new("Invalid TOML inline table")),
                        }
                    }
                }
                Some(_) => {
                    let start = self.i;
                    while let Some(t) = self.peek() {
                        if matches!(t, b',' | b']' | b'}' | b'\n') {
                            break;
                        }
                        if matches!(t, b'[' | b'{') {
                            return Err(PreferenceError::new("Invalid TOML value"));
                        }
                        self.i += 1;
                    }
                    if self.i == start {
                        return Err(PreferenceError::new("Missing TOML value"));
                    }
                    Ok(())
                }
                None => Err(PreferenceError::new("Missing TOML value")),
            }
        }
    }
    let mut guard = Guard {
        tokens: &tokens,
        i: 0,
    };
    let mut table_depth = 0;
    while guard.peek().is_some() {
        guard.whitespace();
        if guard.peek().is_none() {
            break;
        }
        if guard.peek() == Some(b'[') {
            guard.i += 1;
            let array = guard.peek() == Some(b'[');
            if array {
                guard.i += 1;
            }
            table_depth = guard.key(b']')? + usize::from(array);
            Guard::depth(table_depth)?;
            if array {
                if guard.peek() != Some(b']') {
                    return Err(PreferenceError::new("Invalid TOML array table"));
                }
                guard.i += 1;
            }
        } else {
            let segments = guard.key(b'=')?;
            guard.value(table_depth + segments)?;
        }
        if !matches!(guard.peek(), None | Some(b'\n')) {
            return Err(PreferenceError::new("Unexpected trailing TOML content"));
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct PathChoice {
    pub path: PathBuf,
    pub warnings: Vec<String>,
}
pub fn default_config_path() -> Result<PathChoice> {
    choose_config_path(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}
fn choose_config_path(xdg: Option<&OsStr>, home: Option<&OsStr>) -> Result<PathChoice> {
    let mut warnings = Vec::new();
    if let Some(value) = xdg.filter(|v| !v.is_empty()) {
        let path = Path::new(value);
        if path.is_absolute() {
            return Ok(PathChoice {
                path: path.join("siliconwatch/config.toml"),
                warnings,
            });
        }
        warnings.push(
            "Relative XDG_CONFIG_HOME ignored; using the home configuration directory".into(),
        );
    }
    let home = home
        .filter(|v| !v.is_empty())
        .map(Path::new)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| {
            PreferenceError::new(
                "No absolute home/configuration directory; preferences are session-only",
            )
        })?;
    Ok(PathChoice {
        path: home.join(".config/siliconwatch/config.toml"),
        warnings,
    })
}

#[derive(Clone, PartialEq, Eq)]
pub struct Fingerprint {
    metadata: Option<FileIdentity>,
    bytes: Option<Vec<u8>>,
}
impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fingerprint")
            .field("present", &self.metadata.is_some())
            .finish_non_exhaustive()
    }
}
impl Fingerprint {
    fn missing() -> Self {
        Self {
            metadata: None,
            bytes: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileIdentity {
    dev: u64,
    ino: u64,
    len: u64,
    mode: u32,
    uid: u32,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl FileIdentity {
    fn new(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            len: m.len(),
            mode: m.mode(),
            uid: m.uid(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct RecoveryInfo {
    pub reason: String,
    pub original_bytes: u64,
}
#[derive(Clone, Debug)]
pub struct LoadReport {
    pub preferences: Preferences,
    pub recovery: Option<RecoveryInfo>,
    pub fingerprint: Fingerprint,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct SaveOutcome {
    pub preferences: Preferences,
    pub durability_warning: Option<String>,
}
#[derive(Debug)]
pub struct ExportPlan {
    pub path: PathBuf,
    pub replaces_existing: bool,
    bytes: Vec<u8>,
    expected: Fingerprint,
    forbidden: Vec<PathBuf>,
}
#[derive(Clone, Debug)]
pub struct ExportOutcome {
    pub path: PathBuf,
    pub durability_warning: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailPoint {
    None,
    TempCreate,
    TempSync,
    Backup,
    Rename,
    DirectorySync,
    Recovery,
}
fn inject(point: FailPoint, actual: FailPoint) -> Result<()> {
    if point == actual {
        Err(PreferenceError::new("Injected configuration I/O failure"))
    } else {
        Ok(())
    }
}

pub struct Store {
    path: PathBuf,
    expected: Fingerprint,
    recovery: bool,
}
impl Store {
    pub fn load(path: PathBuf) -> Result<(Self, LoadReport)> {
        let report = load_report(&path)?;
        let store = Self {
            path,
            expected: report.fingerprint.clone(),
            recovery: report.recovery.is_some(),
        };
        Ok((store, report))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn reload(&mut self) -> Result<LoadReport> {
        let report = load_report(&self.path)?;
        self.expected = report.fingerprint.clone();
        self.recovery = report.recovery.is_some();
        Ok(report)
    }
    pub fn read_backup(&self) -> Result<Preferences> {
        read_import(&backup_path(&self.path))
    }
    pub fn save(
        &mut self,
        preferences: &Preferences,
        recover_original: bool,
    ) -> Result<SaveOutcome> {
        self.save_internal(
            preferences,
            recover_original,
            &AtomicBool::new(false),
            FailPoint::None,
        )
    }
    fn save_internal(
        &mut self,
        preferences: &Preferences,
        recover_original: bool,
        cancel: &AtomicBool,
        fail: FailPoint,
    ) -> Result<SaveOutcome> {
        check_cancel(cancel)?;
        let bytes = serialize_profile(preferences)?;
        if self.recovery && !recover_original {
            return Err(PreferenceError::new(
                "Configuration needs explicit recovery; automatic saving is disabled",
            ));
        }
        let (parent, name) = split_path(&self.path)?;
        let dir = SecureDir::open(parent, true, true)?;
        let _lock = dir.lock()?;
        let current = dir.read_optional(name, true)?;
        if current != self.expected {
            return Err(PreferenceError::new(
                "Configuration changed on disk; reload and review before saving",
            ));
        }
        let temp = dir.temporary(&bytes, cancel, fail)?;
        if let Some(old) = &current.bytes {
            if parse_profile(old).is_ok() {
                inject(fail, FailPoint::Backup)?;
                dir.replace_auxiliary(&suffix_name(name, ".bak")?, old, cancel)?;
            } else {
                if !recover_original {
                    return Err(PreferenceError::new(
                        "Invalid configuration requires explicit recovery",
                    ));
                }
                inject(fail, FailPoint::Recovery)?;
                dir.preserve_original(name, &current, cancel)?;
            }
        } else if current.metadata.is_some() {
            if !recover_original {
                return Err(PreferenceError::new(
                    "Oversized configuration requires explicit recovery",
                ));
            }
            inject(fail, FailPoint::Recovery)?;
            dir.preserve_original(name, &current, cancel)?;
        }
        check_cancel(cancel)?;
        if dir.read_optional(name, true)? != current {
            return Err(PreferenceError::new(
                "Configuration changed during save; reload and review",
            ));
        }
        inject(fail, FailPoint::Rename)?;
        temp.commit(name)?;
        // The rename is the commit point. Subsequent failures are warnings and
        // must not pretend the previous preference model is still on disk.
        let mut durability_warning = if inject(fail, FailPoint::DirectorySync)
            .and_then(|_| dir.sync())
            .is_err()
        {
            Some("Settings saved; crash durability could not be confirmed".into())
        } else {
            None
        };
        self.expected = match dir.read_optional(name, true) {
            Ok(version) => version,
            Err(_) => {
                durability_warning = Some("Settings saved; the committed file could not be rechecked. Reload before saving again".into());
                Fingerprint {
                    metadata: None,
                    bytes: Some(bytes),
                } // cannot equal a missing file
            }
        };
        self.recovery = false;
        Ok(SaveOutcome {
            preferences: preferences.clone(),
            durability_warning,
        })
    }
    pub fn prepare_export(
        &self,
        path: PathBuf,
        preferences: &Preferences,
        include_groups: bool,
    ) -> Result<ExportPlan> {
        let mut plan = prepare_export(path, preferences, include_groups)?;
        plan.forbidden = vec![self.path.clone(), backup_path(&self.path)];
        reject_export_alias(&plan)?;
        Ok(plan)
    }
}
fn load_report(path: &Path) -> Result<LoadReport> {
    let (parent, name) = split_path(path)?;
    let fingerprint = match SecureDir::open(parent, false, true) {
        Ok(dir) => dir.read_optional(name, true)?,
        Err(e) if e.message == "Configuration directory does not exist" => Fingerprint::missing(),
        Err(e) => return Err(e),
    };
    let parsed = match (&fingerprint.metadata, &fingerprint.bytes) {
        (None, _) => Ok(Preferences::default()),
        (Some(_), Some(bytes)) => parse_profile(bytes),
        _ => Err(PreferenceError::new("Configuration exceeds 256 KiB")),
    };
    let (preferences, recovery) = match parsed {
        Ok(p) => (p, None),
        Err(e) => (
            Preferences::default(),
            Some(RecoveryInfo {
                reason: e.message,
                original_bytes: fingerprint.metadata.as_ref().map_or(0, |m| m.len),
            }),
        ),
    };
    let warnings = if preferences.group_overrides.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "{} saved device group preferences require source review before activation",
            preferences.group_overrides.len()
        )]
    };
    Ok(LoadReport {
        preferences,
        recovery,
        fingerprint,
        warnings,
    })
}
pub fn read_import(path: &Path) -> Result<Preferences> {
    read_import_internal(path, &AtomicBool::new(false))
}
fn read_import_internal(path: &Path, cancel: &AtomicBool) -> Result<Preferences> {
    check_cancel(cancel)?;
    let (parent, name) = split_path(path)?;
    let dir = SecureDir::open(parent, false, false)?;
    let version = dir.read_optional_with_cancel(name, false, cancel)?;
    if version.metadata.is_none() {
        return Err(PreferenceError::new("Import file does not exist"));
    }
    let bytes = version
        .bytes
        .ok_or_else(|| PreferenceError::new("Configuration exceeds 256 KiB"))?;
    check_cancel(cancel)?;
    let preferences = parse_profile(&bytes)?;
    check_cancel(cancel)?;
    Ok(preferences)
}
pub fn prepare_export(
    path: PathBuf,
    preferences: &Preferences,
    include_groups: bool,
) -> Result<ExportPlan> {
    let mut candidate = preferences.clone();
    if !include_groups {
        candidate.group_overrides.clear();
    }
    let bytes = serialize_profile(&candidate)?;
    let (parent, name) = split_path(&path)?;
    let dir = SecureDir::open(parent, false, false)?;
    let expected = dir.read_optional(name, true)?;
    Ok(ExportPlan {
        path,
        replaces_existing: expected.metadata.is_some(),
        bytes,
        expected,
        forbidden: Vec::new(),
    })
}
pub fn commit_export(plan: ExportPlan) -> Result<ExportOutcome> {
    commit_export_internal(plan, &AtomicBool::new(false))
}
fn commit_export_internal(plan: ExportPlan, cancel: &AtomicBool) -> Result<ExportOutcome> {
    check_cancel(cancel)?;
    parse_profile(&plan.bytes)?;
    reject_export_alias(&plan)?;
    let (parent, name) = split_path(&plan.path)?;
    let dir = SecureDir::open(parent, false, false)?;
    let _lock = dir.lock()?;
    if dir.read_optional(name, true)? != plan.expected {
        return Err(PreferenceError::new(
            "Export destination changed; preview it again",
        ));
    }
    let temp = dir.temporary(&plan.bytes, cancel, FailPoint::None)?;
    check_cancel(cancel)?;
    reject_export_alias(&plan)?;
    if dir.read_optional(name, true)? != plan.expected {
        return Err(PreferenceError::new(
            "Export destination changed during export",
        ));
    }
    temp.commit(name)?;
    let durability_warning = dir
        .sync()
        .err()
        .map(|_| "Settings exported; crash durability could not be confirmed".into());
    Ok(ExportOutcome {
        path: plan.path,
        durability_warning,
    })
}
fn reject_export_alias(plan: &ExportPlan) -> Result<()> {
    for forbidden in &plan.forbidden {
        if plan.path.components().eq(forbidden.components()) {
            return Err(PreferenceError::new(
                "Export cannot replace the active configuration or backup",
            ));
        }
        let (parent, name) = split_path(forbidden)?;
        if let Ok(dir) = SecureDir::open(parent, false, false) {
            if let Some(meta) = dir.read_optional(name, true)?.metadata {
                if plan
                    .expected
                    .metadata
                    .as_ref()
                    .is_some_and(|other| meta.dev == other.dev && meta.ino == other.ino)
                {
                    return Err(PreferenceError::new(
                        "Export destination aliases the active configuration or backup",
                    ));
                }
            }
        }
    }
    Ok(())
}
fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".bak");
    PathBuf::from(name)
}
fn split_path(path: &Path) -> Result<(&Path, &OsStr)> {
    if path.as_os_str().as_bytes().len() > 4096
        || !path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
    {
        return Err(PreferenceError::new(
            "Choose an absolute local path without parent traversal",
        ));
    }
    Ok((
        path.parent()
            .ok_or_else(|| PreferenceError::new("File needs a parent directory"))?,
        path.file_name()
            .ok_or_else(|| PreferenceError::new("Choose a file name"))?,
    ))
}
fn cstring(value: &OsStr) -> Result<CString> {
    CString::new(value.as_bytes()).map_err(|_| PreferenceError::new("Path contains a NUL byte"))
}
fn suffix_name(name: &OsStr, suffix: &str) -> Result<CString> {
    let mut bytes = name.as_bytes().to_vec();
    bytes.extend_from_slice(suffix.as_bytes());
    CString::new(bytes).map_err(|_| PreferenceError::new("Invalid file name"))
}
fn owned_file(fd: i32, action: &'static str) -> Result<File> {
    if fd < 0 {
        Err(PreferenceError::io(action, io::Error::last_os_error()))
    } else {
        // SAFETY: successful open/openat returns one newly owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
fn user_id() -> Result<u32> {
    // SAFETY: these calls have no arguments or side effects.
    let (real, effective) = unsafe { (libc::getuid(), libc::geteuid()) };
    if real != effective {
        Err(PreferenceError::new(
            "Preference persistence requires matching real/effective user IDs",
        ))
    } else {
        Ok(effective)
    }
}
struct SecureDir {
    file: File,
    uid: u32,
}
impl SecureDir {
    fn open(path: &Path, create: bool, private: bool) -> Result<Self> {
        if !path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
        {
            return Err(PreferenceError::new(
                "Configuration directory must be an absolute path without traversal",
            ));
        }
        let uid = user_id()?;
        // SAFETY: static NUL-terminated root pathname and valid flags.
        let mut file = owned_file(
            unsafe {
                libc::open(
                    c"/".as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
                )
            },
            "Open root directory",
        )?;
        let components: Vec<_> = path
            .components()
            .filter_map(|c| {
                if let Component::Normal(n) = c {
                    Some(n)
                } else {
                    None
                }
            })
            .collect();
        for (index, component) in components.iter().enumerate() {
            let name = cstring(component)?;
            let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
            // SAFETY: parent fd is live, C string is valid, openat retains neither.
            let mut fd = unsafe { libc::openat(file.as_raw_fd(), name.as_ptr(), flags) };
            if fd < 0 && io::Error::last_os_error().kind() == io::ErrorKind::NotFound {
                if !create {
                    return Err(PreferenceError::new(
                        "Configuration directory does not exist",
                    ));
                }
                // SAFETY: same live parent/path; only a missing component is created.
                let result = unsafe { libc::mkdirat(file.as_raw_fd(), name.as_ptr(), 0o700) };
                if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists {
                    return Err(PreferenceError::io(
                        "Create configuration directory",
                        io::Error::last_os_error(),
                    ));
                }
                if result == 0 {
                    file.sync_all().map_err(|e| {
                        PreferenceError::io("Sync newly created directory entry", e)
                    })?;
                }
                // SAFETY: path is re-opened without following a raced-in symlink.
                fd = unsafe { libc::openat(file.as_raw_fd(), name.as_ptr(), flags) };
            }
            file = owned_file(fd, "Open configuration directory")?;
            let meta = file
                .metadata()
                .map_err(|e| PreferenceError::io("Inspect configuration directory", e))?;
            let sticky_shared = meta.uid() == 0 && meta.mode() & 0o1777 == 0o1777;
            if (meta.uid() != uid && meta.uid() != 0)
                || (meta.mode() & 0o022 != 0 && !sticky_shared)
            {
                return Err(PreferenceError::new(
                    "Configuration directory has unsafe ownership or permissions",
                ));
            }
            if index + 1 == components.len()
                && (meta.uid() != uid || (private && meta.mode() & 0o077 != 0))
            {
                return Err(PreferenceError::new(
                    "Destination directory must be user-owned; local configuration directory must be private",
                ));
            }
        }
        Ok(Self { file, uid })
    }
    fn fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
    fn sync(&self) -> Result<()> {
        self.file
            .sync_all()
            .map_err(|e| PreferenceError::io("Sync configuration directory", e))
    }
    fn open_regular(&self, name: &OsStr, owned: bool) -> Result<Option<File>> {
        let name = cstring(name)?;
        // NONBLOCK prevents a malicious FIFO from blocking before fstat rejects it.
        // SAFETY: valid live directory descriptor and borrowed NUL-terminated name.
        let fd = unsafe {
            libc::openat(
                self.fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if fd < 0 && io::Error::last_os_error().kind() == io::ErrorKind::NotFound {
            return Ok(None);
        }
        let file = owned_file(fd, "Open configuration file")?;
        let metadata = file
            .metadata()
            .map_err(|e| PreferenceError::io("Inspect configuration file", e))?;
        if !metadata.is_file()
            || (owned && (metadata.uid() != self.uid || metadata.mode() & 0o022 != 0))
        {
            return Err(PreferenceError::new(
                "Configuration must be a regular file with safe ownership and permissions",
            ));
        }
        Ok(Some(file))
    }
    fn read_optional(&self, name: &OsStr, owned: bool) -> Result<Fingerprint> {
        self.read_optional_with_cancel(name, owned, &AtomicBool::new(false))
    }
    fn read_optional_with_cancel(
        &self,
        name: &OsStr,
        owned: bool,
        cancel: &AtomicBool,
    ) -> Result<Fingerprint> {
        check_cancel(cancel)?;
        let Some(mut file) = self.open_regular(name, owned)? else {
            return Ok(Fingerprint::missing());
        };
        let before = FileIdentity::new(
            &file
                .metadata()
                .map_err(|e| PreferenceError::io("Inspect configuration file", e))?,
        );
        let mut bytes = Vec::with_capacity(before.len.min((MAX_PROFILE_BYTES + 1) as u64) as usize);
        let mut buffer = [0_u8; 8192];
        while bytes.len() <= MAX_PROFILE_BYTES {
            check_cancel(cancel)?;
            let capacity = (MAX_PROFILE_BYTES + 1 - bytes.len()).min(buffer.len());
            let n = file
                .read(&mut buffer[..capacity])
                .map_err(|e| PreferenceError::io("Read configuration file", e))?;
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..n]);
        }
        check_cancel(cancel)?;
        let after = FileIdentity::new(
            &file
                .metadata()
                .map_err(|e| PreferenceError::io("Reinspect configuration file", e))?,
        );
        if before != after {
            return Err(PreferenceError::new(
                "Configuration changed while reading; retry after review",
            ));
        }
        Ok(Fingerprint {
            metadata: Some(after),
            bytes: (bytes.len() <= MAX_PROFILE_BYTES).then_some(bytes),
        })
    }
    fn lock(&self) -> Result<File> {
        // One stable lock inode per destination directory; never remove it while
        // other cooperating processes may still hold its descriptor.
        // SAFETY: live dir fd, fixed local name, valid flags/mode.
        let fd = unsafe {
            libc::openat(
                self.fd(),
                c".siliconwatch-settings.lock".as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_CLOEXEC
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK,
                0o600,
            )
        };
        let file = owned_file(fd, "Open settings lock")?;
        let m = file
            .metadata()
            .map_err(|e| PreferenceError::io("Inspect settings lock", e))?;
        if !m.is_file() || m.uid() != self.uid || m.mode() & 0o077 != 0 || m.nlink() != 1 {
            return Err(PreferenceError::new("Unsafe settings lock file"));
        }
        // SAFETY: live file descriptor; nonblocking advisory lock changes no data.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } < 0 {
            return Err(PreferenceError::new(
                "Another settings writer is active; try again",
            ));
        }
        Ok(file)
    }
    fn create_random(&self, prefix: &str) -> Result<(CString, File)> {
        for _ in 0..8 {
            let mut bytes = [0_u8; 16];
            let mut offset = 0;
            while offset < bytes.len() {
                // SAFETY: pointer/length identify the unfilled portion of a live buffer.
                let n = unsafe {
                    libc::getrandom(
                        bytes[offset..].as_mut_ptr().cast(),
                        bytes.len() - offset,
                        libc::GRND_NONBLOCK,
                    )
                };
                if n <= 0 {
                    return Err(PreferenceError::new(
                        "Unable to obtain randomness for a temporary configuration file",
                    ));
                }
                offset += n as usize;
            }
            let mut token = String::with_capacity(32);
            const HEX: &[u8; 16] = b"0123456789abcdef";
            for byte in bytes {
                token.push(HEX[usize::from(byte >> 4)] as char);
                token.push(HEX[usize::from(byte & 15)] as char);
            }
            let name =
                CString::new(format!(".{prefix}-{token}")).expect("fixed ASCII temporary name");
            // SAFETY: live dir fd; exclusive no-follow create cannot overwrite a target.
            let fd = unsafe {
                libc::openat(
                    self.fd(),
                    name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_CLOEXEC
                        | libc::O_NOFOLLOW,
                    0o600,
                )
            };
            if fd < 0 && io::Error::last_os_error().kind() == io::ErrorKind::AlreadyExists {
                continue;
            }
            return Ok((name, owned_file(fd, "Create temporary configuration")?));
        }
        Err(PreferenceError::new(
            "Temporary configuration name collision limit reached",
        ))
    }
    fn temporary<'a>(
        &'a self,
        bytes: &[u8],
        cancel: &AtomicBool,
        fail: FailPoint,
    ) -> Result<Temporary<'a>> {
        inject(fail, FailPoint::TempCreate)?;
        let (name, mut file) = self.create_random("siliconwatch-tmp")?;
        let temporary = Temporary {
            dir: self,
            name,
            committed: false,
        };
        for chunk in bytes.chunks(8192) {
            check_cancel(cancel)?;
            file.write_all(chunk)
                .map_err(|e| PreferenceError::io("Write temporary configuration", e))?;
        }
        file.flush()
            .map_err(|e| PreferenceError::io("Flush temporary configuration", e))?;
        inject(fail, FailPoint::TempSync)?;
        file.sync_all()
            .map_err(|e| PreferenceError::io("Sync temporary configuration", e))?;
        Ok(temporary)
    }
    fn replace_auxiliary(&self, name: &CString, bytes: &[u8], cancel: &AtomicBool) -> Result<()> {
        let name = OsStr::from_bytes(name.as_bytes());
        self.read_optional(name, true)?; // rejects symlinks/special files before replacement
        let temp = self.temporary(bytes, cancel, FailPoint::None)?;
        temp.commit(name)?;
        self.sync()
    }
    fn preserve_original(
        &self,
        name: &OsStr,
        expected: &Fingerprint,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let Some(mut source) = self.open_regular(name, true)? else {
            return Err(PreferenceError::new(
                "Original configuration disappeared during recovery",
            ));
        };
        let meta = FileIdentity::new(
            &source
                .metadata()
                .map_err(|e| PreferenceError::io("Inspect recovery source", e))?,
        );
        if expected.metadata.as_ref() != Some(&meta) {
            return Err(PreferenceError::new(
                "Original configuration changed before recovery",
            ));
        }
        let (name, mut destination) = self.create_random("siliconwatch-recovery")?;
        let mut recovery = Temporary {
            dir: self,
            name,
            committed: false,
        };
        let mut remaining = meta.len;
        let mut buffer = [0_u8; 8192];
        while remaining > 0 {
            check_cancel(cancel)?;
            let capacity = remaining.min(buffer.len() as u64) as usize;
            let n = source
                .read(&mut buffer[..capacity])
                .map_err(|e| PreferenceError::io("Read recovery source", e))?;
            if n == 0 {
                return Err(PreferenceError::new(
                    "Recovery source shortened while copying",
                ));
            }
            destination
                .write_all(&buffer[..n])
                .map_err(|e| PreferenceError::io("Preserve recovery original", e))?;
            remaining -= n as u64;
        }
        if FileIdentity::new(
            &source
                .metadata()
                .map_err(|e| PreferenceError::io("Reinspect recovery source", e))?,
        ) != meta
        {
            return Err(PreferenceError::new(
                "Recovery source changed while copying",
            ));
        }
        destination
            .sync_all()
            .map_err(|e| PreferenceError::io("Sync preserved recovery original", e))?;
        self.sync()?;
        recovery.committed = true; // uniquely named preserved evidence is intentionally retained
        Ok(())
    }
}
struct Temporary<'a> {
    dir: &'a SecureDir,
    name: CString,
    committed: bool,
}
impl Temporary<'_> {
    fn commit(mut self, destination: &OsStr) -> Result<()> {
        let destination = cstring(destination)?;
        // SAFETY: both names are valid, descriptor-anchored entries in one live directory.
        if unsafe {
            libc::renameat(
                self.dir.fd(),
                self.name.as_ptr(),
                self.dir.fd(),
                destination.as_ptr(),
            )
        } < 0
        {
            return Err(PreferenceError::io(
                "Commit configuration rename",
                io::Error::last_os_error(),
            ));
        }
        self.committed = true;
        Ok(())
    }
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        if !self.committed {
            // SAFETY: removes only this operation's exclusive temporary entry.
            let _ = unsafe { libc::unlinkat(self.dir.fd(), self.name.as_ptr(), 0) };
        }
    }
}

pub enum ConfigJob {
    Import(PathBuf),
    Save {
        preferences: Preferences,
        recover_original: bool,
    },
    ReadBackup,
    PrepareExport {
        path: PathBuf,
        preferences: Preferences,
        include_groups: bool,
    },
    CommitExport(ExportPlan),
    Reload,
}
pub enum ConfigResponse {
    Imported(Result<Preferences>),
    Saved(Result<SaveOutcome>),
    Backup(Result<Preferences>),
    ExportPrepared(Result<ExportPlan>),
    Exported(Result<ExportOutcome>),
    Reloaded(Result<LoadReport>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    Busy,
    Stopped,
}
pub struct ConfigWorker {
    sender: mpsc::SyncSender<ConfigJob>,
    receiver: mpsc::Receiver<ConfigResponse>,
    busy: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}
impl ConfigWorker {
    pub fn spawn(mut store: Store) -> Result<Self> {
        let (sender, jobs) = mpsc::sync_channel(1);
        let (responses, receiver) = mpsc::sync_channel(1);
        let busy = Arc::new(AtomicBool::new(false));
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let handle = thread::Builder::new()
            .name("siliconwatch-preferences".into())
            .spawn(move || {
                while !worker_cancel.load(Ordering::Acquire) {
                    let job = match jobs.recv_timeout(Duration::from_millis(50)) {
                        Ok(job) => job,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    };
                    if worker_cancel.load(Ordering::Acquire) {
                        break;
                    }
                    let result = match job {
                        ConfigJob::Import(path) => {
                            ConfigResponse::Imported(read_import_internal(&path, &worker_cancel))
                        }
                        ConfigJob::Save {
                            preferences,
                            recover_original,
                        } => ConfigResponse::Saved(store.save_internal(
                            &preferences,
                            recover_original,
                            &worker_cancel,
                            FailPoint::None,
                        )),
                        ConfigJob::ReadBackup => ConfigResponse::Backup(read_import_internal(
                            &backup_path(store.path()),
                            &worker_cancel,
                        )),
                        ConfigJob::PrepareExport {
                            path,
                            preferences,
                            include_groups,
                        } => ConfigResponse::ExportPrepared(store.prepare_export(
                            path,
                            &preferences,
                            include_groups,
                        )),
                        ConfigJob::CommitExport(plan) => {
                            ConfigResponse::Exported(commit_export_internal(plan, &worker_cancel))
                        }
                        ConfigJob::Reload => ConfigResponse::Reloaded(store.reload()),
                    };
                    // At most one job is admitted until its response is consumed, so
                    // this bounded channel cannot accumulate results or block here.
                    if responses.try_send(result).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| PreferenceError::io("Start configuration worker", e))?;
        Ok(Self {
            sender,
            receiver,
            busy,
            cancel,
            handle: Some(handle),
        })
    }
    pub fn submit(&self, job: ConfigJob) -> std::result::Result<(), SubmitError> {
        if self.cancel.load(Ordering::Acquire) {
            return Err(SubmitError::Stopped);
        }
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(SubmitError::Busy);
        }
        match self.sender.try_send(job) {
            Ok(()) => Ok(()),
            Err(_) => {
                self.busy.store(false, Ordering::Release);
                Err(SubmitError::Stopped)
            }
        }
    }
    pub fn try_recv(&self) -> Option<ConfigResponse> {
        match self.receiver.try_recv() {
            Ok(response) => {
                self.busy.store(false, Ordering::Release);
                Some(response)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.cancel.store(true, Ordering::Release);
                self.busy.store(false, Ordering::Release);
                None
            }
        }
    }
    pub fn request_stop(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    /// Call after the GUI event loop exits, never inside its close callback.
    pub fn join(mut self) -> Result<()> {
        self.request_stop();
        if self.handle.take().is_some_and(|h| h.join().is_err()) {
            return Err(PreferenceError::new("Configuration worker panicked"));
        }
        Ok(())
    }
}
impl Drop for ConfigWorker {
    fn drop(&mut self) {
        self.request_stop();
        // Explicit join is the normal shutdown path. Dropping an unjoined worker
        // cancels it without blocking a GUI callback on a filesystem operation.
    }
}

#[cfg(test)]
mod tests;
