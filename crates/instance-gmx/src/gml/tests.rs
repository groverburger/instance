use super::*;

fn tr(src: &str) -> String {
    let t = Translator::default().translate(src);
    assert!(t.error.is_none(), "{:?}", t.error);
    t.code
}

#[test]
fn unchanged_code_is_verbatim() {
    let src = "// comment\nif (a = 1) { b = 2 }  /* keep */\nelse c = 3;\nx = $FF;\n";
    let t = Translator::default().translate(src);
    assert!(!t.changed);
    assert_eq!(t.code, src);
}

#[test]
fn incdec_statements() {
    assert_eq!(tr("i++;"), "i += 1;");
    assert_eq!(tr("--i\n"), "i -= 1\n");
    assert_eq!(tr("other.hp--;"), "other.hp -= 1;");
    assert_eq!(tr("a[3]++"), "a[3] += 1");
    assert_eq!(
        tr("for (i = 0; i < 10; i++) { x += i; }"),
        "for (i = 0; i < 10; i += 1) { x += i; }"
    );
}

#[test]
fn incdec_in_expressions() {
    assert_eq!(tr("a = b++;"), "{ var __gmx_t0; __gmx_t0 = b; b += 1; a = __gmx_t0 };");
    assert_eq!(tr("a = ++b;"), "{ b += 1; a = b };");
    assert_eq!(
        tr("while (n-- > 0) foo();"),
        "while (true) { var __gmx_t0; __gmx_t0 = n; n -= 1; if (!((__gmx_t0 > 0))) break; foo() };"
    );
}

#[test]
fn ternary() {
    assert_eq!(
        tr("x = a > b ? a : b;"),
        "{ var __gmx_t0; if (a > b) { __gmx_t0 = a; } else { __gmx_t0 = b; } x = __gmx_t0 };"
    );
    // Nested inside a call, hoisting stays before the statement.
    assert_eq!(
        tr("if (c) draw_text(0, 0, t ? \"a\" : \"b\");"),
        "if (c) { var __gmx_t0; if (t) { __gmx_t0 = \"a\"; } else { __gmx_t0 = \"b\"; } draw_text(0, 0, __gmx_t0) };"
    );
}

#[test]
fn accessors() {
    assert_eq!(tr("v = l[| 2];"), "v = ds_list_find_value(l, 2);");
    assert_eq!(tr("l[| i] = 5;"), "ds_list_replace(l, i, 5);");
    assert_eq!(tr("l[| i] += 5;"), "ds_list_replace(l, i, ds_list_find_value(l, i) + (5));");
    assert_eq!(tr("v = m[? \"k\"];"), "v = ds_map_find_value(m, \"k\");");
    assert_eq!(tr("m[? \"k\"] = 1;"), "__gmx_ds_map_set(m, \"k\", 1);");
    assert_eq!(tr("v = g[# 1, 2];"), "v = ds_grid_get(g, 1, 2);");
    assert_eq!(tr("g[# 1, 2] = 0;"), "ds_grid_set(g, 1, 2, 0);");
    assert_eq!(tr("g[# 1, 2] += 3;"), "ds_grid_add(g, 1, 2, 3);");
    assert_eq!(tr("a[@ 0] = 1;"), "a[0] = 1;");
    assert_eq!(tr("l[| 0]++;"), "ds_list_replace(l, 0, ds_list_find_value(l, 0) + (1));");
    assert_eq!(
        tr("x = m[? l[| 0]];"),
        "x = ds_map_find_value(m, ds_list_find_value(l, 0));"
    );
}

#[test]
fn misc_syntax() {
    assert_eq!(tr("c = 0xFF00;"), "c = $FF00;");
    assert_eq!(tr("var a = 1, b, c = a + 2;"), "var a, b, c; a = 1; c = a + 2;");
    assert_eq!(tr("if (x) return;"), "if (x) exit;");
    assert_eq!(tr("return 5;"), "return 5;");
    assert_eq!(tr("a %= 3;"), "a = a mod (3);");
    assert_eq!(tr("a <<= 1;"), "a = a << (1);");
    assert_eq!(
        tr("for(var i=0;i<4;i++) x+=i;"),
        "{ var i; for(i = 0;i<4;i += 1) x+=i };"
    );
}

#[test]
fn enums_are_inlined() {
    let src = "enum Dir { left, right = 5, up }\nx = Dir.up + Dir.left;";
    let stmts = parse_program(src, false).unwrap();
    let mut t = Translator::default();
    collect_enums(&stmts, src, &mut t.enums);
    assert_eq!(t.enums["Dir"]["up"], "6");
    let out = t.translate(src).code;
    assert_eq!(out, "/* enum Dir (inlined by gmx2gm81) */\nx = 6 + 0;");
}

#[test]
fn defines_and_macros() {
    let parts = split_defines("#define a\nx = 1;\n#define b\nreturn 2;\n", "file");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], ("a".to_string(), "x = 1;\n".to_string()));
    assert_eq!(parts[1].0, "b");
    let parts = split_defines("x = 1;", "file");
    assert_eq!(parts, vec![("file".to_string(), "x = 1;".to_string())]);
    let (code, macros) = strip_directives("#macro SPEED 4\nx = SPEED;\n#region r\n");
    assert_eq!(macros, vec![("SPEED".to_string(), "4".to_string())]);
    assert_eq!(code, "\nx = SPEED;\n//#region r\n");
}

