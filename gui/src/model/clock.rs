//! Linux BOOTTIME includes suspend; MONOTONIC does not. Bracket the BOOTTIME
//! read to avoid treating ordinary scheduling delay as suspend. This can detect
//! a changed offset while the GUI runs, not prove physical measurement age or
//! detect a suspend that predates startup. A busy scheduler can hide a short gap.
const TOLERANCE_NS: i128 = 1_000_000;
#[derive(Clone, Copy, Debug)]
struct Sample {
    lower_offset: i128,
    upper_offset: i128,
    mono_end: i128,
}
fn nanos(clock: libc::clockid_t) -> Option<i128> {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: initialized, correctly sized output object; retained by no code.
    if unsafe { libc::clock_gettime(clock, &mut t) } != 0
        || t.tv_sec < 0
        || !(0..1_000_000_000).contains(&t.tv_nsec)
    {
        return None;
    }
    i128::from(t.tv_sec)
        .checked_mul(1_000_000_000)?
        .checked_add(i128::from(t.tv_nsec))
}
fn sample() -> Option<Sample> {
    let before = nanos(libc::CLOCK_MONOTONIC)?;
    let boot = nanos(libc::CLOCK_BOOTTIME)?;
    let after = nanos(libc::CLOCK_MONOTONIC)?;
    if after < before {
        return None;
    }
    Some(Sample {
        lower_offset: boot.checked_sub(after)?,
        upper_offset: boot.checked_sub(before)?,
        mono_end: after,
    })
}
pub(super) struct ClockGuard {
    previous: Option<Sample>,
}
impl Default for ClockGuard {
    fn default() -> Self {
        Self { previous: sample() }
    }
}
impl ClockGuard {
    pub(super) fn reset(&mut self) {
        self.previous = sample();
    }
    pub(super) fn check(&mut self) -> bool {
        self.observe(sample())
    }
    fn observe(&mut self, current: Option<Sample>) -> bool {
        let Some(current) = current else {
            return true;
        };
        let changed = self.previous.is_some_and(|old| {
            current.mono_end < old.mono_end
                || current.lower_offset > old.upper_offset.saturating_add(TOLERANCE_NS)
                || current.upper_offset < old.lower_offset.saturating_sub(TOLERANCE_NS)
        });
        self.previous = Some(current);
        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suspend_and_clock_regression_are_detected_but_sampling_latency_is_not() {
        let baseline = Sample {
            lower_offset: 0,
            upper_offset: 100,
            mono_end: 1000,
        };
        let mut guard = ClockGuard {
            previous: Some(baseline),
        };
        assert!(!guard.observe(Some(Sample {
            lower_offset: -5_000_000,
            upper_offset: 100,
            mono_end: 8_000_000
        })));
        assert!(guard.observe(Some(Sample {
            lower_offset: 5_000_000_000,
            upper_offset: 5_000_000_100,
            mono_end: 9_000_000
        })));
        assert!(!guard.observe(Some(Sample {
            lower_offset: 5_000_000_000,
            upper_offset: 5_000_000_100,
            mono_end: 10_000_000
        })));
        assert!(guard.observe(Some(Sample {
            lower_offset: 5_000_000_000,
            upper_offset: 5_000_000_100,
            mono_end: 9_000_000
        })));
        assert!(guard.observe(None));
    }
}
