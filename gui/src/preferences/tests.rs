use super::*;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt, symlink};
use std::sync::atomic::AtomicU64;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "siliconwatch-preferences-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let path = root.join("config.toml");
        Self { root, path }
    }
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        path
    }
    fn load(&self) -> (Store, LoadReport) {
        Store::load(self.path.clone()).unwrap()
    }
    fn recoveries(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.root)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".siliconwatch-recovery-")
            })
            .collect()
    }
    fn temps(&self) -> usize {
        fs::read_dir(&self.root)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".siliconwatch-tmp-")
            })
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn minimal() -> &'static str {
    "schema_version = 1\nprofile_kind = \"siliconwatch.gui\"\n"
}
fn parse_extra(extra: &str) -> Result<Preferences> {
    parse_profile(format!("{}{extra}", minimal()).as_bytes())
}
fn member(n: u32) -> MemberSelector {
    MemberSelector {
        identity_schema_version: 1,
        candidate_id: format!(
            "[\"siliconwatch.hwmon-source\",1,\"fixture\",\"/devices/fixture\",\"temp{n}_input\",\"temperature_input_millidegree_celsius\"]"
        ),
        measurement: "temperature_input_millidegree_celsius".into(),
        unit: "°C".into(),
    }
}

#[test]
fn defaults_require_root_contract_and_round_trip() {
    let p = parse_profile(minimal().as_bytes()).unwrap();
    assert_eq!(p, Preferences::default());
    assert_eq!(p.appearance.theme, Theme::OledBlack);
    assert_eq!(p.appearance.temperature_unit, TemperatureUnit::Celsius);
    assert_eq!(parse_profile(&serialize_profile(&p).unwrap()).unwrap(), p);
    for s in [
        "",
        "schema_version=1",
        "profile_kind='siliconwatch.gui'",
        "schema_version=2\nprofile_kind='siliconwatch.gui'",
        "schema_version=0\nprofile_kind='siliconwatch.gui'",
        "schema_version='1'\nprofile_kind='siliconwatch.gui'",
        "schema_version=1\nprofile_kind='daemon'",
    ] {
        assert!(parse_profile(s.as_bytes()).is_err(), "accepted {s}");
    }
}
#[test]
fn all_three_themes_round_trip_and_system_is_rejected() {
    for theme in [Theme::OledBlack, Theme::Dark, Theme::Light] {
        let mut p = Preferences::default();
        p.appearance.theme = theme;
        assert_eq!(
            parse_profile(&serialize_profile(&p).unwrap())
                .unwrap()
                .appearance
                .theme,
            theme
        );
    }
    for theme in ["system", "OLED Black", "black", "Dark", ""] {
        assert!(parse_extra(&format!("[appearance]\ntheme='{theme}'\n")).is_err());
    }
}
#[test]
fn known_omissions_default_but_present_width_map_is_complete() {
    let p = parse_extra("[appearance]\nfont_size_px=20\n[windows.main]\nwidth_px=800\n[table]\nalternate_rows=true\n").unwrap();
    assert_eq!(p.appearance.theme, Theme::OledBlack);
    assert_eq!(p.windows.main.height_px, 680);
    assert!(p.table.alternate_rows);
    assert!(parse_extra("[table.column_widths_px]\nsensor=400\n").is_err());
}
#[test]
fn unknown_future_security_and_nested_fields_are_rejected() {
    for extra in [
        "polling_interval=1000",
        "socket_path='/tmp/private.sock'",
        "api_token='secret'",
        "sensor_preferences=[]",
        "[appearance]\ncolor='#ffffff'",
        "[appearance.extensions]",
        "[daemon]\ninterval_ms=1000",
        "[table]\nhide_errors=true",
        "[windows.details]\nmaximized=true",
        "[table.column_widths_px]\nsensor=440\ncurrent=152\nminimum=152\nmaximum=152\naverage=152\nother=152",
    ] {
        assert!(parse_extra(extra).is_err(), "accepted {extra}");
    }
}
#[test]
fn invalid_syntax_types_and_nonfinite_values_are_rejected_without_input_echo() {
    for extra in [
        "schema_version=1",
        "[appearance]\nfont_size_px=14.0",
        "[appearance]\nfont_size_px=nan",
        "[appearance]\nfont_size_px=inf",
        "[appearance]\nfont_size_px=-1",
        "[appearance]\nfont_size_px=1979-05-27",
        "[table]\nrow_separators='true'",
        "[table]\nrow_separators=1",
        "[appearance]\ntheme='secret-do-not-echo'",
        "[appearance]\ntheme='dark' trailing",
    ] {
        let error = parse_extra(extra).unwrap_err();
        assert!(!error.message.contains("secret-do-not-echo"));
        assert!(error.message.len() < 256);
    }
    assert!(parse_profile(&[0xff, 0xfe]).is_err());
}
#[test]
fn size_limit_is_inclusive() {
    let mut bytes = minimal().as_bytes().to_vec();
    bytes.extend_from_slice(b"#");
    bytes.resize(MAX_PROFILE_BYTES, b'x');
    assert!(parse_profile(&bytes).is_ok());
    bytes.push(b'x');
    assert!(parse_profile(&bytes).is_err());
}
#[test]
fn depth_guard_precedes_parser_and_ignores_strings_and_comments() {
    assert!(check_depth("a=[[[[[[[1]]]]]]]\n").is_ok());
    assert!(
        check_depth("a=[[[[[[[[1]]]]]]]]\n")
            .unwrap_err()
            .message
            .contains("nesting")
    );
    assert!(
        check_depth("[a.b.c.d.e.f.g.h.i]\nx=1\n")
            .unwrap_err()
            .message
            .contains("nesting")
    );
    assert!(
        check_depth("a.b.c.d.e.f.g.h.i=1\n")
            .unwrap_err()
            .message
            .contains("nesting")
    );
    assert!(
        check_depth("a={b={c={d={e={f={g={h={i=1}}}}}}}}\n")
            .unwrap_err()
            .message
            .contains("nesting")
    );
    for text in [
        "a='[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]'\n",
        "a=\"\"\"[a.b.c.d.e.f.g.h.i]\n[[[[[[[[[[[[[\"\"\"\n",
        "a='''{{{{{{{{{{{{{{{{}}}}}}}}}}}}}'''\n",
        "a=\"escaped \\\" [ [ [\"\n# a.b.c.d.e.f.g.h.i\n",
    ] {
        assert!(check_depth(text).is_ok(), "{text}");
    }
}
#[test]
fn toml_quoted_and_dotted_keys_and_array_table_syntax_work() {
    let p = parse_extra("appearance.\"font_size_px\" = 18\n[\"table\"]\ncolumn_order=['average', 'sensor', 'current', 'minimum', 'maximum',]\n").unwrap();
    assert_eq!(p.appearance.font_size_px, 18);
    assert_eq!(p.table.column_order[0], ColumnId::Average);
    let candidate = member(1);
    let s = format!(
        "{}[[group_overrides]]\nexpanded=false\n[[group_overrides.members]]\nidentity_schema_version=1\ncandidate_id='{}'\nmeasurement='{}'\nunit='{}'\n",
        minimal(),
        candidate.candidate_id,
        candidate.measurement,
        candidate.unit
    );
    assert_eq!(
        parse_profile(s.as_bytes()).unwrap().group_overrides[0].members[0],
        candidate
    );
}
#[test]
fn font_names_are_bounded_plain_installed_family_identifiers() {
    for name in [
        "system-ui",
        "Noto Sans CJK",
        "DejaVu Sans Mono",
        "測試字型",
        "IBM Plex Sans (Text)",
    ] {
        let mut p = Preferences::default();
        p.appearance.font_family = name.into();
        assert!(p.validate().is_ok());
    }
    for name in [
        "",
        "/tmp/font.ttf",
        "https://font",
        "font; color:red",
        "url(font)",
        " Name",
        "Name ",
        "font\nname",
        "a\u{202e}b",
        "a\u{2066}b",
    ] {
        let mut p = Preferences::default();
        p.appearance.font_family = name.into();
        assert!(p.validate().is_err(), "accepted {name}");
    }
    let mut p = Preferences::default();
    p.appearance.font_family = "é".repeat(64);
    assert!(p.validate().is_ok());
    p.appearance.font_family.push('é');
    assert!(p.validate().is_err());
}
#[test]
fn numeric_bounds_and_column_permutations_are_enforced() {
    for value in [10, 24] {
        let mut p = Preferences::default();
        p.appearance.font_size_px = value;
        assert!(p.validate().is_ok());
    }
    for value in [0, 9, 25, u32::MAX] {
        let mut p = Preferences::default();
        p.appearance.font_size_px = value;
        assert!(p.validate().is_err());
    }
    for (width, height) in [(760, 520), (4096, 2160)] {
        let mut p = Preferences::default();
        p.windows.main.width_px = width;
        p.windows.main.height_px = height;
        assert!(p.validate().is_ok());
    }
    for (width, height) in [(759, 520), (760, 519), (4097, 2160), (4096, 2161)] {
        let mut p = Preferences::default();
        p.windows.main.width_px = width;
        p.windows.main.height_px = height;
        assert!(p.validate().is_err());
    }
    let mut p = Preferences::default();
    p.table.column_order.reverse();
    assert!(p.validate().is_ok());
    p.table.column_order[0] = p.table.column_order[1];
    assert!(p.validate().is_err());
    p = Preferences::default();
    p.table.column_order.pop();
    assert!(p.validate().is_err());
    for column in ColumnId::ALL {
        p = Preferences::default();
        let (low, high) = if column == ColumnId::Sensor {
            (240, 1024)
        } else {
            (112, 512)
        };
        for value in [low, high] {
            p.table.column_widths_px.set(column, value);
            assert!(p.validate().is_ok());
        }
        p.table.column_widths_px.set(column, low - 1);
        assert!(p.validate().is_err());
        p.table.column_widths_px.set(column, high + 1);
        assert!(p.validate().is_err());
    }
}
#[test]
fn identity_selectors_require_canonical_consistent_existing_semantics() {
    let good = member(1);
    assert!(good.validate().is_ok());
    let mut bad = good.clone();
    bad.identity_schema_version = 2;
    assert!(bad.validate().is_err());
    bad = good.clone();
    bad.unit = "RPM".into();
    assert!(bad.validate().is_err());
    bad = good.clone();
    bad.candidate_id = bad.candidate_id.replace(",1,", ", 1,");
    assert!(bad.validate().is_err());
    bad = good.clone();
    bad.candidate_id = bad.candidate_id.replace("temp1_input", "fan1_input");
    assert!(bad.validate().is_err());
    bad = good.clone();
    bad.candidate_id = bad
        .candidate_id
        .replace("fixture\",\"/devices", "\",\"/devices");
    assert!(bad.validate().is_err());
    bad = good.clone();
    bad.candidate_id = "x".repeat(4097);
    assert!(bad.validate().is_err());
    let mut p = Preferences::default();
    p.group_overrides.push(GroupOverride {
        expanded: false,
        members: vec![good.clone()],
    });
    assert_eq!(parse_profile(&serialize_profile(&p).unwrap()).unwrap(), p);
    p.group_overrides.push(GroupOverride {
        expanded: true,
        members: vec![good],
    });
    assert!(p.validate().is_err());
}
#[test]
fn group_and_member_caps_reject_unbounded_or_duplicate_records() {
    let mut p = Preferences {
        group_overrides: vec![GroupOverride {
            expanded: true,
            members: vec![],
        }],
        ..Preferences::default()
    };
    assert!(p.validate().is_err());
    p.group_overrides[0].members = (0..17).map(member).collect();
    assert!(p.validate().is_err());
    p.group_overrides = (0..257)
        .map(|i| GroupOverride {
            expanded: true,
            members: vec![member(i)],
        })
        .collect();
    assert!(p.validate().is_err());
}
#[test]
fn xdg_selection_uses_absolute_values_and_warns_on_relative() {
    assert_eq!(
        choose_config_path(Some(OsStr::new("/config")), None)
            .unwrap()
            .path,
        Path::new("/config/siliconwatch/config.toml")
    );
    let p =
        choose_config_path(Some(OsStr::new("relative")), Some(OsStr::new("/home/user"))).unwrap();
    assert_eq!(
        p.path,
        Path::new("/home/user/.config/siliconwatch/config.toml")
    );
    assert_eq!(p.warnings.len(), 1);
    assert!(choose_config_path(None, Some(OsStr::new("relative"))).is_err());
}
#[test]
fn missing_load_is_read_only_and_initial_save_creates_private_directory() {
    let f = Fixture::new();
    let missing = f.root.join("new/config.toml");
    let (mut store, report) = Store::load(missing.clone()).unwrap();
    assert!(report.recovery.is_none());
    assert!(!missing.parent().unwrap().exists());
    store.save(&report.preferences, false).unwrap();
    assert_eq!(
        fs::metadata(missing.parent().unwrap()).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(fs::metadata(&missing).unwrap().mode() & 0o777, 0o600);
    assert_eq!(read_import(&missing).unwrap(), Preferences::default());
}
#[test]
fn saved_backup_is_exact_previous_valid_bytes_and_not_reformatted() {
    let f = Fixture::new();
    let old = format!("{}# preserve this comment\n", minimal());
    f.write("config.toml", old.as_bytes());
    let (mut store, _) = f.load();
    let mut next = Preferences::default();
    next.appearance.theme = Theme::Light;
    let result = store.save(&next, false).unwrap();
    assert_eq!(result.preferences, next);
    assert!(result.durability_warning.is_none());
    assert_eq!(fs::read(backup_path(&f.path)).unwrap(), old.as_bytes());
    assert_eq!(read_import(&f.path).unwrap(), next);
    assert_eq!(f.temps(), 0);
}
#[test]
fn precommit_failures_preserve_active_bytes_and_do_not_leak_temps() {
    for point in [
        FailPoint::TempCreate,
        FailPoint::TempSync,
        FailPoint::Backup,
        FailPoint::Rename,
    ] {
        let f = Fixture::new();
        let old = serialize_profile(&Preferences::default()).unwrap();
        f.write("config.toml", &old);
        let (mut store, _) = f.load();
        let mut next = Preferences::default();
        next.appearance.theme = Theme::Light;
        assert!(
            store
                .save_internal(&next, false, &AtomicBool::new(false), point)
                .is_err(),
            "{point:?}"
        );
        assert_eq!(fs::read(&f.path).unwrap(), old);
        assert_eq!(f.temps(), 0);
    }
}
#[test]
fn directory_sync_failure_is_committed_with_warning_and_updated_fingerprint() {
    let f = Fixture::new();
    let (mut store, _) = f.load();
    let mut next = Preferences::default();
    next.appearance.theme = Theme::Dark;
    let outcome = store
        .save_internal(
            &next,
            false,
            &AtomicBool::new(false),
            FailPoint::DirectorySync,
        )
        .unwrap();
    assert!(outcome.durability_warning.is_some());
    assert_eq!(read_import(&f.path).unwrap(), next);
    next.appearance.theme = Theme::Light;
    assert!(store.save(&next, false).is_ok());
}
#[test]
fn external_edit_and_second_writer_are_conflicts() {
    let f = Fixture::new();
    let (mut one, _) = f.load();
    let (mut two, _) = f.load();
    one.save(&Preferences::default(), false).unwrap();
    assert!(
        two.save(&Preferences::default(), false)
            .unwrap_err()
            .message
            .contains("changed")
    );
    let old = fs::read(&f.path).unwrap();
    f.write("config.toml", b"# external change\n");
    assert!(one.save(&Preferences::default(), false).is_err());
    assert_ne!(fs::read(&f.path).unwrap(), old);
}
#[test]
fn cooperating_writer_lock_is_nonblocking() {
    let f = Fixture::new();
    let (mut store, _) = f.load();
    let dir = SecureDir::open(&f.root, false, true).unwrap();
    let _lock = dir.lock().unwrap();
    let start = Instant::now();
    assert!(store.save(&Preferences::default(), false).is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
}
#[test]
fn malformed_startup_keeps_original_and_requires_explicit_recovery() {
    let f = Fixture::new();
    let old = b"schema_version=99\nsecret='must remain only in original'\n";
    f.write("config.toml", old);
    let good_backup = serialize_profile(&Preferences::default()).unwrap();
    f.write("config.toml.bak", &good_backup);
    let (mut store, report) = f.load();
    assert!(report.recovery.is_some());
    assert_eq!(report.preferences, Preferences::default());
    assert!(store.save(&Preferences::default(), false).is_err());
    assert_eq!(fs::read(&f.path).unwrap(), old);
    assert_eq!(store.read_backup().unwrap(), Preferences::default());
    store.save(&Preferences::default(), true).unwrap();
    let recoveries = f.recoveries();
    assert_eq!(recoveries.len(), 1);
    assert_eq!(fs::read(&recoveries[0]).unwrap(), old);
    assert_eq!(fs::metadata(&recoveries[0]).unwrap().mode() & 0o777, 0o600);
    assert_eq!(fs::read(backup_path(&f.path)).unwrap(), good_backup);
}
#[test]
fn recovery_failure_blocks_replacement_and_keeps_backup() {
    let f = Fixture::new();
    f.write("config.toml", b"bad");
    f.write("config.toml.bak", minimal().as_bytes());
    let (mut store, _) = f.load();
    assert!(
        store
            .save_internal(
                &Preferences::default(),
                true,
                &AtomicBool::new(false),
                FailPoint::Recovery
            )
            .is_err()
    );
    assert_eq!(fs::read(&f.path).unwrap(), b"bad");
    assert_eq!(
        fs::read(backup_path(&f.path)).unwrap(),
        minimal().as_bytes()
    );
    assert_eq!(f.temps(), 0);
}
#[test]
fn oversized_and_nonutf8_files_remain_recoverable_without_parsing() {
    for old in [vec![b'x'; MAX_PROFILE_BYTES + 1], vec![0xff, 0xfe, 0, 1]] {
        let f = Fixture::new();
        f.write("config.toml", &old);
        let (mut store, report) = f.load();
        assert!(report.recovery.is_some());
        assert!(read_import(&f.path).is_err());
        store.save(&Preferences::default(), true).unwrap();
        assert_eq!(fs::read(&f.recoveries()[0]).unwrap(), old);
    }
}
#[test]
fn symlinks_special_files_and_symlink_ancestors_are_rejected() {
    let f = Fixture::new();
    let target = f.write("target", minimal().as_bytes());
    symlink(&target, &f.path).unwrap();
    assert!(read_import(&f.path).is_err());
    assert!(Store::load(f.path.clone()).is_err());
    fs::remove_file(&f.path).unwrap();
    let c = cstring(f.path.as_os_str()).unwrap();
    // SAFETY: valid fixture-only pathname and standard FIFO permissions.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let start = Instant::now();
    assert!(read_import(&f.path).is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
    let link = f.root.join("linked");
    symlink(&f.root, &link).unwrap();
    assert!(read_import(&link.join("target")).is_err());
    assert!(read_import(&f.root).is_err());
}
#[test]
fn unsafe_local_permissions_are_rejected_without_chmod() {
    let f = Fixture::new();
    f.write("config.toml", minimal().as_bytes());
    fs::set_permissions(&f.path, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(Store::load(f.path.clone()).is_err());
    assert_eq!(fs::metadata(&f.path).unwrap().mode() & 0o777, 0o666);
}
#[test]
fn unsafe_backup_aborts_before_active_commit() {
    let f = Fixture::new();
    f.write("config.toml", minimal().as_bytes());
    let protected = f.write("protected", b"preserve");
    symlink(&protected, backup_path(&f.path)).unwrap();
    let (mut store, _) = f.load();
    assert!(store.save(&Preferences::default(), false).is_err());
    assert_eq!(fs::read(&f.path).unwrap(), minimal().as_bytes());
    assert_eq!(fs::read(protected).unwrap(), b"preserve");
    assert_eq!(f.temps(), 0);
}
#[test]
fn import_candidate_is_immutable_if_source_changes_after_preview() {
    let f = Fixture::new();
    let import = f.write("import.toml", minimal().as_bytes());
    let candidate = read_import(&import).unwrap();
    f.write("import.toml", b"not toml");
    let (mut store, _) = f.load();
    store.save(&candidate, false).unwrap();
    assert_eq!(read_import(&f.path).unwrap(), Preferences::default());
    assert_eq!(fs::read(import).unwrap(), b"not toml");
}
#[test]
fn export_omits_groups_by_default_and_is_confirmed_as_an_immutable_plan() {
    let f = Fixture::new();
    let (store, _) = f.load();
    let mut p = Preferences::default();
    p.group_overrides.push(GroupOverride {
        expanded: false,
        members: vec![member(1)],
    });
    let destination = f.root.join("export.toml");
    let plan = store
        .prepare_export(destination.clone(), &p, false)
        .unwrap();
    assert!(!destination.exists());
    assert!(!plan.replaces_existing);
    commit_export(plan).unwrap();
    assert!(
        read_import(&destination)
            .unwrap()
            .group_overrides
            .is_empty()
    );
    let plan = store.prepare_export(destination.clone(), &p, true).unwrap();
    assert!(plan.replaces_existing);
    commit_export(plan).unwrap();
    assert_eq!(read_import(&destination).unwrap(), p);
    assert!(!f.path.exists());
}
#[test]
fn export_conflicts_aliases_and_special_targets_are_rejected() {
    let f = Fixture::new();
    let (mut store, _) = f.load();
    store.save(&Preferences::default(), false).unwrap();
    assert!(
        store
            .prepare_export(f.path.clone(), &Preferences::default(), false)
            .is_err()
    );
    assert!(
        store
            .prepare_export(f.root.join("./config.toml"), &Preferences::default(), false)
            .is_err()
    );
    assert!(
        store
            .prepare_export(backup_path(&f.path), &Preferences::default(), false)
            .is_err()
    );
    let alias = f.root.join("alias");
    fs::hard_link(&f.path, &alias).unwrap();
    assert!(
        store
            .prepare_export(alias, &Preferences::default(), false)
            .is_err()
    );
    let target = f.write("export", b"old");
    let plan = store
        .prepare_export(target.clone(), &Preferences::default(), false)
        .unwrap();
    f.write("export", b"editor changed");
    assert!(commit_export(plan).is_err());
    assert_eq!(fs::read(target).unwrap(), b"editor changed");
}
#[test]
fn cancelled_save_does_not_create_active_file() {
    let f = Fixture::new();
    let (mut store, _) = f.load();
    assert!(
        store
            .save_internal(
                &Preferences::default(),
                false,
                &AtomicBool::new(true),
                FailPoint::None
            )
            .is_err()
    );
    assert!(!f.path.exists());
}
#[test]
fn worker_admits_one_job_until_response_is_consumed_and_joins() {
    let f = Fixture::new();
    let (store, _) = f.load();
    let worker = ConfigWorker::spawn(store).unwrap();
    worker
        .submit(ConfigJob::Save {
            preferences: Preferences::default(),
            recover_original: false,
        })
        .unwrap();
    assert_eq!(worker.submit(ConfigJob::Reload), Err(SubmitError::Busy));
    let start = Instant::now();
    loop {
        if let Some(response) = worker.try_recv() {
            match response {
                ConfigResponse::Saved(r) => {
                    r.unwrap();
                }
                _ => panic!("wrong response"),
            };
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    worker.submit(ConfigJob::Reload).unwrap();
    worker.request_stop();
    assert_eq!(worker.submit(ConfigJob::Reload), Err(SubmitError::Stopped));
    worker.join().unwrap();
}

#[test]
fn path_input_and_cancelled_import_are_bounded_before_io() {
    let long = PathBuf::from(format!("/{}", "x".repeat(4096)));
    assert!(split_path(&long).is_err());
    assert!(split_path(Path::new("/tmp/../config.toml")).is_err());
    let error =
        read_import_internal(Path::new("/not-accessed"), &AtomicBool::new(true)).unwrap_err();
    assert!(error.message.contains("cancelled"));
}

#[test]
fn disconnected_worker_response_releases_busy_and_reports_stopped() {
    let (sender, jobs) = mpsc::sync_channel(1);
    let (responses, receiver) = mpsc::sync_channel(1);
    drop(jobs);
    drop(responses);
    let worker = ConfigWorker {
        sender,
        receiver,
        busy: Arc::new(AtomicBool::new(true)),
        cancel: Arc::new(AtomicBool::new(false)),
        handle: None,
    };
    assert!(worker.try_recv().is_none());
    assert_eq!(worker.submit(ConfigJob::Reload), Err(SubmitError::Stopped));
    worker.join().unwrap();
}

#[test]
fn tray_behavior_defaults_off_for_existing_schema_one_profiles() {
    assert!(!Preferences::default().behavior.minimize_to_tray);
    for extra in ["", "[behavior]\n", "behavior={}\n"] {
        assert!(!parse_extra(extra).unwrap().behavior.minimize_to_tray);
    }
    // A pre-tray schema-1 profile retains every explicit preference, with no migration.
    let previous = parse_extra(
        "[appearance]\ntheme='light'\nfont_size_px=20\ntemperature_unit='fahrenheit'\n[windows.main]\nwidth_px=1200\nheight_px=700\nmaximized=true\n[table]\ncolumn_order=['average','sensor','maximum','minimum','current']\nalternate_rows=true\n",
    ).unwrap();
    assert!(!previous.behavior.minimize_to_tray);
    assert_eq!(previous.appearance.theme, Theme::Light);
    assert_eq!(previous.appearance.font_size_px, 20);
    assert_eq!(
        previous.appearance.temperature_unit,
        TemperatureUnit::Fahrenheit
    );
    assert_eq!(previous.windows.main.width_px, 1200);
    assert_eq!(previous.windows.main.height_px, 700);
    assert!(previous.windows.main.maximized);
    assert_eq!(previous.table.column_order[0], ColumnId::Average);
    assert!(previous.table.alternate_rows);
}

#[test]
fn tray_behavior_requires_an_actual_boolean_and_rejects_unknown_fields() {
    for enabled in [false, true] {
        let parsed = parse_extra(&format!("[behavior]\nminimize_to_tray={enabled}\n")).unwrap();
        assert_eq!(parsed.behavior.minimize_to_tray, enabled);
        assert_eq!(
            parse_profile(&serialize_profile(&parsed).unwrap()).unwrap(),
            parsed
        );
    }
    for extra in [
        "[behavior]\nminimize_to_tray='true'",
        "[behavior]\nminimize_to_tray=1",
        "[behavior]\nminimize_to_tray=0",
        "[behavior]\nminimize_to_tray=1.0",
        "[behavior]\nminimize_to_tray=[]",
        "[behavior]\nminimize_to_tray={enabled=true}",
        "[behavior]\nminimize_to_tray=1979-05-27",
        "[behavior]\nminimize_to_tray=true\nminimize_to_tray=false",
        "[behavior]\nminimize_to_tray=true\nstart_daemon=true",
        "[behavior]\nminimize_to_tray=true\ntrusted=true",
        "[behavior.extra]",
        "behavior=true",
        "minimize_to_tray=true",
    ] {
        assert!(parse_extra(extra).is_err(), "accepted {extra}");
    }
}

#[test]
fn tray_behavior_is_exported_with_or_without_device_groups() {
    let f = Fixture::new();
    let (store, _) = f.load();
    for enabled in [false, true] {
        for include_groups in [false, true] {
            let mut p = Preferences::default();
            p.behavior.minimize_to_tray = enabled;
            p.group_overrides.push(GroupOverride {
                expanded: false,
                members: vec![member(1)],
            });
            let destination = f.root.join(format!("tray-{enabled}-{include_groups}.toml"));
            let plan = store
                .prepare_export(destination.clone(), &p, include_groups)
                .unwrap();
            assert!(!destination.exists());
            commit_export(plan).unwrap();
            let imported = read_import(&destination).unwrap();
            if !include_groups {
                p.group_overrides.clear();
            }
            assert_eq!(imported, p);
            assert_eq!(imported.behavior.minimize_to_tray, enabled);
        }
    }
    assert!(!f.path.exists());
}

#[test]
fn tray_behavior_persists_across_restart_and_reload_with_exact_legacy_backup() {
    let f = Fixture::new();
    let legacy = format!(
        "{}# Original schema-1 bytes must survive\n[appearance]\ntheme='dark'\n",
        minimal()
    );
    f.write("config.toml", legacy.as_bytes());
    let (mut store, report) = f.load();
    assert!(!report.preferences.behavior.minimize_to_tray);
    let mut enabled = report.preferences;
    enabled.behavior.minimize_to_tray = true;
    store.save(&enabled, false).unwrap();
    assert_eq!(fs::read(backup_path(&f.path)).unwrap(), legacy.as_bytes());
    let (mut restarted, report) = f.load();
    assert!(report.recovery.is_none());
    assert_eq!(report.preferences, enabled);
    let mut external = enabled.clone();
    external.behavior.minimize_to_tray = false;
    f.write("config.toml", &serialize_profile(&external).unwrap());
    assert!(restarted.save(&enabled, false).is_err());
    let reloaded = restarted.reload().unwrap();
    assert_eq!(reloaded.preferences, external);
    restarted.save(&enabled, false).unwrap();
    assert_eq!(read_import(&f.path).unwrap(), enabled);
    assert_eq!(restarted.read_backup().unwrap(), external);
}

#[test]
fn invalid_tray_import_preserves_active_profile_backup_and_conflict_fingerprint() {
    let f = Fixture::new();
    let (mut store, _) = f.load();
    store.save(&Preferences::default(), false).unwrap();
    let mut active = Preferences::default();
    active.behavior.minimize_to_tray = true;
    store.save(&active, false).unwrap();
    let before = fs::read(&f.path).unwrap();
    let backup = fs::read(backup_path(&f.path)).unwrap();
    let expected = store.expected.clone();
    let bytes = format!(
        "{}[appearance]\ntheme='light'\n[behavior]\nminimize_to_tray='false'\n",
        minimal()
    );
    let source = f.write("invalid-tray.toml", bytes.as_bytes());
    assert!(read_import(&source).is_err());
    assert_eq!(fs::read(&source).unwrap(), bytes.as_bytes());
    assert_eq!(fs::read(&f.path).unwrap(), before);
    assert_eq!(fs::read(backup_path(&f.path)).unwrap(), backup);
    assert_eq!(store.expected, expected);
    assert_eq!(read_import(&f.path).unwrap(), active);
    assert_eq!(f.temps(), 0);
}

#[test]
fn tray_change_precommit_failure_keeps_old_value_and_allows_retry() {
    let f = Fixture::new();
    let original = minimal().as_bytes();
    f.write("config.toml", original);
    let (mut store, _) = f.load();
    let expected = store.expected.clone();
    let mut next = Preferences::default();
    next.behavior.minimize_to_tray = true;
    assert!(
        store
            .save_internal(&next, false, &AtomicBool::new(false), FailPoint::Rename)
            .is_err()
    );
    assert_eq!(fs::read(&f.path).unwrap(), original);
    assert!(!read_import(&f.path).unwrap().behavior.minimize_to_tray);
    assert_eq!(store.expected, expected);
    assert_eq!(f.temps(), 0);
    store.save(&next, false).unwrap();
    assert_eq!(read_import(&f.path).unwrap(), next);
    assert_eq!(fs::read(backup_path(&f.path)).unwrap(), original);
}

#[test]
fn monitoring_settings_round_trip_and_reject_conflicting_timings() {
    let mut preferences = parse_profile(minimal().as_bytes()).unwrap();
    assert_eq!(preferences.monitoring, Monitoring::default());
    preferences.monitoring = Monitoring {
        interval_ms: 6_000,
        stale_after_ms: 15_000,
        response_timeout_ms: 750,
    };
    let encoded = serialize_profile(&preferences).unwrap();
    assert_eq!(parse_profile(&encoded).unwrap(), preferences);
    assert_eq!(
        preferences.monitoring.command(),
        "siliconwatch daemon --local-api --interval-ms 6000 --stale-after-ms 15000"
    );

    preferences.monitoring.stale_after_ms = 5_999;
    assert!(preferences.validate().is_err());
    preferences.monitoring.stale_after_ms = 15_000;
    preferences.monitoring.response_timeout_ms = 0;
    assert!(preferences.validate().is_err());
}
