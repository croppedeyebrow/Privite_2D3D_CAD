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
}
