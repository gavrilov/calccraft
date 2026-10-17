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
    /// isn't.
    fn defined(&self, x: f64) -> bool {
        let v = self.f(x);
        v.is_finite() && v.abs() < 1e14
    }

    /// `p`, or the smallest part of it that is a period too (|sin x| has π).
    fn verified_period(&self, p: f64) -> Option<f64> {
        let tests: Vec<f64> = (0..23).map(|k| -7.3 + 0.731 * k as f64).collect();
        let repeats = |q: f64| {
            let mut compared = 0;
            for &t in &tests {
                let (a, b) = (self.f(t), self.f(t + q));
                if a.is_finite() != b.is_finite() {
                    return false;
                }
                if a.is_finite() {
                    compared += 1;
                    if (a - b).abs() > 1e-9 * (1.0 + a.abs()) {
                        return false;
                    }
                }
            }
            compared > 5
        };
        if !repeats(p) {
            return None;
        }
        (1..=12).rev().map(|k| p / k as f64).find(|&q| repeats(q))
    }
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

/// Whether `x` is one of `points`, as `dedupe` tells them apart.
fn among(points: &[f64], x: f64) -> bool {
    points
        .iter()
        .any(|&p| (p - x).abs() <= 1e-9 * x.abs().max(1.0))
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
/// Each comes with whether f divides by it: its zeros are never in the domain, even where
/// f evaluates to a number by rounding (sin(2x)/cos(x) at π/2 is 1.2e-16 / 6.1e-17 = 2).
fn boundary_functions(e: &Expr, out: &mut Vec<(Expr, bool)>) {
    use Expr::*;
    let sub = |a: &Expr, b: f64| Bin(BinOp::Sub, Box::new(a.clone()), Box::new(Num(b)));
    let square = |a: &Expr| Bin(BinOp::Pow, Box::new(a.clone()), Box::new(Num(2.0)));
    match e {
        Bin(op, a, b) => {
            match op {
                BinOp::Div => out.push(((**b).clone(), true)),
                BinOp::Pow => out.push(((**a).clone(), false)),
                _ => {}
            }
            boundary_functions(a, out);
            boundary_functions(b, out);
        }
        Call(f, a) => {
            match f {
                Func::Tan | Func::Sec => out.push((Call(Func::Cos, a.clone()), true)),
                Func::Cot | Func::Csc => out.push((Call(Func::Sin, a.clone()), true)),
                Func::Asin | Func::Acos | Func::Atanh | Func::Acoth | Func::Asec | Func::Acsc => {
                    out.push((sub(&square(a), 1.0), false))
                }
                Func::Acosh | Func::Asech => {
                    out.push((sub(a, 1.0), false));
                    out.push(((**a).clone(), false));
                }
                Func::Coth | Func::Csch | Func::Acsch => out.push(((**a).clone(), true)),
                Func::Sqrt | Func::Ln | Func::Log => out.push(((**a).clone(), false)),
                _ => {}
            }
            boundary_functions(a, out);
        }
        Root(a, n) | LogBase(n, a) => {
            out.push(((**a).clone(), false));
            if matches!(e, LogBase(..)) {
                out.push(((**n).clone(), false));
                out.push((sub(n, 1.0), false));
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

/// The points where `fun` may start, stop or be punched out, and those of them where
/// it divides by 0.
fn domain_candidates(fun: &Fun, xs: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut functions: Vec<(Expr, bool)> = Vec::new();
    let mut found = Vec::new();
    boundary_functions(fun.e, &mut found);
    for (g, divides) in found {
        // A polynomial lists x once for each power.
        match functions.iter_mut().find(|(h, _)| *h == g) {
            Some(known) => known.1 |= divides,
            None => functions.push((g, divides)),
        }
    }
    let (mut candidates, mut divisions) = (Vec::new(), Vec::new());
    for (g, divides) in functions.iter().filter(|(g, _)| g.has_x()) {
        let dg = g.derivative(fun.env.angle);
        let gf = |x: f64| g.eval(x, 0.0, &fun.env);
        let dgf = |x: f64| dg.as_ref().map_or(f64::NAN, |d| d.eval(x, 0.0, &fun.env));
        let dgf: &dyn Fn(f64) -> f64 = &dgf;
        let zeros = zeros(&gf, dg.as_ref().map(|_| dgf), xs);
        if *divides {
            // Where g crosses 0 or is 0: x² + 2·10⁻¹⁴ only comes close at 0. A crossing
            // with a vertical tangent counts too, though g isn't small there (∛(x² − 2) is
            // 7.6e-6 at the double closest to √2): |g| dips there all the way in, a
            // millionth of a sample away to below 0.8 of what it is a sample away, and to
            // less still at the crossing. At a pole or a jump of g it doesn't, even where g
            // at the jump lies between its sides ((⌊x⌋ + ⌈x⌉)/2 + 0.1 is 0.1 at 0, −0.4 and
            // 0.6 next to it).
            let crossings: Vec<f64> = sign_changes(&gf, xs, true)
                .into_iter()
                .filter(|&(c, a, b)| {
                    let (at, span) = (gf(c).abs(), b - a);
                    let dips = |side: f64| {
                        let near = gf(c + side * span * 1e-6).abs();
                        near < 0.8 * gf(c + side * span).abs() && at < near
                    };
                    at <= 1e-6 * (gf(a).abs() + gf(b).abs()) + 1e-12 || (dips(-1.0) && dips(1.0))
                })
                .map(|c| c.0)
                .collect();
            divisions.extend(&crossings);
            divisions.extend(zeros.iter().filter(|&&x| gf(x) == 0.0));
            candidates.extend(crossings);
        }
        candidates.extend(zeros);
    }
    (dedupe(candidates), dedupe(divisions))
}

/// Points snapped to the exact values they are near, without repeats. `recognize` is
/// slow: count the points first (tan(x²) has thousands of poles).
fn snapped(points: Vec<f64>) -> Vec<f64> {
    dedupe(points.into_iter().map(|c| exact::snap(c, TIGHT)).collect())
}

/// The zeros of divisors that are exact values to rounding, snapped to them: f is left out
/// there even if it evaluates to a number. 1/(x + 10⁻¹⁰) divides by 0 at −10⁻¹⁰, which
/// snaps to 0, where f is 10¹⁰.
fn exact_zeros(divisions: Vec<f64>) -> Vec<f64> {
    let exact = divisions.into_iter().filter_map(|c| {
        let x = exact::snap(c, TIGHT);
        ((x - c).abs() <= 8.0 * f64::EPSILON * c.abs().max(1.0)).then_some(x)
    });
    dedupe(exact.collect())
}

/// The domain of an aperiodic function, as intervals, or `None` if it has too many
/// pieces to tell.
fn domain(fun: &Fun, xs: &[f64]) -> Option<Vec<Interval>> {
    let (points, divisions) = domain_candidates(fun, xs);
    if points.len() > 40 {
        return None;
    }
    let (points, divisions) = (snapped(points), exact_zeros(divisions));
    let defined_between = |a: f64, b: f64| {
        let probes: Vec<f64> = match (a.is_finite(), b.is_finite()) {
            (true, true) => vec![a + (b - a) * 0.25, (a + b) / 2.0, a + (b - a) * 0.75],
            (false, true) => vec![b - 1.0, b - 10.0, b - 100.0],
            (true, false) => vec![a + 1.0, a + 10.0, a + 100.0],
            (false, false) => vec![-1.0, 0.5, 2.0],
        };
        probes.iter().filter(|&&x| fun.defined(x)).count() >= 2
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
        let closed = |x: f64| x.is_finite() && fun.defined(x) && !among(&divisions, x);
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
    if final_step.abs() <= 1e-7 * (1.0 + last.abs()) {
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

/// The values of `fun` at 10⁻², 10⁻⁴, … 10⁻¹² of max(1, |p|) from `p` towards `side`.
fn approach(fun: &Fun, p: f64, side: f64) -> Vec<f64> {
    let scale = p.abs().max(1.0);
    (1..=6)
        .map(|k| fun.f(p + side * scale * 10f64.powi(-2 * k)))
        .collect()
}

/// The one-sided limit at `p` from the side of `side` (−1 left, 1 right). Next to a
/// hole, the closest values can be rounding noise (0/0 cancels there: (x² − 9)/(x + 3)
/// at −3 + 3e-12 is off by 3e-4); then the four furthest tell its value. Not at a pole,
/// where f is ±∞ but a small term can hide until the closest values (x + 10⁻¹⁴/x is
/// 1.01e-6 at 10⁻⁶ and 10⁻⁸, then 10⁻⁴ and 0.01), nor if those run off further than noise
/// or aren't numbers (√(x − 10⁻¹⁰) at 0, the end of its domain to rounding).
fn limit_at(fun: &Fun, p: f64, side: f64) -> Limit {
    let v = approach(fun, p, side);
    match limit_of(&v) {
        Limit::Unknown if !huge_at(fun, p) && !v.iter().any(|y| y.is_nan()) && !diverges(&v) => {
            match limit_of(&v[..4]) {
                Limit::Value(value) => Limit::Value(value),
                _ => Limit::Unknown,
            }
        }
        limit => limit,
    }
}

/// Whether the closest three of `approach`'s values run off to ±∞, by more than rounding
/// noise next to a hole does (a ten-thousandth of (2x² + 3x + 1)/(x + 1) at −1): next to a
/// pole whose term is small, or dominated further out by another one (1/sin x + 10⁶ sin x
/// is −31442, −3497 and −318313 at 10⁻², 10⁻⁴ and 10⁻⁶ of π past π). Their steps don't
/// shrink then, or shrink ever more slowly, like ln(−ln|sin x|)'s at 0 (by 0.78, then
/// 0.81). Next to a high root they shrink by the same ratio: root(x, 11) is 0.28, 0.19, 0.12
/// and 0.08 at 10⁻⁶ … 10⁻¹², on its way to 0.
fn diverges(v: &[f64]) -> bool {
    let step = |i: usize| (v[i + 1] - v[i]).abs();
    matches!(
        limit_of(&v[3..]),
        Limit::PlusInfinity | Limit::MinusInfinity
    ) && (v[5] - v[3]).abs() > 1e-3 * (1.0 + v[3].abs())
        && (v[5].is_infinite()
            || step(4) >= 0.95 * step(3)
            || step(4) / step(3) > step(3) / step(2) + 0.01)
}

/// Whether f is ±∞ at `x`, or too large for the domain (`Fun::defined`), as at a pole or
/// next to one (1/(x² + 10⁻¹⁴) is 10¹⁴ at 0).
fn huge_at(fun: &Fun, x: f64) -> bool {
    fun.f(x).abs() >= 1e14
}

/// Whether `x`, left out of the domain, is a hole: f tends to the same value there from
/// both sides, and is 0/0 or a rounded value at x itself. Not at a jump, where the limits
/// differ ((x − 500)/|x − 500| + (x − 500.04)² tends to −0.9984 and 1.0016 at 500). The
/// same to 2·10⁻⁵ of f: `limit_of` stops at a step of 10⁻⁷ of f, and the steps it leaves
/// out add up to a hundred times that where they shrink slowly (root(|x|, 99) at 0).
fn hole(fun: &Fun, x: f64) -> bool {
    !huge_at(fun, x)
        && matches!(
            [1.0, -1.0].map(|side| limit_at(fun, x, side)),
            [Limit::Value(a), Limit::Value(b)] if (a - b).abs() <= 2e-5 * (1.0 + a.abs().max(b.abs()))
        )
}

/// Whether `fun` tends to ±∞ at `x` from a side, by all six values of `approach` or by
/// the closest three.
fn pole(fun: &Fun, x: f64) -> bool {
    [1.0, -1.0].iter().any(|&side| {
        let v = approach(fun, x, side);
        matches!(limit_of(&v), Limit::PlusInfinity | Limit::MinusInfinity) || diverges(&v)
    })
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
    if intervals.is_empty() {
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
    let gaps = domain_gaps(&intervals);
    asymptotes(fun, &intervals, &xs, &gaps, &mut features);
    let holes: Vec<f64> = gaps.iter().copied().filter(|&x| hole(fun, x)).collect();

    let inside = |x: f64| intervals.iter().any(|i| x > i.lo && x < i.hi);
    // No turning or inflection points in the noise next to a hole, nor where the domain
    // leaves a point out (f″ of 1/∛(x² − 2) changes sign at its poles).
    let clear = |x: f64| inside(x) && fun.defined(x) && !next_to(&holes, x) && !among(&gaps, x);
    let f = |x: f64| fun.f(x);
    let d1 = |x: f64| fun.d1(x);
    let d2 = |x: f64| fun.d2(x);
    let slope: &dyn Fn(f64) -> f64 = &d1;

    // Zeros, also at the ends of the domain (√(25 − x²) at ±5). Exact values are
    // looked for only when there are few (sin(x)/x has hundreds).
    let mut roots: Vec<f64> = zeros(&f, Some(slope), &xs)
        .into_iter()
        .filter(|&x| inside(x) && fun.defined(x) && !among(&holes, x))
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
        .filter(|&(c, _)| clear(c))
        .collect();
    let too_many_turns = turns.len() > 20;
    if !too_many_turns {
        for turn in &mut turns {
            turn.0 = exact::snap(turn.0, TIGHT);
        }
        turns.dedup_by(|a, b| (a.0 - b.0).abs() <= 1e-9 * a.0.abs().max(1.0));
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
        .filter(|&c| clear(c))
        .collect();
    // Without f″ (too large to use), the bends can't be told.
    if bends.len() > 20 || fun.d2.is_none() {
        features.too_complex.push(Feature::InflectionPoints);
    } else {
        features.inflection_points = snapped(bends)
            .iter()
            .map(|&c| point_text(c, fun.f(c)))
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

/// The finite ends that the pieces of a domain leave out: its holes, poles and open edges.
fn domain_gaps(intervals: &[Interval]) -> Vec<f64> {
    intervals
        .iter()
        .flat_map(|i| [(i.lo, i.lo_closed), (i.hi, i.hi_closed)])
        .filter_map(|(x, closed)| (x.is_finite() && !closed).then_some(x))
        .collect()
}

/// Whether `x` is within 1e-4 (relative) of one of the `holes`. Next to a hole, f′ and f″
/// are rounding noise, and so is f closer still (0/0 cancels there).
fn next_to(holes: &[f64], x: f64) -> bool {
    holes
        .iter()
        .any(|&h| (x - h).abs() <= 1e-4 * h.abs().max(1.0))
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

/// The vertical, horizontal and oblique asymptotes, with the samples `xs` and the `gaps`
/// of the domain.
fn asymptotes(
    fun: &Fun,
    intervals: &[Interval],
    xs: &[f64],
    gaps: &[f64],
    features: &mut Features,
) {
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
    // A line isn't its own asymptote, nor is a line with holes (x − 2 with one at 3), but
    // nothing with a pole is a line (5 + 10⁻¹⁶/(x − 1000) has y = 5, though it's 5 to the
    // last bits further than 10 from 1000). On a line, f(x) − mx is c and f′(x) is m at
    // every sample of the domain, apart from the rounding noise within 1% of its gaps.
    // Fewer samples could miss a bump (1 + e^(−(x − 10)²) has y = 1 both ways), and the
    // values alone one that is small next to the line (10⁹ + e^(−x²) is within a billionth
    // of 10⁹, but its slope isn't 0). A constant (x/x) comes out to the last bits, while
    // 1/(x² + 10⁹), which has y = 0, is never more than 10⁻⁹ from 0.
    let no_poles = gaps.iter().all(|&g| !huge_at(fun, g) && !pole(fun, g));
    let is_line = |m: f64, c: f64| {
        no_poles
            && xs
                .iter()
                .filter(|&&x| {
                    fun.defined(x)
                        && gaps
                            .iter()
                            .all(|&g| (x - g).abs() > 1e-2 * g.abs().max(1.0))
                })
                .all(|&x| {
                    let (value, slope) = (fun.f(x), fun.d1(x));
                    let off = if m == 0.0 {
                        4.0 * f64::EPSILON * c.abs()
                    } else {
                        1e-9 * (1.0 + c.abs() + (m * x).abs())
                    };
                    (value - m * x - c).abs() <= off
                        && (!slope.is_finite() || (slope - m).abs() <= 1e-6 * (1.0 + m.abs()))
                })
    };
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
            Limit::Value(v) if !is_line(0.0, v) => horizontal.push(v),
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
                if !is_line(m, c) {
                    oblique.push((m, c));
                }
            }
            Limit::Value(_) | Limit::Unknown => {}
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

    // The domain: ℝ but for poles (tan's π/2 + πn) and the zeros of denominators (the
    // holes of sin(2x)/cos(x)). Gaps can't be written here.
    let (candidates, divisions) = domain_candidates(fun, &xs);
    let divisions = in_period(exact_zeros(divisions), p);
    let excluded: Vec<f64> = in_period(snapped(candidates), p)
        .into_iter()
        .filter(|&x| !fun.defined(x) || among(&divisions, x))
        .collect();
    if (0..400).any(|i| !fun.f(p * (i as f64 + 0.5) / 400.0).is_finite()) {
        features.too_complex.extend(SMOOTH_FEATURES);
        return features;
    }
    let holes: Vec<f64> = excluded.iter().copied().filter(|&x| hole(fun, x)).collect();
    let defined = |x: f64| fun.defined(x) && !among(&excluded, x);
    // Next to a hole, f′ and f″ are rounding noise, as in `aperiodic`.
    let clear = |x: f64| {
        defined(x)
            && holes.iter().all(|&e| {
                let d = (x - e).abs();
                d.min(p - d) > 1e-4 * e.abs().max(1.0)
            })
    };
    if defined(0.0) {
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
        // The poles, told apart from the holes before they're grouped: sin(x)/sin(2x) has
        // holes at πn and poles at πn + π/2, all in the family πn/2.
        let poles: Vec<f64> = excluded
            .iter()
            .copied()
            .filter(|&x| !among(&holes, x) && pole(fun, x))
            .collect();
        if !poles.is_empty() {
            let (step, firsts) = families(&poles, p, &|_, _| true);
            features.vertical_asymptotes = firsts
                .iter()
                .map(|&i| format!("x = {}, n₁ ∈ ℤ", family_term(step, poles[i])))
                .collect();
        }
    }
    if fun.rough || fun.d1.is_none() {
        features.too_complex.extend(SMOOTH_FEATURES);
        return features;
    }
    let slope: &dyn Fn(f64) -> f64 = &d1;

    let roots: Vec<f64> = in_period(zeros(&f, Some(slope), &xs), p)
        .into_iter()
        .filter(|&x| fun.defined(x) && !among(&holes, x))
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
        .filter(|&(x, _)| clear(x))
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
    .filter(|&x| clear(x))
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
    if known && !ends.is_empty() {
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
        let tolerance = 1e-9 * (1.0 + a.abs() + b.abs());
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
    fn removable_holes() {
        // 2x + 1 but at −1, where 0/0 leaves f′ to rounding noise: no turning points.
        let f = analyze_text("y=(2x^2+3x+1)/(x+1)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ −1}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ∖{−1}}"));
        assert!(f.minima.is_empty() && f.maxima.is_empty());
        // x² + x + 1 but at 1: no inflection point there.
        let f = analyze_text("y=(x^3-1)/(x-1)");
        assert_eq!(f.range.as_deref(), Some("{y ∈ [3/4, ∞)}"));
        assert_eq!(f.minima, vec!["(−1/2, 3/4)"]);
        assert!(f.inflection_points.is_empty());
        // x − 2 but at 3: a line with a hole isn't its own asymptote either.
        let f = analyze_text("y=(x^2-5x+6)/(x-3)");
        assert!(f.oblique_asymptotes.is_empty());
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ∖{1}}"));
        // The values next to the hole at −3 cancel to noise: the range is still found.
        let f = analyze_text("y=(x^2-9)/(x+3)");
        assert_eq!(f.range.as_deref(), Some("{y ∈ ℝ∖{−6}}"));
        // 2 sin x but where cos x is 0, though f evaluates to ±2 there by rounding: the
        // extrema of 2 sin x are holes.
        let f = analyze_text("y=sin(2x)/cos(x)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ πn₁ + π/2, ∀n₁ ∈ ℤ}"));
        assert_eq!(f.range.as_deref(), Some("{y ∈ (−2, 2)}"));
        assert!(f.minima.is_empty() && f.maxima.is_empty());
        assert!(f.vertical_asymptotes.is_empty());
        assert_eq!(f.inflection_points, vec!["{(πn₁, 0), n₁ ∈ ℤ}"]);
        // sin x but where sin x is 0: every zero is a hole.
        let f = analyze_text("y=sin(x)^2/sin(x)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ πn₁, ∀n₁ ∈ ℤ}"));
        assert_eq!(f.x_intercepts, None);
        // Holes at πn and poles at πn + π/2, all in the family πn/2.
        let f = analyze_text("y=sin(x)/sin(2x)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ πn₁/2, ∀n₁ ∈ ℤ}"));
        assert_eq!(f.vertical_asymptotes, vec!["x = πn₁ + π/2, n₁ ∈ ℤ"]);
        // 1 but at 0: a constant isn't its own asymptote.
        let f = analyze_text("y=x/x");
        assert!(f.horizontal_asymptotes.is_empty());
        // 1 but for a bump at 10, which a few probes around 0 don't see: y = 1 is its
        // asymptote both ways.
        let f = analyze_text("y=1+e^(-(x-10)^2)");
        assert_eq!(f.horizontal_asymptotes, vec!["y = 1"]);
        // A bump that is small next to its line: 10⁹ + e^(−x²) is within a billionth of 10⁹.
        let f = analyze_text("y=10^9+e^(-x^2)");
        assert_eq!(f.horizontal_asymptotes, vec!["y = 1000000000"]);
        // A line far from the origin is still a line.
        let f = analyze_text("y=x+10^9");
        assert!(f.oblique_asymptotes.is_empty());
        let f = analyze_text("y=10^9 x/x");
        assert!(f.horizontal_asymptotes.is_empty());
        // A divisor that only comes close to 0 (2·10⁻¹⁴ at 0) leaves the domain whole.
        let f = analyze_text("y=1/(x^2+0.00000000000002)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.maxima, vec!["(0, 50000000000000)"]);
        // Nothing with a pole is a line, though 5 + 10⁻¹⁶/(x − 1000) is 5 to the last bits
        // away from 1000; and 1/(x² + 10⁹), within 10⁻⁹ of 0, is no constant.
        let f = analyze_text("y=5+10^(-16)/(x-1000)");
        assert_eq!(f.horizontal_asymptotes, vec!["y = 5"]);
        let f = analyze_text("y=1/(x^2+10^9)");
        assert_eq!(f.horizontal_asymptotes, vec!["y = 0"]);
        // 1/(x + 10⁻¹⁰) divides by 0 at −10⁻¹⁰, not at 0, the exact value next to it.
        let f = analyze_text("y=1/(x+10^(-10))");
        assert_eq!(f.y_intercept.as_deref(), Some("10000000000"));
        // Next to a pole, f′ isn't noise: 3x + 1 + 10⁻⁸/(x − 2) turns at 2 ± 5.8e-5.
        let f = analyze_text("y=3x+1+10^(-8)/(x-2)");
        assert_eq!(f.minima, vec!["(2.00006, 7.00035)"]);
        assert_eq!(f.maxima, vec!["(1.99994, 6.99965)"]);
        // Poles at πn, though 10⁶ sin x dominates 1/sin x until 10⁻⁶ of π past π.
        let f = analyze_text("y=1/sin(x)+10^6 sin(x)");
        assert_eq!(f.vertical_asymptotes, vec!["x = πn₁, n₁ ∈ ℤ"]);
        // A jump isn't a hole: f tends to −0.9984 and 1.0016 at 500, and keeps the minimum
        // next to it.
        let f = analyze_text("y=(x-500)/abs(x-500)+(x-500.04)^2");
        assert_eq!(f.minima, vec!["(500.04, 1)"]);
        // A divisor crossing 0 with a vertical tangent divides by 0 there, though it is
        // 7.6e-6 at the double closest to √2.
        let f = analyze_text("y=1/cbrt(x^2-2)");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ −√2, x ≠ √2}"));
        assert_eq!(f.vertical_asymptotes, vec!["x = −√2", "x = √2"]);
        assert!(f.inflection_points.is_empty());
        let f = analyze_text("y=1/cbrt(sin(x))");
        assert_eq!(f.domain.as_deref(), Some("{x ≠ πn₁, ∀n₁ ∈ ℤ}"));
        assert_eq!(f.vertical_asymptotes, vec!["x = πn₁, n₁ ∈ ℤ"]);
        // root(sin x, 11) tends to 0 at 0, slowly: no pole.
        let f = analyze_text("y=root(sin(x),11)^3/root(sin(x),11)^2");
        assert!(f.vertical_asymptotes.is_empty());
        // ln(−ln|sin x|) runs off to ∞ at πn ever more slowly: a pole all the same.
        let f = analyze_text("y=ln(-ln(abs(sin(x))))");
        assert_eq!(f.vertical_asymptotes, vec!["x = πn₁/2, n₁ ∈ ℤ"]);
        // A divisor that jumps past 0 doesn't divide by it, though at the jump it lies
        // between its sides: (⌊x − 1⌋ + ⌈x − 1⌉)/2 + 0.1 is 0.1 at 1.
        let f = analyze_text("y=1/((floor(x-1)+ceil(x-1))/2+0.1)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
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
