//! Behaviours verified black-box against GM 8.1 (gm-blackbox bb_lang / bb_args0 /
//! bb_clash / bb_random keys named in each test).

mod common;
use common::*;
use gml::*;

fn scripts(check: bool) -> (Vm, Mock) {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    vm.settings.check_script_args = check;
    h.add(0);
    h.compile_scripts(
        &mut vm,
        &[
            ("sc_count", "return argument_count;"),
            ("sc_arg0", "return argument0;"),
            ("sc_arg2", "return argument2;"),
            ("sc_argarr5", "return argument[5];"),
            ("sc_argarr_i", "return argument[argument0];"),
            ("sc_noreturn", "global.q = 1;"),
            ("sc_exit", "global.q = 2; exit; global.q = 3;"),
            ("sc_call_last", "global.q = 1; abs(5);"),
            ("sc_if_last", "global.q = 3; if (global.q == 0) global.q = 4;"),
            ("sc_var_last", "var a; a = 6;"),
            ("sc_mixed", "return argument1 + argument[0];"),
        ],
    );
    (vm, h)
}

fn go(vm: &mut Vm, h: &mut Mock, src: &str) -> Result<Value, Error> {
    run_in(vm, h, src)
}

/// bb_lang args.* (settings flag bit 1 set): exact argument counts.
#[test]
fn script_argument_check_enabled() {
    let (mut vm, mut h) = scripts(true);
    let e = go(&mut vm, &mut h, "return sc_count(1, 2)").unwrap_err();
    assert_eq!(
        e.message,
        "Illegal argument count calling script \"sc_count\".\nScript requires 0 arguments, 2 have been supplied."
    );
    assert!(go(&mut vm, &mut h, "return sc_arg2(1, 2)").unwrap_err().message.contains("requires 3 arguments, 2"));
    assert!(go(&mut vm, &mut h, "return sc_arg0()").is_err());
    assert_eq!(go(&mut vm, &mut h, "return sc_count()").unwrap(), Value::Real(0.0));
    // argument[] users accept extra arguments, but reading past the count fails
    assert_eq!(go(&mut vm, &mut h, "return sc_argarr_i(1, 22)").unwrap(), Value::Real(22.0));
    assert_eq!(
        go(&mut vm, &mut h, "return sc_argarr5(1)").unwrap_err().message,
        "Unknown variable argument or array index out of bounds"
    );
    // script_execute is checked as well
    assert!(go(&mut vm, &mut h, "return script_execute(sc_arg0)").is_err());
}

/// bb_args0 (flag bit 1 cleared): any count, missing arguments read 0.
#[test]
fn script_argument_check_disabled() {
    let (mut vm, mut h) = scripts(false);
    assert_eq!(go(&mut vm, &mut h, "return sc_count(1, 2)").unwrap(), Value::Real(2.0));
    assert_eq!(go(&mut vm, &mut h, "return sc_arg2(1, 2)").unwrap(), Value::Real(0.0));
    assert_eq!(go(&mut vm, &mut h, "return sc_argarr5(1)").unwrap(), Value::Real(0.0));
}

/// bb_lang args.noreturn / args.exit: no `return` gives the last assigned value.
#[test]
fn implicit_script_result() {
    let (mut vm, mut h) = scripts(true);
    assert_eq!(go(&mut vm, &mut h, "return sc_noreturn()").unwrap(), Value::Real(1.0));
    assert_eq!(go(&mut vm, &mut h, "return sc_exit()").unwrap(), Value::Real(2.0));
    // bb_lang2 ret.*: a call statement's value counts as well; an if condition doesn't
    assert_eq!(go(&mut vm, &mut h, "return sc_call_last()").unwrap(), Value::Real(5.0));
    assert_eq!(go(&mut vm, &mut h, "return sc_if_last()").unwrap(), Value::Real(3.0));
    assert_eq!(go(&mut vm, &mut h, "return sc_var_last()").unwrap(), Value::Real(6.0));
}

