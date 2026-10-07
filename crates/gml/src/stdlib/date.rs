//! Date and time functions (chapter 402, "dates" section).
//!
//! A date-time value is a real: whole days since 1899-12-30 in the integral part
//! and the time of day as the fraction of a day.

use super::{n, Args, NResult, NativeFn};
use crate::value::Value;

const EPOCH_OFFSET: i64 = 25569; // days from 1899-12-30 to 1970-01-01
const MS_PER_DAY: i64 = 86_400_000;

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub(crate) fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_civil`].
pub(crate) fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if leap(y) {
                29
            } else {
                28
            }
        }
    }
}

/// Decoded date-time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dt {
    pub y: i64,
    pub m: u32,
    pub d: u32,
    pub hh: i64,
    pub mi: i64,
    pub ss: i64,
    pub ms: i64,
    /// Day number (days since 1899-12-30).
    pub day: i64,
}

pub(crate) fn decode(v: f64) -> Dt {
    let v = if v.is_finite() { v } else { 0.0 };
    let mut day = v.floor() as i64;
    let mut ms = ((v - v.floor()) * MS_PER_DAY as f64).round() as i64;
    if ms >= MS_PER_DAY {
        day += 1;
        ms -= MS_PER_DAY;
    }
    let (y, m, d) = civil_from_days(day - EPOCH_OFFSET);
    Dt {
        y,
        m,
        d,
        hh: ms / 3_600_000,
        mi: ms / 60_000 % 60,
        ss: ms / 1000 % 60,
        ms: ms % 1000,
        day,
    }
}

pub(crate) fn encode_date(y: i64, m: u32, d: u32) -> f64 {
    (days_from_civil(y, m, d) + EPOCH_OFFSET) as f64
}

fn encode_time(h: i64, mi: i64, s: i64) -> f64 {
    (h * 3600 + mi * 60 + s) as f64 / 86400.0
}

fn valid_date(y: i64, m: i64, d: i64) -> bool {
    (1..=9999).contains(&y)
        && (1..=12).contains(&m)
        && d >= 1
        && d <= days_in_month(y, m as u32) as i64
}

fn valid_time(h: i64, mi: i64, s: i64) -> bool {
    (0..24).contains(&h) && (0..60).contains(&mi) && (0..60).contains(&s)
}

fn day_of_year(dt: &Dt) -> i64 {
    days_from_civil(dt.y, dt.m, dt.d) - days_from_civil(dt.y, 1, 1) + 1
}

fn iso_week(dt: &Dt) -> i64 {
    let days = days_from_civil(dt.y, dt.m, dt.d);
    let wd = (days + 3).rem_euclid(7); // Monday = 0
    let thursday = days - wd + 3;
    let (ty, _, _) = civil_from_days(thursday);
    (thursday - days_from_civil(ty, 1, 1)) / 7 + 1
}

fn add_months(v: f64, months: i64) -> f64 {
    let dt = decode(v);
    let total = dt.y * 12 + (dt.m as i64 - 1) + months;
    let y = total.div_euclid(12);
    let m = (total.rem_euclid(12) + 1) as u32;
    let d = dt.d.min(days_in_month(y, m));
    encode_date(y, m, d) + (v - v.floor())
}

fn cmp(a: f64, b: f64) -> f64 {
    if a < b {
        -1.0
    } else if a > b {
        1.0
    } else {
        0.0
    }
}

fn fmt_date(dt: &Dt) -> String {
    format!("{:04}-{:02}-{:02}", dt.y, dt.m, dt.d)
}
fn fmt_time(dt: &Dt) -> String {
    format!("{:02}:{:02}:{:02}", dt.hh, dt.mi, dt.ss)
}

/// The current local date-time from the operating system.
pub(crate) fn system_now() -> f64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct SystemTime {
            year: u16,
            month: u16,
            day_of_week: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            milliseconds: u16,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetLocalTime(st: *mut SystemTime);
        }
        let mut st = SystemTime::default();
        // SAFETY: GetLocalTime only writes the provided, properly sized struct.
        unsafe { GetLocalTime(&mut st) };
        encode_date(st.year as i64, st.month as u32, st.day as u32)
            + (st.hour as f64 * 3600.0
                + st.minute as f64 * 60.0
                + st.second as f64
                + st.milliseconds as f64 / 1000.0)
                / 86400.0
    }
    #[cfg(not(windows))]
    {
        // Without a time-zone database we use UTC.
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        EPOCH_OFFSET as f64 + secs / 86400.0
    }
}

fn d1(a: &Args, f: fn(&Dt, f64) -> f64) -> NResult {
    let v = a.real(0)?;
    Ok(Value::Real(f(&decode(v), v)))
}

fn span(a: &Args, per_day: f64) -> NResult {
    let (x, y) = (a.real(0)?, a.real(1)?);
    Ok(Value::Real((y - x).abs() * per_day))
}

pub(super) fn register(t: &mut Vec<(&'static str, NativeFn)>) {
    t.extend([
        n("date_current_datetime", |vm, _, _, _| {
            Ok(Value::Real(vm.now()))
        }),
        n("date_current_date", |vm, _, _, _| {
            Ok(Value::Real(vm.now().floor()))
        }),
        n("date_current_time", |vm, _, _, _| {
            let v = vm.now();
            Ok(Value::Real(v - v.floor()))
        }),
        n("date_create_datetime", |_, _, _, a| {
            let p: Vec<i64> = (0..6).map(|i| a.int(i)).collect::<Result<_, _>>()?;
            if !valid_date(p[0], p[1], p[2]) || !valid_time(p[3], p[4], p[5]) {
                return Ok(Value::Real(0.0));
            }
            Ok(Value::Real(
                encode_date(p[0], p[1] as u32, p[2] as u32) + encode_time(p[3], p[4], p[5]),
            ))
        }),
        n("date_create_date", |_, _, _, a| {
            let (y, m, d) = (a.int(0)?, a.int(1)?, a.int(2)?);
            if !valid_date(y, m, d) {
                return Ok(Value::Real(0.0));
            }
            Ok(Value::Real(encode_date(y, m as u32, d as u32)))
        }),
        n("date_create_time", |_, _, _, a| {
            let (h, mi, s) = (a.int(0)?, a.int(1)?, a.int(2)?);
            if !valid_time(h, mi, s) {
                return Ok(Value::Real(0.0));
            }
            Ok(Value::Real(encode_time(h, mi, s)))
        }),
        n("date_valid_datetime", |_, _, _, a| {
            let p: Vec<i64> = (0..6).map(|i| a.int(i)).collect::<Result<_, _>>()?;
            Ok(Value::from(
                valid_date(p[0], p[1], p[2]) && valid_time(p[3], p[4], p[5]),
            ))
        }),
        n("date_valid_date", |_, _, _, a| {
            Ok(Value::from(valid_date(a.int(0)?, a.int(1)?, a.int(2)?)))
        }),
        n("date_valid_time", |_, _, _, a| {
            Ok(Value::from(valid_time(a.int(0)?, a.int(1)?, a.int(2)?)))
        }),
        n("date_inc_year", |_, _, _, a| {
            Ok(Value::Real(add_months(
                a.real(0)?,
                a.int(1)?.saturating_mul(12),
            )))
        }),
        n("date_inc_month", |_, _, _, a| {
            Ok(Value::Real(add_months(a.real(0)?, a.int(1)?)))
        }),
        n("date_inc_week", |_, _, _, a| {
            Ok(Value::Real(a.real(0)? + 7.0 * a.int(1)? as f64))
        }),
        n("date_inc_day", |_, _, _, a| {
            Ok(Value::Real(a.real(0)? + a.int(1)? as f64))
        }),
        n("date_inc_hour", |_, _, _, a| {
            Ok(Value::Real(a.real(0)? + a.int(1)? as f64 / 24.0))
        }),
        n("date_inc_minute", |_, _, _, a| {
            Ok(Value::Real(a.real(0)? + a.int(1)? as f64 / 1440.0))
        }),
        n("date_inc_second", |_, _, _, a| {
            Ok(Value::Real(a.real(0)? + a.int(1)? as f64 / 86400.0))
        }),
        n("date_get_year", |_, _, _, a| d1(&a, |d, _| d.y as f64)),
        n("date_get_month", |_, _, _, a| d1(&a, |d, _| d.m as f64)),
        n("date_get_week", |_, _, _, a| {
            d1(&a, |d, _| iso_week(d) as f64)
        }),
        n("date_get_day", |_, _, _, a| d1(&a, |d, _| d.d as f64)),
        n("date_get_hour", |_, _, _, a| d1(&a, |d, _| d.hh as f64)),
        n("date_get_minute", |_, _, _, a| d1(&a, |d, _| d.mi as f64)),
        n("date_get_second", |_, _, _, a| d1(&a, |d, _| d.ss as f64)),
        n("date_get_weekday", |_, _, _, a| {
            d1(&a, |d, _| {
                ((days_from_civil(d.y, d.m, d.d) + 4).rem_euclid(7) + 1) as f64
            })
        }),
        n("date_get_day_of_year", |_, _, _, a| {
            d1(&a, |d, _| day_of_year(d) as f64)
        }),
        n("date_get_hour_of_year", |_, _, _, a| {
            d1(&a, |d, _| ((day_of_year(d) - 1) * 24 + d.hh) as f64)
        }),
        n("date_get_minute_of_year", |_, _, _, a| {
            d1(&a, |d, _| {
                (((day_of_year(d) - 1) * 24 + d.hh) * 60 + d.mi) as f64
            })
        }),
        n("date_get_second_of_year", |_, _, _, a| {
            d1(&a, |d, _| {
                ((((day_of_year(d) - 1) * 24 + d.hh) * 60 + d.mi) * 60 + d.ss) as f64
            })
        }),
        n("date_year_span", |_, _, _, a| span(&a, 1.0 / 365.25)),
        n("date_month_span", |_, _, _, a| span(&a, 1.0 / 30.4375)),
        n("date_week_span", |_, _, _, a| span(&a, 1.0 / 7.0)),
        n("date_day_span", |_, _, _, a| span(&a, 1.0)),
        n("date_hour_span", |_, _, _, a| span(&a, 24.0)),
        n("date_minute_span", |_, _, _, a| span(&a, 1440.0)),
        n("date_second_span", |_, _, _, a| span(&a, 86400.0)),
        n("date_compare_datetime", |_, _, _, a| {
            Ok(Value::Real(cmp(a.real(0)?, a.real(1)?)))
        }),
        n("date_compare_date", |_, _, _, a| {
            Ok(Value::Real(cmp(
                decode(a.real(0)?).day as f64,
                decode(a.real(1)?).day as f64,
            )))
        }),
        n("date_compare_time", |_, _, _, a| {
            let (x, y) = (decode(a.real(0)?), decode(a.real(1)?));
            let tx = ((x.hh * 60 + x.mi) * 60 + x.ss) * 1000 + x.ms;
            let ty = ((y.hh * 60 + y.mi) * 60 + y.ss) * 1000 + y.ms;
            Ok(Value::Real(cmp(tx as f64, ty as f64)))
        }),
        n("date_date_of", |_, _, _, a| {
            Ok(Value::Real(a.real(0)?.floor()))
        }),
        n("date_time_of", |_, _, _, a| {
            let v = a.real(0)?;
            Ok(Value::Real(v - v.floor()))
        }),
        n("date_datetime_string", |_, _, _, a| {
            let d = decode(a.real(0)?);
            Ok(Value::from(format!("{} {}", fmt_date(&d), fmt_time(&d))))
        }),
        n("date_date_string", |_, _, _, a| {
            Ok(Value::from(fmt_date(&decode(a.real(0)?))))
        }),
        n("date_time_string", |_, _, _, a| {
            Ok(Value::from(fmt_time(&decode(a.real(0)?))))
        }),
        n("date_days_in_month", |_, _, _, a| {
            d1(&a, |d, _| days_in_month(d.y, d.m) as f64)
        }),
        n("date_days_in_year", |_, _, _, a| {
            d1(&a, |d, _| if leap(d.y) { 366.0 } else { 365.0 })
        }),
        n("date_leap_year", |_, _, _, a| {
            d1(&a, |d, _| if leap(d.y) { 1.0 } else { 0.0 })
        }),
        n("date_is_today", |vm, _, _, a| {
            let v = a.real(0)?;
            Ok(Value::from(decode(v).day == decode(vm.now()).day))
        }),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(encode_date(1899, 12, 30), 0.0);
        assert_eq!(encode_date(1900, 1, 1), 2.0);
        for z in [-800_000i64, -1, 0, 1, 10957, 20000, 2_000_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        let dt = decode(encode_date(2024, 2, 29) + 0.75);
        assert_eq!(
            (dt.y, dt.m, dt.d, dt.hh, dt.mi, dt.ss),
            (2024, 2, 29, 18, 0, 0)
        );
        assert_eq!(iso_week(&decode(encode_date(2021, 1, 3))), 53);
        assert_eq!(iso_week(&decode(encode_date(2021, 1, 4))), 1);
        assert_eq!(decode(add_months(encode_date(2023, 1, 31), 1)).d, 28);
    }
}
