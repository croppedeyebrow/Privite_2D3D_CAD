#![forbid(unsafe_code)]

use cad_core::LengthMm;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToleranceSpec {
    None,
    Symmetric {
        nominal: LengthMm,
        plus_minus: LengthMm,
    },
    Bilateral {
        nominal: LengthMm,
        upper: LengthMm,
        lower: LengthMm,
    },
    Limit {
        min: LengthMm,
        max: LengthMm,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToleranceRange {
    pub nominal: LengthMm,
    pub min: LengthMm,
    pub max: LengthMm,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalculationTrace {
    pub expression: String,
    pub result: ToleranceRange,
    pub warnings: Vec<String>,
}

#[must_use]
pub fn calculate(spec: ToleranceSpec) -> CalculationTrace {
    let (nominal, min, max) = match spec {
        ToleranceSpec::None => (0.0, 0.0, 0.0),
        ToleranceSpec::Symmetric {
            nominal,
            plus_minus,
        } => (
            nominal.0,
            nominal.0 - plus_minus.0,
            nominal.0 + plus_minus.0,
        ),
        ToleranceSpec::Bilateral {
            nominal,
            upper,
            lower,
        } => (nominal.0, nominal.0 - lower.0, nominal.0 + upper.0),
        ToleranceSpec::Limit { min, max } => (min.0.midpoint(max.0), min.0, max.0),
    };
    let mut warnings = Vec::new();
    if min > max {
        warnings.push("minimum exceeds maximum".to_owned());
    }
    CalculationTrace {
        expression: format!("{nominal:.9} -> [{min:.9}, {max:.9}] mm"),
        result: ToleranceRange {
            nominal: LengthMm(nominal),
            min: LengthMm(min),
            max: LengthMm(max),
        },
        warnings,
    }
}

/// Accumulates several tolerance specs into one overall range using
/// worst-case (linear) stacking, as required for MVP accumulation: the
/// nominal, minimum, and maximum of each spec are summed directly, so the
/// result represents the widest possible stack-up.
///
/// Returns a `CalculationTrace` whose `result` carries full-precision
/// values; nothing here rounds intermediate values. Use
/// `format_range_for_display` to render a rounded value for the UI.
#[must_use]
pub fn accumulate(specs: &[ToleranceSpec]) -> CalculationTrace {
    let mut nominal_sum = 0.0;
    let mut min_sum = 0.0;
    let mut max_sum = 0.0;
    let mut warnings = Vec::new();
    let mut terms = Vec::new();

    if specs.is_empty() {
        warnings.push("no tolerance specs to accumulate".to_owned());
    }

    for spec in specs {
        let trace = calculate(*spec);
        nominal_sum += trace.result.nominal.0;
        min_sum += trace.result.min.0;
        max_sum += trace.result.max.0;
        warnings.extend(trace.warnings);
        terms.push(trace.expression);
    }

    CalculationTrace {
        expression: format!(
            "worst-case sum of [{}] -> {nominal_sum:.9} -> [{min_sum:.9}, {max_sum:.9}] mm",
            terms.join(", ")
        ),
        result: ToleranceRange {
            nominal: LengthMm(nominal_sum),
            min: LengthMm(min_sum),
            max: LengthMm(max_sum),
        },
        warnings,
    }
}

/// Rounds a length to `decimals` places for display purposes only. The
/// caller must not feed the result back into a calculation — internal
/// calculations always use the full-precision `LengthMm` values in
/// `ToleranceRange`.
#[must_use]
pub fn round_for_display(value: LengthMm, decimals: u32) -> f64 {
    let factor = 10f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX));
    (value.0 * factor).round() / factor
}

/// Formats a `ToleranceRange` as `nominal [min, max] mm`, rounded to
/// `decimals` places for display. The source `ToleranceRange` is untouched.
#[must_use]
pub fn format_range_for_display(range: &ToleranceRange, decimals: usize) -> String {
    format!(
        "{:.decimals$} [{:.decimals$}, {:.decimals$}] mm",
        range.nominal.0, range.min.0, range.max.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_tolerance_is_deterministic() {
        let trace = calculate(ToleranceSpec::Symmetric {
            nominal: LengthMm(10.0),
            plus_minus: LengthMm(0.2),
        });
        assert_eq!(trace.result.min, LengthMm(9.8));
        assert_eq!(trace.result.max, LengthMm(10.2));
        assert!(trace.warnings.is_empty());
    }

    #[test]
    fn bilateral_tolerance_uses_upper_and_lower() {
        let trace = calculate(ToleranceSpec::Bilateral {
            nominal: LengthMm(20.0),
            upper: LengthMm(0.3),
            lower: LengthMm(0.1),
        });
        assert_eq!(trace.result.min, LengthMm(19.9));
        assert_eq!(trace.result.max, LengthMm(20.3));
    }

    #[test]
    fn limit_tolerance_derives_nominal_as_midpoint() {
        let trace = calculate(ToleranceSpec::Limit {
            min: LengthMm(9.8),
            max: LengthMm(10.2),
        });
        assert_eq!(trace.result.nominal, LengthMm(10.0));
    }

    #[test]
    fn calculate_warns_when_minimum_exceeds_maximum() {
        let trace = calculate(ToleranceSpec::Limit {
            min: LengthMm(10.0),
            max: LengthMm(9.0),
        });
        assert_eq!(trace.warnings, vec!["minimum exceeds maximum".to_owned()]);
    }

    #[test]
    fn accumulate_sums_worst_case_min_and_max() {
        let specs = [
            ToleranceSpec::Symmetric {
                nominal: LengthMm(10.0),
                plus_minus: LengthMm(0.1),
            },
            ToleranceSpec::Symmetric {
                nominal: LengthMm(20.0),
                plus_minus: LengthMm(0.2),
            },
        ];
        let trace = accumulate(&specs);
        assert!((trace.result.nominal.0 - 30.0).abs() < 1.0e-9);
        assert!((trace.result.min.0 - 29.7).abs() < 1.0e-9);
        assert!((trace.result.max.0 - 30.3).abs() < 1.0e-9);
        assert!(trace.warnings.is_empty());
    }

    #[test]
    fn accumulate_propagates_warnings_from_each_spec() {
        let specs = [
            ToleranceSpec::Limit {
                min: LengthMm(10.0),
                max: LengthMm(9.0),
            },
            ToleranceSpec::Symmetric {
                nominal: LengthMm(5.0),
                plus_minus: LengthMm(0.1),
            },
        ];
        let trace = accumulate(&specs);
        assert_eq!(trace.warnings, vec!["minimum exceeds maximum".to_owned()]);
    }

    #[test]
    fn accumulate_warns_on_empty_input() {
        let trace = accumulate(&[]);
        assert_eq!(trace.result.nominal, LengthMm(0.0));
        assert_eq!(
            trace.warnings,
            vec!["no tolerance specs to accumulate".to_owned()]
        );
    }

    #[test]
    fn round_for_display_does_not_change_source_value() {
        let value = LengthMm(1.0 / 3.0);
        let rounded = round_for_display(value, 3);
        assert!((rounded - 0.333).abs() < 1.0e-9);
        // The original LengthMm passed in is untouched (values are Copy).
        assert!((value.0 - 1.0 / 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn format_range_for_display_rounds_without_mutating_range() {
        let range = ToleranceRange {
            nominal: LengthMm(10.0),
            min: LengthMm(9.0 + 1.0 / 3.0),
            max: LengthMm(10.0 + 1.0 / 3.0),
        };
        let formatted = format_range_for_display(&range, 2);
        assert_eq!(formatted, "10.00 [9.33, 10.33] mm");
        // Full precision is preserved on the original struct.
        assert!((range.min.0 - (9.0 + 1.0 / 3.0)).abs() < 1.0e-12);
    }
}
