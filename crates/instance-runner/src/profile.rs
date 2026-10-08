//! Optional timing instrumentation (diagnostics only). Enabled when the
//! environment variable `INSTANCE_PROFILE` (or `GM_PROFILE`) is set; `Game::profile_report` prints the
//! hottest events, functions, scripts and step phases (inclusive times).

use std::collections::HashMap;
use web_time::Instant;

#[derive(Default)]
pub struct Profiler {
    /// (owner object, event type, number) → (seconds, calls).
    pub events: HashMap<(i32, i32, i32), (f64, u64)>,
    /// Built-in function id → (seconds, calls).
    pub funcs: HashMap<u16, (f64, u64)>,
    /// Script index → (seconds, calls).
    pub scripts: HashMap<i32, (f64, u64)>,
    /// Step phase name → seconds.
    pub phases: HashMap<&'static str, f64>,
}

impl Profiler {
    pub fn from_env() -> Option<Box<Profiler>> {
        std::env::var_os("INSTANCE_PROFILE").or_else(|| std::env::var_os("GM_PROFILE")).map(|_| Box::default())
    }

    #[inline]
    pub fn add<K: std::hash::Hash + Eq>(m: &mut HashMap<K, (f64, u64)>, k: K, t0: Instant) {
        let e = m.entry(k).or_insert((0.0, 0));
        e.0 += t0.elapsed().as_secs_f64();
        e.1 += 1;
    }
}

impl crate::game::Game {
    /// Prints the profile to stderr (no-op unless profiling is enabled).
    pub fn profile_report(&self) {
        let Some(p) = &self.prof else { return };
        fn top<K: Clone>(m: &HashMap<K, (f64, u64)>, n: usize) -> Vec<(K, f64, u64)> {
            let mut v: Vec<(K, f64, u64)> = m.iter().map(|(k, (t, c))| (k.clone(), *t, *c)).collect();
            v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            v.truncate(n);
            v
        }
        eprintln!("--- profile: phases");
        let mut ph: Vec<_> = p.phases.iter().collect();
        ph.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (k, t) in ph {
            eprintln!("  {:>9.3}s  {}", t, k);
        }
        eprintln!("--- profile: events (inclusive)");
        for ((o, t, n), s, c) in top(&p.events, 25) {
            let on = self.object(o).map(|x| x.name.clone()).unwrap_or_else(|| o.to_string());
            eprintln!("  {:>9.3}s {:>9} calls  {} / {}", s, c, on, crate::events::event_name(t, n, self));
        }
        eprintln!("--- profile: scripts (inclusive)");
        for (k, s, c) in top(&p.scripts, 25) {
            let name = self
                .scripts
                .get(k.max(0) as usize)
                .and_then(|x| x.as_ref())
                .map(|x| x.name.clone())
                .unwrap_or_default();
            eprintln!("  {:>9.3}s {:>9} calls  {}", s, c, name);
        }
        eprintln!("--- profile: builtin functions (inclusive)");
        for (k, s, c) in top(&p.funcs, 30) {
            eprintln!("  {:>9.3}s {:>9} calls  {}", s, c, gml::FunctionId(k).name());
        }
    }
}
