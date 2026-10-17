//! Exact values for the numbers the analysis finds.
//!
//! Windows' engine solves symbolically and shows `π/2` or `−2√3 − 6`. CalcCraft finds
//! the same features numerically, then recognizes the doubles as the simple exact values
//! they are: integers, fractions, rational multiples of π, and (a + b√c)/d. Anything
//! else is shown as a decimal with 6 significant digits.

use std::f64::consts::PI;

/// A recognized number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Exact {
    /// p/q, in lowest terms with q > 0.
    Rational(i64, i64),
    /// pπ/q.
    Pi(i64, i64),
    /// (p/q)√c + r/s, with c square-free and above 1.
    Surd {
        p: i64,
        q: i64,
        c: i64,
        r: i64,
        s: i64,
    },
    Decimal(f64),
}

/// The greatest common divisor (the analysis reduces periods with it too).
pub(super) fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

fn reduce(p: i64, q: i64) -> (i64, i64) {
    let g = gcd(p, q).max(1);
    let (p, q) = (p / g, q / g);
    if q < 0 { (-p, -q) } else { (p, q) }
}

/// Whether `v` is `target` within `tolerance` (relative to the size of the numbers).
fn near(v: f64, target: f64, tolerance: f64) -> bool {
    (v - target).abs() <= tolerance * v.abs().max(1.0)
}

/// p/q with q ≤ `max_q`, if `v` is one.
fn rational(v: f64, max_q: i64, tolerance: f64) -> Option<(i64, i64)> {
    if v.abs() > 1e12 {
        return None;
    }
    (1..=max_q).find_map(|q| {
        let p = (v * q as f64).round();
        near(v, p / q as f64, tolerance).then(|| reduce(p as i64, q))
    })
}

const SQUARE_FREE: [i64; 18] = [
    2, 3, 5, 6, 7, 10, 11, 13, 14, 15, 17, 19, 21, 22, 23, 26, 29, 30,
];

/// Recognizes `v` within `tolerance` (relative). Simpler forms win.
pub fn recognize(v: f64, tolerance: f64) -> Exact {
    if !v.is_finite() {
        return Exact::Decimal(v);
    }
    if let Some((p, q)) = rational(v, 12, tolerance) {
        return Exact::Rational(p, q);
    }
    // Large numbers have too many near misses to be told apart.
    if v.abs() > 1000.0 {
        return Exact::Decimal(v);
    }
    if let Some((p, q)) = rational(v / PI, 12, tolerance) {
        return Exact::Pi(p, q);
    }
    // (p/q)√c + r/s: try each radicand and coefficient, and see whether the rest is a
    // simple fraction.
    for c in SQUARE_FREE {
        let root = (c as f64).sqrt();
        for q in 1..=12i64 {
            for p in -12..=12i64 {
                if p == 0 {
                    continue;
                }
                let rest = v - p as f64 / q as f64 * root;
                if let Some((r, s)) = rational(rest, 12, tolerance)
                    && near(
                        v,
                        p as f64 / q as f64 * root + r as f64 / s as f64,
                        tolerance,
                    )
                {
                    let (p, q) = reduce(p, q);
                    return Exact::Surd { p, q, c, r, s };
                }
            }
        }
    }
    Exact::Decimal(v)
}

impl Exact {
    /// The number, computed from its exact form.
    pub fn value(self) -> f64 {
        match self {
            Exact::Rational(p, q) => p as f64 / q as f64,
            Exact::Pi(p, q) => p as f64 * PI / q as f64,
            Exact::Surd { p, q, c, r, s } => {
                p as f64 / q as f64 * (c as f64).sqrt() + r as f64 / s as f64
            }
            Exact::Decimal(v) => v,
        }
    }
}

/// `v` moved to the exact value it is near, if any.
pub fn snap(v: f64, tolerance: f64) -> f64 {
    recognize(v, tolerance).value()
}

/// The tolerance for values found by root finding, which are accurate to a few ulps.
pub const TIGHT: f64 = 1e-9;
/// The tolerance for limits, which are estimated from far-away values.
pub const LOOSE: f64 = 1e-6;

/// `v` written the way Windows shows numbers in the analysis.
pub fn format(v: f64) -> String {
    format_exact(recognize(v, TIGHT))
}

/// `v` written exactly if it is a limit-like estimate.
pub fn format_loose(v: f64) -> String {
    format_exact(recognize(v, LOOSE))
}

const MINUS: char = '−';

fn fraction(p: i64, q: i64) -> String {
    let sign = if p < 0 {
        MINUS.to_string()
    } else {
        String::new()
    };
    if q == 1 {
        format!("{sign}{}", p.abs())
    } else {
        format!("{sign}{}/{q}", p.abs())
    }
}

