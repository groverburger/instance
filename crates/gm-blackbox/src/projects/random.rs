//! bb_random: random number generator sequences after random_set_seed.

use crate::builder::Gen;
use crate::framework::*;

fn seq(key: &str, seed: &str, n: u32, expr: &str) -> Test {
    raw(
        key,
        &[&format!(
            r#"var i;
random_set_seed({seed});
for (i = 0; i < {n}; i += 1) bb_log('{key}.' + bb_pad(i), {expr});"#
        )],
    )
}

pub fn build() -> Spec {
    let mut g = Gen::new("bb_random", 81002);
    let fw = install(&mut g);
    let raw32 = "floor(random(1) * 4294967296)";
    let mut tests = vec![
        // value of the seed right after game start (randomize? fixed?): informational
        vals("rnd.start", &[("rnd.start.seed", "random_get_seed()")]),
        seq("rnd.s0.raw", "0", 20, raw32),
        seq("rnd.s1.raw", "1", 20, raw32),
        seq("rnd.s12345.raw", "12345", 20, raw32),
        seq("rnd.s1.r1", "1", 5, "random(1)"),
        seq("rnd.sm1.raw", "-1", 5, raw32),
        seq("rnd.s2p31.raw", "2147483648", 5, raw32),
        seq("rnd.s2p32.raw", "4294967296", 3, raw32),
        seq("rnd.s0_5.raw", "0.5", 3, raw32),
        seq("rnd.s1_5.raw", "1.5", 3, raw32),
        seq("rnd.s1.irandom100", "1", 20, "irandom(100)"),
        seq("rnd.s1.choose5", "1", 20, "choose(1, 2, 3, 4, 5)"),
        seq("rnd.s1.choose_str", "1", 5, "choose('a', 'b', 'c')"),
        seq("rnd.s2.range5_10", "2", 5, "random_range(5, 10)"),
        seq("rnd.s2.irange3_7", "2", 10, "irandom_range(3, 7)"),
        seq("rnd.s2.irange7_3", "2", 5, "irandom_range(7, 3)"),
        seq("rnd.s2.irange_frac", "2", 5, "irandom_range(0.5, 2.5)"),
        seq("rnd.s3.random10", "3", 5, "random(10)"),
        seq("rnd.s3.irandom1", "3", 10, "irandom(1)"),
        seq("rnd.s3.irandom2_5", "3", 10, "irandom(2.5)"),
        seq("rnd.s3.irandom2_7", "3", 5, "irandom(2.7)"),
        // the generator state after each call (reveals the update rule)
        raw(
            "rnd.s1.getseed_after",
            &[r#"var i;
random_set_seed(1);
for (i = 0; i < 5; i += 1) { random(1); bb_log('rnd.s1.getseed_after.' + bb_pad(i), random_get_seed()); }"#],
        ),
        raw(
            "rnd.getseed",
            &[r#"random_set_seed(1);
bb_log('rnd.getseed.after_set1', random_get_seed());
random_set_seed(12345);
bb_log('rnd.getseed.after_set12345', random_get_seed());
random_set_seed(-1);
bb_log('rnd.getseed.after_setm1', random_get_seed());
random_set_seed(2.7);
bb_log('rnd.getseed.after_set2_7', random_get_seed());
random_set_seed(1); random(1);
bb_log('rnd.getseed.after_set1_random', random_get_seed());
random_set_seed(1); irandom(100);
bb_log('rnd.getseed.after_set1_irandom', random_get_seed());
random_set_seed(1); choose(1, 2);
bb_log('rnd.getseed.after_set1_choose', random_get_seed());"#],
        ),
        err_stmt("rnd.random0", "random_set_seed(4); global.bb_r = random(0);"),
        err_stmt("rnd.random_neg", "random_set_seed(4); global.bb_r = random(-5);"),
        err_stmt("rnd.irandom_neg", "random_set_seed(4); global.bb_r = irandom(-5);"),
        err_stmt("rnd.irandom0", "random_set_seed(4); global.bb_r = irandom(0);"),
        err_stmt("rnd.choose1", "random_set_seed(4); global.bb_r = choose(9);"),
        // does random() consume one step per call regardless of the range?
        raw(
            "rnd.mix",
            &[r#"var a, b;
random_set_seed(5); random(1); a = random(1);
random_set_seed(5); random(1000); b = random(1);
bb_log('rnd.mix.same_after_scaled', a == b);
random_set_seed(5); irandom(10); b = random(1);
bb_log('rnd.mix.same_after_irandom', a == b);
random_set_seed(5); choose(1, 2, 3); b = random(1);
bb_log('rnd.mix.same_after_choose', a == b);
random_set_seed(5); random_range(1, 2); b = random(1);
bb_log('rnd.mix.same_after_range', a == b);"#],
        ),
        // randomize() changes the seed (informational: value differs per run)
        raw(
            "rnd.randomize",
            &[r#"var s;
random_set_seed(1);
randomize();
s = random_get_seed();
bb_log('rnd.randomize.changed', s != 1);"#],
        ),
    ];
    tests.push(vals("rnd.done", &[("rnd.done", "1")]));
    let frames = unit_project(&mut g, &fw, tests, 60);
    Spec {
        name: "bb_random",
        title: "Random generator sequences (random_set_seed, random, irandom, choose)",
        lite: "Lite-safe.",
        frames,
        keys: vec![
            ki("rnd.start", "G-26, runtime.md §2 step 5", "seed at game start (differs per run unless fixed)"),
            k("rnd.s", "G-26", "value sequences after random_set_seed (raw = floor(random(1)*2^32))"),
            k("rnd.getseed", "G-26", "random_get_seed after set and after calls"),
            k("rnd.random", "G-26", "random(0) / negative ranges"),
            k("rnd.irandom", "G-26", "irandom edge cases"),
            k("rnd.choose", "G-26", "choose with one argument"),
            k("rnd.mix", "G-26", "whether every function consumes exactly one generator step"),
            k("rnd.randomize", "G-26", "randomize changes the seed"),
            k("rnd.done", "-", "end marker"),
        ],
        project: g.finish(),
    }
}