/// bb_lang2 args.*: "at least" wording for argument[] users; script_execute with
/// too many arguments reports a non-aborting error and still runs the script.
#[test]
fn script_argument_check_lang2() {
    let (mut vm, mut h) = scripts(true);
    assert_eq!(
        go(&mut vm, &mut h, "return sc_mixed(5)").unwrap_err().message,
        "Illegal argument count calling script \"sc_mixed\".\nScript requires at least 2 arguments, 1 have been supplied."
    );
    assert_eq!(go(&mut vm, &mut h, "return sc_mixed(5, 6, 7)").unwrap(), Value::Real(11.0));
    h.soft_errors = Some(Vec::new());
    assert_eq!(go(&mut vm, &mut h, "return script_execute(sc_arg0, 1, 2)").unwrap(), Value::Real(1.0));
    assert_eq!(
        h.soft_errors.as_ref().unwrap().as_slice(),
        ["Illegal argument count calling script \"sc_arg0\".\nScript requires 1 arguments, 2 have been supplied."]
    );
    // too few through script_execute: reported, the script still runs (bb_lang3
    // args3.execute_few_runs "0/7"), missing arguments read 0
    assert_eq!(go(&mut vm, &mut h, "return script_execute(sc_arg0)").unwrap(), Value::Real(0.0));
    assert_eq!(h.soft_errors.as_ref().unwrap().len(), 2);
    assert_eq!(go(&mut vm, &mut h, "global.q = 0; script_execute(sc_noreturn); return global.q").unwrap(), Value::Real(1.0));
    // a later direct call with too many arguments is still an aborting error
    assert!(go(&mut vm, &mut h, "return sc_count(1)").is_err());
}

/// bb_lang args.arr_index_16 / code_action_arg0: argument[16] abandons the event and
/// the callee's arguments stay in the global argument array; code run with
/// `run_code` (code actions) sees them.
#[test]
fn argument_array_is_global() {
    let (mut vm, mut h) = scripts(false);
    let e = go(&mut vm, &mut h, "return sc_argarr_i(16)").unwrap_err();
    assert_eq!(e.kind, ErrorKind::AbortEvent);
    let p = vm.compile("act", b"return string(argument0) + '/' + string(argument_count)", &h).unwrap();
    let me = h.insts[0].id;
    assert_eq!(vm.run_code(&mut h, &p, Ctx::new(me, me)).unwrap(), Value::from("16/1"));
    // ordinary errors restore the caller's arguments
    let _ = go(&mut vm, &mut h, "return sc_argarr_i(-1)");
    assert_eq!(vm.run_code(&mut h, &p, Ctx::new(me, me)).unwrap(), Value::from("16/1"));
    // execute_string writes the slots but keeps argument_count (args.exec_string_args)
    h.soft_errors = Some(Vec::new());
    let p2 = vm
        .compile("act", b"return execute_string(\"return string(argument_count) + '/' + argument1;\", 'a', 'b')", &h)
        .unwrap();
    assert_eq!(vm.run_code(&mut h, &p2, Ctx::new(me, me)).unwrap(), Value::from("1/b"));
}

/// bb_lang math.*, real.*, type.readonly_id: function errors and errors inside
/// execute_string are reported but do not abandon the caller.
#[test]
fn soft_errors() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.soft_errors = Some(Vec::new());
    assert_eq!(run_in(&mut vm, &mut h, "a = ln(0) b = 7 return b + a").unwrap(), Value::Real(7.0));
    assert_eq!(run_in(&mut vm, &mut h, "return real('12abc')").unwrap(), Value::Real(0.0));
    assert_eq!(run_in(&mut vm, &mut h, "return sqrt(-1)").unwrap(), Value::Real(0.0));
    assert_eq!(run_in(&mut vm, &mut h, "execute_string('id = 5;') return 3").unwrap(), Value::Real(3.0));
    assert_eq!(run_in(&mut vm, &mut h, "execute_string('var x;') return 4").unwrap(), Value::Real(4.0));
    let log = h.soft_errors.clone().unwrap();
    assert_eq!(log[0], "Error in function ln().");
    assert_eq!(log[1], "Error in function real().");
    assert_eq!(log[2], "Cannot apply sqrt to negative number.");
    assert_eq!(log[3], "Cannot assign to the variable");
    assert!(log[4].starts_with("COMPILATION ERROR in string to be executed\nError in code at line 1:\n   var x;\n"));
    // operator errors still abandon the code
    assert!(run_in(&mut vm, &mut h, "a = 'a' b = a + 1 return 9").is_err());
}

