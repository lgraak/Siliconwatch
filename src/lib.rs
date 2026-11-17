//! Small, read-only Linux hwmon reader. Successful reads are not sensor validation.

pub mod daemon;
pub mod hwmon;
pub mod identity;
pub mod sampling;
pub mod snapshot_cache;
