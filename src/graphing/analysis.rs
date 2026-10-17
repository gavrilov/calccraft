//! The key graph features of a function y = f(x), Windows' "Function analysis": domain,
//! range, intercepts, extrema, inflection points, asymptotes, parity, period and
//! monotonicity.
//!
//! Windows computes them symbolically with a proprietary engine. CalcCraft finds them
//! numerically, from the function and its symbolic derivatives: zeros by sign changes
//! (refined by bisection) on a grid from −1000 to 1000, limits from far-away and nearby
//! values, and the exact values with `exact`. A function made of trigonometric
//! functions of linear arguments (sin x, tan 2x, cos² x …) is periodic: it's analyzed
//! over one period, and its features are written as families (`{πn₁, n₁ ∈ ℤ}`), as in
//! Windows. What can't be found that way is listed as too complex, as Windows lists
//! what its engine can't do.

use super::exact::{self, LOOSE, TIGHT};
use super::expr::{BinOp, Env, Expr, Func, radians_per_unit};
use super::{Equation, Shape};
use crate::engine::Angle;

/// Why there is no analysis (Windows' `AnalysisErrorType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisError {
    /// Inequalities, functions with free variables (`y = mx + b`) and functions without
    /// real values (`y = √(−1)`).
    NotSupported,
    /// Equations that aren't y = f(x).
    VariableIsNotX,
}

impl AnalysisError {
    /// The key of the message in Windows' resources.
    pub fn key(self) -> &'static str {
        match self {
            AnalysisError::NotSupported => "KGFAnalysisNotSupported",
            AnalysisError::VariableIsNotX => "KGFVariableIsNotX",
        }
    }
}

/// A feature, to name the ones that are too complex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Domain,
    Range,
    XIntercept,
    Minima,
    Maxima,
    InflectionPoints,
    VerticalAsymptotes,
    HorizontalAsymptotes,
    ObliqueAsymptotes,
    Monotonicity,
}

impl Feature {
    /// The key of the feature's title in Windows' resources.
    pub fn key(self) -> &'static str {
        match self {
            Feature::Domain => "Domain",
            Feature::Range => "Range",
            Feature::XIntercept => "XIntercept",
            Feature::Minima => "Minima",
            Feature::Maxima => "Maxima",
            Feature::InflectionPoints => "InflectionPoints",
            Feature::VerticalAsymptotes => "VerticalAsymptotes",
            Feature::HorizontalAsymptotes => "HorizontalAsymptotes",
            Feature::ObliqueAsymptotes => "ObliqueAsymptotes",
            Feature::Monotonicity => "Monotonicity",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    Odd,
    Even,
    Neither,
    Unknown,
}

impl Parity {
    pub fn key(self) -> &'static str {
        match self {
            Parity::Odd => "KGFParityOdd",
            Parity::Even => "KGFParityEven",
            Parity::Neither => "KGFParityNeither",
            Parity::Unknown => "KGFParityUnknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Increasing,
    Decreasing,
    Constant,
}

impl Direction {
    pub fn key(self) -> &'static str {
        match self {
            Direction::Increasing => "KGFMonotonicityIncreasing",
            Direction::Decreasing => "KGFMonotonicityDecreasing",
            Direction::Constant => "KGFMonotonicityConstant",
        }
    }
}

/// The period: shown only for periodic functions (Windows' `PeriodicityDirection` 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Period {
    None,
    Periodic(String),
}

/// The features, as the analysis panel shows them. An empty list or `None` gets the
/// feature's "does not have" message (or "Unable to calculate" for domain and range).
#[derive(Clone, Debug, PartialEq)]
pub struct Features {
    pub domain: Option<String>,
    pub range: Option<String>,
    pub x_intercepts: Option<String>,
    pub y_intercept: Option<String>,
    pub minima: Vec<String>,
    pub maxima: Vec<String>,
    pub inflection_points: Vec<String>,
    pub vertical_asymptotes: Vec<String>,
    pub horizontal_asymptotes: Vec<String>,
    pub oblique_asymptotes: Vec<String>,
    pub parity: Parity,
    pub period: Period,
    pub monotonicity: Vec<(String, Direction)>,
    pub too_complex: Vec<Feature>,
}

impl Features {
    fn empty() -> Features {
        Features {
            domain: None,
            range: None,
            x_intercepts: None,
            y_intercept: None,
            minima: Vec::new(),
            maxima: Vec::new(),
            inflection_points: Vec::new(),
            vertical_asymptotes: Vec::new(),
            horizontal_asymptotes: Vec::new(),
            oblique_asymptotes: Vec::new(),
            parity: Parity::Unknown,
            period: Period::None,
            monotonicity: Vec::new(),
            too_complex: Vec::new(),
        }
    }
}

/// Analyzes an equation in the angle unit `angle`.
pub fn analyze(equation: &Equation, angle: Angle) -> Result<Features, AnalysisError> {
    let Shape::Function(e) = &equation.shape else {
        return Err(AnalysisError::VariableIsNotX);
    };
    if equation.is_inequality() || !equation.variables.is_empty() {
        return Err(AnalysisError::NotSupported);
    }
    let env = Env { angle, vars: &[] };
    let fun = Fun::new(e, env);
    // A function without real values (√(−1), √(−1 − x²)) has no features to show. When
    // Windows' analysis fails, it shows `AnalysisCouldNotBePerformed`'s message, which isn't
    // among the strings here; "not supported" is the closest.
    if !e.has_x() {
        let c = fun.f(0.0);
        return if c.is_finite() {
            Ok(constant(c))
        } else {
            Err(AnalysisError::NotSupported)
        };
    }
    let mut features = match period(e, angle).and_then(|p| fun.verified_period(p)) {
        Some(p) => periodic(&fun, p),
        None => aperiodic(&fun).ok_or(AnalysisError::NotSupported)?,
    };
    features.parity = parity(&fun);
    if features.domain.is_none() {
        features.too_complex.insert(0, Feature::Domain);
    }
    Ok(features)
}

/// The features of a constant function with a real value `c`.
fn constant(c: f64) -> Features {
    let mut features = Features::empty();
    features.domain = Some("{x ∈ ℝ}".to_string());
    let value = exact::format(c);
    features.range = Some(format!("{{y = {value}}}"));
    features.y_intercept = Some(value);
    if c == 0.0 {
        features.too_complex.push(Feature::XIntercept);
    }
    features.parity = Parity::Even;
    features
        .monotonicity
        .push(("(−∞, ∞)".to_string(), Direction::Constant));
    features
}

/// The function with its first two derivatives.
struct Fun<'a> {
    e: &'a Expr,
    d1: Option<Expr>,
    d2: Option<Expr>,
    env: Env<'a>,
    /// Whether it jumps (floor, ceiling, sign) or has a factorial, whose features need
    /// more than derivatives.
    rough: bool,
}

impl<'a> Fun<'a> {
    fn new(e: &'a Expr, env: Env<'a>) -> Self {
        let d1 = e.derivative(env.angle);
        let d2 = d1.as_ref().and_then(|d| d.derivative(env.angle));
        let rough = e.any(&|n| {
            matches!(
                n,
                Expr::Call(Func::Floor | Func::Ceil | Func::Sign, _) | Expr::Factorial(_)
            )
        });
        Fun {
            e,
            d1,
            d2,
            env,
            rough,
        }
    }

    fn f(&self, x: f64) -> f64 {
        self.e.eval(x, 0.0, &self.env)
    }

    fn d1(&self, x: f64) -> f64 {
        self.d1
            .as_ref()
            .map_or(f64::NAN, |d| d.eval(x, 0.0, &self.env))
    }

    fn d2(&self, x: f64) -> f64 {
        self.d2
            .as_ref()
            .map_or(f64::NAN, |d| d.eval(x, 0.0, &self.env))
    }

    /// Whether f is defined at x. A pole that doubles land next to (tan(π/2) is 1.6e16)
    /// isn't, while a value beyond 1e14 that f doesn't grow toward is (`large_value`).
    fn defined(&self, x: f64) -> bool {
        let v = self.f(x);
        v.is_finite() && (v.abs() < 1e14 || large_value(self, x))
    }

    /// `p`, or the smallest part of it that is a period too (|sin x| has π).
    fn verified_period(&self, p: f64) -> Option<f64> {
        let tests: Vec<f64> = (0..23).map(|k| -7.3 + 0.731 * k as f64).collect();
        // Beyond 1e14, where main had no values, an allowance relative to the value would
        // swallow f's variation on a large constant (sin(x) + 10^15 would repeat every
        // π/6): there it is 10⁻⁹ of f's spread over the tests plus two ulps, and f′, which
        // the constant leaves out, has to repeat too (sin(x°) hardly varies there).
        let (lo, hi) = tests
            .iter()
            .map(|&t| self.f(t))
            .filter(|v| v.is_finite())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        let (spread, large) = (
            1e-9 * hi - 1e-9 * lo,
            lo <= hi && lo.abs().max(hi.abs()) >= 1e14,
        );
        let slopes_repeat = |q: f64| {
            tests.iter().all(|&t| {
                let (a, b) = (self.d1(t), self.d1(t + q));
                a.is_finite() == b.is_finite()
                    && (!a.is_finite() || (a - b).abs() <= 1e-9 * (1.0 + a.abs()))
            })
        };
        let repeats = |q: f64| {
            let mut compared = 0;
            for &t in &tests {
                let (a, b) = (self.f(t), self.f(t + q));
                if a.is_finite() != b.is_finite() {
                    return false;
                }
                if a.is_finite() {
                    compared += 1;
                    let allowance = if a.abs() < 1e14 {
                        1e-9 * (1.0 + a.abs())
                    } else {
                        spread + 2.0 * (a.abs() - a.abs().next_down())
                    };
                    if (a - b).abs() > allowance {
                        return false;
                    }
                }
            }
            compared > 5 && (!large || slopes_repeat(q))
        };
        if !repeats(p) {
            return None;
        }
        (1..=12).rev().map(|k| p / k as f64).find(|&q| repeats(q))
    }
}

/// Whether `e` is exact at x: computed without rounding from x and the numbers as
/// written, so that a 0 there is a true 0 (x² − 4 at 2), not one rounding made
/// ((1 + 10⁻²⁰) − 1, e^(−1000)).
fn is_exact(e: &Expr, x: f64, env: &Env) -> bool {
    let at = |p: &Expr| p.eval(x, 0.0, env);
    let part = |p: &Expr| is_exact(p, x, env);
    let v = at(e);
    match e {
        Expr::Num(_) | Expr::Pi | Expr::E | Expr::X | Expr::Y | Expr::Var(_) => true,
        Expr::Neg(a) => part(a),
        Expr::Bin(op, a, b) => {
            let (p, q) = (at(a), at(b));
            let parts = || part(a) && part(b);
            // 0 times a finite number, or over one, is 0 however that number was rounded.
            let zero = |z: f64, of: &Expr, w: f64| z == 0.0 && w.is_finite() && part(of);
            match op {
                BinOp::Add => parts() && exact_sum(p, q, v),
                BinOp::Sub => parts() && exact_sum(p, -q, v),
                BinOp::Mul => zero(p, a, q) || zero(q, b, p) || (parts() && exact_product(p, q, v)),
                BinOp::Div => q != 0.0 && (zero(p, a, q) || (parts() && exact_product(v, q, p))),
                BinOp::Pow => {
                    parts()
                        && ((p == 0.0 && q > 0.0) || q == 0.0 || p == 1.0 || exact_power(p, q, v))
                }
            }
        }
        Expr::Root(a, n) => {
            let (p, q) = (at(a), at(n));
            part(a) && part(n) && ((p == 0.0 && q > 0.0) || p == 1.0 || exact_power(v, q, p))
        }
        // log_b 1 is 0 and log_b b is 1.
        Expr::LogBase(b, a) => {
            let (p, q) = (at(b), at(a));
            part(b) && part(a) && (q == 1.0 || q == p)
        }
        Expr::Call(f, a) => {
            let p = at(a);
            part(a)
                && match f {
                    Func::Abs | Func::Floor | Func::Ceil | Func::Sign => true,
                    Func::Sqrt => exact_power(v, 2.0, p),
                    Func::Cbrt => exact_power(v, 3.0, p),
                    // sin 0 = 0, cos 0 = 1, ln 1 = 0, acos 1 = 0 …
                    _ => [p, v].iter().all(|w| [0.0, 1.0, -1.0].contains(w)),
                }
        }
        Expr::Factorial(_) => false,
    }
}

/// Whether p + q is exactly v (Fast2Sum: the larger minus the sum is exact).
fn exact_sum(p: f64, q: f64, v: f64) -> bool {
    let (big, small) = if p.abs() >= q.abs() { (p, q) } else { (q, p) };
    v - big == small
}

/// Whether p·q is exactly v: a fused multiply-add's residual is, away from the smallest
/// doubles, where it would round too (a product that underflows isn't exact).
fn exact_product(p: f64, q: f64, v: f64) -> bool {
    if v == 0.0 {
        p == 0.0 || q == 0.0
    } else {
        v.abs() > 1e-280 && p.mul_add(q, -v) == 0.0
    }
}

