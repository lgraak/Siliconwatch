//! Small, read-only Linux hwmon reader. Successful reads are not sensor validation.

pub mod daemon;
pub mod hwmon;
pub mod identity;
#[cfg(target_os = "linux")]
mod local_api;
#[cfg(target_os = "linux")]
mod nvml;
pub mod sampling;
pub mod snapshot_cache;

/// Acquire the default hardware inventory, or only hwmon for an explicit fixture root.
pub fn scan(root: &std::path::Path) -> std::io::Result<hwmon::Snapshot> {
    #[allow(unused_mut)]
    let mut snapshot = hwmon::scan(root)?;
    #[cfg(target_os = "linux")]
    if root == std::path::Path::new("/sys/class/hwmon") {
        let first = snapshot.sensors.len();
        nvml::append(&mut snapshot);
        identity::mark_collisions(&mut snapshot.sensors[first..]);
    }
    Ok(snapshot)
}
