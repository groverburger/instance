//! Language semantics tests (spec gml-language.md) using the mock host.

mod common;
use common::*;
use gml::*;

#[test]
fn arithmetic_and_precedence() {
    assert_eq!(real("return 1 + 2 * 3"), 7.0);
    assert_eq!(real("return (1 + 2) * 3"), 9.0);
    assert_eq!(real("return 7.5 div 2"), 3.0);
    assert_eq!(real("return 7.5 mod 2"), 1.5);
    assert_eq!(real("return -7 div 2"), -3.0);
    assert_eq!(real("return -7 mod 2"), -1.0);
    assert_eq!(real("return 7 % 4"), 3.0);
    assert_eq!(real("return 2 & 3 == 2"), 1.0); // (2 & 3) == 2
    assert_eq!(real("return 1 || 0 && 0"), 0.0); // (1 || 0) && 0
    assert_eq!(real("return 1 < 2 < 3"), 1.0);
    assert_eq!(real("return 3 > 2 > 1"), 0.0); // (3>2)=1 > 1 = 0
    assert_eq!(real("return 1 << 3 + 1"), 16.0);
    assert_eq!(real("return ~0"), -1.0);
    assert_eq!(real("return - - 5"), 5.0);
    assert_eq!(real("a = 3 return --a"), 3.0);
    assert_eq!(real("return !0"), 1.0);
    assert_eq!(real("return not 1"), 0.0);
    assert_eq!(real("return 1 xor 1"), 0.0);
    assert_eq!(real("return 1 ^^ 0"), 1.0);
    assert_eq!(real("return 5 ^ 3"), 6.0);
    assert_eq!(real("return 5 | 2"), 7.0);
    assert_eq!(real("return $FF"), 255.0);
    assert_eq!(real("return .5 + 12."), 12.5);
    assert_eq!(
        real("return 0.5 == 0.5 and 2 >= 2 and 2 <= 2 and 1 != 2 and 1 <> 2"),
        1.0
    );
    assert_eq!(real("return 10 / 4"), 2.5);
    assert_eq!(real("return true + false + pi - pi"), 1.0);
}

#[test]
fn equals_has_dual_meaning() {
    assert_eq!(real("b = 2 c = 2 a = b = c return a"), 1.0);
    assert_eq!(real("x2 = 3 if (x2 = 3) return 10 return 20"), 10.0);
    assert_eq!(real("a := 4 return a := 4"), 1.0);
}

