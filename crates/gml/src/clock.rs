//! Wall-clock access that works on every target, including `wasm32-unknown-unknown`
//! (where `std::time::SystemTime::now` panics).

/// Seconds since 1970-01-01 UTC.
pub fn unix_seconds() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

/// Offset of local time from UTC in seconds (east positive), at the current moment.
pub fn local_offset_seconds() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        -js_sys::Date::new_0().get_timezone_offset() * 60.0
    }
    #[cfg(unix)]
    {
        // `struct tm` has the same leading layout on glibc, musl and Darwin,
        // with `tm_gmtoff` after the nine ints.
        #[repr(C)]
        struct Tm {
            sec: i32,
            min: i32,
            hour: i32,
            mday: i32,
            mon: i32,
            year: i32,
            wday: i32,
            yday: i32,
            isdst: i32,
            gmtoff: std::ffi::c_long,
            zone: *const std::ffi::c_char,
        }
        extern "C" {
            fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
        }
        let now = unix_seconds() as i64;
        let mut tm = std::mem::MaybeUninit::<Tm>::zeroed();
        // SAFETY: localtime_r only writes the provided struct; `time_t` is 64-bit
        // on every unix target we build for.
        let r = unsafe { localtime_r(&now, tm.as_mut_ptr()) };
        if r.is_null() {
            0.0
        } else {
            // SAFETY: initialised by the successful call above.
            unsafe { tm.assume_init() }.gmtoff as f64
        }
    }
    #[cfg(not(any(unix, target_arch = "wasm32")))]
    {
        0.0
    }
}

/// Nanosecond-ish clock value for seeding random generators.
pub fn seed_nanos() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() * 1.0e6) as u64 ^ ((js_sys::Math::random() * 1.0e9) as u64)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}