/// Whether a^n is exactly v for a whole n from 1 to 64: every product on the way is.
fn exact_power(a: f64, n: f64, v: f64) -> bool {
    if n.fract() != 0.0 || !(1.0..=64.0).contains(&n) {
        return false;
    }
    let mut p = a;
    for _ in 1..n as u32 {
        let q = p * a;
        if !exact_product(p, a, q) {
            return false;
        }
        p = q;
    }
    p == v
}

/// e at t, and how far rounding can have moved it from its exact value: the parts' error
/// carried through each operation (evaluated at the ends of the parts' ranges), plus four
/// ULPs for the operation itself. x and the numbers as written are exact, π and e within
/// an ULP (sin π is 1.2e−16 in doubles); ∞ when unknown.
fn with_rounding_error(e: &Expr, t: f64, env: &Env) -> (f64, f64) {
    let (a, b) = match e {
        Expr::Neg(a) | Expr::Call(_, a) | Expr::Factorial(a) => (a, None),
        Expr::Bin(_, a, b) | Expr::Root(a, b) | Expr::LogBase(a, b) => (a, Some(b)),
        Expr::Pi | Expr::E => {
            let v = e.eval(t, 0.0, env);
            return (v, v.next_up() - v);
        }
        _ => return (e.eval(t, 0.0, env), 0.0),
    };
    let (a, da) = with_rounding_error(a, t, env);
    let (b, db) = b.map_or((0.0, 0.0), |b| with_rounding_error(b, t, env));
    let apply = |p: f64, q: f64| {
        let (p, q) = (Box::new(Expr::Num(p)), Box::new(Expr::Num(q)));
        match e {
            Expr::Neg(_) => Expr::Neg(p),
            Expr::Call(f, _) => Expr::Call(*f, p),
            Expr::Factorial(_) => Expr::Factorial(p),
            Expr::Bin(op, ..) => Expr::Bin(*op, p, q),
            Expr::Root(..) => Expr::Root(p, q),
            _ => Expr::LogBase(p, q),
        }
        .eval(t, 0.0, env)
    };
    let v = apply(a, b);
    let carried = [(-da, -db), (-da, db), (da, -db), (da, db)]
        .iter()
        .map(|(ea, eb)| (apply(a + ea, b + eb) - v).abs())
        .fold(
            0.0,
            |m: f64, d| if d.is_nan() { f64::INFINITY } else { m.max(d) },
        );
    (v, carried + 4.0 * (v.abs().next_up() - v.abs()))
}

/// The factors of g that are 0 where g is: those of products and quotients' numerators,
/// the bases of positive powers, and the arguments of functions that are 0 at 0 (and sin
/// and tan themselves, 0 at πn too).
fn factors(g: &Expr, env: &Env, out: &mut Vec<Expr>) {
    match g {
        Expr::Bin(BinOp::Mul, a, b) => {
            factors(a, env, out);
            factors(b, env, out);
        }
        Expr::Bin(BinOp::Div, a, _) | Expr::Neg(a) => factors(a, env, out),
        Expr::Bin(BinOp::Pow, a, n) if !n.has_x() && n.eval(0.0, 0.0, env) > 0.0 => {
            factors(a, env, out)
        }
        Expr::Call(Func::Sin | Func::Tan, a) => {
            out.push(g.clone());
            factors(a, env, out);
        }
        Expr::Call(
            Func::Asin
            | Func::Atan
            | Func::Sinh
            | Func::Tanh
            | Func::Asinh
            | Func::Atanh
            | Func::Sqrt
            | Func::Cbrt
            | Func::Abs
            | Func::Sign,
            a,
        ) => factors(a, env, out),
        _ => out.push(g.clone()),
    }
}

/// Whether g has a root at x within the doubles' reach: it is exactly 0 there (x² at 0),
/// or a factor of it crosses 0 right there by more than rounding (x² − 2 at √2, (3x − 1)²
/// at 1/3), not at a jump (x² + ceil(x) − 1 near 0) or in rounding noise ((x + 0.1) − x −
/// 0.1). x² + e^(−1000) and x² + 10⁻²⁰ only touch 0.
fn root_at(g: &Expr, x: f64, env: &Env) -> bool {
    if g.eval(x, 0.0, env) == 0.0 && is_exact(g, x, env) {
        return true;
    }
    let mut parts = Vec::new();
    factors(g, env, &mut parts);
    let d = 1e-9 * x.abs().max(1.0);
    parts.iter().any(|h| {
        let (l, r) = (
            with_rounding_error(h, x - d, env),
            with_rounding_error(h, x + d, env),
        );
        (l.0 < 0.0) != (r.0 < 0.0) && l.0.abs() > l.1 && r.0.abs() > r.1
    })
}

// ------------------------------------------------------------------ period

/// The period of `e` if it is built from trigonometric functions of linear arguments
/// (and doesn't use x anywhere else): the least common multiple of their periods.
fn period(e: &Expr, angle: Angle) -> Option<f64> {
    fn walk(e: &Expr, angle: Angle) -> Result<Option<f64>, ()> {
        match e {
            Expr::X => Err(()),
            Expr::Call(f, arg) if f.takes_angle() => {
                if let Some(slope) = linear_slope(arg) {
                    let turn = 2.0 * std::f64::consts::PI / radians_per_unit(angle);
                    let base = if matches!(f, Func::Tan | Func::Cot) {
                        turn / 2.0
                    } else {
                        turn
                    };
                    return Ok(Some(base / slope.abs()));
                }
                walk(arg, angle)
            }
            Expr::Neg(a) | Expr::Call(_, a) | Expr::Factorial(a) => walk(a, angle),
            Expr::Bin(_, a, b) | Expr::Root(a, b) | Expr::LogBase(a, b) => {
                match (walk(a, angle)?, walk(b, angle)?) {
                    (Some(p), Some(q)) => lcm(p, q).map(Some).ok_or(()),
                    (p, q) => Ok(p.or(q)),
                }
            }
            _ => Ok(None),
        }
    }
    walk(e, angle).ok().flatten()
}

/// The slope a of a linear argument a·x + b with a ≠ 0.
fn linear_slope(arg: &Expr) -> Option<f64> {
    if !arg.has_x() {
        return None;
    }
    let at = |x: f64| arg.eval(x, 0.0, &Env::RADIANS);
    let (b, a) = (at(0.0), at(1.0) - at(0.0));
    if !a.is_finite() || a == 0.0 {
        return None;
    }
    [-3.7, -1.1, 2.3, 5.9, 17.0]
        .iter()
        .all(|&x| (at(x) - (a * x + b)).abs() <= 1e-9 * (1.0 + (a * x).abs()))
        .then_some(a)
}

/// The least common multiple of two periods whose ratio is a simple fraction.
fn lcm(p: f64, q: f64) -> Option<f64> {
    // The larger over the smaller, so that the order doesn't matter: 13 is 13/1, but
    // its inverse 1/13 has a denominator beyond the 12 tried.
    let (p, q) = (p.min(q), p.max(q));
    let ratio = q / p;
    (1..=12).find_map(|d| {
        let n = (ratio * d as f64).round();
        ((ratio * d as f64 - n).abs() < 1e-9 && n >= 1.0)
            .then(|| p * n / exact::gcd(n as i64, d) as f64)
    })
}

// ------------------------------------------------------------------ roots

/// The x values from −1000 to 1000 the aperiodic analysis samples: every 0.01 near
/// the origin, sparser further out.
fn grid() -> Vec<f64> {
    let mut right = vec![0.0];
    let mut x: f64 = 0.0;
    while x < 1000.0 {
        x += 0.01 * (x / 10.0).max(1.0);
        right.push(x);
    }
    let mut xs: Vec<f64> = right.iter().skip(1).rev().map(|x| -x).collect();
    xs.extend(right);
    xs
}

/// The points between consecutive samples where `g` changes sign, by bisection; and
/// the samples where it's 0. With `through_poles`, a change through infinity counts
/// too (a cusp of the derivative); otherwise `g` has to be small there, so poles and
/// jumps don't.
fn sign_changes(g: &dyn Fn(f64) -> f64, xs: &[f64], through_poles: bool) -> Vec<(f64, f64, f64)> {
    let values: Vec<f64> = xs.iter().map(|&x| g(x)).collect();
    let mut out = Vec::new();
    for i in 1..xs.len() {
        let (prev, cur) = ((xs[i - 1], values[i - 1]), (xs[i], values[i]));
        if through_poles && !cur.1.is_finite() {
            // A sample right on a cusp (x^(2/3)′ at 0).
            if let Some(&after) = values.get(i + 1)
                && prev.1.is_finite()
                && after.is_finite()
                && prev.1 != 0.0
                && after != 0.0
                && (prev.1 > 0.0) != (after > 0.0)
            {
                out.push((cur.0, prev.0, xs[i + 1]));
            }
        } else if cur.1 == 0.0 {
            // A sample right on a zero: a sign change if the neighbors differ.
            if let (Some(&before), Some(&after)) = (values.get(i - 1), values.get(i + 1))
                && before.is_finite()
                && after.is_finite()
                && (before > 0.0) != (after > 0.0)
                && before != 0.0
                && after != 0.0
            {
                out.push((cur.0, prev.0, xs[i + 1]));
            }
        } else if prev.1.is_finite()
            && cur.1.is_finite()
            && prev.1 != 0.0
            && (prev.1 > 0.0) != (cur.1 > 0.0)
        {
            let (mut a, mut b) = (prev, cur);
            for _ in 0..80 {
                let m = (a.0 + b.0) / 2.0;
                if m <= a.0 || m >= b.0 {
                    break;
                }
                let v = g(m);
                if v == 0.0 {
                    a = (m, v);
                    b = (m, v);
                    break;
                }
                if !v.is_finite() {
                    break;
                }
                if (v > 0.0) == (a.1 > 0.0) {
                    a = (m, v);
                } else {
                    b = (m, v);
                }
            }
            let c = (a.0 + b.0) / 2.0;
            let small = g(c).abs() <= 1e-6 * (prev.1.abs() + cur.1.abs()) + 1e-12;
            if through_poles || small {
                out.push((c, prev.0, cur.0));
            }
        }
    }
    out
}

/// The zeros of `g`: sign changes, and touching zeros (zeros of `dg` where g is 0, like
/// x² at 0). Runs of zeros, where a value underflows (e^(−x²) far out), don't count.
fn zeros(g: &dyn Fn(f64) -> f64, dg: Option<&dyn Fn(f64) -> f64>, xs: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = sign_changes(g, xs, false)
        .into_iter()
        .map(|c| c.0)
        .collect();
    if let Some(dg) = dg {
        for (c, a, b) in sign_changes(dg, xs, true) {
            let scale = g(a).abs().max(g(b).abs());
            if g(c).abs() <= 1e-9 * scale.max(1e-12) || g(c) == 0.0 {
                out.push(c);
            }
        }
    }
    dedupe(out)
}

/// The sign of `d` just left of `c`: whether a turning point there is a minimum.
fn falls_before(d: &dyn Fn(f64) -> f64, c: f64) -> bool {
    d(c - 1e-7 * c.abs().max(1.0)) < 0.0
}

fn dedupe(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * a.abs().max(1.0));
    v
}

/// The key points for the best fit: zeros of f, f′ and f″ from `lo` to `hi`.
pub fn key_points(
    f: &dyn Fn(f64) -> f64,
    derivative: Option<&Expr>,
    env: &Env,
    lo: f64,
    hi: f64,
) -> Vec<f64> {
    let xs: Vec<f64> = grid()
        .into_iter()
        .filter(|x| (lo..=hi).contains(x))
        .collect();
    let mut points: Vec<f64> = sign_changes(f, &xs, false)
        .into_iter()
        .map(|c| c.0)
        .collect();
    if let Some(d1) = derivative {
        let g = |x: f64| d1.eval(x, 0.0, env);
        points.extend(sign_changes(&g, &xs, true).into_iter().map(|c| c.0));
        if let Some(d2) = d1.derivative(env.angle) {
            let g = |x: f64| d2.eval(x, 0.0, env);
            points.extend(sign_changes(&g, &xs, true).into_iter().map(|c| c.0));
        }
    }
    dedupe(points)
}

// ------------------------------------------------------------------ domain

