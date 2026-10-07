//! Real-valued functions (chapter 402, "real" section).

use super::{n, Args, NResult, NativeFn};
use crate::value::Value;
use std::cmp::Ordering;
use std::f64::consts::PI;

/// Returns `r`, or an "invalid argument" error when a finite input produced a
/// non-finite result (domain/overflow errors, e.g. `sqrt(-1)`, `ln(0)`).
fn chk(a: &Args, r: f64, inputs: &[f64]) -> NResult {
    if !r.is_finite() && inputs.iter().all(|x| x.is_finite()) {
        return Err(a.invalid());
    }
    Ok(Value::Real(r))
}

fn r1(a: &Args, f: fn(f64) -> f64) -> NResult {
    let x = a.real(0)?;
    chk(a, f(x), &[x])
}

fn r2(a: &Args, f: fn(f64, f64) -> f64) -> NResult {
    let x = a.real(0)?;
    let y = a.real(1)?;
    chk(a, f(x, y), &[x, y])
}

fn reals(a: &Args) -> Result<Vec<f64>, crate::Error> {
    (0..a.len()).map(|i| a.real(i)).collect()
}

/// `point_direction` in degrees, counter-clockwise with y pointing down, in [0, 360).
pub(crate) fn point_direction(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let d = (-(y2 - y1)).atan2(x2 - x1).to_degrees();
    if d < 0.0 {
        d + 360.0
    } else {
        d
    }
}

fn min_max(a: &Args, want: Ordering) -> NResult {
    let first = a.val(0);
    let all_real = first.is_real();
    let mut best = first;
    for i in 1..a.len() {
        let v = a.val(i);
        if v.is_real() != all_real {
            return Err(a.type_err());
        }
        if v.total_cmp(best) == want {
            best = v;
        }
    }
    Ok(best.clone())
}

