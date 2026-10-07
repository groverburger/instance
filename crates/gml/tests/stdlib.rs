//! Tests for the built-in functions implemented by the interpreter.
#![allow(clippy::needless_range_loop)]

mod common;
use common::*;
use gml::*;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn registry() {
    // Every listed function/variable/constant is known.
    assert_eq!(builtins::FUNCTIONS.len(), 1077);
    assert!(builtins::function_id("draw_sprite").is_some());
    assert!(builtins::variable_id("view_xview").is_some());
    assert_eq!(builtins::constant_value("c_red"), Some(255.0));
    assert_eq!(builtins::constant_value("pi"), Some(std::f64::consts::PI));
    // natives vs host
    assert!(is_native(builtins::functions::STRING_COPY));
    assert!(is_native(builtins::functions::DS_GRID_ADD_DISK));
    assert!(is_native(builtins::functions::DATE_INC_MONTH));
    assert!(!is_native(builtins::functions::DRAW_SPRITE));
    assert!(!is_native(builtins::functions::INSTANCE_CREATE));
    let natives = builtins::FUNCTIONS
        .iter()
        .enumerate()
        .filter(|(i, _)| is_native(FunctionId(*i as u16)))
        .count();
    assert!(natives > 200, "{}", natives);
    // every function of chapters 402 (except clipboard) and 411 is native
    for (i, f) in builtins::FUNCTIONS.iter().enumerate() {
        if (f.chapter == "402" && !f.name.starts_with("clipboard")) || f.chapter == "411" {
            assert!(
                is_native(FunctionId(i as u16)),
                "{} should be native",
                f.name
            );
        }
    }
    // variadic arities
    let choose = builtins::functions::CHOOSE.info();
    assert_eq!((choose.min_args, choose.max_args), (1, 16));
    let ext = builtins::function_id("external_define").unwrap().info();
    assert_eq!((ext.min_args, ext.max_args), (5, 16));
}

#[test]
fn real_functions() {
    assert_eq!(real("return abs(-3) + sign(-2) + sign(0) + sign(5)"), 3.0);
    assert_eq!(real("return round(2.5) + round(3.5) + round(-0.5)"), 6.0);
    assert_eq!(real("return floor(-1.5) + ceil(1.2)"), 0.0);
    assert!(near(real("return frac(-3.25)"), -0.25));
    assert_eq!(
        real("return sqrt(16) + sqr(3) + power(2, 10)"),
        4.0 + 9.0 + 1024.0
    );
    assert!(near(real("return exp(1)"), std::f64::consts::E));
    assert!(near(
        real("return ln(exp(2)) + log2(8) + log10(1000) + logn(3, 81)"),
        2.0 + 3.0 + 3.0 + 4.0
    ));
    assert!(near(real("return sin(pi / 2) + cos(0) + tan(0)"), 2.0));
    assert!(near(
        real("return arcsin(1) + arccos(1) + arctan(1) - pi / 2 - pi / 4"),
        0.0
    ));
    assert!(near(
        real("return arctan2(1, -1)"),
        3.0 * std::f64::consts::FRAC_PI_4
    ));
    assert!(near(real("return radtodeg(degtorad(123))"), 123.0));
    assert_eq!(real("return min(3, 1, 2) + max(3, 1, 2) * 10"), 31.0);
    assert_eq!(
        string("return min('b', 'a', 'c') + max('b', 'a', 'c')"),
        "ac"
    );
    assert!(err("return min(1, 'a')").message.contains("Wrong type"));
    assert_eq!(real("return mean(1, 2, 3, 4)"), 2.5);
    assert_eq!(real("return median(5, 1, 3)"), 3.0);
    assert_eq!(real("return median(4, 1, 3, 2)"), 2.0); // lower middle
    assert_eq!(
        real("return clamp(5, 0, 3) + clamp(-1, 0, 3) + clamp(2, 0, 3)"),
        5.0
    );
    assert_eq!(real("return lerp(10, 20, 0.25)"), 12.5);
    assert!(near(real("return dot_product(2, 0, 0, 5)"), 0.0));
    assert!(near(real("return dot_product(2, 0, 3, 0)"), 1.0));
    assert!(near(real("return dot_product_3d(1, 1, 0, 2, 2, 0)"), 1.0));
    assert_eq!(real("return point_distance(0, 0, 3, 4)"), 5.0);
    assert_eq!(real("return point_distance_3d(0, 0, 0, 2, 3, 6)"), 7.0);
    assert!(near(real("return point_direction(0, 0, 0, -10)"), 90.0)); // y grows downward
    assert!(near(real("return point_direction(0, 0, -1, 0)"), 180.0));
    assert!(near(real("return point_direction(0, 0, 0, 10)"), 270.0));
    assert!(near(
        real("return lengthdir_x(10, 0) + lengthdir_y(10, 90)"),
        0.0
    ));
    assert!(near(real("return lengthdir_y(10, 270)"), 10.0));
    assert_eq!(
        real("return is_real(1) + is_string('a') * 10 + is_real('a') * 100"),
        11.0
    );
    assert!(err("return sqrt(-1)").message.contains("sqrt"));
    assert!(err("return ln(0)").message.contains("ln"));
    assert!(err("return arcsin(2)").message.contains("arcsin"));
    assert!(err("return abs('x')")
        .message
        .contains("Wrong type of arguments to function abs"));
}

