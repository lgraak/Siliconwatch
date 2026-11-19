use super::*;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::DirBuilderExt;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("sw-nvml-{}-{id}", std::process::id()));
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => break path,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("creating NVML fixture: {e}"),
            }
        };
        let fixture = Self { root };
        let root = &fixture.root;
        let path = root.join("fixture.so");
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/nvml/provider_fixture.c");
        let output = Command::new("cc")
            .args([
                "-std=c11", "-Wall", "-Wextra", "-Werror", "-fPIC", "-shared", "-o",
            ])
            .arg(&path)
            .arg(source)
            .output()
            .expect("the Linux NVML ABI fixture requires a C compiler");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fs::create_dir(root.join("hwmon0")).unwrap();
        fs::write(root.join("hwmon0/name"), "fixture_cpu").unwrap();
        fs::write(root.join("hwmon0/temp1_input"), "42000").unwrap();
        fixture
    }

    fn open(&self) -> Library {
        let path = self.root.join("fixture.so");
        let name = CString::new(path.as_os_str().as_bytes()).unwrap();
        Library::open_named(&name).expect("test library must load")
    }

    fn scan(&self, scenario: &'static CStr, memory_status: c_int) -> Snapshot {
        let library = self.open();
        let configure: unsafe extern "C" fn(*const c_char, c_int) =
            unsafe { library.symbol(c"fixture_configure") }.unwrap();
        unsafe { configure(scenario.as_ptr(), memory_status) };
        let mut snapshot = crate::hwmon::scan(&self.root).unwrap();
        // The provider owns the only open handle and closes it before returning.
        append_with_library(&mut snapshot, library);
        let cpu = snapshot
            .sensors
            .iter()
            .find(|s| s.device == "fixture_cpu")
            .unwrap();
        assert_eq!(cpu.value, Some(42.0));
        snapshot
    }

    fn counts(&self) -> (u32, u32, u32) {
        // Reopen after every scan. Counters survive only if the mapping was retained.
        let library = self.open();
        let counts: unsafe extern "C" fn(*mut u32, *mut u32, *mut u32) =
            unsafe { library.symbol(c"fixture_counts") }.unwrap();
        let (mut init, mut shutdown, mut legacy) = (0, 0, 0);
        unsafe { counts(&mut init, &mut shutdown, &mut legacy) };
        (init, shutdown, legacy)
    }
}

fn gpu_rows(snapshot: &Snapshot) -> impl Iterator<Item = &Sensor> {
    snapshot
        .sensors
        .iter()
        .filter(|s| s.identity.evidence.provider.as_deref() == Some("nvml"))
}

fn row<'a>(snapshot: &'a Snapshot, id: u8, attribute: &str) -> &'a Sensor {
    let uuid = format!("GPU-00000000-0000-0000-0000-{id:012}");
    gpu_rows(snapshot)
        .find(|s| s.device_path.ends_with(&uuid) && s.attribute == attribute)
        .unwrap()
}

#[test]
fn loader_mapping_survives_balanced_shutdown_and_reopen_after_enumeration_errors() {
    let f = Fixture::new();
    for (scenario, diagnostics, init, shutdown) in [
        (c"driver absent", true, 1, 0),
        (c"enumeration failed", true, 2, 1),
        (c"zero", false, 3, 2),
        (c"zero", false, 4, 3),
    ] {
        let snapshot = f.scan(scenario, 0);
        assert_eq!(gpu_rows(&snapshot).count(), 0);
        assert_eq!(
            snapshot
                .warnings
                .iter()
                .any(|w| w.path == Path::new("nvml")),
            diagnostics
        );
        assert_eq!(f.counts(), (init, shutdown, 0));
    }
    assert!(gpu_rows(&f.scan(c"normal", 0)).count() > 0);
    assert_eq!(f.counts(), (5, 4, 0));
}