#[test]
fn strings() {
    assert_eq!(string("return 'ab' + \"cd\""), "abcd");
    assert_eq!(string("a = 'x' a += 'y' return a"), "xy");
    assert_eq!(real("return 'abc' < 'abd'"), 1.0);
    assert_eq!(real("return 'ab' < 'abc'"), 1.0);
    assert_eq!(real("return 'B' < 'a'"), 1.0);
    assert_eq!(real("return 'a' == 'a'"), 1.0);
    assert_eq!(real("return 'a' == 'A'"), 0.0);
    // mixed comparisons are false / not-equal, no error
    assert_eq!(real("return '1' == 1"), 0.0);
    assert_eq!(real("return '1' != 1"), 1.0);
    assert_eq!(real("return '1' < 1 or '1' > 1"), 0.0);
    // no escapes, multi-line
    assert_eq!(string(r#"return "a\nb""#), "a\\nb");
    assert_eq!(string("return 'line1\nline2'"), "line1\nline2");
    assert_eq!(string("return '\"' + \"'\""), "\"'");
}

#[test]
fn type_errors() {
    assert!(err("return 'a' + 1").message.contains("Wrong type"));
    assert!(err("return 'a' - 'b'").message.contains("Wrong type"));
    assert!(err("return -'a'").message.contains("Wrong type"));
    assert!(err("a = 1 a += 'x'").message.contains("Wrong type"));
    assert!(err("return 'a' * 2").message.contains("Wrong type"));
    assert_eq!(err("return 1 / 0").message, "Division by 0.");
    assert_eq!(err("return 1 div 0").message, "Division by 0.");
    assert_eq!(err("return 1 mod 0").message, "Division by 0.");
    assert_eq!(real("return !'a'"), 1.0);
    assert_eq!(real("return 'a' && 1"), 0.0);
}

#[test]
fn truthiness() {
    assert_eq!(real("if 0.5 return 1 return 0"), 1.0);
    assert_eq!(real("if 0.4999 return 1 return 0"), 0.0);
    assert_eq!(real("if -1 return 1 return 0"), 0.0);
    assert_eq!(real("if 'yes' return 1 return 0"), 0.0);
}

#[test]
fn no_short_circuit() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.compile_scripts(&mut vm, &[("side", "global.calls += 1 return argument0")]);
    let r = run_in(
        &mut vm,
        &mut h,
        "global.calls = 0 a = side(0) && side(1) b = side(1) || side(0) return global.calls",
    )
    .unwrap();
    assert_eq!(r, Value::Real(4.0));
}

#[test]
fn optional_syntax() {
    assert_eq!(real("a = 1 b = 2 return a + b"), 3.0);
    assert_eq!(real("{ a = 1; b = 2; }; ; return a + b;"), 3.0);
    assert_eq!(real("begin a = 5 end return a"), 5.0);
    assert_eq!(real("a = 1 if a = 1 then b = 2 else b = 3 return b"), 2.0);
    assert_eq!(real("a = 0 if a b = 2; else b = 3; return b"), 3.0);
    assert_eq!(real("a = 1 if (a > 0) {b = 1} else {b = 2} return b"), 1.0);
    // dangling else
    assert_eq!(real("b = 0 if 1 if 0 b = 1 else b = 2 return b"), 2.0);
    assert_eq!(real("b = 0 if 0 if 1 b = 1 else b = 2 return b"), 0.0);
}

#[test]
fn loops() {
    assert_eq!(real("n = 0 repeat 5 n += 1 return n"), 5.0);
    assert_eq!(real("n = 0 repeat (2.5) n += 1 return n"), 2.0); // ties to even
    assert_eq!(real("n = 0 repeat (3.5) n += 1 return n"), 4.0);
    assert_eq!(real("n = 0 repeat -3 n += 1 return n"), 0.0);
    assert_eq!(
        real("n = 0 k = 3 repeat k { n += 1 k = 100 } return n"),
        3.0
    ); // evaluated once
    assert_eq!(real("n = 0 while n < 10 n += 1 return n"), 10.0);
    assert_eq!(real("n = 0 do n += 1 until n >= 5 return n"), 5.0);
    assert_eq!(real("n = 10 do n += 1 until true return n"), 11.0);
    assert_eq!(
        real("s = 0 for (i = 0; i < 5; i += 1) s += i return s"),
        10.0
    );
    assert_eq!(real("s = 0 for (i = 0 i < 5 i += 1) s += i return s"), 10.0);
    assert_eq!(
        real("s = 0 for (i = 0; i < 10; i += 1) { if i == 3 break s += 1 } return s * 100 + i"),
        303.0
    );
    assert_eq!(
        real("s = 0 for (i = 0; i < 5; i += 1) { if i mod 2 continue s += i } return s"),
        6.0
    );
    assert_eq!(
        real("s = 0 i = 0 while 1 { i += 1 if i > 4 break if i == 2 continue s += i } return s"),
        8.0
    );
    assert_eq!(
        real("s = 0 i = 0 do { i += 1 if i == 2 continue s += i } until i >= 4 return s"),
        8.0
    );
    assert_eq!(
        real("s = 0 repeat 10 { s += 1 if s == 3 break } return s"),
        3.0
    );
    assert_eq!(
        real("s = 0 repeat 4 { s += 1 continue s += 100 } return s"),
        4.0
    );
    assert!(err("repeat 'a' {}").message.contains("number"));
}

#[test]
fn switch_statement() {
    let prog = |v: &str| {
        format!(
            "v = {} r = '' switch (v) {{ case 1: r += 'one' case 2: r += 'two' break case 'a': r = 'A' break default: r += 'def' case 9: r += 'nine' }} return r",
            v
        )
    };
    assert_eq!(string(&prog("1")), "onetwo");
    assert_eq!(string(&prog("2")), "two");
    assert_eq!(string(&prog("'a'")), "A");
    assert_eq!(string(&prog("9")), "nine");
    assert_eq!(string(&prog("5")), "defnine");
    assert_eq!(string(&prog("'1'")), "defnine"); // mixed types never match
    assert_eq!(real("r = 0 switch 3 { case 1: r = 1 } return r"), 0.0);
    // case expressions evaluated at run time
    assert_eq!(
        real("a = 4 r = 0 switch 8 { case a * 2: r = 1 break case 8: r = 2 } return r"),
        1.0
    );
    // continue in a switch inside a loop applies to the loop
    assert_eq!(real("s = 0 for (i = 0; i < 4; i += 1) { switch i { case 1: continue default: s += 1 } s += 10 } return s"), 33.0);
    // break inside switch leaves only the switch
    assert_eq!(
        real("s = 0 repeat 3 { switch 1 { case 1: break } s += 1 } return s"),
        3.0
    );
}

#[test]
fn exit_and_break_at_top_level() {
    assert_eq!(real("a = 1 exit a = 2"), 0.0);
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    run_in(&mut vm, &mut h, "a = 1 break a = 2").unwrap();
    let id = h.insts[0].id;
    assert_eq!(h.var(&mut vm, id, "a"), Some(Value::Real(1.0)));
    run_in(&mut vm, &mut h, "b = 1 continue b = 2").unwrap();
    assert_eq!(h.var(&mut vm, id, "b"), Some(Value::Real(1.0)));
}

#[test]
fn instance_variables_and_uninitialised() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    run_in(&mut vm, &mut h, "score_x = 10").unwrap();
    assert_eq!(h.var(&mut vm, 100000, "score_x"), Some(Value::Real(10.0)));
    let e = run_in(&mut vm, &mut h, "a = 1\nb = undefined_thing + 1").unwrap_err();
    assert_eq!(e.message, "Unknown variable undefined_thing");
    let loc = e.location.unwrap();
    assert_eq!((loc.line, loc.col), (2, 5));
    assert_eq!(&*loc.program, "test");
    vm.settings.uninitialized_as_zero = true;
    assert_eq!(
        run_in(&mut vm, &mut h, "return undefined_thing + 1").unwrap(),
        Value::Real(1.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "zz += 5 return zz").unwrap(),
        Value::Real(5.0)
    );
    // identifiers are case-sensitive
    vm.settings.uninitialized_as_zero = false;
    run_in(&mut vm, &mut h, "Foo = 1").unwrap();
    assert!(run_in(&mut vm, &mut h, "return foo").is_err());
}