#[test]
fn random_functions() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    let src = "random_set_seed(42) a = random(10) + irandom(100) + random_range(5, 6) + irandom_range(3, 7) random_set_seed(42) b = random(10) + irandom(100) + random_range(5, 6) + irandom_range(3, 7) return a == b && random_get_seed() == 42";
    assert_eq!(run_in(&mut vm, &mut h, src).unwrap(), Value::Real(1.0));
    let src = "for (i = 0; i < 2000; i += 1) {
        r = random(5); if r < 0 || r >= 5 return 'random ' + string(r)
        k = irandom(3); if k != floor(k) || k < 0 || k > 3 return 'irandom'
        k = irandom_range(2.7, 5.9); if k != floor(k) || k < 2 || k > 5 return 'irandom_range ' + string(k)
        r = random_range(-2, 2); if r < -2 || r >= 2 return 'random_range'
        c = choose('a', 'b', 'c'); if c != 'a' && c != 'b' && c != 'c' return 'choose'
        seen[irandom(3)] = 1
    } return seen[0] + seen[1] + seen[2] + seen[3]";
    assert_eq!(run_in(&mut vm, &mut h, src).unwrap(), Value::Real(4.0));
    run_in(&mut vm, &mut h, "randomize()").unwrap();
}

#[test]
fn string_functions() {
    assert_eq!(
        real("return real('12.5') + real(' -2') + real('1e2') + real('junk') + real(3)"),
        12.5 - 2.0 + 100.0 + 3.0
    );
    assert_eq!(
        string("return string(5) + '|' + string(0.5) + '|' + string(-1/3) + '|' + string('s')"),
        "5|0.50|-0.33|s"
    );
    assert_eq!(
        string("return string_format(3.14159, 8, 3) + '|' + string_format(-2, 0, 2)"),
        "   3.142|-2.00"
    );
    assert_eq!(string("return chr(65) + chr(97 + 256)"), "Aa");
    assert_eq!(real("return ord('A') + ord('')"), 65.0);
    assert_eq!(
        real("return string_length('hello') + string_length('')"),
        5.0
    );
    assert_eq!(
        real("return string_pos('lo', 'hello') * 10 + string_pos('x', 'hello')"),
        40.0
    );
    assert_eq!(string("return string_copy('abcdef', 2, 3) + '|' + string_copy('abc', 0, 2) + '|' + string_copy('abc', 3, 10) + '|' + string_copy('abc', 5, 1)"), "bcd|ab|c|");
    assert_eq!(
        string(
            "return string_char_at('abc', 2) + string_char_at('abc', 9) + string_char_at('abc', 0)"
        ),
        "b"
    );
    assert_eq!(
        string("return string_delete('abcdef', 2, 3) + '|' + string_delete('abc', 3, 9)"),
        "aef|ab"
    );
    assert_eq!(
        string("return string_insert('XY', 'abc', 2) + '|' + string_insert('!', 'abc', 99)"),
        "aXYbc|abc!"
    );
    assert_eq!(
        string("return string_lower('AbC9') + string_upper('aBc9')"),
        "abc9ABC9"
    );
    assert_eq!(
        string("return string_repeat('ab', 3) + string_repeat('x', 0)"),
        "ababab"
    );
    assert_eq!(string("return string_letters('a1-b2 C') + '|' + string_digits('a1-b2 C') + '|' + string_lettersdigits('a1-b2 C')"), "abC|12|a1b2C");
    assert_eq!(string("return string_replace('aXbXc', 'X', '--') + '|' + string_replace_all('aXbXc', 'X', '--')"), "a--bXc|a--b--c");
    assert_eq!(
        string("return string_replace_all('aaa', 'a', 'aa')"),
        "aaaaaa"
    );
    assert_eq!(
        real("return string_count('ab', 'abcabab') + string_count('aa', 'aaaa') * 10"),
        23.0
    );
    assert!(err("return string_length(5)")
        .message
        .contains("string_length"));
    // byte strings: chr(200) is a single character
    assert_eq!(real("return string_length(chr(200) + chr(0))"), 2.0);
    assert_eq!(real("return ord(string_upper(chr(233)))"), 201.0);
}

