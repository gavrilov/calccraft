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
        // f repeats if its values differ by no more than a billionth of how much f varies
        // (not of f itself: 10¹² + sin x would repeat at every shift), or by rounding.
        let values: Vec<f64> = tests
            .iter()
            .map(|&t| self.f(t))
            .filter(|v| v.is_finite())
            .collect();
        let spread = values.iter().fold(f64::NEG_INFINITY, |m: f64, &v| m.max(v))
            - values.iter().fold(f64::INFINITY, |m: f64, &v| m.min(v));
        let repeats = |q: f64| {
            let mut compared = 0;
            for &t in &tests {
                let (a, b) = (self.f(t), self.f(t + q));
                if a.is_finite() != b.is_finite() {
                    return false;
                }
                if a.is_finite() {
                    compared += 1;
                    let rounding = self.rounding(t).max(self.rounding(t + q));
                    let tolerance = 1e-9 * spread.min(1.0 + a.abs()) + rounding;
                    if (a - b).abs() > tolerance {
                        return false;
                    }
                }
            }
            compared > 5
        };
        if !repeats(p) {
            return None;
        }
        // A constant repeats at every shift, so p/12 would pass as well as p: keep p, with
        // which the holes of its domain repeat (tan x · cot x). So does a function whose
        // values change too little next to their rounding to tell: 6·10¹³ + sin x changes by
        // at most 0.52 over p/12, and 64 units in the last place of 6·10¹³ are 0.85.
        let rounding = tests.iter().fold(0.0, |m: f64, &t| m.max(self.rounding(t)));
        if self.constant_over(p).is_some() || 20.0 * rounding > spread {
            return Some(p);
        }
        (1..=12).rev().map(|k| p / k as f64).find(|&q| repeats(q))
    }

    /// The value of f if it's the same at the `golden_points` of the period `p` but for
    /// rounding: sin²x + cos²x, or tan x · cot x (whose holes the points miss), but not
    /// 10¹² + sin x or 1 + 10⁻¹⁰ sin x. The value is snapped within that rounding. A function
    /// that jumps (sign, floor …) is none: sign(sin x)² is 1 at all the points, but 0 at πn.
    /// Nor is one whose derivative isn't `level` at the points: 10¹⁵ + sin x − 10¹⁵ moves
    /// within the rounding of 10¹⁵, but its derivative is cos x.
    fn constant_over(&self, p: f64) -> Option<f64> {
        if self.rough {
            return None;
        }
        let (mut lo, mut hi, mut most) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64);
        for x in golden_points(p) {
            let v = self.f(x);
            if !v.is_finite() || self.level(x) != Some(true) {
                return None;
            }
            (lo, hi, most) = (lo.min(v), hi.max(v), most.max(self.rounding(x)));
        }
        // Halves first: lo + hi overflows next to 10³⁰⁸. Only the rounding snaps: within
        // `TIGHT`, 10⁻¹⁴(sin²x + cos²x) would be 0, which has no x-intercepts to list.
        let c = lo / 2.0 + hi / 2.0;
        (hi - lo <= most && c.is_finite()).then(|| exact::snap(c, most / c.abs().max(1.0)))
    }

    /// Whether rounding hides how f varies over the period `p`: its values at the
    /// `golden_points` are all within a unit in the last place of its largest term, though
    /// its derivative isn't `level` there (10¹⁶ + sin x − 10¹⁶ is 0 at all of them).
    fn hidden(&self, p: f64) -> bool {
        let (mut lo, mut hi, mut unit) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64);
        let mut moves = false;
        for x in golden_points(p) {
            let v = self.f(x);
            if !v.is_finite() {
                return false;
            }
            moves |= self.level(x) == Some(false);
            (lo, hi, unit) = (lo.min(v), hi.max(v), unit.max(self.rounding(x) / 64.0));
        }
        moves && hi - lo <= unit
    }

    /// Whether f′(x) is 0 but for its own rounding (64 units in the last place of its
    /// `magnitude`), or `None` if f′ isn't a number there.
    fn level(&self, x: f64) -> Option<bool> {
        let d1 = self.d1.as_ref()?;
        let slope = d1.eval(x, 0.0, &self.env);
        let rounding = 64.0 * f64::EPSILON * d1.magnitude(x, 0.0, &self.env);
        slope.is_finite().then_some(slope.abs() <= rounding)
    }

    /// How far rounding may set two values of f apart near x: 64 units in the last place of
    /// f's largest term there (10¹² for 10¹² + sin x, 1 for sin²x + cos²x).
    fn rounding(&self, x: f64) -> f64 {
        let size = summands(self.e)
            .iter()
            .fold(0.0, |m: f64, t| m.max(t.eval(x, 0.0, &self.env).abs()));
        64.0 * f64::EPSILON * size
    }
}

