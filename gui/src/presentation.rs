//! Display conversions never mutate the acquisition record.
use crate::preferences::TemperatureUnit;
use crate::protocol::Sensor;

pub fn supported_unit(sensor: &Sensor) -> Option<&'static str> {
    match (
        sensor.identity.evidence.measurement.as_deref(),
        sensor.unit.as_deref(),
    ) {
        (Some("temperature_input_millidegree_celsius"), Some("°C")) => Some("°C"),
        (Some("voltage_input_millivolt"), Some("V")) => Some("V"),
        (Some("fan_speed_input_rpm"), Some("RPM")) => Some("RPM"),
        _ => None,
    }
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
    fn labels_do_not_hide_control_characters() {
        assert_eq!(visible("CPU\nA\u{202e}"), "CPU\\u{a}A\\u{202e}");
        assert_eq!(visible("<b>CPU</b>"), "<b>CPU</b>");
    }
}