/// Where the expression is undefined: the zeros of these functions bound or punch the
/// domain (denominators, radicands, logarithms' arguments, cos of tan's argument …).
fn boundary_functions(e: &Expr, out: &mut Vec<Expr>) {
    use Expr::*;
    let sub = |a: &Expr, b: f64| Bin(BinOp::Sub, Box::new(a.clone()), Box::new(Num(b)));
    let square = |a: &Expr| Bin(BinOp::Pow, Box::new(a.clone()), Box::new(Num(2.0)));
    match e {
        Bin(op, a, b) => {
            match op {
                BinOp::Div => out.push((**b).clone()),
                BinOp::Pow => out.push((**a).clone()),
                _ => {}
            }
            boundary_functions(a, out);
            boundary_functions(b, out);
        }
        Call(f, a) => {
            match f {
                Func::Tan | Func::Sec => out.push(Call(Func::Cos, a.clone())),
                Func::Cot | Func::Csc => out.push(Call(Func::Sin, a.clone())),
                Func::Asin | Func::Acos | Func::Atanh | Func::Acoth | Func::Asec | Func::Acsc => {
                    out.push(sub(&square(a), 1.0))
                }
                Func::Acosh | Func::Asech => {
                    out.push(sub(a, 1.0));
                    out.push((**a).clone());
                }
                Func::Sqrt | Func::Ln | Func::Log | Func::Coth | Func::Csch | Func::Acsch => {
                    out.push((**a).clone())
                }
                _ => {}
            }
            boundary_functions(a, out);
        }
        Root(a, n) | LogBase(n, a) => {
            out.push((**a).clone());
            if matches!(e, LogBase(..)) {
                out.push((**n).clone());
                out.push(sub(n, 1.0));
            }
            boundary_functions(a, out);
            boundary_functions(n, out);
        }
        Neg(a) | Factorial(a) => boundary_functions(a, out),
        _ => {}
    }
}

/// An interval of the real line; an infinite end is open.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Interval {
    lo: f64,
    hi: f64,
    lo_closed: bool,
    hi_closed: bool,
}

/// The points where `fun` may start, stop or be punched out.
fn domain_candidates(fun: &Fun, xs: &[f64]) -> Vec<f64> {
    let mut functions = Vec::new();
    boundary_functions(fun.e, &mut functions);
    let mut candidates = Vec::new();
    for (i, g) in functions.iter().enumerate() {
        // A polynomial lists x once for each power.
        if !g.has_x() || functions[..i].contains(g) {
            continue;
        }
        let dg = g.derivative(fun.env.angle);
        let gf = |x: f64| g.eval(x, 0.0, &fun.env);
        let dgf = |x: f64| dg.as_ref().map_or(f64::NAN, |d| d.eval(x, 0.0, &fun.env));
        let dgf: &dyn Fn(f64) -> f64 = &dgf;
        candidates.extend(zeros(&gf, dg.as_ref().map(|_| dgf), xs));
    }
    dedupe(candidates)
}

/// Whether g has a root at x itself, not just within 10⁻⁹ of it as `root_at` allows: it
/// is exactly 0 there, or a factor of it changes sign across x as near as rounding lets
/// its sign be told (x² − 2 at √2, 3x − 1 at 1/3), while (x − 1) + 5·10⁻¹⁰ is positive
/// on both sides of 1.
fn root_right_at(g: &Expr, x: f64, env: &Env) -> bool {
    if !root_at(g, x, env) {
        return false;
    }
    if g.eval(x, 0.0, env) == 0.0 && is_exact(g, x, env) {
        return true;
    }
    let mut parts = Vec::new();
    factors(g, env, &mut parts);
    let ulp = x.abs().max(1.0) * f64::EPSILON;
    parts.iter().any(|h| {
        (0..23).map(|k| ulp * 2f64.powi(k)).find_map(|d| {
            let (l, r) = (
                with_rounding_error(h, x - d, env),
                with_rounding_error(h, x + d, env),
            );
            (l.0.abs() > l.1 && r.0.abs() > r.1).then_some((l.0 < 0.0) != (r.0 < 0.0))
        }) == Some(true)
    })
}

/// Whether a power with a variable exponent is 0⁰ at x: that's 1 in doubles, but x^x is
/// real only for x > 0 (and at single points left of 0). Base and exponent have to have a
/// root at x itself (`root_right_at`): 0^(10⁻¹³) and 0^(e^(−1000)) are 0, and so is
/// 0^((x − 1) + 5·10⁻¹⁰) at 1, while (3x − 1)² is 0 at 1/3 though doubles can't make it so.
/// Where both are exact there, their values tell ((x − 1) + 10⁻¹⁶ is 10⁻¹⁶ at 1), and an
/// exponent that is certainly not 0 (`nonzero`: x + e^(−1000) at 0) has no root there.
/// And the exponent has to vary arbitrarily near x, on each side where it has values:
/// x^(x − x) is x⁰, 1 at 0 too, and so are x^(((x + 0.1) − x) − 0.1), whose exponent
/// rounding makes −2.8e−17 at 0.37, and x^floor(10x²/(1 + x²)), whose exponent is 0 on
/// (−1/3, 1/3).
fn zero_to_the_zero(fun: &Fun, x: f64) -> bool {
    let env = &fun.env;
    let exact_zero = |e: &Expr| e.eval(x, 0.0, env) == 0.0 && is_exact(e, x, env);
    let exponent_zero = |base: &Expr, e: &Expr| {
        if exact_zero(base) && is_exact(e, x, env) {
            exact_zero(e)
        } else {
            !nonzero(e, x, env) && root_right_at(e, x, env)
        }
    };
    let scale = x.abs().max(1.0);
    // Whether e varies at x ± 10⁻², 10⁻⁴ … of x's scale, as near as the doubles resolve x
    // (a thousand ULPs: 3x − 1 is rounding next to 1/3); `None` if it has no values there.
    let ulp = x.abs().next_up() - x.abs();
    let side = |e: &Expr, sign: f64| {
        let probes: Vec<f64> = (1..=200)
            .map(|k| x + sign * scale * 10f64.powi(-2 * k))
            .take_while(|&t| (t - x).abs() > 1000.0 * ulp)
            .collect();
        if probes.iter().all(|&t| e.eval(t, 0.0, &fun.env).is_nan()) {
            return None;
        }
        Some(probes.iter().all(|&t| nonzero(e, t, &fun.env)))
    };
    let varies = |e: &Expr| match (side(e, -1.0), side(e, 1.0)) {
        (None, None) => false,
        (left, right) => left.unwrap_or(true) && right.unwrap_or(true),
    };
    fun.e.any(&|e| {
        matches!(e, Expr::Bin(BinOp::Pow, base, exponent)
            if root_right_at(base, x, env) && exponent_zero(base, exponent) && varies(exponent))
    })
}

/// Whether f's value at x, beyond 1e14, is one: finite, f doesn't grow toward it from
/// either side (`limit_at`: tan(π/2) is 1.6e16 in doubles, a pole's), x isn't a pole of a
/// part of f (`at_pole`), and no part without x is at one (`constant_pole`). x² + 10^15 is
/// 10^15 at 0, while x + tan(90°) has no values and tan(90° + x²/(1 + x²)) none at 0.
fn large_value(fun: &Fun, x: f64) -> bool {
    let grows = |side: f64| {
        matches!(
            limit_at(fun, x, side),
            Limit::PlusInfinity | Limit::MinusInfinity
        )
    };
    fun.f(x).is_finite()
        && !constant_pole(fun.e, &fun.env)
        && !at_pole(fun, x)
        && !grows(-1.0)
        && !grows(1.0)
}

/// Whether x is a pole of a part of f with x, as near as doubles tell, where f may not
/// grow toward it: tan(90° + x²/(1 + x²)) is tan(90°) = 1.6e16 next to 0 too, as 90 + 10⁻¹⁶
/// is 90 in doubles. A quotient's denominator, a base under a negative power or root, a
/// logarithm's base less 1 or the argument of coth or csch is 0 there as far as rounding
/// tells (`nonzero`), or so is cos (sin) of the argument of tan or sec (cot or csc): π/2
/// is rounded (cos(π/2) is 6.1e−17), and in degrees or gradians the argument is at 90° +
/// 180°n (180°n), as in 1/cos(90°).
fn at_pole(fun: &Fun, x: f64) -> bool {
    let env = &fun.env;
    let quarter = match env.angle {
        Angle::Degrees => 90.0,
        Angle::Gradians => 100.0,
        Angle::Radians => f64::NAN,
    };
    let turn = |a: &Expr, from: f64| (a.eval(x, 0.0, env) - from).rem_euclid(2.0 * quarter) == 0.0;
    let zero = |g: &Expr| {
        let mut parts = Vec::new();
        factors(g, env, &mut parts);
        !nonzero(g, x, env)
            || parts.iter().any(|h| match h {
                Expr::Call(Func::Cos, a) => turn(a, quarter),
                Expr::Call(Func::Sin | Func::Tan, a) => turn(a, 0.0),
                _ => false,
            })
    };
    let of = |f: Func, a: &Expr| Expr::Call(f, Box::new(a.clone()));
    fun.e.any(&|e| match e {
        Expr::Bin(BinOp::Div, _, b) => b.has_x() && zero(b),
        Expr::Bin(BinOp::Pow, a, n) | Expr::Root(a, n) => {
            a.has_x() && n.eval(x, 0.0, env) < 0.0 && zero(a)
        }
        Expr::LogBase(b, _) => {
            b.has_x() && zero(&Expr::Bin(BinOp::Sub, b.clone(), Box::new(Expr::Num(1.0))))
        }
        Expr::Call(Func::Tan | Func::Sec, a) => a.has_x() && zero(&of(Func::Cos, a)),
        Expr::Call(Func::Cot | Func::Csc, a) => a.has_x() && zero(&of(Func::Sin, a)),
        Expr::Call(Func::Coth | Func::Csch, a) => a.has_x() && zero(a),
        _ => false,
    })
}

/// Whether a part of f without x is at a pole, as main tells one: beyond 1e14, if its
/// operation has poles (tan(90°) is 1.6e16 in doubles, and 1/sin(180°) 8.2e15), however
/// deep it lies (x + 2·tan(90°), x + |tan(90°)|). A large number as written isn't (x +
/// 10^15), and where f's own values are below 1e14 they count (x/tan(π/2) is a line).
fn constant_pole(e: &Expr, env: &Env) -> bool {
    let poles = matches!(
        e,
        Expr::Bin(BinOp::Div, ..)
            | Expr::LogBase(..)
            | Expr::Call(
                Func::Tan
                    | Func::Sec
                    | Func::Cot
                    | Func::Csc
                    | Func::Ln
                    | Func::Log
                    | Func::Coth
                    | Func::Csch
                    | Func::Atanh
                    | Func::Acoth
                    | Func::Asech
                    | Func::Acsch,
                _
            )
    );
    if poles && !e.has_x() {
        let v = e.eval(0.0, 0.0, env).abs();
        if v >= 1e14 || v.is_nan() {
            return true;
        }
    }
    match e {
        Expr::Neg(a) | Expr::Call(_, a) | Expr::Factorial(a) => constant_pole(a, env),
        Expr::Bin(_, a, b) | Expr::Root(a, b) | Expr::LogBase(a, b) => {
            constant_pole(a, env) || constant_pole(b, env)
        }
        _ => false,
    }
}

/// Whether f's domain is cut, again and again, where a periodic part of a boundary is 0,
/// up to the grid's end or next to a piece that runs to ±∞, so that the cuts go on past
/// the grid and can't be listed. x·tan(x) in degrees is cut at 90° + 180°n, which the grid
/// holds up to 990°, and the piece past them was taken whole, as main took it where the
/// values stayed below 1e14 (x^10·tan(x) has values beyond 1e14 there). The boundaries are
/// main's (`boundary_functions`): denominators, bases and radicands, logarithms' arguments
/// and bases, cos of tan's and sec's argument, sin of cot's and csc's …; a periodic factor
/// of one (`period`) cuts where it crosses or touches 0 (1 + sin(x) at 270°). A single cut
/// isn't a recurring one: (sin x)²^(x² − 1) has its pole at 0 only.
fn recurring_cuts(fun: &Fun, intervals: &[Interval], edge: f64) -> bool {
    let env = &fun.env;
    let less =
        |a: &Expr, c: f64| Expr::Bin(BinOp::Sub, Box::new(a.clone()), Box::new(Expr::Num(c)));
    let of = |f: Func, a: &Expr| Expr::Call(f, Box::new(a.clone()));
    let found = std::cell::RefCell::new(Vec::new());
    fun.e.any(&|e| {
        let mut found = found.borrow_mut();
        match e {
            Expr::Bin(BinOp::Div, _, b) => found.push((**b).clone()),
            Expr::Bin(BinOp::Pow, a, _) | Expr::Root(a, _) => found.push((**a).clone()),
            Expr::Call(Func::Tan | Func::Sec, a) => found.push(of(Func::Cos, a)),
            Expr::Call(Func::Cot | Func::Csc, a) => found.push(of(Func::Sin, a)),
            Expr::Call(
                Func::Asin | Func::Acos | Func::Atanh | Func::Acoth | Func::Asec | Func::Acsc,
                a,
            ) => found.extend([less(a, 1.0), less(a, -1.0)]),
            Expr::Call(Func::Acosh | Func::Asech, a) => found.extend([less(a, 1.0), (**a).clone()]),
            Expr::Call(
                Func::Sqrt | Func::Ln | Func::Log | Func::Coth | Func::Csch | Func::Acsch,
                a,
            ) => found.push((**a).clone()),
            Expr::LogBase(b, a) => found.extend([(**a).clone(), (**b).clone(), less(b, 1.0)]),
            _ => {}
        }
        false
    });
    let boundaries = found.into_inner();
    // The period of a periodic factor of a boundary that is 0 at x.
    let period_at = |x: f64| {
        boundaries.iter().find_map(|g| {
            let mut parts = Vec::new();
            factors(g, env, &mut parts);
            parts.iter().find_map(|h| {
                period(h, env.angle).filter(|_| root_at(h, x, env) || !nonzero(h, x, env))
            })
        })
    };
    let ends = dedupe(
        intervals
            .iter()
            .flat_map(|i| [i.lo, i.hi])
            .filter(|x| x.is_finite())
            .collect(),
    );
    let cuts: Vec<(f64, f64)> = ends
        .into_iter()
        .filter_map(|x| period_at(x).map(|p| (x, p)))
        .collect();
    let unbounded = intervals
        .iter()
        .any(|i| i.lo.is_infinite() || i.hi.is_infinite());
    cuts.len() >= 2 && (unbounded || cuts.iter().any(|&(x, p)| x.abs() + p > edge))
}