#[test]
fn date_functions() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    // 2024-03-15 13:45:30
    let now = 45366.0 + (13.0 * 3600.0 + 45.0 * 60.0 + 30.0) / 86400.0;
    vm.set_clock(Box::new(move || now));
    let r = |vm: &mut Vm, h: &mut Mock, s: &str| {
        run_in(vm, h, s).unwrap_or_else(|e| panic!("{}: {}", s, e))
    };
    assert_eq!(
        r(&mut vm, &mut h, "return date_create_date(2024, 3, 15)"),
        Value::Real(45366.0)
    );
    assert_eq!(
        r(&mut vm, &mut h, "return date_current_date()"),
        Value::Real(45366.0)
    );
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_create_datetime(2024, 3, 15, 13, 45, 30) == date_current_datetime()"
        ),
        Value::Real(1.0)
    );
    assert_eq!(
        r(&mut vm, &mut h, "d = date_current_datetime() return string(date_get_year(d)) + '-' + string(date_get_month(d)) + '-' + string(date_get_day(d)) + ' ' + string(date_get_hour(d)) + ':' + string(date_get_minute(d)) + ':' + string(date_get_second(d))"),
        Value::from("2024-3-15 13:45:30")
    );
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_get_weekday(date_create_date(2024, 3, 15))"
        ),
        Value::Real(6.0)
    ); // Friday, 1 = Sunday
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_get_week(date_create_date(2024, 3, 15))"
        ),
        Value::Real(11.0)
    );
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_get_day_of_year(date_create_date(2024, 3, 1))"
        ),
        Value::Real(61.0)
    );
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_get_hour_of_year(date_create_datetime(2024, 1, 2, 3, 0, 0))"
        ),
        Value::Real(27.0)
    );
    assert_eq!(r(&mut vm, &mut h, "return date_days_in_month(date_create_date(2024, 2, 1)) + date_days_in_year(date_create_date(2023, 1, 1))"), Value::Real(29.0 + 365.0));
    assert_eq!(r(&mut vm, &mut h, "return date_leap_year(date_create_date(2000, 1, 1)) + date_leap_year(date_create_date(1900, 1, 1))"), Value::Real(1.0));
    assert_eq!(r(&mut vm, &mut h, "return date_valid_date(2023, 2, 29) + date_valid_date(2024, 2, 29) + date_valid_time(24, 0, 0) + date_valid_datetime(2024, 1, 1, 23, 59, 59)"), Value::Real(2.0));
    assert_eq!(
        r(&mut vm, &mut h, "return date_create_date(2023, 2, 29)"),
        Value::Real(0.0)
    );
    assert_eq!(r(&mut vm, &mut h, "return date_inc_month(date_create_date(2024, 1, 31), 1) == date_create_date(2024, 2, 29)"), Value::Real(1.0));
    assert_eq!(r(&mut vm, &mut h, "return date_inc_year(date_create_date(2024, 2, 29), 1) == date_create_date(2025, 2, 28)"), Value::Real(1.0));
    assert_eq!(r(&mut vm, &mut h, "return date_inc_day(date_create_date(2024, 12, 31), 1) == date_create_date(2025, 1, 1)"), Value::Real(1.0));
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_get_hour(date_inc_hour(date_create_date(2024, 1, 1), 5))"
        ),
        Value::Real(5.0)
    );
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_day_span(date_create_date(2024, 1, 1), date_create_date(2024, 1, 11))"
        ),
        Value::Real(10.0)
    );
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_hour_span(date_create_date(2024, 1, 2), date_create_date(2024, 1, 1))"
        ),
        Value::Real(24.0)
    );
    assert_eq!(r(&mut vm, &mut h, "return date_compare_date(date_current_datetime(), date_current_date()) + date_compare_datetime(1, 2) * 10 + date_compare_time(0.5, 1.25) * 100"), Value::Real(-10.0 + 100.0));
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_is_today(date_create_date(2024, 3, 15)) + date_is_today(1)"
        ),
        Value::Real(1.0)
    );
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_datetime_string(date_current_datetime())"
        ),
        Value::from("2024-03-15 13:45:30")
    );
    assert_eq!(r(&mut vm, &mut h, "return date_date_string(date_current_datetime()) + ' ' + date_time_string(date_current_datetime())"), Value::from("2024-03-15 13:45:30"));
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return date_time_of(date_create_datetime(2024, 1, 1, 12, 0, 0))"
        ),
        Value::Real(0.5)
    );
    // real clock works and is plausible (after 2020)
    vm.set_clock(Box::new(|| 0.0));
    let mut vm2 = Vm::new();
    let v = run_in(
        &mut vm2,
        &mut h,
        "return date_get_year(date_current_datetime())",
    )
    .unwrap();
    assert!(v.real_or_zero() >= 2020.0);
}

