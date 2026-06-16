//! Numeric formatting helpers that reproduce JavaScript's `Math.round` and
//! `Number.prototype.toFixed` semantics exactly.
//!
//! The grader's output (GRADE.md and `--json`) must stay byte-identical to the
//! original Node implementation, so rounding cannot be left to Rust's default
//! `{:.N}` formatter (which rounds half-to-even). All project scores are
//! non-negative, which lets these helpers stay simple and exact.

/// JavaScript `Math.round`: round half toward +∞. Only ever called on
/// non-negative values here, where it coincides with "round half away from
/// zero" — but the definition is written to match `Math.round` precisely.
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// JavaScript `Number.prototype.toFixed(digits)` for non-negative inputs.
///
/// `toFixed` rounds the *true* value of the float (not `x * 10^digits`, which
/// would double-round: e.g. `0.15` is stored as `0.1499…`, so `0.15.toFixed(1)`
/// is `"0.1"`, but `(0.15*10).round()` is `2`). We reproduce it by rendering the
/// faithful decimal expansion — far more digits than an `f64` carries — and then
/// rounding that decimal string half away from zero, which is what ECMAScript
/// specifies for the (here always non-negative) inputs.
pub fn to_fixed(x: f64, digits: usize) -> String {
    let faithful = format!("{:.*}", digits + 25, x);
    round_decimal_str(&faithful, digits)
}

/// Round a non-negative decimal string to `digits` fractional places, half away
/// from zero, propagating any carry into the integer part.
fn round_decimal_str(s: &str, digits: usize) -> String {
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i.to_string(), f.to_string()),
        None => (s.to_string(), String::new()),
    };

    // Digits we keep, padded with zeros if the input was shorter.
    let mut kept: Vec<u8> = frac_part.bytes().take(digits).collect();
    while kept.len() < digits {
        kept.push(b'0');
    }

    // Round up when the first dropped digit is >= 5 (ties round away).
    let round_up = frac_part.as_bytes().get(digits).is_some_and(|&d| d >= b'5');

    let mut int_digits: Vec<u8> = int_part.into_bytes();
    if round_up {
        let mut carry = true;
        for d in kept.iter_mut().rev() {
            if !carry {
                break;
            }
            if *d == b'9' {
                *d = b'0';
            } else {
                *d += 1;
                carry = false;
            }
        }
        if carry {
            propagate_carry(&mut int_digits);
        }
    }

    let int_str = String::from_utf8(int_digits).unwrap();
    if digits == 0 {
        int_str
    } else {
        format!("{int_str}.{}", String::from_utf8(kept).unwrap())
    }
}

/// Add one to a base-10 digit string, prepending a new leading digit on overflow.
fn propagate_carry(int_digits: &mut Vec<u8>) {
    let mut carry = true;
    for d in int_digits.iter_mut().rev() {
        if !carry {
            break;
        }
        if *d == b'9' {
            *d = b'0';
        } else {
            *d += 1;
            carry = false;
        }
    }
    if carry {
        int_digits.insert(0, b'1');
    }
}

/// Render a number the way `JSON.stringify` does: integral values print with no
/// fractional part (`20`, not `20.0`); everything else uses the shortest
/// round-trippable decimal (which `serde_json`/ryu and V8 both produce).
pub fn json_number(x: f64) -> serde_json::Value {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 9.007_199_254_740_992e15 {
        serde_json::Value::from(x as i64)
    } else {
        serde_json::Value::from(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_matches_js() {
        assert_eq!(to_fixed(13.5, 1), "13.5");
        assert_eq!(to_fixed(20.0, 1), "20.0");
        assert_eq!(to_fixed(0.0, 2), "0.00");
        assert_eq!(to_fixed(0.675, 2), "0.68"); // 0.675*100 = 67.5 -> 68
        assert_eq!(to_fixed(0.666_666, 2), "0.67");
        assert_eq!(to_fixed(84.25, 1), "84.3");
        // 0.15 stored as 0.1499.. -> rounds down, exactly like JS toFixed.
        assert_eq!(to_fixed(0.15, 1), "0.1");
        // Exact tie (12.25 is representable) rounds away from zero.
        assert_eq!(to_fixed(12.25, 1), "12.3");
        // Carry that propagates into and grows the integer part.
        assert_eq!(to_fixed(9.96, 1), "10.0");
        assert_eq!(to_fixed(99.99, 1), "100.0");
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(100.0, 1), "100.0");
    }

    #[test]
    fn percent_formatting() {
        // pct = Math.round(total*10)/10
        assert_eq!(js_round(84.234 * 10.0) / 10.0, 84.2);
        assert_eq!(to_fixed(js_round(100.0 * 10.0) / 10.0, 1), "100.0");
    }

    #[test]
    fn json_number_trims_integers() {
        assert_eq!(json_number(20.0).to_string(), "20");
        assert_eq!(json_number(13.5).to_string(), "13.5");
        assert_eq!(json_number(0.0).to_string(), "0");
    }
}