pub(super) fn register(t: &mut Vec<(&'static str, NativeFn)>) {
    t.extend([
        n("is_real", |_, _, _, a| Ok(Value::from(a.val(0).is_real()))),
        n("is_string", |_, _, _, a| Ok(Value::from(a.val(0).is_str()))),
        n("random", |vm, _, _, a| {
            let x = a.real(0)?;
            Ok(Value::Real(vm.random() * x))
        }),
        n("random_range", |vm, _, _, a| {
            let (x1, x2) = (a.real(0)?, a.real(1)?);
            Ok(Value::Real(x1 + vm.random() * (x2 - x1)))
        }),
        n("irandom", |vm, _, _, a| {
            let x = a.real(0)?.floor();
            if x < 0.0 {
                return Ok(Value::Real(-((vm.random() * (-x + 1.0)).floor())));
            }
            Ok(Value::Real((vm.random() * (x + 1.0)).floor()))
        }),
        n("irandom_range", |vm, _, _, a| {
            let (x1, x2) = (a.real(0)?.floor(), a.real(1)?.floor());
            let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
            Ok(Value::Real(lo + (vm.random() * (hi - lo + 1.0)).floor()))
        }),
        n("random_set_seed", |vm, _, _, a| {
            let s = a.real(0)?;
            vm.set_seed(s);
            Ok(Value::Real(0.0))
        }),
        n("random_get_seed", |vm, _, _, _| Ok(Value::Real(vm.seed()))),
        n("randomize", |vm, _, _, _| {
            vm.randomize();
            Ok(Value::Real(0.0))
        }),
        n("choose", |vm, _, _, a| {
            let k = ((vm.random() * a.len() as f64) as usize).min(a.len() - 1);
            Ok(a.val(k).clone())
        }),
        n("abs", |_, _, _, a| r1(&a, f64::abs)),
        n("round", |_, _, _, a| r1(&a, f64::round_ties_even)),
        n("floor", |_, _, _, a| r1(&a, f64::floor)),
        n("ceil", |_, _, _, a| r1(&a, f64::ceil)),
        n("sign", |_, _, _, a| {
            r1(&a, |x| {
                if x > 0.0 {
                    1.0
                } else if x < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            })
        }),
        n("frac", |_, _, _, a| r1(&a, f64::fract)),
        n("sqrt", |_, _, _, a| {
            let x = a.real(0)?;
            if x < 0.0 {
                return Err(a.invalid());
            }
            Ok(Value::Real(x.sqrt()))
        }),
        n("sqr", |_, _, _, a| r1(&a, |x| x * x)),
        n("exp", |_, _, _, a| r1(&a, f64::exp)),
        n("ln", |_, _, _, a| {
            r1(&a, |x| if x <= 0.0 { f64::NAN } else { x.ln() })
        }),
        n("log2", |_, _, _, a| {
            r1(&a, |x| if x <= 0.0 { f64::NAN } else { x.log2() })
        }),
        n("log10", |_, _, _, a| {
            r1(&a, |x| if x <= 0.0 { f64::NAN } else { x.log10() })
        }),
        n("logn", |_, _, _, a| {
            r2(&a, |b, x| {
                if x <= 0.0 || b <= 0.0 || b == 1.0 {
                    f64::NAN
                } else {
                    x.ln() / b.ln()
                }
            })
        }),
        n("sin", |_, _, _, a| r1(&a, f64::sin)),
        n("cos", |_, _, _, a| r1(&a, f64::cos)),
        n("tan", |_, _, _, a| r1(&a, f64::tan)),
        n("arcsin", |_, _, _, a| r1(&a, f64::asin)),
        n("arccos", |_, _, _, a| r1(&a, f64::acos)),
        n("arctan", |_, _, _, a| r1(&a, f64::atan)),
        n("arctan2", |_, _, _, a| r2(&a, f64::atan2)),
        n("degtorad", |_, _, _, a| r1(&a, |x| x * PI / 180.0)),
        n("radtodeg", |_, _, _, a| r1(&a, |x| x * 180.0 / PI)),
        n("power", |_, _, _, a| r2(&a, f64::powf)),
        n("min", |_, _, _, a| min_max(&a, Ordering::Less)),
        n("max", |_, _, _, a| min_max(&a, Ordering::Greater)),
        n("mean", |_, _, _, a| {
            let v = reals(&a)?;
            Ok(Value::Real(v.iter().sum::<f64>() / v.len() as f64))
        }),
        n("median", |_, _, _, a| {
            let mut v = reals(&a)?;
            v.sort_by(|x, y| x.total_cmp(y));
            Ok(Value::Real(v[(v.len() - 1) / 2]))
        }),
        n("clamp", |_, _, _, a| {
            let (v, lo, hi) = (a.real(0)?, a.real(1)?, a.real(2)?);
            Ok(Value::Real(if v < lo {
                lo
            } else if v > hi {
                hi
            } else {
                v
            }))
        }),
        n("lerp", |_, _, _, a| {
            let (x, y, t) = (a.real(0)?, a.real(1)?, a.real(2)?);
            Ok(Value::Real(x + (y - x) * t))
        }),
        n("dot_product", |_, _, _, a| {
            let v = reals(&a)?;
            let (l1, l2) = (v[0].hypot(v[1]), v[2].hypot(v[3]));
            if l1 == 0.0 || l2 == 0.0 {
                return Ok(Value::Real(0.0));
            }
            Ok(Value::Real((v[0] * v[2] + v[1] * v[3]) / (l1 * l2)))
        }),
        n("dot_product_3d", |_, _, _, a| {
            let v = reals(&a)?;
            let l1 = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            let l2 = (v[3] * v[3] + v[4] * v[4] + v[5] * v[5]).sqrt();
            if l1 == 0.0 || l2 == 0.0 {
                return Ok(Value::Real(0.0));
            }
            Ok(Value::Real(
                (v[0] * v[3] + v[1] * v[4] + v[2] * v[5]) / (l1 * l2),
            ))
        }),
        n("point_distance_3d", |_, _, _, a| {
            let v = reals(&a)?;
            let (dx, dy, dz) = (v[3] - v[0], v[4] - v[1], v[5] - v[2]);
            Ok(Value::Real((dx * dx + dy * dy + dz * dz).sqrt()))
        }),
        n("point_distance", |_, _, _, a| {
            let v = reals(&a)?;
            Ok(Value::Real((v[2] - v[0]).hypot(v[3] - v[1])))
        }),
        n("point_direction", |_, _, _, a| {
            let v = reals(&a)?;
            Ok(Value::Real(point_direction(v[0], v[1], v[2], v[3])))
        }),
        n("lengthdir_x", |_, _, _, a| {
            let (l, d) = (a.real(0)?, a.real(1)?);
            Ok(Value::Real(l * d.to_radians().cos()))
        }),
        n("lengthdir_y", |_, _, _, a| {
            let (l, d) = (a.real(0)?, a.real(1)?);
            Ok(Value::Real(-l * d.to_radians().sin()))
        }),
    ]);
}