/// Whether f has its value at t only by chance: a negative base to a power that varies
/// around t is real only where the power is a fraction with an odd denominator, as
/// x^(3125x + 13/64) is at −0.2371594 (−740.92), and not around it. The power varies if
/// its slope or its change around t is more than rounding explains (`with_rounding_error`),
/// however large it is: 3125x + 2⁴⁰ + 13/64 changes by 3·10⁻⁴ there, its rounding by 2·10⁻³.
/// A constant, locally constant or whole power is real all around (x^(1/3), x^floor(x),
/// x^floor(10³⁰¹|x|)).
fn real_by_chance(fun: &Fun, t: f64) -> bool {
    let at = |e: &Expr, s: f64| e.eval(s, 0.0, &fun.env);
    let rounded = |e: &Expr, s: f64| with_rounding_error(e, s, &fun.env);
    let d = 1e-7 * t.abs().max(1.0);
    let around = [t - d, t, t + d];
    let varies = |e: &Expr| {
        let (v, error) = rounded(e, t);
        let sloped = e.derivative(fun.env.angle).is_some_and(|slope| {
            let (s, error) = rounded(&slope, t);
            s.abs() > error
        });
        sloped
            || around.iter().any(|&s| {
                let (w, other) = rounded(e, s);
                (w - v).abs() > error + other
            })
    };
    let whole = |e: &Expr| around.iter().all(|&s| at(e, s).fract() == 0.0);
    fun.e.any(&|e| match e {
        Expr::Bin(BinOp::Pow, base, power) | Expr::Root(base, power) => {
            at(base, t) < 0.0 && varies(power) && !whole(power)
        }
        _ => false,
    })
}

/// Whether e is certainly not 0 at t: further from it than rounding explains, beyond the
/// doubles, or 0 only as a number too small for them that can't be 0 (x² at 10⁻²⁰⁰,
/// e^(−1/x²), (1000x)^1000 at 10⁻⁴).
fn nonzero(e: &Expr, t: f64, env: &Env) -> bool {
    let (v, error) = with_rounding_error(e, t, env);
    if v.is_nan() {
        return false;
    }
    if v.is_infinite() || v.abs() > error {
        return true;
    }
    let part = |p: &Expr| nonzero(p, t, env);
    match e {
        Expr::Neg(a) => part(a),
        Expr::Bin(BinOp::Mul, a, b) => part(a) && part(b),
        Expr::Bin(BinOp::Div | BinOp::Pow, a, _) | Expr::Root(a, _) => part(a),
        Expr::Call(Func::Exp | Func::Sech | Func::Csch | Func::Acot, _) | Expr::Factorial(_) => {
            true
        }
        Expr::Call(
            Func::Sinh
            | Func::Tanh
            | Func::Asin
            | Func::Atan
            | Func::Asinh
            | Func::Atanh
            | Func::Sqrt
            | Func::Cbrt
            | Func::Abs,
            a,
        ) => part(a),
        // sin and tan are 0 at πn too, but not at a number too small for doubles.
        Expr::Call(Func::Sin | Func::Tan, a) => a.eval(t, 0.0, env) == 0.0 && part(a),
        // A sum with an exact 0 in it is its other term: x + e^(−1000) at 0.
        Expr::Bin(BinOp::Add | BinOp::Sub, a, b) => {
            let exact_zero = |p: &Expr| p.eval(t, 0.0, env) == 0.0 && is_exact(p, t, env);
            (exact_zero(a) && part(b)) || (exact_zero(b) && part(a))
        }
        _ => false,
    }
}

/// Points snapped to the exact values they are near, without repeats. `recognize` is
/// slow: count the points first (tan(x²) has thousands of poles).
fn snapped(points: Vec<f64>) -> Vec<f64> {
    dedupe(points.into_iter().map(|c| exact::snap(c, TIGHT)).collect())
}

/// The domain of an aperiodic function, as intervals, or `None` if it has too many
/// pieces to tell.
fn domain(fun: &Fun, xs: &[f64]) -> Option<Vec<Interval>> {
    let points = domain_candidates(fun, xs);
    if points.len() > 40 {
        return None;
    }
    let points = snapped(points);
    // Probes off the simple numbers (x^x is real at −1 and −10, and at fractions with odd
    // denominators, but not between them), near and far from the pieces' ends: x^x^x is ∞
    // from 4.3 on, e^(1000/x) below 1.4. A value may be large (`defined`: e^(x²)/x is
    // 4e66 at 12.4). One a probe has only by chance doesn't count (`real_by_chance`).
    let defined_between = |a: f64, b: f64| {
        const STEPS: [f64; 4] = [0.2371594, 1.2371594, 12.371594, 123.71594];
        let probes: Vec<f64> = match (a.is_finite(), b.is_finite()) {
            (true, true) => [0.2371594, 0.5128473, 0.7642918]
                .map(|t| a + (b - a) * t)
                .to_vec(),
            (false, true) => STEPS.map(|d| b - d).to_vec(),
            (true, false) => STEPS.map(|d| a + d).to_vec(),
            (false, false) => vec![-1.2371594, 0.5128473, 2.3719481],
        };
        probes
            .iter()
            .filter(|&&x| fun.defined(x) && !real_by_chance(fun, x))
            .count()
            >= 2
    };
    let mut ends = vec![f64::NEG_INFINITY];
    ends.extend(&points);
    ends.push(f64::INFINITY);
    let mut intervals: Vec<Interval> = Vec::new();
    for pair in ends.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if !defined_between(a, b) {
            continue;
        }
        let closed = |x: f64| x.is_finite() && fun.defined(x) && !zero_to_the_zero(fun, x);
        let piece = Interval {
            lo: a,
            hi: b,
            lo_closed: closed(a),
            hi_closed: closed(b),
        };
        match intervals.last_mut() {
            Some(last) if last.hi == a && last.hi_closed => {
                last.hi = b;
                last.hi_closed = piece.hi_closed;
            }
            _ => intervals.push(piece),
        }
    }
    Some(intervals)
}

fn bound(v: f64) -> String {
    if v == f64::INFINITY {
        "∞".to_string()
    } else if v == f64::NEG_INFINITY {
        "−∞".to_string()
    } else {
        exact::format(v)
    }
}

fn interval_text(i: &Interval) -> String {
    if i.lo == i.hi {
        // A single value, which only a range has (x/|x| is −1, then 1).
        return format!("{{{}}}", bound(i.lo));
    }
    format!(
        "{}{}, {}{}",
        if i.lo_closed { '[' } else { '(' },
        bound(i.lo),
        bound(i.hi),
        if i.hi_closed { ']' } else { ')' }
    )
}

/// Whether intervals are the whole line but for single points, which they return.
fn line_without_points(intervals: &[Interval]) -> Option<Vec<f64>> {
    let first = intervals.first()?;
    let last = intervals.last()?;
    if first.lo != f64::NEG_INFINITY || last.hi != f64::INFINITY {
        return None;
    }
    intervals
        .windows(2)
        .map(|pair| {
            (pair[0].hi == pair[1].lo && !pair[0].hi_closed && !pair[1].lo_closed)
                .then_some(pair[0].hi)
        })
        .collect()
}

/// A set written for the variable `v`: `x ∈ ℝ`, `x ≠ 0`, `x > 0`, `x ∈ [−5, 5]`.
fn set_text(v: char, intervals: &[Interval]) -> String {
    let body = if let Some(points) = line_without_points(intervals) {
        if points.is_empty() {
            format!("{v} ∈ ℝ")
        } else if v == 'x' {
            let parts: Vec<String> = points
                .iter()
                .map(|p| format!("x ≠ {}", exact::format(*p)))
                .collect();
            parts.join(", ")
        } else {
            let parts: Vec<String> = points.iter().map(|p| exact::format(*p)).collect();
            format!("{v} ∈ ℝ∖{{{}}}", parts.join(", "))
        }
    } else if !intervals.is_empty() && intervals.iter().all(|i| i.lo == i.hi) {
        // Single values: `y = 1` like a constant's, or `y ∈ {−1, 1}`.
        let values: Vec<String> = intervals.iter().map(|i| exact::format(i.lo)).collect();
        match values.as_slice() {
            [value] => format!("{v} = {value}"),
            _ => format!("{v} ∈ {{{}}}", values.join(", ")),
        }
    } else if let [only] = intervals
        && v == 'x'
    {
        // Windows writes a domain's half-line as an inequality (x > 0), and a range's
        // as an interval (y ∈ (0, ∞)).
        match (only.lo.is_finite(), only.hi.is_finite()) {
            (true, false) => format!(
                "{v} {} {}",
                if only.lo_closed { '≥' } else { '>' },
                bound(only.lo)
            ),
            (false, true) => format!(
                "{v} {} {}",
                if only.hi_closed { '≤' } else { '<' },
                bound(only.hi)
            ),
            _ => format!("{v} ∈ {}", interval_text(only)),
        }
    } else {
        let parts: Vec<String> = intervals.iter().map(interval_text).collect();
        format!("{v} ∈ {}", parts.join(" ∪ "))
    };
    format!("{{{body}}}")
}

// ------------------------------------------------------------------ limits

#[derive(Clone, Copy, Debug, PartialEq)]
enum Limit {
    Value(f64),
    PlusInfinity,
    MinusInfinity,
    Unknown,
}

/// The limit of the values `v`, taken ever closer to the point (or ever further out).
fn limit_of(v: &[f64]) -> Limit {
    if v.iter().any(|x| x.is_nan()) {
        return Limit::Unknown;
    }
    let last = v[v.len() - 1];
    if last.is_infinite() {
        return if last > 0.0 {
            Limit::PlusInfinity
        } else {
            Limit::MinusInfinity
        };
    }
    let steps: Vec<f64> = v.windows(2).map(|p| p[1] - p[0]).collect();
    let n = steps.len();
    let (first, final_step) = (steps[0], steps[n - 1]);
    // Beyond 1e14, where main had no values, a step that small next to the value can be
    // growth on a large constant (ln(x) + 10^15 changes by 4.6 a step): only an ulp settles.
    let settled = if last.abs() < 1e14 {
        1e-7 * (1.0 + last.abs())
    } else {
        last.abs() - last.abs().next_down()
    };
    if final_step.abs() <= settled {
        return Limit::Value(exact::snap(last, LOOSE));
    }
    // Steps shrinking geometrically (like c/x) converge: add the rest of the series.
    let ratio = |k: usize| (steps[k] / steps[k - 1]).abs();
    if n >= 3 && ratio(n - 1) < 0.5 && ratio(n - 2) < 0.5 {
        let r = ratio(n - 1);
        let tail = final_step * r / (1.0 - r);
        return Limit::Value(exact::snap(last + tail, LOOSE));
    }
    let monotonic = steps.iter().all(|s| *s > 0.0) || steps.iter().all(|s| *s < 0.0);
    if monotonic && final_step.abs() >= 0.5 * first.abs() {
        return if final_step > 0.0 {
            Limit::PlusInfinity
        } else {
            Limit::MinusInfinity
        };
    }
    Limit::Unknown
}

/// The limit at x → ±∞ of `g`.
fn limit_at_infinity(g: &dyn Fn(f64) -> f64, sign: f64) -> Limit {
    let v: Vec<f64> = (1..=6).map(|k| g(sign * 10f64.powi(2 * k))).collect();
    limit_of(&v)
}

/// The one-sided limit at `p` from the side of `side` (−1 left, 1 right).
fn limit_at(fun: &Fun, p: f64, side: f64) -> Limit {
    let scale = p.abs().max(1.0);
    let v: Vec<f64> = (1..=6)
        .map(|k| fun.f(p + side * scale * 10f64.powi(-2 * k)))
        .collect();
    limit_of(&v)
}

// ------------------------------------------------------------------ text

fn point_text(x: f64, y: f64) -> String {
    format!("({}, {})", exact::format(x), exact::format(y))
}

/// The value of a limit, written for a range end.
fn limit_end(l: Limit) -> Option<f64> {
    match l {
        Limit::Value(v) => Some(v),
        Limit::PlusInfinity => Some(f64::INFINITY),
        Limit::MinusInfinity => Some(f64::NEG_INFINITY),
        Limit::Unknown => None,
    }
}