/// bb_lang eq.*, bb_lang2 eq2.*, bb_lang3 eq3.*: equality is `|a - b| < 1e-13`,
/// absolute also for large values.
#[test]
fn equality_tolerance() {
    for (src, want) in [
        ("0 == power(10, -14)", 1.0),
        ("0 == 5 * power(10, -16)", 1.0),
        ("1 == 1 + power(10, -14)", 1.0),
        ("1 == 1 + 5 * power(10, -14)", 1.0),
        ("0 == power(10, -13)", 0.0),
        ("0 == 2 * power(10, -12)", 0.0),
        ("1 == 1 + 5 * power(10, -13)", 0.0),
        ("100 == 100 + 1.5 * power(10, -13)", 0.0),
        ("1000000 == 1000000 + 5 * power(10, -10)", 0.0),
        ("1000000 == 1000000 + 5 * power(10, -9)", 0.0),
        ("1000000 == 1000000 + power(10, -7)", 0.0),
        ("1000000 < 1000000 + power(10, -7)", 1.0),
        ("0.3 >= 0.1 + 0.2", 1.0),
        ("0.3 < 0.1 + 0.2", 0.0),
    ] {
        assert_eq!(real(&format!("return ({src})")), want, "{src}");
    }
}

/// bb_lang syn.*: lexer details.
#[test]
fn lexer_details() {
    assert_eq!(real("return 1.2.3"), 1.23);
    assert!(compile_err("a = 7 % 3").message.contains("Unexpected symbol"));
    // fraction digits are accumulated as d * p, p /= 10: 0.3 is 0.1 + 0.2's double
    assert_eq!(real("return 0.3"), 0.30000000000000004);
    assert_eq!(real("return 1.23456"), 1.2345599999999999);
    assert_eq!(real("return .5 + 12."), 12.5);
}

/// bb_lang eq.*, bit.*, idx.*: number semantics.
#[test]
fn numbers() {
    assert_eq!(real("return 0.1 + 0.2 == 0.3"), 1.0);
    assert_eq!(real("a = 0 return sin(pi) == a"), 1.0);
    assert_eq!(real("return 1 == 1.0000000001"), 0.0);
    assert_eq!(real("return 100000000 == 100000000.5"), 0.0);
    assert_eq!(real("return (1 << 40) >> 8"), 4294967296.0);
    assert_eq!(real("a = -1 return a >> 1"), 9223372036854775807.0);
    assert_eq!(real("a = -1 return 1 << a"), 2147483648.0);
    assert_eq!(real("return ~1.5"), -3.0);
    assert_eq!(real("return 3.5 & 6.5"), 4.0);
    assert_eq!(real("return floor(4294967295.5)"), -1.0); // bb_random: 32-bit wrap
    assert_eq!(real("return floor(-0.5)"), -1.0);
    assert_eq!(string("return string(4294967296.5)"), "4294967297.00");
    assert_eq!(string("return string(power(10, 20))"), "ERROR");
    assert_eq!(string("return string_format(1.23456, 0, 20)"), "1.234559999999999840");
    assert_eq!(string("return string_format('a', 0, 1)"), "a");
    assert_eq!(err("z = 0 return 5 mod z").message, "Modulo by 0.");
}

/// bb_lang truth.str_*, mix.*: type errors.
#[test]
fn type_errors() {
    assert_eq!(err("a = 'a' if (a) return 1").message, "Expression expected");
    assert_eq!(err("a = 'a' return a == 1").message, "Cannot compare arguments.");
    assert_eq!(err("a = 'a' return a | 1").message, "Wrong type of arguments for |.");
    assert_eq!(err("a = 'a' return -a").message, "Wrong type of arguments to unary operator.");
    // switch compares without an error (syn.switch_mixed)
    assert_eq!(string("switch (1) { case '1': return 'str' case 1: return 'real' }"), "real");
}

/// bb_lang sf.*, real.*: library edge cases.
#[test]
fn library_edges() {
    assert_eq!(string("return string_delete('abc', 0, 1)"), "abc");
    assert_eq!(string("return string_copy('abcdef', 0, 2)"), "ab");
    assert_eq!(string("return string_upper(5)"), "");
    assert_eq!(real("return real(' 12') + real('12 ') + real('-.5') + real('') + real('.')"), 23.5);
    assert!(err("return real('1,5')").message.contains("real()"));
    assert_eq!(string("return max('a', 1)"), "a");
    assert_eq!(real("return choose() + mean() + median()"), 0.0);
}

/// bb_clash: scripts beat built-in functions, user constants beat built-in constants.
#[test]
fn name_resolution() {
    let (mut vm, mut h) = (Vm::new(), Mock::new());
    h.add(0);
    h.compile_scripts(&mut vm, &[("abs", "return 42;")]);
    h.constants.insert("c_red".into(), Value::Real(5.0));
    assert_eq!(run_in(&mut vm, &mut h, "return abs() + c_red").unwrap(), Value::Real(47.0));
}