#[test]
fn var_locals() {
    assert_eq!(real("var a; a = 5; return a"), 5.0);
    assert!(err("var a; return a")
        .message
        .contains("Unknown variable a"));
    // locals do not persist on the instance
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    run_in(&mut vm, &mut h, "var t; t = 3; u = t").unwrap();
    assert_eq!(h.var(&mut vm, 100000, "t"), None);
    assert_eq!(h.var(&mut vm, 100000, "u"), Some(Value::Real(3.0)));
    // locals are fresh each run
    assert!(run_in(&mut vm, &mut h, "var t; return t").is_err());
    // var shadows built-in variables
    assert_eq!(string("var x; x = 'local'; return x"), "local");
    // lexical scope: uses before the declaration refer to the instance variable
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    run_in(&mut vm, &mut h, "q = 1; var q; q = 2;").unwrap();
    assert_eq!(h.var(&mut vm, 100000, "q"), Some(Value::Real(1.0)));
    // declaration effect is lexical, even when not executed
    assert_eq!(real("if 0 { var w; } w = 4; return w"), 4.0);
    // local arrays
    assert_eq!(
        real("var a; a[3] = 2; a[1, 2] = 5; return a[3] + a[1, 2] + a[0]"),
        7.0
    );
}

#[test]
fn globals() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    run_in(&mut vm, &mut h, "global.g = 5").unwrap();
    assert_eq!(
        run_in(&mut vm, &mut h, "return global.g").unwrap(),
        Value::Real(5.0)
    );
    // plain g is still an instance variable
    assert!(run_in(&mut vm, &mut h, "return g").is_err());
    run_in(&mut vm, &mut h, "globalvar g; g += 1").unwrap();
    assert_eq!(
        run_in(&mut vm, &mut h, "return g").unwrap(),
        Value::Real(6.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "g = 10 return global.g").unwrap(),
        Value::Real(10.0)
    );
    // self.g still names the instance variable
    run_in(&mut vm, &mut h, "self.g = 99").unwrap();
    assert_eq!(h.var(&mut vm, 100000, "g"), Some(Value::Real(99.0)));
    assert_eq!(
        run_in(&mut vm, &mut h, "return g").unwrap(),
        Value::Real(10.0)
    );
    // global is a value (-5) usable as an owner
    assert_eq!(
        run_in(&mut vm, &mut h, "o = global return o.g + global").unwrap(),
        Value::Real(5.0)
    );
    // globalvar names in other "programs" (fresh compile)
    assert_eq!(
        run_in(&mut vm, &mut h, "with (self) return g").unwrap(),
        Value::Real(10.0)
    );
    assert!(vm.is_globalvar(vm.lookup_var("g").unwrap()));
}

