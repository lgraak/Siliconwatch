//! Display conversions never mutate the acquisition record.
use crate::preferences::TemperatureUnit;
use crate::protocol::Sensor;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Temperatures,
    Voltages,
    Fans,
    Utilization,
    Memory,
    Clocks,
    Power,
    PerformanceState,
    Other,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Temperatures => "Temperatures",
            Self::Voltages => "Voltages",
            Self::Fans => "Fans",
            Self::Utilization => "Utilization",
            Self::Memory => "Memory",
            Self::Clocks => "Clocks",
            Self::Power => "Power",
            Self::PerformanceState => "Performance state",
            Self::Other => "Other / unsupported",
        }
    }
}

fn measurement(sensor: &Sensor) -> (Category, Option<&'static str>) {
    match (
        sensor.identity.evidence.measurement.as_deref(),
        sensor.unit.as_deref(),
    ) {
        (
            Some("temperature_input_millidegree_celsius" | "nvml_temperature_celsius"),
            Some("°C"),
        ) => (Category::Temperatures, Some("°C")),
        (Some("voltage_input_millivolt"), Some("V")) => (Category::Voltages, Some("V")),
        (Some("fan_speed_input_rpm"), Some("RPM")) => (Category::Fans, Some("RPM")),
        (Some("nvml_fan_percent"), Some("%")) => (Category::Fans, Some("%")),
        (Some("nvml_utilization_percent"), Some("%")) => (Category::Utilization, Some("%")),
        (
            Some(
                "nvml_memory_bytes"
                | "nvml_memory_total_legacy_bytes"
                | "nvml_memory_reserved_v2_bytes"
                | "nvml_memory_used_v2_bytes"
                | "nvml_memory_used_legacy_bytes",
            ),
            Some("B"),
        ) => (Category::Memory, Some("B")),
        (Some("nvml_clock_mhz"), Some("MHz")) => (Category::Clocks, Some("MHz")),
        (Some("nvml_power_milliwatts"), Some("W")) => (Category::Power, Some("W")),
        (Some("nvml_performance_state"), Some("P-state")) => {
            (Category::PerformanceState, Some("P-state"))
        }
        _ => (Category::Other, None),
    }
}

pub fn supported_unit(sensor: &Sensor) -> Option<&'static str> {
    measurement(sensor).1
}

pub fn detail_categories(sensors: &[Sensor], indices: &[usize]) -> BTreeMap<Category, Vec<usize>> {
    let mut categories = BTreeMap::new();
    for &index in indices {
        let (category, _) = measurement(&sensors[index]);
        categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(index);
    }
    categories
}

pub fn converted(
    value: f64,
    unit: &str,
    preference: TemperatureUnit,
) -> Option<(f64, &'static str)> {
    if !value.is_finite() {
        return None;
    }
    let pair = match (unit, preference) {
        ("°C", TemperatureUnit::Fahrenheit) => (value * 1.8 + 32.0, "°F"),
        ("°C", _) => (value, "°C"),
        ("V", _) => (value, "V"),
        ("RPM", _) => (value, "RPM"),
        ("%", _) => (value, "%"),
        ("B", _) => (value / 1_048_576.0, "MiB"),
        ("MHz", _) => (value, "MHz"),
        ("W", _) => (value, "W"),
        ("P-state", _) => (value, "P-state"),
        _ => return None,
    };
    pair.0.is_finite().then_some(pair)
}

pub fn reading(value: Option<f64>, unit: Option<&str>, preference: TemperatureUnit) -> String {
    let Some(value) = value else {
        return "—".into();
    };
    let Some(unit) = unit else {
        return "— (unknown semantics)".into();
    };
    let Some((value, display_unit)) = converted(value, unit, preference) else {
        return "— (display error)".into();
    };
    if unit == "P-state" {
        return format!("P{value:.0}");
    }
    let number = if value.abs() >= 1_000_000.0 {
        format!("{value:.4e}")
    } else if unit == "RPM" {
        format!("{value:.0}")
    } else if unit == "V" {
        format!("{value:.3}")
    } else {
        format!("{value:.1}")
    };
    format!("{number} {display_unit}")
}