#[test]
fn ds_stacks_queues() {
    assert_eq!(
        string("s = ds_stack_create() ds_stack_push(s, 1) ds_stack_push(s, 'two') r = ds_stack_top(s) + string(ds_stack_size(s)) ds_stack_pop(s) r += string(ds_stack_pop(s)) + string(ds_stack_empty(s)) + string(ds_stack_pop(s)) return r"),
        "two2110"
    );
    assert_eq!(
        string("q = ds_queue_create() ds_queue_enqueue(q, 'a') ds_queue_enqueue(q, 'b') ds_queue_enqueue(q, 'c') r = ds_queue_head(q) + ds_queue_tail(q) + ds_queue_dequeue(q) + ds_queue_dequeue(q) + string(ds_queue_size(q)) return r"),
        "acab1"
    );
    // ids are per kind, reused after destroy
    assert_eq!(real("a = ds_stack_create() b = ds_stack_create() ds_stack_destroy(a) c = ds_stack_create() q = ds_queue_create() return a * 1000 + b * 100 + c * 10 + q"), 100.0);
    assert!(err("ds_stack_push(7, 1)")
        .message
        .contains("does not exist"));
    assert!(
        err("s = ds_stack_create() ds_stack_destroy(s) ds_stack_push(s, 1)")
            .message
            .contains("does not exist")
    );
}

#[test]
fn ds_stack_roundtrip() {
    assert_eq!(
        string("s = ds_stack_create() ds_stack_push(s, 1.5) ds_stack_push(s, 'x\"y') t = ds_stack_create() ds_stack_copy(t, s) ds_stack_clear(s) u = ds_stack_create() ds_stack_read(u, ds_stack_write(t)) return ds_stack_pop(u) + string(ds_stack_pop(u)) + string(ds_stack_size(s))"),
        "x\"y1.500"
    );
    assert!(err("s = ds_stack_create() ds_stack_read(s, 'garbage')")
        .message
        .contains("Invalid"));
    assert!(
        err("s = ds_stack_create() l = ds_list_create() ds_stack_read(s, ds_list_write(l))")
            .message
            .contains("Invalid")
    );
}