/// `coefficient · symbol` for a fraction p/q: `π`, `−π`, `3π/2`, `√3/3`, `2√3`.
fn times(p: i64, q: i64, symbol: &str) -> String {
    let sign = if p < 0 {
        MINUS.to_string()
    } else {
        String::new()
    };
    let a = p.abs();
    let mut s = sign;
    if a != 1 {
        s.push_str(&a.to_string());
    }
    s.push_str(symbol);
    if q != 1 {
        s.push('/');
        s.push_str(&q.to_string());
    }
    s
}

pub fn format_exact(exact: Exact) -> String {
    match exact {
        Exact::Rational(p, q) => fraction(p, q),
        Exact::Pi(0, _) => "0".to_string(),
        Exact::Pi(p, q) => times(p, q, "π"),
        Exact::Surd { p, q, c, r, s } => {
            let mut out = times(p, q, &format!("√{c}"));
            if r != 0 {
                out.push_str(if r > 0 { " + " } else { " − " });
                out.push_str(&fraction(r.abs(), s));
            }
            out
        }
        Exact::Decimal(v) => decimal(v),
    }
}

/// A decimal with 6 significant digits, without trailing zeros.
pub fn decimal(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let digits = (5 - v.abs().log10().floor() as i32).clamp(0, 15) as usize;
    let mut s = format!("{:.*}", digits, v);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s.replace('-', &MINUS.to_string())
}

/// `k·x`, the slope part of a line: `x`, `−x`, `2x`, `x/2`, `−3x`, with x before the
/// denominator as there (`3πx/2`, `√3x/3`), and a slope of two terms as one factor:
/// `(√2 + 1)x`, not `√2 + 1x`.
pub fn coefficient_of_x(m: f64) -> String {
    match recognize(m, LOOSE) {
        Exact::Rational(p, q) => times(p, q, "x"),
        Exact::Pi(p, q) => times(p, q, "πx"),
        Exact::Surd { p, q, c, r: 0, .. } => times(p, q, &format!("√{c}x")),
        other @ Exact::Surd { .. } => format!("({})x", format_exact(other)),
        other => format!("{}x", format_exact(other)),
    }
}

/// `text` as a term after another: ` + text`, or ` − rest` if `text` is `−rest`, so that a
/// value of two terms adds with its own signs: x + (1 − √2), whose value is written
/// `−√2 + 1`, is `x − √2 + 1`.
pub fn plus_term(text: &str) -> String {
    match text.strip_prefix(MINUS) {
        Some(rest) => format!(" {MINUS} {rest}"),
        None => format!(" + {text}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rationals() {
        assert_eq!(format(0.0), "0");
        assert_eq!(format(5.0), "5");
        assert_eq!(format(-2.5), "−5/2");
        assert_eq!(format(1.0 / 3.0), "1/3");
        assert_eq!(format(1e-12), "0");
    }

    #[test]
    fn multiples_of_pi() {
        assert_eq!(format(PI), "π");
        assert_eq!(format(-PI), "−π");
        assert_eq!(format(2.0 * PI), "2π");
        assert_eq!(format(PI / 2.0), "π/2");
        assert_eq!(format(3.0 * PI / 2.0), "3π/2");
        assert_eq!(format(-PI / 4.0), "−π/4");
    }

    #[test]
    fn square_roots() {
        // The extrema of (−3x² + 2)/(x − 1) in Windows' manual tests.
        let s3 = 3f64.sqrt();
        assert_eq!(format(s3 / 3.0 + 1.0), "√3/3 + 1");
        assert_eq!(format(-s3 / 3.0 + 1.0), "−√3/3 + 1");
        assert_eq!(format(-2.0 * s3 - 6.0), "−2√3 − 6");
        assert_eq!(format(2.0 * s3 - 6.0), "2√3 − 6");
        assert_eq!(format(6f64.sqrt() / 3.0), "√6/3");
        assert_eq!(format(-(2f64.sqrt())), "−√2");
    }

    #[test]
    fn decimals() {
        assert_eq!(format(1f64.sin()), "0.841471");
        assert_eq!(format(-123456.789), "−123457");
        assert_eq!(decimal(0.000123456789), "0.000123457");
    }

    #[test]
    fn slopes() {
        assert_eq!(coefficient_of_x(1.0), "x");
        assert_eq!(coefficient_of_x(-1.0), "−x");
        assert_eq!(coefficient_of_x(-3.0000001), "−3x");
        assert_eq!(coefficient_of_x(0.5), "x/2");
        // x before the denominator, as in x/2, and a slope of two terms in parentheses.
        assert_eq!(coefficient_of_x(1.5 * PI), "3πx/2");
        assert_eq!(coefficient_of_x(-PI / 4.0), "−πx/4");
        assert_eq!(coefficient_of_x(3f64.sqrt() / 3.0), "√3x/3");
        assert_eq!(coefficient_of_x(2f64.sqrt() + 1.0), "(√2 + 1)x");
    }
}