/// Sensor labels are always plain text. Control characters are made visible.
pub fn visible(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
                c.escape_unicode().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn temperature_conversion_and_other_units() {
        for (c, f) in [(-40.0, -40.0), (0.0, 32.0), (100.0, 212.0)] {
            assert_eq!(
                converted(c, "°C", TemperatureUnit::Fahrenheit),
                Some((f, "°F"))
            );
        }
        assert_eq!(
            converted(1200.0, "RPM", TemperatureUnit::Fahrenheit),
            Some((1200.0, "RPM"))
        );
        assert_eq!(
            converted(12.0, "V", TemperatureUnit::Fahrenheit),
            Some((12.0, "V"))
        );
    }
    #[test]
    fn unavailable_and_overflow_are_not_measurements() {
        assert_eq!(reading(None, Some("°C"), TemperatureUnit::Celsius), "—");
        assert!(converted(f64::MAX, "°C", TemperatureUnit::Fahrenheit).is_none());
        assert!(converted(f64::NAN, "V", TemperatureUnit::Celsius).is_none());
        assert!(converted(1.0, "unknown", TemperatureUnit::Celsius).is_none());
    }
    #[test]
    fn details_categories_include_all_gpu_metrics_and_unknown_rows() {
        use crate::protocol::parse_snapshot;
        use crate::protocol::tests::{frame, sample_value};
        let snapshot = parse_snapshot(&frame(&sample_value(1))).unwrap();
        let mut rows = snapshot.assessment.observation.unwrap().snapshot.sensors;
        let sensor = &mut rows[0];
        // Unavailable metrics keep their category when their unit and semantics are known.
        sensor.status = "unsupported".into();
        sensor.value = None;
        let mut samples = Vec::new();
        for (semantics, unit, expected) in [
            ("nvml_temperature_celsius", "°C", Category::Temperatures),
            ("nvml_utilization_percent", "%", Category::Utilization),
            ("nvml_fan_percent", "%", Category::Fans),
            ("nvml_memory_bytes", "B", Category::Memory),
            ("nvml_memory_total_legacy_bytes", "B", Category::Memory),
            ("nvml_memory_used_v2_bytes", "B", Category::Memory),
            ("nvml_memory_reserved_v2_bytes", "B", Category::Memory),
            ("nvml_memory_used_legacy_bytes", "B", Category::Memory),
            ("nvml_clock_mhz", "MHz", Category::Clocks),
            ("nvml_power_milliwatts", "W", Category::Power),
            (
                "nvml_performance_state",
                "P-state",
                Category::PerformanceState,
            ),
        ] {
            sensor.identity.evidence.measurement = Some(semantics.into());
            sensor.unit = Some(unit.into());
            assert_eq!(measurement(sensor), (expected, Some(unit)));
            samples.push(sensor.clone());
        }
        sensor.unit = Some("unexpected".into());
        assert_eq!(measurement(sensor), (Category::Other, None));
        samples.push(sensor.clone());
        let indices: Vec<_> = (0..samples.len()).rev().collect();
        let categories = detail_categories(&samples, &indices);
        assert_eq!(categories[&Category::Memory], [7, 6, 5, 4, 3]);
        assert_eq!(categories[&Category::Other], [samples.len() - 1]);
        let mut all: Vec<_> = categories.into_values().flatten().collect();
        all.sort_unstable();
        assert_eq!(all, (0..samples.len()).collect::<Vec<_>>());
    }

    #[test]
    fn labels_do_not_hide_control_characters() {
        assert_eq!(visible("CPU\nA\u{202e}"), "CPU\\u{a}A\\u{202e}");
        assert_eq!(visible("<b>CPU</b>"), "<b>CPU</b>");
    }
}