#[test]
fn string_escapes_optional() {
    // GMS1: backslash is literal, nothing changes.
    assert!(!Translator::default().translate("s = \"a\\\";").changed);
    let t = Translator {
        string_escapes: true,
        ..Default::default()
    };
    assert_eq!(t.translate("s = \"say \\\"hi\\\"\";").code, "s = 'say \"hi\"';");
    assert_eq!(t.translate("s = \"a\\nb\";").code, "s = \"a#b\";");
}

#[test]
fn short_circuit() {
    let t = Translator {
        short_circuit: true,
        ..Default::default()
    };
    // Field read of another instance on the right: guarded.
    assert_eq!(
        t.translate("if (instance_exists(o) && o.hp > 0) x = 1;").code,
        "{ var __gmx_t0; __gmx_t0 = false; if (instance_exists(o)) { if (o.hp > 0) __gmx_t0 = true; } if (__gmx_t0) x = 1 };"
    );
    assert_eq!(
        t.translate("ok = a or f().b;").code,
        "{ var __gmx_t0; __gmx_t0 = true; if (!(a)) { if (!(f().b)) __gmx_t0 = false; } ok = __gmx_t0 };"
    );
    // Harmless right operands and global/self/other fields stay as they are.
    for src in ["if (a && b) c = 1;", "if (a and global.b) c = 1;", "if (a || other.x) c = 1;"] {
        assert!(!t.translate(src).changed, "{src}");
    }
}

#[test]
fn array_length_functions() {
    let t = Translator {
        array_funcs: true,
        ..Default::default()
    };
    let out = t.translate("n = array_length_1d(text) + array_length_1d(global.list) + array_length_2d(o.grid, 2);");
    assert_eq!(
        out.code,
        "n = __gmx_array_length_1d(id, \"text\") + __gmx_array_length_1d(global, \"list\") + __gmx_array_length_2d(o, \"grid\", 2);"
    );
    assert!(out.usage.calls.contains("__gmx_array_length_1d"));
    // Not a variable: left to the plain compat script.
    assert!(!t.translate("n = array_length_1d(f());").changed);
}

#[test]
fn renames() {
    let mut t = Translator::default();
    t.var_renames.insert("browser_width".into(), "window_get_width()".into());
    t.call_renames.insert("audio_play_sound".into(), "__gmx_audio_play_sound".into());
    let out = t.translate("w = browser_width; audio_play_sound(snd, 1, false);");
    assert_eq!(
        out.code,
        "w = window_get_width(); __gmx_audio_play_sound(snd, 1, false);"
    );
    assert!(out.usage.calls.contains("__gmx_audio_play_sound"));
}

#[test]
fn usage_and_errors() {
    let t = Translator::default().translate("a = foo(b); c.d = 1; var e;");
    assert!(t.usage.calls.contains("foo"));
    assert!(t.usage.idents.contains("b"));
    assert!(t.usage.assigned.contains("a") && t.usage.assigned.contains("d"));
    let bad = Translator::default().translate("a = ;");
    assert!(bad.error.is_some());
    assert_eq!(bad.code, "a = ;");
}

#[test]
fn expression_args() {
    let t = Translator::default();
    assert_eq!(t.translate_expr("l[| 0] + 0x10").code, "ds_list_find_value(l, 0) + $10");
    assert!(!t.translate_expr("x + 1").changed);
    // Would need hoisting: left alone.
    assert!(!t.translate_expr("a ? 1 : 2").changed);
}

#[test]
fn eight_one_quirks_parse() {
    // GM 8.1 code that must survive untouched.
    for src in [
        "if a b = 1 else c = 2",
        "pSurf = -1 surface_create(10, 10);",
        "with obj do x = 1",
        "repeat 3 { a += 1 }",
        "switch (a) begin case 1: b = 2; break; default: b = 3 end",
        "do a += 1 until a > 3",
        "x = (instance_nearest(x, y, all)).x; y = instance_nearest(x, y, all).y",
        "if not a and b or c then exit",
        "a = b div 2 mod 3; c = 'it'",
    ] {
        let t = Translator::default().translate(src);
        assert!(t.error.is_none(), "{src}: {:?}", t.error);
        assert_eq!(t.code, src);
    }
}

#[test]
fn outer_block_followed_by_code_is_wrapped() {
    let src = "{ var __gmx_t0; __gmx_t0 = 1; }\r\nx = __gmx_t0;";
    assert_eq!(wrap_outer_block(src).unwrap(), format!("{{ {src}\r\n}}"));
    // A lone outer block (comments after it are fine), or code not starting with `{`.
    assert!(wrap_outer_block("{ a = 1 } // done\r\n").is_none());
    assert!(wrap_outer_block("a = 1; { b = 2 } c = 3").is_none());
    assert!(wrap_outer_block("{ a = 1 { b } } ;").is_some());
}

#[test]
fn crlf_line_endings() {
    assert_eq!(to_crlf("a\nb\r\nc\rd\n"), "a\r\nb\r\nc\r\nd\r\n");
}