/// Merges ranges that overlap or touch.
fn union(mut pieces: Vec<Interval>) -> Vec<Interval> {
    pieces.sort_by(|a, b| a.lo.total_cmp(&b.lo).then(b.lo_closed.cmp(&a.lo_closed)));
    let mut out: Vec<Interval> = Vec::new();
    for p in pieces {
        if let Some(last) = out.last_mut() {
            let touches = p.lo < last.hi || (p.lo == last.hi && (last.hi_closed || p.lo_closed));
            if touches {
                if p.hi > last.hi {
                    last.hi = p.hi;
                    last.hi_closed = p.hi_closed;
                } else if p.hi == last.hi {
                    last.hi_closed |= p.hi_closed;
                }
                continue;
            }
        }
        out.push(p);
    }
    out
}

/// One value range end with whether it is attained.
#[derive(Clone, Copy)]
struct End {
    value: f64,
    attained: bool,
}

// ------------------------------------------------------------------ aperiodic

/// Features that need the derivatives (or none of floor's jumps).
const SMOOTH_FEATURES: [Feature; 6] = [
    Feature::Range,
    Feature::XIntercept,
    Feature::Minima,
    Feature::Maxima,
    Feature::InflectionPoints,
    Feature::Monotonicity,
];

/// The features of a function that isn't periodic, or `None` if it is defined on no
/// interval (`√(−1 − x²)` nowhere, `√(−x²)` only at 0).
fn aperiodic(fun: &Fun) -> Option<Features> {
    let mut features = Features::empty();
    let xs = grid();
    let Some(intervals) = domain(fun, &xs) else {
        features.too_complex.extend(SMOOTH_FEATURES);
        return Some(features);
    };
    if intervals.is_empty() || recurring_cuts(fun, &intervals, xs[xs.len() - 1]) {
        return None;
    }
    features.domain = Some(set_text('x', &intervals));
    if intervals.iter().any(|i| contains(i, 0.0)) && fun.defined(0.0) {
        features.y_intercept = Some(exact::format(fun.f(0.0)));
    }
    if fun.rough || fun.d1.is_none() {
        features.too_complex.extend(SMOOTH_FEATURES);
        features.too_complex.extend([
            Feature::VerticalAsymptotes,
            Feature::HorizontalAsymptotes,
            Feature::ObliqueAsymptotes,
        ]);
        return Some(features);
    }
    asymptotes(fun, &intervals, &mut features);

    let inside = |x: f64| intervals.iter().any(|i| x > i.lo && x < i.hi);
    let f = |x: f64| fun.f(x);
    let d1 = |x: f64| fun.d1(x);
    let d2 = |x: f64| fun.d2(x);
    let slope: &dyn Fn(f64) -> f64 = &d1;

    // Zeros, also at the ends of the domain (√(25 − x²) at ±5). Exact values are
    // looked for only when there are few (sin(x)/x has hundreds).
    let mut roots: Vec<f64> = zeros(&f, Some(slope), &xs)
        .into_iter()
        .filter(|&x| inside(x) && fun.defined(x))
        .collect();
    for i in &intervals {
        for (end, closed) in [(i.lo, i.lo_closed), (i.hi, i.hi_closed)] {
            if closed && fun.f(end).abs() <= 1e-12 {
                roots.push(end);
            }
        }
    }
    if roots.len() > 20 {
        features.too_complex.push(Feature::XIntercept);
    } else if !roots.is_empty() {
        let roots = dedupe(roots.iter().map(|&x| exact::snap(x, TIGHT)).collect());
        let parts: Vec<String> = roots.iter().map(|x| exact::format(*x)).collect();
        features.x_intercepts = Some(parts.join(", "));
    }

    // Turning points, where f′ changes sign (through ∞ at a cusp).
    let mut turns: Vec<(f64, bool)> = sign_changes(&d1, &xs, true)
        .into_iter()
        .map(|(c, _, _)| (c, falls_before(&d1, c)))
        .filter(|&(c, _)| inside(c) && fun.defined(c))
        .collect();
    let too_many_turns = turns.len() > 20;
    if !too_many_turns {
        for turn in &mut turns {
            turn.0 = exact::snap(turn.0, TIGHT);
        }
    }
    if too_many_turns {
        features.too_complex.extend([
            Feature::Range,
            Feature::Minima,
            Feature::Maxima,
            Feature::Monotonicity,
        ]);
    } else {
        for &(c, minimum) in &turns {
            let text = point_text(c, fun.f(c));
            if minimum {
                features.minima.push(text);
            } else {
                features.maxima.push(text);
            }
        }
        // The closed ends of the domain are extrema too.
        for i in &intervals {
            for (end, closed, side) in [(i.lo, i.lo_closed, 1.0), (i.hi, i.hi_closed, -1.0)] {
                if !closed {
                    continue;
                }
                let slope = d1(end + side * 1e-9 * end.abs().max(1.0));
                if slope == 0.0 || slope.is_nan() {
                    continue;
                }
                let text = point_text(end, fun.f(end));
                if (slope > 0.0) == (side > 0.0) {
                    features.minima.push(text);
                } else {
                    features.maxima.push(text);
                }
            }
        }
    }

    let bends: Vec<f64> = sign_changes(&d2, &xs, true)
        .into_iter()
        .map(|c| c.0)
        .filter(|&c| inside(c) && fun.defined(c))
        .collect();
    // Without f″ (too large to use), the bends can't be told.
    if bends.len() > 20 || fun.d2.is_none() {
        features.too_complex.push(Feature::InflectionPoints);
    } else {
        features.inflection_points = bends
            .iter()
            .map(|&c| {
                let c = exact::snap(c, TIGHT);
                point_text(c, fun.f(c))
            })
            .collect();
    }

    if too_many_turns {
        return Some(features);
    }

    // The range: on each piece of the domain, from its lowest to its highest value
    // (at the turning points, the closed ends, or in the limit).
    let mut pieces = Vec::new();
    let mut known = true;
    for i in &intervals {
        let mut ends: Vec<End> = turns
            .iter()
            .filter(|(c, _)| *c > i.lo && *c < i.hi)
            .map(|&(c, _)| End {
                value: exact::snap(fun.f(c), TIGHT),
                attained: true,
            })
            .collect();
        for (x, closed, side) in [(i.lo, i.lo_closed, 1.0), (i.hi, i.hi_closed, -1.0)] {
            if closed {
                ends.push(End {
                    value: exact::snap(fun.f(x), TIGHT),
                    attained: true,
                });
                continue;
            }
            let limit = if x.is_finite() {
                limit_at(fun, x, side)
            } else {
                limit_at_infinity(&f, if x > 0.0 { 1.0 } else { -1.0 })
            };
            match limit_end(limit) {
                Some(value) => ends.push(End {
                    value,
                    attained: false,
                }),
                None => known = false,
            }
        }
        if ends.is_empty() {
            continue;
        }
        let lo = ends.iter().map(|e| e.value).fold(f64::INFINITY, f64::min);
        let hi = ends
            .iter()
            .map(|e| e.value)
            .fold(f64::NEG_INFINITY, f64::max);
        // Only ±∞ (a turning value not found) is no range to tell, rather than ∞ in one.
        if lo == hi && lo.is_infinite() {
            known = false;
        }
        // Constant on the piece (x/x): the value is taken inside it.
        let attained = |v: f64| lo == hi || ends.iter().any(|e| e.value == v && e.attained);
        pieces.push(Interval {
            lo,
            hi,
            lo_closed: attained(lo) && lo.is_finite(),
            hi_closed: attained(hi) && hi.is_finite(),
        });
    }
    if known {
        let pieces = union(pieces);
        features.range = Some(set_text('y', &pieces));
    } else {
        features.too_complex.push(Feature::Range);
    }

    // Monotonicity between the turning points.
    for i in &intervals {
        let mut cuts = vec![i.lo];
        cuts.extend(turns.iter().map(|t| t.0).filter(|&c| c > i.lo && c < i.hi));
        cuts.push(i.hi);
        for pair in cuts.windows(2) {
            let direction = direction_between(fun, pair[0], pair[1]);
            let text = format!("({}, {})", bound(pair[0]), bound(pair[1]));
            features.monotonicity.push((text, direction));
        }
    }
    Some(features)
}

fn contains(i: &Interval, x: f64) -> bool {
    (x > i.lo || (x == i.lo && i.lo_closed)) && (x < i.hi || (x == i.hi && i.hi_closed))
}

/// Whether f rises, falls or stays between two points (one may be infinite).
fn direction_between(fun: &Fun, a: f64, b: f64) -> Direction {
    let probe = match (a.is_finite(), b.is_finite()) {
        (true, true) => vec![a + (b - a) * 0.3, (a + b) / 2.0, a + (b - a) * 0.7],
        (false, true) => vec![b - 0.5, b - 5.0, b - 50.0],
        (true, false) => vec![a + 0.5, a + 5.0, a + 50.0],
        (false, false) => vec![-1.3, 0.4, 2.1],
    };
    let slopes: Vec<f64> = probe
        .iter()
        .map(|&x| fun.d1(x))
        .filter(|s| s.is_finite())
        .collect();
    if slopes.iter().all(|s| s.abs() < 1e-12) {
        Direction::Constant
    } else if slopes.iter().map(|s| s.signum()).sum::<f64>() > 0.0 {
        Direction::Increasing
    } else {
        Direction::Decreasing
    }
}

/// The vertical, horizontal and oblique asymptotes.
fn asymptotes(fun: &Fun, intervals: &[Interval], features: &mut Features) {
    let f = |x: f64| fun.f(x);
    let infinite = |l: Limit| matches!(l, Limit::PlusInfinity | Limit::MinusInfinity);
    let mut vertical = Vec::new();
    for i in intervals {
        if i.lo.is_finite() && !i.lo_closed && infinite(limit_at(fun, i.lo, 1.0)) {
            vertical.push(i.lo);
        }
        if i.hi.is_finite() && !i.hi_closed && infinite(limit_at(fun, i.hi, -1.0)) {
            vertical.push(i.hi);
        }
    }
    features.vertical_asymptotes = dedupe(vertical)
        .iter()
        .map(|x| format!("x = {}", exact::format(*x)))
        .collect();

    let mut horizontal = Vec::new();
    let mut oblique = Vec::new();
    let reaches = |sign: f64| {
        intervals.iter().any(|i| {
            if sign > 0.0 {
                i.hi == f64::INFINITY
            } else {
                i.lo == f64::NEG_INFINITY
            }
        })
    };
    for sign in [-1.0, 1.0] {
        if !reaches(sign) {
            continue;
        }
        match limit_at_infinity(&f, sign) {
            Limit::Value(v) => horizontal.push(v),
            Limit::PlusInfinity | Limit::MinusInfinity => {
                let slope = |x: f64| fun.f(x) / x;
                let Limit::Value(m) = limit_at_infinity(&slope, sign) else {
                    continue;
                };
                if m == 0.0 {
                    continue;
                }
                let rest = |x: f64| fun.f(x) - m * x;
                // Up to 10⁸: further out, f(x) − mx loses its digits.
                let v: Vec<f64> = (1..=4).map(|k| rest(sign * 10f64.powi(2 * k))).collect();
                let Limit::Value(c) = limit_of(&v) else {
                    continue;
                };
                // A line isn't its own asymptote.
                let is_line = [-2.5, -0.5, 0.75, 3.0]
                    .iter()
                    .all(|&x| (rest(x) - c).abs() <= 1e-9 * (1.0 + c.abs()));
                if !is_line {
                    oblique.push((m, c));
                }
            }
            Limit::Unknown => {}
        }
    }
    features.horizontal_asymptotes = dedupe(horizontal)
        .iter()
        .map(|v| format!("y = {}", exact::format_loose(*v)))
        .collect();
    oblique.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    features.oblique_asymptotes = oblique
        .iter()
        .map(|&(m, c)| {
            let mut s = format!("y = {}", exact::coefficient_of_x(m));
            if c != 0.0 {
                s.push_str(if c > 0.0 { " + " } else { " − " });
                s.push_str(&exact::format_loose(c.abs()));
            }
            s
        })
        .collect();
}

// ------------------------------------------------------------------ periodic

/// `step·n₁ + offset`: `πn₁`, `2πn₁ + π/2`, `360n₁ + 90`.
fn family_term(step: f64, offset: f64) -> String {
    let n = match exact::recognize(step, TIGHT) {
        exact::Exact::Rational(p, q) => times_n(p, q, ""),
        exact::Exact::Pi(p, q) => times_n(p, q, "π"),
        other => format!("{}n₁", exact::format_exact(other)),
    };
    if offset == 0.0 {
        n
    } else {
        format!("{n} + {}", exact::format(offset))
    }
}