#[test]
fn arrays() {
    assert_eq!(real("a[0] = 1 a[5] = 6 return a[5] + a[0] + a[3]"), 7.0);
    assert_eq!(real("a = 5 return a[0] + a[0, 0]"), 10.0);
    assert_eq!(real("a[0] = 7 return a"), 7.0);
    assert_eq!(real("a[3] = 7 return a[0, 3]"), 7.0);
    assert_eq!(real("b[4, 6] = 1 return b[4, 0] + b[4, 5]"), 0.0);
    assert!(err("b[4, 6] = 1 return b[4, 7]")
        .message
        .contains("Unknown variable b[4,7]"));
    assert!(err("b[4, 6] = 1 return b[3, 0]")
        .message
        .contains("Unknown variable"));
    assert!(err("a[2] = 1 return a[3]")
        .message
        .contains("Unknown variable a[3]"));
    assert!(err("a[-1] = 1").message.contains("Negative"));
    assert!(err("a[32000] = 1").message.contains("32000"));
    assert!(err("a['x'] = 1").message.contains("number"));
    assert_eq!(real("a[31999] = 1 return a[31999]"), 1.0);
    assert_eq!(real("a[1.5] = 3 return a[2]"), 3.0); // index rounded (ties to even)
    assert_eq!(string("a[0] = 1 a[1] = 'x' return a[1]"), "x");
    // arrays are not values: copying copies element 0
    assert_eq!(real("a[0] = 1 a[1] = 2 c = a return c"), 1.0);
    assert!(err("a[0] = 1 a[1] = 2 c = a return c[1]")
        .message
        .contains("Unknown variable c[1]"));
    // compound assignment on elements
    assert_eq!(real("a[2] = 1 a[2] += 4 a[2] *= 2 return a[2]"), 10.0);
    // global arrays
    assert_eq!(real("global.t[2] = 3 return global.t[2]"), 3.0);
    assert!(err("for (i = 0; i < 40; i += 1) a[i, 31999] = 1")
        .message
        .contains("too large"));
}