#[test]
fn gpu_identity_follows_device_through_reorder_missing_uuid_and_loss() {
    let f = Fixture::new();
    let original = f.scan(c"normal", 0);
    let reordered = f.scan(c"reorder", 0);
    let inventory = |snapshot: &Snapshot| -> BTreeMap<String, Option<i64>> {
        gpu_rows(snapshot)
            .map(|s| (s.identity.candidate.clone().unwrap(), s.raw_value))
            .collect()
    };
    assert_ne!(
        gpu_rows(&original).next().unwrap().device_path,
        gpu_rows(&reordered).next().unwrap().device_path
    );
    assert_eq!(inventory(&original).len(), gpu_rows(&original).count());
    assert_eq!(inventory(&original), inventory(&reordered));
    let unidentified = f.scan(c"no UUID", 0);
    assert_eq!(gpu_rows(&unidentified).count(), gpu_rows(&original).count());
    assert!(
        gpu_rows(&unidentified).all(|s| s.identity.candidate.is_none()
            && s.identity.status == IdentityStatus::InsufficientEvidence)
    );
    let query_lost = f.scan(c"query lost", 0);
    let lost = row(&query_lost, 1, "temperature");
    assert_eq!(
        (lost.value, lost.raw_value, lost.status),
        (None, None, Status::Unavailable)
    );
    assert_eq!(row(&query_lost, 2, "temperature").raw_value, Some(62));
    let handle_lost = f.scan(c"handle lost", 0);
    assert_eq!(
        gpu_rows(&handle_lost).count() * 2,
        gpu_rows(&original).count()
    );
    assert_eq!(row(&handle_lost, 2, "temperature").raw_value, Some(62));
    assert_eq!(inventory(&original), inventory(&f.scan(c"normal", 0)));
}

#[test]
fn query_abi_preserves_raw_values_units_and_memory_fallback_rules() {
    let f = Fixture::new();
    let modern = f.scan(c"normal", 0);
    for (attribute, raw, scaled, unit) in [
        ("temperature", 61, 61.0, "°C"),
        ("utilization", 45, 45.0, "%"),
        ("memory_used", 2048, 2048.0, "B"),
        ("memory_reserved", 1024, 1024.0, "B"),
        ("memory_total", 8_589_934_592, 8_589_934_592.0, "B"),
        ("graphics_clock", 734, 734.0, "MHz"),
        ("memory_clock", 2504, 2504.0, "MHz"),
        ("power", 25750, 25.75, "W"),
        ("performance_state", 3, 3.0, "P-state"),
    ] {
        let sensor = row(&modern, 1, attribute);
        assert_eq!(sensor.status, Status::Ok, "{attribute}");
        assert_eq!(
            (sensor.raw_value, sensor.value, sensor.unit),
            (Some(raw), Some(scaled), Some(unit))
        );
        assert_eq!(sensor.raw_text.as_ref(), Some(&raw.to_string()));
    }
    assert_eq!(
        row(&modern, 2, "performance_state").status,
        Status::Unavailable
    );
    let fan = row(&modern, 1, "fan_speed");
    assert_eq!(
        (fan.status, fan.raw_value, fan.value),
        (Status::Unsupported, None, None)
    );
    for status in [3, 13, 25] {
        let legacy = f.scan(c"normal", status);
        let used = row(&legacy, 1, "memory_used");
        assert_eq!(used.raw_value, Some(3072));
        assert_ne!(
            used.identity.candidate,
            row(&modern, 1, "memory_used").identity.candidate
        );
        let reserved = row(&legacy, 1, "memory_reserved");
        assert_eq!(
            (reserved.status, reserved.raw_value, reserved.value),
            (Status::Unsupported, None, None)
        );
    }
    for (code, status) in [(4, Status::PermissionDenied), (15, Status::Unavailable)] {
        let failed = f.scan(c"normal", code);
        for metric in ["memory_used", "memory_total", "memory_reserved"] {
            let sensor = row(&failed, 1, metric);
            assert_eq!(
                (sensor.status, sensor.raw_value, sensor.value),
                (status, None, None)
            );
        }
    }
    let oversized = f.scan(c"large unsigned", 0);
    let value = row(&oversized, 1, "memory_used");
    assert_eq!(
        (value.status, value.raw_value, value.value),
        (Status::Invalid, None, None)
    );
    assert_eq!(value.raw_text.as_deref(), Some("18446744073709551615"));
    assert_eq!(f.counts(), (7, 7, 6));
}