/// `pπn₁/q`, without the ones.
fn times_n(p: i64, q: i64, symbol: &str) -> String {
    let mut s = String::new();
    if p == -1 {
        s.push('−');
    } else if p != 1 {
        s.push_str(&p.to_string().replace('-', "−"));
    }
    s.push_str(symbol);
    s.push_str("n₁");
    if q != 1 {
        s.push('/');
        s.push_str(&q.to_string());
    }
    s
}

/// Points in one period [0, p), as families: the largest k with the points repeating
/// every p/k (and `same` holding between repeats), and the points of the first p/k.
fn families(points: &[f64], p: f64, same: &dyn Fn(usize, usize) -> bool) -> (f64, Vec<usize>) {
    let n = points.len();
    for k in (1..=n).rev() {
        if !n.is_multiple_of(k) {
            continue;
        }
        let m = n / k;
        let step = p / k as f64;
        let repeats = (0..n - m)
            .all(|i| (points[i + m] - points[i] - step).abs() <= 1e-7 * p && same(i, i + m));
        if repeats {
            return (step, (0..m).collect());
        }
    }
    (p, (0..n).collect())
}

/// Points of [0, p) found on a slightly larger window, without repeats.
fn in_period(mut points: Vec<f64>, p: f64) -> Vec<f64> {
    for x in &mut points {
        let mut v = x.rem_euclid(p);
        if (p - v).abs() <= 1e-9 * p {
            v = 0.0;
        }
        *x = exact::snap(v, TIGHT);
        if *x >= p {
            *x = 0.0;
        }
    }
    dedupe(points)
}