#[test]
fn with_statement() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    let me = h.add(0);
    let b1 = h.add(1);
    let b2 = h.add(1);
    let c1 = h.add(2); // child of obj_b
    run_in(&mut vm, &mut h, "with (obj_b) hit = other.id").unwrap();
    for id in [b1, b2, c1] {
        assert_eq!(h.var(&mut vm, id, "hit"), Some(Value::Real(me as f64)));
    }
    assert_eq!(h.var(&mut vm, me, "hit"), None);
    // count iterations, self restored afterwards
    assert_eq!(
        run_in(
            &mut vm,
            &mut h,
            "n = 0 with (all) other.n += 1 return n + (id == 100000) * 100"
        )
        .unwrap(),
        Value::Real(104.0)
    );
    // break / continue
    assert_eq!(
        run_in(
            &mut vm,
            &mut h,
            "n = 0 with (obj_b) { other.n += 1 break } return n"
        )
        .unwrap(),
        Value::Real(1.0)
    );
    assert_eq!(
        run_in(
            &mut vm,
            &mut h,
            "n = 0 with (obj_b) { continue other.n += 1 } return n"
        )
        .unwrap(),
        Value::Real(0.0)
    );
    // instance id target, noone, self, other
    run_in(&mut vm, &mut h, &format!("with ({}) tag = 'c1'", c1)).unwrap();
    assert_eq!(h.var(&mut vm, c1, "tag"), Some(Value::from("c1")));
    assert_eq!(
        run_in(&mut vm, &mut h, "n = 0 with (noone) other.n += 1 return n").unwrap(),
        Value::Real(0.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "n = 0 with (self) n += 1 return n").unwrap(),
        Value::Real(1.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "n = 0 with (123456) other.n += 1 return n").unwrap(),
        Value::Real(0.0)
    );
    // nested with: other is the outer target
    run_in(
        &mut vm,
        &mut h,
        &format!("with ({}) with ({}) partner = other.id", b1, b2),
    )
    .unwrap();
    assert_eq!(h.var(&mut vm, b2, "partner"), Some(Value::Real(b1 as f64)));
    // var locals visible inside with
    run_in(&mut vm, &mut h, "var l; l = 42; with (obj_child) got = l").unwrap();
    assert_eq!(h.var(&mut vm, c1, "got"), Some(Value::Real(42.0)));
    // exit / return inside with leave the program
    assert_eq!(
        run_in(
            &mut vm,
            &mut h,
            "n = 0 with (obj_b) { other.n += 1 exit } n = 50 return n"
        )
        .unwrap(),
        Value::Real(0.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "with (obj_b) return id return -1").unwrap(),
        Value::Real(b1 as f64)
    );
    // destroyed instances are skipped
    let r = run_in(&mut vm, &mut h, "n = 0 with (obj_b) { other.n += 1 with (obj_b) if id != other.id instance_destroy() } return n").unwrap();
    assert_eq!(r, Value::Real(1.0));
}

#[test]
fn dot_access() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    let me = h.add(0);
    let b1 = h.add(1);
    let b2 = h.add(1);
    // write via object: all instances; read: first
    run_in(&mut vm, &mut h, "obj_b.v = 3").unwrap();
    assert_eq!(h.var(&mut vm, b2, "v"), Some(Value::Real(3.0)));
    run_in(&mut vm, &mut h, &format!("({}).v = 10", b2)).unwrap();
    assert_eq!(
        run_in(&mut vm, &mut h, "return obj_b.v").unwrap(),
        Value::Real(3.0)
    );
    // compound: read from first, write to all
    run_in(&mut vm, &mut h, "obj_b.v += 8").unwrap();
    assert_eq!(h.var(&mut vm, b1, "v"), Some(Value::Real(11.0)));
    assert_eq!(h.var(&mut vm, b2, "v"), Some(Value::Real(11.0)));
    // built-in instance variables through owners
    run_in(&mut vm, &mut h, "obj_b.x = 5").unwrap();
    assert_eq!(
        run_in(&mut vm, &mut h, &format!("return ({}).x", b2)).unwrap(),
        Value::Real(5.0)
    );
    // chains: a.b.c, array element owner, id
    run_in(
        &mut vm,
        &mut h,
        &format!("refs[1] = {} ; refs[1].w = 7", b2),
    )
    .unwrap();
    assert_eq!(h.var(&mut vm, b2, "w"), Some(Value::Real(7.0)));
    run_in(&mut vm, &mut h, &format!("link = {}", b1)).unwrap();
    run_in(&mut vm, &mut h, &format!("({}).link = {}", b1, b2)).unwrap();
    assert_eq!(
        run_in(&mut vm, &mut h, "return link.link.w").unwrap(),
        Value::Real(7.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "obj_b.id.z = 1 return obj_b.id").unwrap(),
        Value::Real(b1 as f64)
    );
    // alarm array via owner
    run_in(&mut vm, &mut h, "obj_b.alarm[4] = 12").unwrap();
    assert_eq!(
        run_in(&mut vm, &mut h, &format!("return ({}).alarm[4]", b2)).unwrap(),
        Value::Real(12.0)
    );
    // errors: noone, missing instance, string owner
    assert!(run_in(&mut vm, &mut h, "return noone.v").is_err());
    assert!(run_in(&mut vm, &mut h, "return (199999).v").is_err());
    assert!(run_in(&mut vm, &mut h, "return obj_child.v").is_err()); // no instances
    assert!(run_in(&mut vm, &mut h, "return ('a').v").is_err());
    // writes to nothing are no-ops
    run_in(
        &mut vm,
        &mut h,
        "noone.v = 1 (199999).v = 1 obj_child.v = 1",
    )
    .unwrap();
    // all
    run_in(&mut vm, &mut h, "all.flag = 1").unwrap();
    assert_eq!(h.var(&mut vm, me, "flag"), Some(Value::Real(1.0)));
    // other outside with = self (as set by the runner's ctx)
    assert_eq!(
        run_in(&mut vm, &mut h, "other.q = 2 return q").unwrap(),
        Value::Real(2.0)
    );
    // global built-in through an instance owner is the global value
    run_in(&mut vm, &mut h, "score = 40").unwrap();
    assert_eq!(
        run_in(&mut vm, &mut h, "return obj_b.score + global.score").unwrap(),
        Value::Real(80.0)
    );
    // number literal followed by dot-letter is a syntax error; parenthesised works
    assert!(vm.compile("t", b"a = 100032.speed", &h).is_err());
}