#[test]
fn ds_lists() {
    assert_eq!(
        string("l = ds_list_create() ds_list_add(l, 3) ds_list_add(l, 'b') ds_list_add(l, 1) ds_list_insert(l, 0, 'a') ds_list_replace(l, 3, 2) ds_list_delete(l, 1)
            r = '' for (i = 0; i < ds_list_size(l); i += 1) r += string(ds_list_find_value(l, i)) + ','
            r += string(ds_list_find_index(l, 'b')) + string(ds_list_find_index(l, 'zz')) + string(ds_list_find_value(l, 99))
            return r"),
        "a,b,2,1-10"
    );
    assert_eq!(
        string("l = ds_list_create() ds_list_add(l, 'b') ds_list_add(l, 3) ds_list_add(l, 'a') ds_list_add(l, 1) ds_list_sort(l, true)
            r = '' for (i = 0; i < 4; i += 1) r += string(ds_list_find_value(l, i))
            ds_list_sort(l, false) for (i = 0; i < 4; i += 1) r += string(ds_list_find_value(l, i)) return r"),
        "13abba31"
    );
    // precision
    assert_eq!(
        real("l = ds_list_create() ds_list_add(l, 0.1 + 0.2) return ds_list_find_index(l, 0.3)"),
        0.0
    );
    assert_eq!(real("ds_set_precision(0.5) l = ds_list_create() ds_list_add(l, 1) return ds_list_find_index(l, 1.4)"), 0.0);
    // shuffle keeps elements; write/read roundtrip
    assert_eq!(
        real("l = ds_list_create() for (i = 0; i < 50; i += 1) ds_list_add(l, i) ds_list_shuffle(l) s = 0 for (i = 0; i < 50; i += 1) s += ds_list_find_value(l, i)
            m = ds_list_create() ds_list_read(m, ds_list_write(l)) t = 0 for (i = 0; i < 50; i += 1) t += ds_list_find_value(m, i) * (ds_list_find_value(m, i) == ds_list_find_value(l, i))
            return s * 10000 + t"),
        1225.0 * 10000.0 + 1225.0
    );
    assert_eq!(real("l = ds_list_create() e = ds_list_empty(l) ds_list_add(l, 1) m = ds_list_create() ds_list_copy(m, l) ds_list_clear(l) return e * 100 + ds_list_size(m) * 10 + ds_list_size(l)"), 110.0);
}

#[test]
fn ds_maps() {
    assert_eq!(
        string("m = ds_map_create() ds_map_add(m, 'b', 2) ds_map_add(m, 'a', 1) ds_map_add(m, 5, 'five') ds_map_add(m, 'c', 3)
            r = string(ds_map_find_value(m, 'a')) + ds_map_find_value(m, 5) + string(ds_map_exists(m, 'c')) + string(ds_map_exists(m, 'zz'))
            ds_map_replace(m, 'a', 10) ds_map_delete(m, 'c')
            r += '|' + string(ds_map_find_value(m, 'a')) + string(ds_map_size(m)) + '|'
            k = ds_map_find_first(m) repeat ds_map_size(m) { r += string(k) + ';' k = ds_map_find_next(m, k) }
            r += '|' + ds_map_find_last(m) + ds_map_find_previous(m, 'b') + string(ds_map_find_previous(m, 'a')) + string(ds_map_find_value(m, 'nokey'))
            return r"),
        "1five10|103|5;a;b;|ba50"
    );
    // duplicates allowed; delete removes one
    assert_eq!(real("m = ds_map_create() ds_map_add(m, 1, 'x') ds_map_add(m, 1, 'y') ds_map_delete(m, 1) return ds_map_size(m) * 10 + ds_map_exists(m, 1)"), 11.0);
    assert_eq!(
        string("m = ds_map_create() ds_map_add(m, 'k', 'v') ds_map_add(m, 2, 3) n = ds_map_create() ds_map_read(n, ds_map_write(m)) return ds_map_find_value(n, 'k') + string(ds_map_find_value(n, 2))"),
        "v3"
    );
}

#[test]
fn ds_priority_queues() {
    assert_eq!(
        string("p = ds_priority_create() ds_priority_add(p, 'low', 1) ds_priority_add(p, 'high', 10) ds_priority_add(p, 'mid', 5) ds_priority_add(p, 'low2', 1)
            r = ds_priority_find_min(p) + ds_priority_find_max(p) + string(ds_priority_find_priority(p, 'mid'))
            ds_priority_change_priority(p, 'mid', 20) r += ds_priority_delete_max(p) + ds_priority_delete_min(p) + ds_priority_delete_min(p)
            ds_priority_delete_value(p, 'high') r += string(ds_priority_size(p)) + string(ds_priority_empty(p)) + string(ds_priority_delete_min(p))
            return r"),
        "lowhigh5midlowlow2010"
    );
}

#[test]
fn ds_grids() {
    assert_eq!(
        string("g = ds_grid_create(4, 3) ds_grid_set(g, 1, 1, 5) ds_grid_add(g, 1, 1, 2) ds_grid_multiply(g, 1, 1, 3) ds_grid_set(g, 9, 9, 1)
            r = string(ds_grid_get(g, 1, 1)) + string(ds_grid_width(g)) + string(ds_grid_height(g)) + string(ds_grid_get(g, 7, 7))
            ds_grid_set(g, 0, 0, 'a') ds_grid_add(g, 0, 0, 'b') ds_grid_multiply(g, 0, 0, 2) r += ds_grid_get(g, 0, 0) return r"),
        "21430ab"
    );
    assert_eq!(
        real("g = ds_grid_create(5, 5) ds_grid_set_region(g, 3, 3, 1, 1, 2) ds_grid_add_region(g, 0, 0, 1, 1, 1) ds_grid_multiply_region(g, 2, 2, 2, 2, 10)
            return ds_grid_get_sum(g, 0, 0, 4, 4) * 1000000 + ds_grid_get_max(g, 0, 0, 4, 4) * 10000 + ds_grid_get_min(g, 1, 1, 3, 3) * 100 + ds_grid_get_mean(g, 0, 0, 1, 1)"),
        // region 1..3 = 2 (9 cells), (0,0),(1,0),(0,1) = 1, (1,1) = 3, (2,2) = 20 -> sum = 16 + 3 + 3 + 18 = 40? computed below
        {
            let mut g = [[0.0f64; 5]; 5];
            for y in 1..=3 { for x in 1..=3 { g[y][x] = 2.0; } }
            for y in 0..=1 { for x in 0..=1 { g[y][x] += 1.0; } }
            g[2][2] *= 10.0;
            let sum: f64 = g.iter().flatten().sum();
            let max = g.iter().flatten().cloned().fold(f64::MIN, f64::max);
            let min = (1..=3).flat_map(|y| (1..=3).map(move |x| (x, y))).map(|(x, y)| g[y][x]).fold(f64::MAX, f64::min);
            let mean = (g[0][0] + g[0][1] + g[1][0] + g[1][1]) / 4.0;
            sum * 1000000.0 + max * 10000.0 + min * 100.0 + mean
        }
    );
    // disks
    assert_eq!(real("g = ds_grid_create(7, 7) ds_grid_set_disk(g, 3, 3, 1, 1) return ds_grid_get_sum(g, 0, 0, 6, 6) + ds_grid_get_disk_sum(g, 3, 3, 0) * 100"), 105.0);
    assert_eq!(real("g = ds_grid_create(7, 7) ds_grid_add_disk(g, 3, 3, 2, 2) ds_grid_multiply_disk(g, 3, 3, 1, 3) return ds_grid_get_disk_max(g, 3, 3, 2) * 100 + ds_grid_get_disk_min(g, 3, 3, 2)"), 602.0);
    // value search
    assert_eq!(
        real("g = ds_grid_create(4, 4) ds_grid_set(g, 2, 3, 'x') ds_grid_set(g, 1, 1, 7)
            return ds_grid_value_exists(g, 0, 0, 3, 3, 'x') * 1000 + ds_grid_value_x(g, 0, 0, 3, 3, 'x') * 100 + ds_grid_value_y(g, 0, 0, 3, 3, 'x') * 10 + ds_grid_value_x(g, 0, 0, 3, 3, 'none')"),
        1000.0 + 200.0 + 30.0 - 1.0
    );
    assert_eq!(real("g = ds_grid_create(5, 5) ds_grid_set(g, 2, 2, 9) return ds_grid_value_disk_exists(g, 2, 2, 1, 9) + ds_grid_value_disk_x(g, 0, 0, 1, 9) * 10"), -9.0);
    // grid region copies (overlapping within one grid)
    assert_eq!(
        string("g = ds_grid_create(4, 1) for (i = 0; i < 4; i += 1) ds_grid_set(g, i, 0, i) ds_grid_set_grid_region(g, g, 0, 0, 2, 0, 1, 0)
            r = '' for (i = 0; i < 4; i += 1) r += string(ds_grid_get(g, i, 0)) return r"),
        "0012"
    );
    assert_eq!(
        real("a = ds_grid_create(2, 2) ds_grid_clear(a, 3) b = ds_grid_create(3, 3) ds_grid_clear(b, 1) ds_grid_add_grid_region(b, a, 0, 0, 1, 1, 1, 1) ds_grid_multiply_grid_region(b, a, 0, 0, 0, 0, 0, 0) return ds_grid_get_sum(b, 0, 0, 2, 2)"),
        3.0 + 4.0 + 4.0 * 4.0
    );
    // resize, copy, write/read, shuffle
    assert_eq!(
        real("g = ds_grid_create(2, 2) ds_grid_set(g, 1, 1, 5) ds_grid_resize(g, 3, 4) h = ds_grid_create(1, 1) ds_grid_copy(h, g) k = ds_grid_create(1, 1) ds_grid_read(k, ds_grid_write(h)) ds_grid_shuffle(k)
            return ds_grid_get(g, 1, 1) * 1000 + ds_grid_width(k) * 100 + ds_grid_height(k) * 10 + ds_grid_get_sum(k, 0, 0, 9, 9) - 5"),
        5000.0 + 300.0 + 40.0
    );
}