fn periodic(fun: &Fun, p: f64) -> Features {
    let mut features = Features::empty();
    features.period = Period::Periodic(exact::format(p));
    // A uniform grid over a little more than one period.
    const N: usize = 4000;
    let xs: Vec<f64> = (0..=N + 2)
        .map(|i| -p / N as f64 + p * i as f64 / N as f64)
        .collect();
    let f = |x: f64| fun.f(x);
    let d1 = |x: f64| fun.d1(x);
    let d2 = |x: f64| fun.d2(x);

    // The domain: ℝ but for poles (tan's π/2 + πn). Gaps can't be written here.
    let candidates = in_period(snapped(domain_candidates(fun, &xs)), p);
    let excluded: Vec<f64> = candidates
        .iter()
        .copied()
        .filter(|&x| !fun.defined(x))
        .collect();
    if (0..400).any(|i| !fun.f(p * (i as f64 + 0.5) / 400.0).is_finite()) {
        features.too_complex.extend(SMOOTH_FEATURES);
        return features;
    }
    if fun.defined(0.0) {
        features.y_intercept = Some(exact::format(fun.f(0.0)));
    }
    if excluded.is_empty() {
        features.domain = Some("{x ∈ ℝ}".to_string());
    } else {
        let (step, firsts) = families(&excluded, p, &|_, _| true);
        let parts: Vec<String> = firsts
            .iter()
            .map(|&i| format!("x ≠ {}", family_term(step, excluded[i])))
            .collect();
        features.domain = Some(format!("{{{}, ∀n₁ ∈ ℤ}}", parts.join(", ")));
        let vertical: Vec<usize> = firsts
            .iter()
            .copied()
            .filter(|&i| {
                let x = excluded[i];
                matches!(
                    limit_at(fun, x, 1.0),
                    Limit::PlusInfinity | Limit::MinusInfinity
                ) || matches!(
                    limit_at(fun, x, -1.0),
                    Limit::PlusInfinity | Limit::MinusInfinity
                )
            })
            .collect();
        features.vertical_asymptotes = vertical
            .iter()
            .map(|&i| format!("x = {}, n₁ ∈ ℤ", family_term(step, excluded[i])))
            .collect();
    }
    if fun.rough || fun.d1.is_none() {
        features.too_complex.extend(SMOOTH_FEATURES);
        return features;
    }
    let defined = |x: f64| fun.defined(x);
    let slope: &dyn Fn(f64) -> f64 = &d1;

    let roots: Vec<f64> = in_period(zeros(&f, Some(slope), &xs), p)
        .into_iter()
        .filter(|&x| defined(x))
        .collect();
    if !roots.is_empty() {
        let (step, firsts) = families(&roots, p, &|_, _| true);
        let parts: Vec<String> = firsts
            .iter()
            .map(|&i| family_term(step, roots[i]))
            .collect();
        features.x_intercepts = Some(format!("{{{}, n₁ ∈ ℤ}}", parts.join(", ")));
    }

    // Turning points and their kind, in one period.
    let positions = in_period(
        sign_changes(&d1, &xs, true)
            .into_iter()
            .map(|c| c.0)
            .collect(),
        p,
    );
    let turns: Vec<(f64, bool)> = positions
        .iter()
        .map(|&x| (x, falls_before(&d1, x)))
        .filter(|&(x, _)| defined(x))
        .collect();
    for minimum in [true, false] {
        let points: Vec<f64> = turns
            .iter()
            .filter(|t| t.1 == minimum)
            .map(|t| t.0)
            .collect();
        if points.is_empty() {
            continue;
        }
        let values: Vec<f64> = points
            .iter()
            .map(|&x| exact::snap(fun.f(x), TIGHT))
            .collect();
        let (step, firsts) = families(&points, p, &|a, b| (values[a] - values[b]).abs() < 1e-9);
        let texts: Vec<String> = firsts
            .iter()
            .map(|&i| {
                format!(
                    "{{({}, {}), n₁ ∈ ℤ}}",
                    family_term(step, points[i]),
                    exact::format(values[i])
                )
            })
            .collect();
        if minimum {
            features.minima = texts;
        } else {
            features.maxima = texts;
        }
    }

    let bends: Vec<f64> = in_period(
        sign_changes(&d2, &xs, true)
            .into_iter()
            .map(|c| c.0)
            .collect(),
        p,
    )
    .into_iter()
    .filter(|&x| defined(x))
    .collect();
    if fun.d2.is_none() {
        features.too_complex.push(Feature::InflectionPoints);
    } else if !bends.is_empty() {
        let values: Vec<f64> = bends
            .iter()
            .map(|&x| exact::snap(fun.f(x), TIGHT))
            .collect();
        let (step, firsts) = families(&bends, p, &|a, b| (values[a] - values[b]).abs() < 1e-9);
        features.inflection_points = firsts
            .iter()
            .map(|&i| {
                format!(
                    "{{({}, {}), n₁ ∈ ℤ}}",
                    family_term(step, bends[i]),
                    exact::format(values[i])
                )
            })
            .collect();
    }

    // The range over a period: the turning values, or ±∞ next to a pole.
    let mut ends: Vec<End> = turns
        .iter()
        .map(|t| End {
            value: exact::snap(fun.f(t.0), TIGHT),
            attained: true,
        })
        .collect();
    let mut known = true;
    for &x in &excluded {
        for side in [-1.0, 1.0] {
            match limit_end(limit_at(fun, x, side)) {
                Some(value) => ends.push(End {
                    value,
                    attained: false,
                }),
                None => known = false,
            }
        }
    }
    let one_infinity = ends
        .iter()
        .all(|e| e.value.is_infinite() && e.value == ends[0].value);
    if known && !ends.is_empty() && !one_infinity {
        let lo = ends.iter().map(|e| e.value).fold(f64::INFINITY, f64::min);
        let hi = ends
            .iter()
            .map(|e| e.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let attained = |v: f64| ends.iter().any(|e| e.value == v && e.attained);
        let range = Interval {
            lo,
            hi,
            lo_closed: attained(lo) && lo.is_finite(),
            hi_closed: attained(hi) && hi.is_finite(),
        };
        features.range = Some(set_text('y', &[range]));
    } else {
        features.too_complex.push(Feature::Range);
    }

    // Monotonicity between the turning points and poles, from the first one at or
    // after 0, around one period.
    let mut cuts: Vec<f64> = turns
        .iter()
        .map(|t| t.0)
        .chain(excluded.iter().copied())
        .collect();
    cuts = dedupe(cuts);
    if cuts.is_empty() {
        let direction = direction_between(fun, 0.0, p);
        features
            .monotonicity
            .push(("(−∞, ∞)".to_string(), direction));
        return features;
    }
    let n = cuts.len();
    let pieces: Vec<(f64, f64, Direction)> = (0..n)
        .map(|i| {
            let a = cuts[i];
            let b = if i + 1 < n { cuts[i + 1] } else { cuts[0] + p };
            (a, b, direction_between(fun, a, b))
        })
        .collect();
    let (step, firsts) = families(&cuts, p, &|a, b| pieces[a].2 == pieces[b].2);
    for &i in &firsts {
        let (a, b, direction) = pieces[i];
        let text = format!(
            "{{({}, {}), n₁ ∈ ℤ}}",
            family_term(step, a),
            family_term(step, b)
        );
        features.monotonicity.push((text, direction));
    }
    features
}

// ------------------------------------------------------------------ parity

fn parity(fun: &Fun) -> Parity {
    let (mut even, mut odd, mut compared) = (true, true, 0);
    // Beyond 1e14, 10⁻⁹ of the values' distance from f(0) (or the first value), plus four
    // ulps: 10⁻⁹ of x + 10^15 would make it even, while x^10·atanh(sin(x)) is odd with
    // atanh rounding differently for −x by far more than ulps.
    let f0 = fun.f(0.0);
    let mut offset = f0.is_finite().then_some(f0);
    for k in 0..24 {
        let t = 0.173 * 1.6f64.powi(k);
        let (a, b) = (fun.f(t), fun.f(-t));
        if a.is_finite() != b.is_finite() {
            return Parity::Neither;
        }
        if !a.is_finite() {
            continue;
        }
        compared += 1;
        let c = *offset.get_or_insert(a);
        let large = a.abs().max(b.abs());
        let tolerance = if large < 1e14 {
            1e-9 * (1.0 + a.abs() + b.abs())
        } else {
            1e-9 * (1.0 + (a - c).abs() + (b - c).abs()) + 4.0 * (large - large.next_down())
        };
        even &= (a - b).abs() <= tolerance;
        odd &= (a + b).abs() <= tolerance;
    }
    match (compared > 3, even, odd) {
        (false, _, _) => Parity::Unknown,
        (true, true, _) => Parity::Even,
        (true, false, true) => Parity::Odd,
        (true, false, false) => Parity::Neither,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphing::Syntax;

    fn analyze_text(text: &str) -> Features {
        let equation = Equation::parse(text, Syntax::default()).unwrap();
        analyze(&equation, Angle::Radians).unwrap()
    }

    fn directions(f: &Features) -> Vec<(&str, Direction)> {
        f.monotonicity
            .iter()
            .map(|(s, d)| (s.as_str(), *d))
            .collect()
    }

    // The functions of Windows' manual tests (docs/ManualTests.md, "Key Graph Features").

    #[test]
    fn line() {
        let f = analyze_text("y=x");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ}"));
        assert_eq!(f.x_intercepts.as_deref(), Some("0"));
        assert_eq!(f.y_intercept.as_deref(), Some("0"));
        assert!(f.minima.is_empty() && f.maxima.is_empty() && f.inflection_points.is_empty());
        assert!(f.vertical_asymptotes.is_empty() && f.horizontal_asymptotes.is_empty());
        assert!(f.oblique_asymptotes.is_empty());
        assert_eq!(f.parity, Parity::Odd);
        assert_eq!(directions(&f), vec![("(−∞, ∞)", Direction::Increasing)]);
        assert_eq!(f.period, Period::None);
        assert!(f.too_complex.is_empty());
    }

    #[test]
    fn reciprocal() {
        let f = analyze_text("y=1/x");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ 0}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ∖{0}}"));
        assert_eq!(f.x_intercepts, None);
        assert_eq!(f.y_intercept, None);
        assert!(f.minima.is_empty() && f.maxima.is_empty() && f.inflection_points.is_empty());
        assert_eq!(f.vertical_asymptotes, vec!["x = 0"]);
        assert_eq!(f.horizontal_asymptotes, vec!["y = 0"]);
        assert!(f.oblique_asymptotes.is_empty());
        assert_eq!(f.parity, Parity::Odd);
        assert_eq!(
            directions(&f),
            vec![
                ("(−∞, 0)", Direction::Decreasing),
                ("(0, ∞)", Direction::Decreasing)
            ]
        );
    }

    #[test]
    fn parabola() {
        let f = analyze_text("y=x^2");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [0, ∞)}"));
        assert_eq!(f.x_intercepts.as_deref(), Some("0"));
        assert_eq!(f.y_intercept.as_deref(), Some("0"));
        assert_eq!(f.minima, vec!["(0, 0)"]);
        assert!(f.maxima.is_empty() && f.inflection_points.is_empty());
        assert_eq!(f.parity, Parity::Even);
        assert_eq!(
            directions(&f),
            vec![
                ("(−∞, 0)", Direction::Decreasing),
                ("(0, ∞)", Direction::Increasing)
            ]
        );
    }

    #[test]
    fn cubic() {
        let f = analyze_text("y=x^3");
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ}"));
        assert_eq!(f.inflection_points, vec!["(0, 0)"]);
        assert!(f.minima.is_empty() && f.maxima.is_empty());
        assert_eq!(f.parity, Parity::Odd);
        assert_eq!(directions(&f), vec![("(−∞, ∞)", Direction::Increasing)]);
    }

    #[test]
    fn exponential() {
        let f = analyze_text("y=e^x");
        assert_eq!(f.range.as_deref(), Some("{y ∈ (0, ∞)}"));
        assert_eq!(f.x_intercepts, None);
        assert_eq!(f.y_intercept.as_deref(), Some("1"));
        assert_eq!(f.horizontal_asymptotes, vec!["y = 0"]);
        assert_eq!(f.parity, Parity::Neither);
        assert_eq!(directions(&f), vec![("(−∞, ∞)", Direction::Increasing)]);
    }

    #[test]
    fn logarithm() {
        let f = analyze_text("y=ln(x)");
        assert_eq!(f.domain.as_deref(), Some("{x > 0}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ}"));
        assert_eq!(f.x_intercepts.as_deref(), Some("1"));
        assert_eq!(f.y_intercept, None);
        assert_eq!(f.vertical_asymptotes, vec!["x = 0"]);
        assert!(f.horizontal_asymptotes.is_empty());
        assert_eq!(f.parity, Parity::Neither);
        assert_eq!(directions(&f), vec![("(0, ∞)", Direction::Increasing)]);
    }

    #[test]
    fn sine() {
        let f = analyze_text("y=sin(x)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [−1, 1]}"));
        assert_eq!(f.x_intercepts.as_deref(), Some("{πn₁, n₁ ∈ ℤ}"));
        assert_eq!(f.y_intercept.as_deref(), Some("0"));
        assert_eq!(f.maxima, vec!["{(2πn₁ + π/2, 1), n₁ ∈ ℤ}"]);
        assert_eq!(f.minima, vec!["{(2πn₁ + 3π/2, −1), n₁ ∈ ℤ}"]);
        assert_eq!(f.inflection_points, vec!["{(πn₁, 0), n₁ ∈ ℤ}"]);
        assert!(f.vertical_asymptotes.is_empty());
        assert_eq!(f.parity, Parity::Odd);
        assert_eq!(f.period, Period::Periodic("2π".to_string()));
        assert_eq!(
            directions(&f),
            vec![
                ("{(2πn₁ + π/2, 2πn₁ + 3π/2), n₁ ∈ ℤ}", Direction::Decreasing),
                (
                    "{(2πn₁ + 3π/2, 2πn₁ + 5π/2), n₁ ∈ ℤ}",
                    Direction::Increasing
                ),
            ]
        );
    }

    #[test]
    fn cosine() {
        let f = analyze_text("y=cos(x)");
        assert_eq!(f.x_intercepts.as_deref(), Some("{πn₁ + π/2, n₁ ∈ ℤ}"));
        assert_eq!(f.y_intercept.as_deref(), Some("1"));
        assert_eq!(f.maxima, vec!["{(2πn₁, 1), n₁ ∈ ℤ}"]);
        assert_eq!(f.minima, vec!["{(2πn₁ + π, −1), n₁ ∈ ℤ}"]);
        assert_eq!(f.inflection_points, vec!["{(πn₁ + π/2, 0), n₁ ∈ ℤ}"]);
        assert_eq!(f.parity, Parity::Even);
        assert_eq!(
            directions(&f),
            vec![
                ("{(2πn₁, 2πn₁ + π), n₁ ∈ ℤ}", Direction::Decreasing),
                ("{(2πn₁ + π, 2πn₁ + 2π), n₁ ∈ ℤ}", Direction::Increasing),
            ]
        );
    }

    #[test]
    fn tangent() {
        let f = analyze_text("y=tan(x)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ πn₁ + π/2, ∀n₁ ∈ ℤ}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ}"));
        assert_eq!(f.x_intercepts.as_deref(), Some("{πn₁, n₁ ∈ ℤ}"));
        assert_eq!(f.y_intercept.as_deref(), Some("0"));
        assert!(f.minima.is_empty() && f.maxima.is_empty());
        assert_eq!(f.inflection_points, vec!["{(πn₁, 0), n₁ ∈ ℤ}"]);
        assert_eq!(f.vertical_asymptotes, vec!["x = πn₁ + π/2, n₁ ∈ ℤ"]);
        assert_eq!(f.parity, Parity::Odd);
        assert_eq!(f.period, Period::Periodic("π".to_string()));
        assert_eq!(
            directions(&f),
            vec![("{(πn₁ + π/2, πn₁ + 3π/2), n₁ ∈ ℤ}", Direction::Increasing)]
        );
    }

    #[test]
    fn semicircle() {
        let f = analyze_text("y=sqrt(25-x^2)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ [−5, 5]}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [0, 5]}"));
        assert_eq!(f.x_intercepts.as_deref(), Some("−5, 5"));
        assert_eq!(f.y_intercept.as_deref(), Some("5"));
        assert_eq!(f.maxima, vec!["(0, 5)"]);
        assert_eq!(f.minima, vec!["(−5, 0)", "(5, 0)"]);
        assert!(f.inflection_points.is_empty());
        assert_eq!(f.parity, Parity::Even);
        assert_eq!(
            directions(&f),
            vec![
                ("(−5, 0)", Direction::Increasing),
                ("(0, 5)", Direction::Decreasing)
            ]
        );
    }

    #[test]
    fn rational_with_an_oblique_asymptote() {
        let f = analyze_text("y=(-3x^2+2)/(x-1)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ 1}"));
        assert_eq!(
            f.range.as_deref(),
            Some("{y ∈ (−∞, −2√3 − 6] ∪ [2√3 − 6, ∞)}")
        );
        assert_eq!(f.x_intercepts.as_deref(), Some("−√6/3, √6/3"));
        assert_eq!(f.y_intercept.as_deref(), Some("−2"));
        assert_eq!(f.maxima, vec!["(√3/3 + 1, −2√3 − 6)"]);
        assert_eq!(f.minima, vec!["(−√3/3 + 1, 2√3 − 6)"]);
        assert!(f.inflection_points.is_empty());
        assert_eq!(f.vertical_asymptotes, vec!["x = 1"]);
        assert!(f.horizontal_asymptotes.is_empty());
        assert_eq!(f.oblique_asymptotes, vec!["y = −3x − 3"]);
        assert_eq!(f.parity, Parity::Neither);
        assert_eq!(
            directions(&f),
            vec![
                ("(−∞, −√3/3 + 1)", Direction::Decreasing),
                ("(−√3/3 + 1, 1)", Direction::Increasing),
                ("(1, √3/3 + 1)", Direction::Increasing),
                ("(√3/3 + 1, ∞)", Direction::Decreasing),
            ]
        );
    }

    #[test]
    fn unsupported() {
        let error = |text| {
            let equation = Equation::parse(text, Syntax::default()).unwrap();
            analyze(&equation, Angle::Radians).unwrap_err()
        };
        assert_eq!(error("y=mx+b"), AnalysisError::NotSupported);
        assert_eq!(error("y<x"), AnalysisError::NotSupported);
        assert_eq!(error("x=y^2"), AnalysisError::VariableIsNotX);
        assert_eq!(error("x^2+y^2=1"), AnalysisError::VariableIsNotX);
    }

    #[test]
    fn rough_functions_are_too_complex() {
        let f = analyze_text("floor(x)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert!(f.too_complex.contains(&Feature::Monotonicity));
    }

    #[test]
    fn more_functions() {
        let f = analyze_text("y=sqrt(x)");
        assert_eq!(f.domain.as_deref(), Some("{x ≥ 0}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [0, ∞)}"));
        assert_eq!(f.minima, vec!["(0, 0)"]);
        let f = analyze_text("y=abs(x)");
        assert_eq!(f.minima, vec!["(0, 0)"]);
        assert_eq!(f.parity, Parity::Even);
        let f = analyze_text("y=5");
        assert_eq!(f.range.as_deref(), Some("{y = 5}"));
        assert_eq!(f.y_intercept.as_deref(), Some("5"));
        let f = analyze_text("y=atan(x)");
        assert_eq!(f.range.as_deref(), Some("{y ∈ (−π/2, π/2)}"));
        assert_eq!(f.horizontal_asymptotes, vec!["y = −π/2", "y = π/2"]);
        let f = analyze_text("y=1/(x^2-1)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ −1, x ≠ 1}"));
        assert_eq!(f.vertical_asymptotes, vec!["x = −1", "x = 1"]);
        let f = analyze_text("y=sin(2x)");
        assert_eq!(f.period, Period::Periodic("π".to_string()));
        let f = analyze_text("y=sin(x)^2");
        assert_eq!(f.period, Period::Periodic("π".to_string()));
        let f = analyze_text("y=x^2-4");
        assert_eq!(f.x_intercepts.as_deref(), Some("−2, 2"));
        // A cusp.
        let f = analyze_text("y=x^(2/3)");
        assert_eq!(f.x_intercepts.as_deref(), Some("0"));
        assert_eq!(f.minima, vec!["(0, 0)"]);
        // A hole is no asymptote.
        let f = analyze_text("y=(x^2-1)/(x-1)");
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ∖{2}}"));
        assert!(f.vertical_asymptotes.is_empty());
        let f = analyze_text("y=sqrt(x^2-4)");
        assert_eq!(f.oblique_asymptotes, vec!["y = −x", "y = x"]);
        // e^(−x) underflows to 0 far out: no zeros there.
        let f = analyze_text("y=x e^(-x)");
        assert_eq!(f.x_intercepts.as_deref(), Some("0"));
        let f = analyze_text("y=cos(2x)+sin(x)");
        assert_eq!(f.x_intercepts.as_deref(), Some("{2πn₁/3 + π/2, n₁ ∈ ℤ}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [−2, 9/8]}"));
    }

    #[test]
    fn ranges() {
        // [0, max] on [−1, 0] and [0, ∞) on [1, ∞): the union is open at ∞.
        let f = analyze_text("y=sqrt(x^3-x)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ [−1, 0] ∪ [1, ∞)}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [0, ∞)}"));
        // Constant on each piece of the domain.
        let f = analyze_text("y=x/x");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ 0}"));
        assert_eq!(f.range.as_deref(), Some("{y = 1}"));
        let f = analyze_text("y=x/|x|");
        assert_eq!(f.range.as_deref(), Some("{y ∈ {−1, 1}}"));
        assert_eq!(
            directions(&f),
            vec![
                ("(−∞, 0)", Direction::Constant),
                ("(0, ∞)", Direction::Constant)
            ]
        );
    }

    #[test]
    fn power_domains() {
        // x^x is real at −1, −10 and −100, where the domain was probed, but not between
        // them, and 0⁰ is 1 in doubles.
        let f = analyze_text("y=x^x");
        assert_eq!(f.domain.as_deref(), Some("{x > 0}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [0.692201, ∞)}"));
        assert_eq!(f.y_intercept, None);
        assert_eq!(
            directions(&f),
            vec![
                ("(0, 0.367879)", Direction::Decreasing),
                ("(0.367879, ∞)", Direction::Increasing)
            ]
        );
        let f = analyze_text("y=x^(2x)");
        assert_eq!(f.domain.as_deref(), Some("{x > 0}"));
        // Huge values aren't poles: 10^100 at 10, e^100 at −10.
        let f = analyze_text("y=x^(x^2)");
        assert_eq!(f.domain.as_deref(), Some("{x > 0}"));
        let f = analyze_text("y=e^(x^2)/x");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ 0}"));
        assert_eq!(
            f.range.as_deref(),
            Some("{y ∈ (−∞, −2.33164] ∪ [2.33164, ∞)}")
        );
        let f = analyze_text("y=x^10 e^(-x)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        // An exponent that is 0 everywhere makes x^(x − x) x⁰, which is 1 at 0 too.
        let f = analyze_text("y=x^(x-x)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.y_intercept.as_deref(), Some("1"));
        // Only an exact 0⁰ counts: 0^(10⁻¹³) is 0, (10⁻¹³)⁰ is 1.
        let f = analyze_text("y=x^(x^2+10^(-13))");
        assert_eq!(f.domain.as_deref(), Some("{x ≥ 0}"));
        let f = analyze_text("y=(x^2+10^(-13))^x");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        // And the power's root has to be the base's itself: (x − 1) + 5·10⁻¹⁰ crosses 0
        // 5·10⁻¹⁰ left of 1, and is 5·10⁻¹⁰ there.
        let f = analyze_text("y=(x-1)^((x-1)+5*10^(-10))");
        assert_eq!(f.domain.as_deref(), Some("{x ≥ 1}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [0, ∞)}"));
        assert_eq!(f.x_intercepts.as_deref(), Some("1"));
        // Also beyond the doubles' reach: (x − 1) + 10⁻¹⁶ is 10⁻¹⁶ at 1, and x + e^(−1000)
        // isn't 0 at 0, though doubles make it 0 and cross 0 there.
        for (text, domain) in [
            ("y=x^(x+5*10^(-13))", "{x ≥ 0}"),
            ("y=(x-1)^((x-1)+10^(-16))", "{x ≥ 1}"),
            ("y=x^(x+e^(-1000))", "{x ≥ 0}"),
        ] {
            assert_eq!(analyze_text(text).domain.as_deref(), Some(domain), "{text}");
        }
        // A part without x that has poles is at one beyond 1e14, as on main: tan(90°) is
        // 1.6e16 in doubles, however deep in f it is. A large number as written is a number.
        for (text, angle) in [
            ("y=x+tan(90)", Angle::Degrees),
            ("y=x+tan(100)", Angle::Gradians),
            ("y=x+sec(90)", Angle::Degrees),
            ("y=x+tan(pi/2)", Angle::Radians),
            ("y=x+2*tan(90)", Angle::Degrees),
            ("y=x+abs(tan(90))", Angle::Degrees),
            ("y=x+tan(90)^2", Angle::Degrees),
            ("y=x+1+tan(90)", Angle::Degrees),
        ] {
            let equation = Equation::parse(text, Syntax::default()).unwrap();
            assert_eq!(
                analyze(&equation, angle),
                Err(AnalysisError::NotSupported),
                "{text}"
            );
        }
        for text in ["y=x/10^15", "y=x+10^15", "y=x+1/tan(pi/2)"] {
            assert_eq!(
                analyze_text(text).domain.as_deref(),
                Some("{x ∈ ℝ}"),
                "{text}"
            );
        }
        // Only values beyond 1e14 are: x/tan(π/2) is a line, as on main.
        let f = analyze_text("y=x/tan(pi/2)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        // A pole of a part with x isn't a value however f nears it: next to 0,
        // tan(90° + x²/(1 + x²)) is tan(90°) = 1.6e16 in doubles, and so for roots with a
        // negative index and logarithms' bases. The domain is main's, whose ends are where
        // doubles flip cos(90° + x²/(1 + x²))'s sign.
        for (text, angle, domain) in [
            (
                "y=tan(90+x^2/(1+x^2))",
                Angle::Degrees,
                "{x ∈ (−∞, −0.0000000842937) ∪ (0.0000000842937, ∞)}",
            ),
            (
                "y=sec(90+x^2/(1+x^2))",
                Angle::Degrees,
                "{x ∈ (−∞, −0.0000000842937) ∪ (0.0000000842937, ∞)}",
            ),
            (
                "y=tan(pi/2+x^2/(1+x^2))",
                Angle::Radians,
                "{x ∈ (−∞, −0.0000000105367) ∪ (0.0000000105367, ∞)}",
            ),
            (
                "y=root(cos(pi/2+x^2/(1+x^2)),-1)",
                Angle::Radians,
                "{x ∈ (−∞, −0.0000000105367) ∪ (0.0000000105367, ∞)}",
            ),
            (
                "y=log(1+sin(pi+x^2/(1+x^2)),10)",
                Angle::Radians,
                "{x ∈ (−∞, −0.0000000149012) ∪ (0.0000000149012, ∞)}",
            ),
        ] {
            let equation = Equation::parse(text, Syntax::default()).unwrap();
            let f = analyze(&equation, angle).unwrap();
            assert_eq!(f.domain.as_deref(), Some(domain), "{text}");
        }
        // Cuts that go on past the grid can't be listed for a function that isn't periodic:
        // main listed x·tan(x)'s poles up to ±990° and took the rest for one piece, and
        // x^10·tan(x) has values beyond 1e14 there, which it didn't take. So for poles where
        // a periodic factor only touches 0 (1 + sin(x) at 270°) and for gaps (√(cos x)).
        for (text, angle) in [
            ("y=x^10*tan(x)", Angle::Degrees),
            ("y=x^10*tan(x)", Angle::Gradians),
            ("y=x^3*sec(x)", Angle::Degrees),
            ("y=x^3*sec(x)", Angle::Gradians),
            ("y=x^10/(1+sin(x))", Angle::Degrees),
            ("y=x^10*sqrt(cos(x))", Angle::Degrees),
        ] {
            let equation = Equation::parse(text, Syntax::default()).unwrap();
            assert_eq!(
                analyze(&equation, angle),
                Err(AnalysisError::NotSupported),
                "{text} in {angle:?}"
            );
        }
        // A single cut doesn't recur: (sin x)²^(x² − 1) has its pole at 0 only.
        let equation = Equation::parse("y=(sin(x)^2)^(x^2-1)", Syntax::default()).unwrap();
        let f = analyze(&equation, Angle::Degrees).unwrap();
        assert_eq!(f.domain.as_deref(), Some("{x ≠ 0}"));
        // Nor is a range written with ∞ in it: |x − 1| + |x + 1|'s flat minimum isn't found,
        // and main gave {y = inf}.
        let f = analyze_text("y=|x-1|+|x+1|");
        assert_eq!(f.range, None);
        assert!(f.too_complex.contains(&Feature::Range));
        // A large value f doesn't grow toward is a value, at the ends, intercepts and
        // turning points as at the probes; beyond 1e14, limits, periods and parity are told
        // at the scale of the doubles there, not by 10⁻⁷ or 10⁻⁹ of the value.
        let f = analyze_text("y=x^2+10^15");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [1000000000000000, ∞)}"));
        assert_eq!(f.y_intercept.as_deref(), Some("1000000000000000"));
        assert_eq!(f.minima, vec!["(0, 1000000000000000)"]);
        assert_eq!(f.parity, Parity::Even);
        let f = analyze_text("y=sqrt(x)+10^15");
        assert_eq!(f.domain.as_deref(), Some("{x ≥ 0}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ [1000000000000000, ∞)}"));
        assert_eq!(f.y_intercept.as_deref(), Some("1000000000000000"));
        assert_eq!(f.minima, vec!["(0, 1000000000000000)"]);
        assert!(f.horizontal_asymptotes.is_empty());
        let f = analyze_text("y=x+10^15");
        assert_eq!(f.y_intercept.as_deref(), Some("1000000000000000"));
        assert_eq!(f.parity, Parity::Neither);
        // Measured from f(0), as main measures from 0: atanh rounds differently at −x.
        assert_eq!(analyze_text("y=x^10*atanh(sin(x))").parity, Parity::Odd);
        let f = analyze_text("y=ln(x)+10^15");
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ}"));
        assert_eq!(f.vertical_asymptotes, vec!["x = 0"]);
        assert!(f.horizontal_asymptotes.is_empty());
        let equation = Equation::parse("y=sin(x)+10^15", Syntax::default()).unwrap();
        let f = analyze(&equation, Angle::Degrees).unwrap();
        assert_eq!(f.period, Period::Periodic("360".to_string()));
        assert_eq!(
            f.range.as_deref(),
            Some("{y ∈ [999999999999999, 1000000000000001]}")
        );
        assert_eq!(f.parity, Parity::Neither);
        // So is one rounding made: e^(−1000) and (1 + 10⁻²⁰) − 1 aren't 0.
        let f = analyze_text("y=x^(x^2/(1+x^2)+e^(-1000))");
        assert_eq!(f.domain.as_deref(), Some("{x ≥ 0}"));
        let f = analyze_text("y=x^(x^2+(1+10^(-20))-1)");
        assert_eq!(f.domain.as_deref(), Some("{x ≥ 0}"));
        // The exponent has to vary arbitrarily near the point, on each side where it has
        // values: floor(10x²/(1 + x²)) is 0 on (−1/3, 1/3), floor(x) on [0, 1).
        let f = analyze_text("y=x^floor(10*x^2/(1+x^2))");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.y_intercept.as_deref(), Some("1"));
        let f = analyze_text("y=x^floor(x)");
        assert_eq!(f.y_intercept.as_deref(), Some("1"));
        // As near as the doubles get: floor(10³⁰¹|x|) is 0 below 10⁻³⁰¹, and sin π is 0
        // though doubles make it 1.2e−16.
        let f = analyze_text("y=atan(x^floor(10^301*abs(x)))");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        let f = analyze_text("y=x^(x*(sin(pi)^1000))");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        // A root doubles can't hit is a root all the same: 3x − 1 crosses 0 at 1/3.
        let f = analyze_text("y=((3*x-1)^2)^((3*x-1)^2)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ 1/3}"));
        let f = analyze_text("y=(x^2-2)^(x^2-2)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ (−∞, −√2) ∪ (√2, ∞)}"));
        // A negative base to a power that varies is real at single points only, wherever the
        // probes land: x^(3125x + 13/64) has the power −740.92 = −18523/25 at −0.2371594.
        let f = analyze_text("y=atan(x^(3125*x+13/64))");
        assert_eq!(f.domain.as_deref(), Some("{x ≥ 0}"));
        // Also where the power is large: 999259.08 at −0.2371594 changes by 3·10⁻⁴ within
        // 10⁻⁷, more than rounding, though less than 10⁻⁹ of it; 3125x + 2⁴⁰ + 13/64 changes
        // less than its rounding, but its slope is 3125.
        for text in [
            "y=atan(x^(3125*x+1000000+13/64))",
            "y=atan(x^(3125*x+2^40+13/64))",
        ] {
            assert_eq!(
                analyze_text(text).domain.as_deref(),
                Some("{x ≥ 0}"),
                "{text}"
            );
        }
        // Constant, locally constant and whole powers keep negative bases, as on main, while
        // (−2)^x has no interval at all (main counted its probes at whole numbers).
        for text in [
            "y=x^3",
            "y=x^(1/3)",
            "y=x^2",
            "y=x^floor(x)",
            "y=(-2)^floor(x)",
        ] {
            assert_eq!(
                analyze_text(text).domain.as_deref(),
                Some("{x ∈ ℝ}"),
                "{text}"
            );
        }
        let equation = Equation::parse("y=(-2)^x", Syntax::default()).unwrap();
        assert_eq!(
            analyze(&equation, Angle::Radians),
            Err(AnalysisError::NotSupported)
        );
        // ((x + 0.1) − x) − 0.1 is 0, rounded to −2.8e−17 at 0.37: that isn't varying.
        let f = analyze_text("y=x^(((x+0.1)-x)-0.1)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        // Beyond the doubles, (1000x)^1000 is ∞ at 0.01 and underflows from 10⁻⁴ on, but is
        // never 0; this one is 0 wherever it isn't ∞.
        let f = analyze_text("y=(x^2/(1+x^2))^((1000*x)^1000)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ 0}"));
        let f = analyze_text("y=(x^2/(1+x^2))^((10^200*x^2*10^200)/10^200-10^200*x^2)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
    }

    #[test]
    fn no_real_values() {
        let error = |text| {
            let equation = Equation::parse(text, Syntax::default()).unwrap();
            analyze(&equation, Angle::Radians).unwrap_err()
        };
        assert_eq!(error("y=sqrt(-1-x^2)"), AnalysisError::NotSupported);
        assert_eq!(error("y=ln(-x^2-1)"), AnalysisError::NotSupported);
        assert_eq!(error("y=sqrt(-1)"), AnalysisError::NotSupported);
        assert_eq!(error("y=ln(0)"), AnalysisError::NotSupported);
    }

    #[test]
    fn many_domain_candidates_are_too_complex() {
        // Thousands of poles: counted before any exact value is looked for, which took
        // seconds.
        for text in ["y=tan(x^2)", "y=1/sin(x^2)"] {
            let f = analyze_text(text);
            assert_eq!(f.domain, None);
            assert!(f.too_complex.contains(&Feature::Domain));
        }
    }

    #[test]
    fn large_derivatives_are_too_complex() {
        // f″ of x·x·…·x is too large to use: no inflection points can be told.
        let product = format!("y={}x", "x·".repeat(25));
        let f = analyze_text(&product);
        assert!(f.too_complex.contains(&Feature::InflectionPoints), "{f:?}");
        assert!(f.inflection_points.is_empty());
    }

    #[test]
    fn degrees() {
        let equation = Equation::parse("y=sin(x)", Syntax::default()).unwrap();
        let f = analyze(&equation, Angle::Degrees).unwrap();
        assert_eq!(f.period, Period::Periodic("360".to_string()));
        assert_eq!(f.x_intercepts.as_deref(), Some("{180n₁, n₁ ∈ ℤ}"));
    }

    #[test]
    fn periods() {
        let p = |text: &str| {
            let equation = Equation::parse(text, Syntax::default()).unwrap();
            let Shape::Function(e) = equation.shape else {
                unreachable!()
            };
            period(&e, Angle::Radians)
        };
        let pi = std::f64::consts::PI;
        assert_eq!(p("sin(x)"), Some(2.0 * pi));
        assert_eq!(p("tan(x)"), Some(pi));
        assert!((p("sin(2x)+cos(3x)").unwrap() - 2.0 * pi).abs() < 1e-12);
        assert_eq!(p("x sin(x)"), None);
        assert_eq!(p("sin(x^2)"), None);
        assert!((p("sin(sin(x))").unwrap() - 2.0 * pi).abs() < 1e-12);
        // In either order.
        assert!((p("sin(13x)+sin(x)").unwrap() - 2.0 * pi).abs() < 1e-12);
        assert!((p("sin(x)+sin(13x)").unwrap() - 2.0 * pi).abs() < 1e-12);
    }
}