#[test]
fn builtin_variables() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    assert_eq!(
        run_in(&mut vm, &mut h, "x = 4 y = x * 2 return y").unwrap(),
        Value::Real(8.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "alarm[3] = 5 return alarm[3] + alarm").unwrap(),
        Value::Real(5.0)
    );
    assert!(run_in(&mut vm, &mut h, "alarm[12] = 1")
        .unwrap_err()
        .message
        .contains("out of bounds"));
    assert!(run_in(&mut vm, &mut h, "x[1] = 1")
        .unwrap_err()
        .message
        .contains("out of bounds"));
    assert!(run_in(&mut vm, &mut h, "x = 'a'")
        .unwrap_err()
        .message
        .contains("Wrong type"));
    assert_eq!(
        run_in(&mut vm, &mut h, "view_xview[7] = 3 return view_xview[7]").unwrap(),
        Value::Real(3.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "return id").unwrap(),
        Value::Real(100000.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "return instance_count").unwrap(),
        Value::Real(1.0)
    );
    // read-only built-ins are compile errors
    for src in [
        "id = 3",
        "instance_count = 1",
        "other.id = 4",
        "argument_count = 2",
        "argument_relative = 1",
        "fps += 1",
    ] {
        let e = vm.compile("t", src.as_bytes(), &h).unwrap_err();
        assert!(e.message.contains("read-only"), "{}: {}", src, e);
    }
}

/// Runs `f` on a thread with a large stack (deep recursion in debug builds).
fn big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn scripts_and_arguments() {
    big_stack(scripts_and_arguments_body);
}