/// 400 points across the period `p`: p times the fractional parts of the multiples of the
/// golden ratio, not evenly spaced (at multiples of p/400 + p/800, sin x · sin 400x is 0
/// everywhere).
fn golden_points(p: f64) -> impl Iterator<Item = f64> {
    (1..=400).map(move |i| p * (i as f64 * 0.618_033_988_749_895).fract())
}

/// The terms of a sum, without their signs: sin²x and cos²x for sin²x + cos²x.
fn summands(e: &Expr) -> Vec<&Expr> {
    match e {
        Expr::Bin(BinOp::Add | BinOp::Sub, a, b) => {
            let mut terms = summands(a);
            terms.extend(summands(b));
            terms
        }
        Expr::Neg(a) => summands(a),
        _ => vec![e],
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
        let closed = |x: f64| x.is_finite() && fun.defined(x);
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

    // The domain: ℝ but for poles (tan's π/2 + πn). Gaps can't be written here. As on the
    // grid of `aperiodic`, more points in a period than it writes (40 families of points out
    // of the domain, 20 zeros, turning points or inflection points) are too complex. Only
    // candidates out of the domain count: the 42 zeros of sin(21x)² are no gaps in
    // √(sin(21x)²) + sin x. So do poles that rounding keeps below `defined`'s bound, where f
    // is far larger than right beside them (tan 45x reaches 3·10¹³ at some). Regularly
    // spaced ones are one family, as `families` writes them: tan 45x + tan x has 45 poles a
    // period but one family, and tan 50x + tan x 51 that aren't one. They're counted before
    // `in_period` looks for their exact values: `recognize` is slow, and more than 400 are
    // too many to look up.
    let pole = |x: f64| {
        let h = 1e-6 * x.abs().max(1.0);
        let beside = fun.f(x - h).abs() + fun.f(x + h).abs();
        !fun.defined(x) || fun.f(x).abs() > 1e6 * beside
    };
    let candidates: Vec<f64> = domain_candidates(fun, &xs)
        .into_iter()
        .filter(|&x| pole(x))
        .collect();
    let rough_period: Vec<f64> = dedupe(
        candidates
            .iter()
            .map(|&x| {
                let v = x.rem_euclid(p);
                if (p - v).abs() <= 1e-9 * p { 0.0 } else { v }
            })
            .collect(),
    );
    if families(&rough_period, p, &|_, _| true).1.len() > 40 || rough_period.len() > 400 {
        // Nor are the poles among them: no vertical asymptotes would be wrong.
        features.too_complex.extend(SMOOTH_FEATURES);
        features.too_complex.push(Feature::VerticalAsymptotes);
        if fun.defined(0.0) {
            features.y_intercept = Some(exact::format(fun.f(0.0)));
        }
        return features;
    }
    let candidates = in_period(snapped(candidates), p);
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
    // A constant (sin²x + cos²x, tan x · cot x) has no smallest period, and its derivatives
    // are only rounding noise, whose sign changes would be taken for hundreds of extrema:
    // it's analyzed like y = c, between the holes of its domain.
    if let Some(c) = fun.constant_over(p) {
        features.period = Period::None;
        features.range = Some(format!("{{y = {}}}", exact::format(c)));
        if features.y_intercept.is_some() {
            features.y_intercept = Some(exact::format(c));
        }
        if c == 0.0 {
            features.too_complex.push(Feature::XIntercept);
        }
        features.monotonicity = monotonicity(excluded, p, &|_, _| Direction::Constant);
        return features;
    }
    // Nor do derivatives help where rounding hides f's values (10¹⁶ + sin x − 10¹⁶ is 0).
    if fun.rough || fun.d1.is_none() || fun.hidden(p) {
        features.too_complex.extend(SMOOTH_FEATURES);
        return features;
    }
    let defined = |x: f64| fun.defined(x);
    let slope: &dyn Fn(f64) -> f64 = &d1;

    let roots = zeros(&f, Some(slope), &xs);
    let roots: Vec<f64> = if count_in_period(&roots, p) > 20 {
        features.too_complex.push(Feature::XIntercept);
        Vec::new()
    } else {
        in_period(roots, p)
            .into_iter()
            .filter(|&x| defined(x))
            .collect()
    };
    if !roots.is_empty() {
        let (step, firsts) = families(&roots, p, &|_, _| true);
        let parts: Vec<String> = firsts
            .iter()
            .map(|&i| family_term(step, roots[i]))
            .collect();
        features.x_intercepts = Some(format!("{{{}, n₁ ∈ ℤ}}", parts.join(", ")));
    }

    // Turning points and their kind, in one period.
    let positions: Vec<f64> = sign_changes(&d1, &xs, true)
        .into_iter()
        .map(|c| c.0)
        .collect();
    let too_many_turns = count_in_period(&positions, p) > 20;
    let positions = if too_many_turns {
        features.too_complex.extend([
            Feature::Range,
            Feature::Minima,
            Feature::Maxima,
            Feature::Monotonicity,
        ]);
        Vec::new()
    } else {
        in_period(positions, p)
    };
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

    let bends: Vec<f64> = sign_changes(&d2, &xs, true)
        .into_iter()
        .map(|c| c.0)
        .collect();
    let too_many_bends = count_in_period(&bends, p) > 20;
    let bends: Vec<f64> = if too_many_bends {
        Vec::new()
    } else {
        in_period(bends, p)
            .into_iter()
            .filter(|&x| defined(x))
            .collect()
    };
    if fun.d2.is_none() || too_many_bends {
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

    if too_many_turns {
        return features;
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
    // A function with turning points takes more than one value: one means the others were
    // lost (1 + 10⁻¹⁴ sin x's extremes snap to 1; 10¹⁴ + sin x's maximum is too large for
    // `defined`). Without turning points, it's a constant between holes.
    let lo = ends.iter().map(|e| e.value).fold(f64::INFINITY, f64::min);
    let hi = ends
        .iter()
        .map(|e| e.value)
        .fold(f64::NEG_INFINITY, f64::max);
    let lost = lo == hi && ends.iter().any(|e| e.attained);
    if known && lo <= hi && !lost {
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

    // Monotonicity between the turning points and poles.
    let cuts: Vec<f64> = turns
        .iter()
        .map(|t| t.0)
        .chain(excluded.iter().copied())
        .collect();
    features.monotonicity = monotonicity(cuts, p, &|a, b| direction_between(fun, a, b));
    features
}

/// Monotonicity between `cuts` (turning points and poles of one period [0, p)), from the
/// first one at or after 0, around one period, as families; `direction` tells how f goes
/// between two cuts.
fn monotonicity(
    cuts: Vec<f64>,
    p: f64,
    direction: &dyn Fn(f64, f64) -> Direction,
) -> Vec<(String, Direction)> {
    let cuts = dedupe(cuts);
    if cuts.is_empty() {
        return vec![("(−∞, ∞)".to_string(), direction(0.0, p))];
    }
    let n = cuts.len();
    let pieces: Vec<(f64, f64, Direction)> = (0..n)
        .map(|i| {
            let a = cuts[i];
            let b = if i + 1 < n { cuts[i + 1] } else { cuts[0] + p };
            (a, b, direction(a, b))
        })
        .collect();
    let (step, firsts) = families(&cuts, p, &|a, b| pieces[a].2 == pieces[b].2);
    firsts
        .iter()
        .map(|&i| {
            let (a, b, direction) = pieces[i];
            let text = format!(
                "{{({}, {}), n₁ ∈ ℤ}}",
                family_term(step, a),
                family_term(step, b)
            );
            (text, direction)
        })
        .collect()
}

/// How many of `points`, found on a window a little larger than one period, lie in the
/// period [0, p).
fn count_in_period(points: &[f64], p: f64) -> usize {
    points.iter().filter(|&&x| (0.0..p).contains(&x)).count()
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
    fn trigonometric_identities_are_constants() {
        // Every shift repeats them, and their derivatives are only rounding noise: they
        // have no period, extrema or inflection points, like y = 1 and y = 0.
        assert_eq!(analyze_text("y=sin(x)^2+cos(x)^2"), analyze_text("y=1"));
        assert_eq!(
            analyze_text("y=cos(x)^2-sin(x)^2-cos(2x)"),
            analyze_text("y=0")
        );
        // 1 where both are defined.
        let f = analyze_text("y=tan(x)cot(x)");
        assert_eq!(f.range.as_deref(), Some("{y = 1}"));
        assert_eq!(f.period, Period::None);
        assert!(f.minima.is_empty() && f.maxima.is_empty() && f.inflection_points.is_empty());
        // 0 at every multiple of 2π/400 + 2π/800, but no constant.
        let f = analyze_text("y=sin(x)sin(400x)");
        assert_eq!(f.period, Period::Periodic("2π".to_string()));
        // Rounding is measured on the terms: next to 10¹², sin x is far above it, and
        // 10⁻¹⁰ sin x is far above the rounding of 1.
        for text in [
            "y=1000000000000+sin(x)",
            "y=60000000000000+sin(x)",
            "y=1+0.0000000001sin(x)",
        ] {
            let f = analyze_text(text);
            assert_eq!(f.period, Period::Periodic("2π".to_string()), "{text}");
            assert_eq!((f.minima.len(), f.maxima.len()), (1, 1), "{text}");
        }
        // 1 but at πn, where it's 0: no constant, but too complex as other jumps are.
        let f = analyze_text("y=sign(sin(x))^2");
        assert_eq!(f.y_intercept.as_deref(), Some("0"));
        assert!(f.too_complex.contains(&Feature::Range), "{f:?}");
        // A constant is only snapped within its rounding: 10⁻¹⁴ isn't 0.
        assert_eq!(
            analyze_text("y=10^(-14)*(sin(x)^2+cos(x)^2)"),
            analyze_text("y=10^(-14)")
        );
        // Next to 10³⁰⁸: the range of y = 10³⁰⁸, not ∞. (Written out: the derivative of
        // 10^308 overflows to NaN.)
        let f = analyze_text(&format!("y=1{}(sin(x)^2+cos(x)^2)", "0".repeat(308)));
        assert_eq!(f.range, analyze_text("y=10^308").range);
        assert_eq!(f.period, Period::None);
        // A constant's derivative is 0 too: 10¹⁵ + sin x − 10¹⁵ takes values an eighth
        // apart, within the rounding of 10¹⁵, but its derivative is cos x.
        let f = analyze_text("y=10^15+sin(x)-10^15");
        assert_eq!(f.range.as_deref(), Some("{y ∈ [−1, 1]}"));
        assert_eq!(f.period, Period::Periodic("2π".to_string()));
        // 10¹⁶ + sin x − 10¹⁶ is 0 in doubles, but its derivative is cos x: rounding hides
        // its values, and features read from them would be wrong.
        let f = analyze_text("y=10^16+sin(x)-10^16");
        assert_eq!(f.period, Period::Periodic("2π".to_string()));
        for feature in SMOOTH_FEATURES {
            assert!(f.too_complex.contains(&feature), "{feature:?}: {f:?}");
        }
        assert!(f.x_intercepts.is_none() && f.minima.is_empty() && f.maxima.is_empty());
        // One value is no range of a function that isn't constant: 1 ± 10⁻¹⁴ snap to 1, and
        // 10¹⁴ + 1 is too large for `defined`, so only the minimum is left.
        for text in ["y=1+10^(-14)*sin(x)", "y=100000000000000+sin(x)"] {
            let f = analyze_text(text);
            assert_eq!(f.period, Period::Periodic("2π".to_string()), "{text}");
            assert_eq!(f.range, None, "{text}");
            assert!(f.too_complex.contains(&Feature::Range), "{text}: {f:?}");
        }
    }

    #[test]
    fn many_features_in_a_period_are_too_complex() {
        // 50 zeros, turning points and inflection points in each period: too many to
        // write, as on the grid of a function that isn't periodic.
        let f = analyze_text("y=sin(25x)+sin(x)");
        assert_eq!(f.period, Period::Periodic("2π".to_string()));
        for feature in [
            Feature::XIntercept,
            Feature::Minima,
            Feature::Maxima,
            Feature::InflectionPoints,
            Feature::Range,
            Feature::Monotonicity,
        ] {
            assert!(
                f.too_complex.contains(&feature),
                "{feature:?} isn't too complex: {:?}",
                f.too_complex
            );
        }
        assert_eq!(f.x_intercepts, None);
        assert!(f.minima.is_empty() && f.maxima.is_empty() && f.inflection_points.is_empty());
        assert!(f.monotonicity.is_empty());
        // More than 40 families of poles in a period (tan 50x + tan x has 51 poles, not
        // evenly spaced): its domain and vertical asymptotes are too complex to write, not
        // missing.
        let f = analyze_text("y=tan(50x)+tan(x)");
        assert!(f.vertical_asymptotes.is_empty());
        for feature in [Feature::Domain, Feature::VerticalAsymptotes] {
            assert!(
                f.too_complex.contains(&feature),
                "{feature:?}: {:?}",
                f.too_complex
            );
        }
        assert_eq!(f.y_intercept.as_deref(), Some("0"));
        // 45 poles in a period, evenly spaced, are one family: they're written out, in every
        // unit.
        for angle in [Angle::Radians, Angle::Degrees, Angle::Gradians] {
            let equation = Equation::parse("y=tan(45x)+tan(x)", Syntax::default()).unwrap();
            let f = analyze(&equation, angle).unwrap();
            assert!(f.domain.is_some(), "{angle:?}: {f:?}");
            assert!(!f.vertical_asymptotes.is_empty(), "{angle:?}");
            assert!(
                !f.too_complex.contains(&Feature::VerticalAsymptotes),
                "{angle:?}"
            );
        }
        let f = analyze_text("y=tan(45x)+tan(x)");
        assert_eq!(
            f.vertical_asymptotes.first().map(String::as_str),
            Some("x = πn₁ + 0.0349066, n₁ ∈ ℤ")
        );
        // But points where f has a value are no gaps, however many: sin(21x)² has 42 zeros
        // in a period, and √(sin(21x)²) + sin x the domain ℝ.
        let f = analyze_text("y=sqrt(sin(21x)^2)+sin(x)");
        assert_eq!(f.domain.as_deref(), Some("{x ∈ ℝ}"));
        assert_eq!(f.period, Period::Periodic("2π".to_string()));
        assert!(!f.too_complex.contains(&Feature::VerticalAsymptotes));
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
