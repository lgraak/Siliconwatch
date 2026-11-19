//! Read-only NVML access. The driver library is an optional runtime dependency.
use crate::hwmon::{Sensor, Snapshot, Status, Warning};
use crate::identity::{Evidence, IdentityStatus, SourceIdentity};
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::ptr::{self, NonNull};

type Device = *mut c_void;
type ResultCode = c_int;
type Scalar = unsafe extern "C" fn(Device, *mut c_uint) -> ResultCode;
type Indexed = unsafe extern "C" fn(Device, c_uint, *mut c_uint) -> ResultCode;
type DeviceText = unsafe extern "C" fn(Device, *mut c_char, c_uint) -> ResultCode;

#[repr(C)]
#[derive(Default)]
struct Memory {
    total: u64,
    free: u64,
    used: u64,
}

#[repr(C)]
struct MemoryV2 {
    version: c_uint,
    total: u64,
    reserved: u64,
    free: u64,
    used: u64,
}

impl Default for MemoryV2 {
    fn default() -> Self {
        Self {
            // NVML_STRUCT_VERSION encodes sizeof(struct) and the revision.
            version: std::mem::size_of::<Self>() as c_uint | (2 << 24),
            total: 0,
            reserved: 0,
            free: 0,
            used: 0,
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct Utilization {
    gpu: c_uint,
    memory: c_uint,
}

struct Library(NonNull<c_void>);

impl Library {
    fn open() -> Option<Self> {
        Self::open_named(c"libnvidia-ml.so.1")
    }

    fn open_named(name: &CStr) -> Option<Self> {
        // On driver 580.196.07, repeated unload/reload leaks one descriptor per scan.
        // Keep one mapping resident. Each scan still balances init/shutdown and dlopen/dlclose.
        NonNull::new(unsafe {
            libc::dlopen(
                name.as_ptr(),
                libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_NODELETE,
            )
        })
        .map(Self)
    }

    /// Each caller supplies the exact NVML C signature for the named symbol.
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Option<T> {
        let address = unsafe { libc::dlsym(self.0.as_ptr(), name.as_ptr()) };
        if address.is_null() {
            return None;
        }
        Some(unsafe { std::mem::transmute_copy(&address) })
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe { libc::dlclose(self.0.as_ptr()) };
    }
}

struct Session {
    library: Library,
    shutdown: unsafe extern "C" fn() -> ResultCode,
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { (self.shutdown)() };
    }
}

type Reading = Result<u64, ResultCode>;

const NOT_SUPPORTED: ResultCode = 3;
const FUNCTION_NOT_FOUND: ResultCode = 13;
const ARGUMENT_VERSION_MISMATCH: ResultCode = 25;

fn query<T: Default>(call: Option<impl FnOnce(*mut T) -> ResultCode>) -> Result<T, ResultCode> {
    let mut value = T::default();
    let result = call.ok_or(FUNCTION_NOT_FOUND)?(&mut value);
    if result == 0 { Ok(value) } else { Err(result) }
}

fn device_text(library: &Library, device: Device, symbol: &CStr) -> Option<String> {
    let call: DeviceText = unsafe { library.symbol(symbol) }?;
    let mut buffer = [0u8; 96];
    if unsafe { call(device, buffer.as_mut_ptr().cast(), buffer.len() as c_uint) } != 0 {
        return None;
    }
    let value = CStr::from_bytes_until_nul(&buffer).ok()?.to_str().ok()?;
    (!value.is_empty()).then(|| value.to_owned())
}

fn status(code: ResultCode) -> Status {
    match code {
        NOT_SUPPORTED | FUNCTION_NOT_FOUND => Status::Unsupported,
        4 => Status::PermissionDenied,
        1 | 6 | 9 | 10 | 15 | 21 | 27 | 28 => Status::Unavailable,
        _ => Status::Error,
    }
}

fn diagnostic(path: &Path, snapshot: &mut Snapshot, message: String) {
    snapshot.warnings.push(Warning {
        path: path.into(),
        message,
    });
}

pub(crate) fn append(snapshot: &mut Snapshot) {
    let Some(library) = Library::open() else {
        return;
    };
    append_with_library(snapshot, library);
}

fn append_with_library(snapshot: &mut Snapshot, library: Library) {
    let source = Path::new("nvml");
    let init: Option<unsafe extern "C" fn() -> ResultCode> =
        unsafe { library.symbol(c"nvmlInit_v2") };
    let shutdown = unsafe { library.symbol(c"nvmlShutdown") };
    let (Some(init), Some(shutdown)) = (init, shutdown) else {
        diagnostic(
            source,
            snapshot,
            "NVML initialization symbols unavailable".into(),
        );
        return;
    };
    let result = unsafe { init() };
    if result != 0 {
        diagnostic(
            source,
            snapshot,
            format!("nvmlInit_v2 returned NVML status {result}"),
        );
        return;
    }
    let session = Session { library, shutdown };
    let library = &session.library;
    let count: Option<unsafe extern "C" fn(*mut c_uint) -> ResultCode> =
        unsafe { library.symbol(c"nvmlDeviceGetCount_v2") };
    let get_device: Option<unsafe extern "C" fn(c_uint, *mut Device) -> ResultCode> =
        unsafe { library.symbol(c"nvmlDeviceGetHandleByIndex_v2") };
    let count = match query(count.map(|call| move |out| unsafe { call(out) })) {
        Ok(count) => count,
        Err(code) => {
            diagnostic(
                source,
                snapshot,
                format!("nvmlDeviceGetCount_v2 returned NVML status {code}"),
            );
            return;
        }
    };
    let Some(get_device) = get_device else {
        diagnostic(
            source,
            snapshot,
            "nvmlDeviceGetHandleByIndex_v2 is unavailable".into(),
        );
        return;
    };
    for index in 0..count {
        let mut device = ptr::null_mut();
        let code = unsafe { get_device(index, &mut device) };
        if code != 0 || device.is_null() {
            diagnostic(
                source,
                snapshot,
                format!("NVML device index {index} unavailable (status {code})"),
            );
            continue;
        }
        append_device(snapshot, library, device, index);
    }
}

struct MemoryReadings {
    used: Reading,
    reserved: Reading,
    total: Reading,
    legacy: bool,
}

fn memory_readings(library: &Library, device: Device) -> MemoryReadings {
    let modern: Option<unsafe extern "C" fn(Device, *mut MemoryV2) -> ResultCode> =
        unsafe { library.symbol(c"nvmlDeviceGetMemoryInfo_v2") };
    let result = query(modern.map(|call| move |out| unsafe { call(device, out) }));
    match result {
        Err(NOT_SUPPORTED | FUNCTION_NOT_FOUND | ARGUMENT_VERSION_MISMATCH) => {
            let legacy: Option<unsafe extern "C" fn(Device, *mut Memory) -> ResultCode> =
                unsafe { library.symbol(c"nvmlDeviceGetMemoryInfo") };
            let result = query(legacy.map(|call| move |out| unsafe { call(device, out) }));
            MemoryReadings {
                used: result.as_ref().map(|m| m.used).map_err(|&rc| rc),
                total: result.map(|m| m.total),
                reserved: Err(NOT_SUPPORTED),
                legacy: true,
            }
        }
        result => MemoryReadings {
            used: result.as_ref().map(|m| m.used).map_err(|&rc| rc),
            reserved: result.as_ref().map(|m| m.reserved).map_err(|&rc| rc),
            total: result.map(|m| m.total),
            legacy: false,
        },
    }
}

fn append_device(snapshot: &mut Snapshot, library: &Library, device: Device, index: u32) {
    let uuid = device_text(library, device, c"nvmlDeviceGetUUID").filter(|uuid| {
        uuid.strip_prefix("GPU-").is_some_and(|id| {
            id.len() == 36
                && id.bytes().enumerate().all(|(offset, byte)| match offset {
                    8 | 13 | 18 | 23 => byte == b'-',
                    _ => byte.is_ascii_hexdigit(),
                })
        })
    });
    let name = device_text(library, device, c"nvmlDeviceGetName")
        .unwrap_or_else(|| format!("NVIDIA GPU {index}"));
    let path = PathBuf::from("nvml").join(uuid.clone().unwrap_or_else(|| format!("index-{index}")));
    let scalar = |symbol| {
        let call: Option<Scalar> = unsafe { library.symbol(symbol) };
        query(call.map(|call| move |out| unsafe { call(device, out) })).map(u64::from)
    };
    let indexed = |symbol, selector| {
        let call: Option<Indexed> = unsafe { library.symbol(symbol) };
        query(call.map(|call| move |out| unsafe { call(device, selector, out) })).map(u64::from)
    };
    let utilization: Option<unsafe extern "C" fn(Device, *mut Utilization) -> ResultCode> =
        unsafe { library.symbol(c"nvmlDeviceGetUtilizationRates") };
    let utilization = query(utilization.map(|call| move |out| unsafe { call(device, out) }));
    let memory = memory_readings(library, device);
    let (memory_label, memory_measurement, total_measurement) = if memory.legacy {
        (
            "VRAM used (legacy)",
            "nvml_memory_used_legacy_bytes",
            "nvml_memory_total_legacy_bytes",
        )
    } else {
        (
            "VRAM used",
            "nvml_memory_used_v2_bytes",
            "nvml_memory_bytes",
        )
    };
    let metrics: [(&str, &str, &'static str, &'static str, f64, Reading); 10] = [
        (
            "temperature",
            "GPU temperature",
            "nvml_temperature_celsius",
            "°C",
            1.0,
            indexed(c"nvmlDeviceGetTemperature", 0),
        ),
        (
            "utilization",
            "GPU utilization",
            "nvml_utilization_percent",
            "%",
            1.0,
            utilization.map(|u| u64::from(u.gpu)),
        ),
        (
            "memory_used",
            memory_label,
            memory_measurement,
            "B",
            1.0,
            memory.used,
        ),
        (
            "memory_reserved",
            "VRAM reserved",
            "nvml_memory_reserved_v2_bytes",
            "B",
            1.0,
            memory.reserved,
        ),
        (
            "memory_total",
            "VRAM total",
            total_measurement,
            "B",
            1.0,
            memory.total,
        ),
        (
            "graphics_clock",
            "Graphics clock",
            "nvml_clock_mhz",
            "MHz",
            1.0,
            indexed(c"nvmlDeviceGetClockInfo", 0),
        ),
        (
            "memory_clock",
            "Memory clock",
            "nvml_clock_mhz",
            "MHz",
            1.0,
            indexed(c"nvmlDeviceGetClockInfo", 2),
        ),
        (
            "fan_speed",
            "Fan speed",
            "nvml_fan_percent",
            "%",
            1.0,
            scalar(c"nvmlDeviceGetFanSpeed"),
        ),
        (
            "power",
            "GPU power",
            "nvml_power_milliwatts",
            "W",
            1000.0,
            scalar(c"nvmlDeviceGetPowerUsage"),
        ),
        (
            "performance_state",
            "Performance state",
            "nvml_performance_state",
            "P-state",
            1.0,
            scalar(c"nvmlDeviceGetPerformanceState"),
        ),
    ];
    for (attribute, label, measurement, unit, divisor, reading) in metrics {
        let (raw, status, error) = match reading {
            Ok(32) if attribute == "performance_state" => (
                Some(32),
                Status::Unavailable,
                Some("NVML reports an unknown performance state".into()),
            ),
            Ok(value) => match i64::try_from(value) {
                Ok(value) => (Some(value), Status::Ok, None),
                Err(_) => (
                    None,
                    Status::Invalid,
                    Some(format!("NVML reading exceeds signed wire range: {value}")),
                ),
            },
            Err(code) => (None, status(code), Some(format!("NVML status {code}"))),
        };
        let mut identity = SourceIdentity::unresolved();
        identity.evidence = Evidence {
            provider: Some("nvml".into()),
            parent: uuid.as_ref().map(|_| path.clone()),
            channel: Some(attribute.into()),
            measurement: Some(measurement),
            ..Evidence::default()
        };
        if let Some(uuid) = &uuid {
            identity.status = IdentityStatus::BestEffort;
            identity.candidate = Some(
                serde_json::json!(["siliconwatch.nvml-source", 1, uuid, attribute, measurement])
                    .to_string(),
            );
        } else {
            identity.issues.push("NVML GPU UUID unavailable".into());
        }
        snapshot.sensors.push(Sensor {
            identity,
            device: name.clone(),
            device_path: path.clone(),
            attribute: attribute.into(),
            label: Some(label.into()),
            source_path: path.join(attribute),
            raw_text: reading.ok().map(|value| value.to_string()),
            raw_value: raw,
            value: if status == Status::Ok {
                raw.map(|value| value as f64 / divisor)
            } else {
                None
            },
            unit: Some(unit),
            status,
            error,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_nvml_queries_discard_output_and_retain_failure_status() {
        let result = query(Some(|output: *mut c_uint| {
            unsafe { *output = 900 };
            15
        }));
        assert_eq!(result, Err(15));
        assert_eq!(status(15), Status::Unavailable);
        assert_eq!(status(4), Status::PermissionDenied);
        let missing = query::<c_uint>(None::<fn(*mut c_uint) -> ResultCode>);
        assert_eq!(missing, Err(FUNCTION_NOT_FOUND));
        assert_eq!(status(3), Status::Unsupported);
    }
}

#[cfg(test)]
#[path = "nvml/provider_tests.rs"]
mod provider_tests;