fn scripts_and_arguments_body() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.compile_scripts(
        &mut vm,
        &[
            ("add", "return argument0 + argument1"),
            ("count", "return argument_count"),
            ("idx", "var i; i = argument0; return argument[i]"),
            (
                "fact",
                "if argument0 <= 1 return 1 return argument0 * fact(argument0 - 1)",
            ),
            ("noreturn", "a = 1"),
            ("early", "exit; return 5"),
            (
                "modify",
                "argument0 = 100; argument[1] = 200; return argument0 + argument1",
            ),
            ("who", "return id"),
            ("unpassed", "return argument5"),
            ("recurse", "return recurse()"),
            ("loc", "var v; v = 1; return v"),
            (
                "in_with",
                "with (obj_b) other.total += argument0; return total",
            ),
            ("relative", "return argument_relative"),
            ("bad_index", "return argument[16]"),
        ],
    );
    let r = |vm: &mut Vm, h: &mut Mock, s: &str| {
        run_in(vm, h, s).unwrap_or_else(|e| panic!("{}: {}", s, e))
    };
    assert_eq!(r(&mut vm, &mut h, "return add(2, 3)"), Value::Real(5.0));
    assert_eq!(
        r(&mut vm, &mut h, "return add('a', 'b')"),
        Value::from("ab")
    );
    assert_eq!(
        r(&mut vm, &mut h, "return count() + count(1, 2, 3) * 10"),
        Value::Real(30.0)
    );
    assert_eq!(
        r(&mut vm, &mut h, "return idx(2, 'x', 'y')"),
        Value::from("y")
    );
    assert_eq!(
        r(&mut vm, &mut h, "return fact(10)"),
        Value::Real(3628800.0)
    );
    assert_eq!(r(&mut vm, &mut h, "return noreturn()"), Value::Real(0.0));
    assert_eq!(r(&mut vm, &mut h, "return early()"), Value::Real(0.0));
    assert_eq!(
        r(&mut vm, &mut h, "a0 = 1 return modify(a0, 2) + a0"),
        Value::Real(301.0)
    );
    assert_eq!(r(&mut vm, &mut h, "return who()"), Value::Real(100000.0));
    assert_eq!(r(&mut vm, &mut h, "return unpassed(1)"), Value::Real(0.0));
    assert_eq!(
        r(&mut vm, &mut h, "return script_execute(add, 4, 5)"),
        Value::Real(9.0)
    );
    assert_eq!(
        r(&mut vm, &mut h, "s = fact return script_execute(s, 5)"),
        Value::Real(120.0)
    );
    assert_eq!(
        r(&mut vm, &mut h, "var v; v = 7; return loc() + v"),
        Value::Real(8.0)
    );
    assert_eq!(r(&mut vm, &mut h, "return relative()"), Value::Real(0.0));
    // a code action's arguments read as 0
    assert_eq!(
        r(
            &mut vm,
            &mut h,
            "return argument0 + argument_count + argument[3]"
        ),
        Value::Real(0.0)
    );
    // script sees caller's self; with inside a script keeps arguments
    h.add(1);
    h.add(1);
    assert_eq!(
        r(&mut vm, &mut h, "total = 0 return in_with(5)"),
        Value::Real(10.0)
    );
    // recursion limit
    let e = run_in(&mut vm, &mut h, "return recurse()").unwrap_err();
    assert!(e.message.contains("recursion"), "{}", e);
    assert!(run_in(&mut vm, &mut h, "return bad_index()").is_err());
    // unknown script index
    assert!(run_in(&mut vm, &mut h, "return script_execute(99)").is_err());
    // the error location is inside the script
    let e = run_in(&mut vm, &mut h, "return add(1, 'a')").unwrap_err();
    assert_eq!(&*e.location.unwrap().program, "add");
}

#[test]
fn script_called_with_args_from_host() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.add(0);
    h.compile_scripts(
        &mut vm,
        &[(
            "sum",
            "var i, s; s = 0; for (i = 0; i < argument_count; i += 1) s += argument[i]; return s",
        )],
    );
    let p = h.scripts[&0].clone();
    let args: Vec<Value> = (1..=16).map(|i| Value::Real(i as f64)).collect();
    let mut ctx = Ctx::new(100000, 100000);
    ctx.argument_relative = true;
    assert_eq!(vm.run(&mut h, &p, ctx, &args).unwrap(), Value::Real(136.0));
}

#[test]
fn execute_string_and_reentrancy() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    assert_eq!(
        run_in(
            &mut vm,
            &mut h,
            "return execute_string('return argument0 * 2', 21)"
        )
        .unwrap(),
        Value::Real(42.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "execute_string('zz = 5') return zz").unwrap(),
        Value::Real(5.0)
    );
    let e = run_in(&mut vm, &mut h, "execute_string('a = ')").unwrap_err();
    assert!(e.message.contains("line 1"), "{}", e);
    // the host runs GML from inside a function call (create event)
    h.compile_scripts(&mut vm, &[("create_event", "made_by = other.id; x += 1")]);
    let r = run_in(
        &mut vm,
        &mut h,
        "n = instance_create(10, 20, obj_b) return n.made_by * 1000 + n.x",
    )
    .unwrap();
    assert_eq!(r, Value::Real(100000.0 * 1000.0 + 11.0));
}

#[test]
fn local_owner() {
    assert_eq!(real("var a; a = 3; return local.a"), 3.0);
    assert_eq!(real("local.b = 4; return local.b"), 4.0);
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    run_in(&mut vm, &mut h, "local.b = 4").unwrap();
    assert_eq!(h.var(&mut vm, 100000, "b"), None);
    assert_eq!(real("var a; a = 5; o = local; return o.a"), 5.0);
}

#[test]
fn compile_errors() {
    let e = compile_err("a = 1\nb = nosuchfunc(1)");
    assert_eq!(e.message, "Unknown function or script: nosuchfunc");
    assert_eq!((e.line, e.col), (2, 5));
    assert!(compile_err("a = abs(1, 2)")
        .message
        .contains("Wrong number of arguments"));
    assert!(compile_err("a = abs()")
        .message
        .contains("Wrong number of arguments"));
    assert!(compile_err("a = choose()")
        .message
        .contains("Wrong number of arguments"));
    assert!(compile_err("a = abs").message.contains("without brackets"));
    assert!(compile_err("pi = 3").message.contains("constant"));
    assert!(compile_err("c_red = 3").message.contains("constant"));
    assert!(compile_err("obj_a = 3").message.contains("constant"));
    assert!(compile_err("self = 3").message.contains("constant"));
    assert!(compile_err("a = 1e5")
        .message
        .contains("Assignment operator expected"));
    assert!(compile_err("a = 1 +").message.contains("end"));
    assert!(compile_err("if").message.contains("end"));
    let args17 = format!("a = max({})", vec!["1"; 17].join(","));
    assert!(compile_err(&args17).message.contains("Too many arguments"));
    let e = compile_err("a = 'unterminated");
    assert_eq!((e.line, e.col), (1, 5));
    assert!(e.to_string().contains("line 1"));
}

#[test]
fn halt_propagates() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.add(1);
    let e = run_in(&mut vm, &mut h, "with (obj_b) game_end() never = 1").unwrap_err();
    assert_eq!(e.kind, ErrorKind::Halt);
    assert!(e.location.is_none());
}

#[test]
fn constants_and_resources() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.constants.insert("MAX_HP".into(), Value::Real(100.0));
    h.constants.insert("GREETING".into(), Value::from("hi"));
    assert_eq!(
        run_in(&mut vm, &mut h, "return MAX_HP + obj_b + c_red").unwrap(),
        Value::Real(100.0 + 1.0 + 255.0)
    );
    assert_eq!(
        run_in(&mut vm, &mut h, "return GREETING").unwrap(),
        Value::from("hi")
    );
    assert_eq!(
        run_in(
            &mut vm,
            &mut h,
            "return self + other + all + noone + global + local"
        )
        .unwrap(),
        Value::Real(-22.0)
    );
    // compile_expression
    let p = vm.compile_expression("arg", b"MAX_HP * 2", &h).unwrap();
    assert_eq!(
        vm.run(&mut h, &p, Ctx::new(special::NOONE, special::NOONE), &[])
            .unwrap(),
        Value::Real(200.0)
    );
    assert!(vm.compile_expression("arg", b"1 + 2 x = 3", &h).is_err());
}

#[test]
fn variables_by_name() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    let r = run_in(
        &mut vm,
        &mut h,
        "variable_local_set('hp', 5) variable_global_set('lvl', 2) variable_local_array_set('arr', 3, 9)
         variable_global_array2_set('g2', 1, 2, 'z')
         return string(hp) + string(global.lvl) + string(arr[3]) + global.g2[1, 2] + string(variable_local_get('hp'))
            + string(variable_local_exists('hp')) + string(variable_local_exists('nope')) + string(variable_global_exists('lvl'))
            + string(variable_global_array2_get('g2', 1, 2)) + string(variable_local_exists('x'))",
    )
    .unwrap();
    assert_eq!(r, Value::from("529z5101z1"));
    assert!(run_in(&mut vm, &mut h, "return variable_global_get('missing')").is_err());
    assert_eq!(
        run_in(&mut vm, &mut h, "variable_local_set('x', 12) return x").unwrap(),
        Value::Real(12.0)
    );
}

#[test]
fn performance_one_million_iterations() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.add(0);
    let src = "var i, s; s = 0; for (i = 0; i < 1000000; i += 1) { s += i mod 7; } total = s; n = 0; repeat 1000000 n += 1; return s";
    let prog = vm.compile("bench", src.as_bytes(), &h).unwrap();
    let t = std::time::Instant::now();
    let r = vm
        .run(&mut h, &prog, Ctx::new(100000, 100000), &[])
        .unwrap();
    let el = t.elapsed();
    let expected: f64 = (0..1_000_000u64).map(|i| (i % 7) as f64).sum();
    assert_eq!(r, Value::Real(expected));
    eprintln!("1e6-iteration loops took {:?}", el);
    if !cfg!(debug_assertions) {
        assert!(el.as_secs_f64() < 1.0, "too slow: {:?}", el);
    }
}
