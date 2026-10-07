//! Compatibility layer: GML scripts and constants injected into converted projects so
//! that common GameMaker: Studio functions missing from GM 8.1 keep working.
//!
//! Only entries that the project actually uses (and that GM 8.1 does not already
//! provide, per specs/builtins.json) are added.

use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

/// Names known to GM 8.1 (functions, variables, constants) from specs/builtins.json.
pub struct Builtins {
    pub functions: HashSet<String>,
    pub variables: HashSet<String>,
    pub constants: HashSet<String>,
}

impl Builtins {
    pub fn contains(&self, n: &str) -> bool {
        self.functions.contains(n) || self.variables.contains(n) || self.constants.contains(n)
    }
}

pub fn builtins() -> &'static Builtins {
    static B: OnceLock<Builtins> = OnceLock::new();
    B.get_or_init(|| {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/builtins.json")).expect("builtins.json");
        let mut b = Builtins {
            functions: HashSet::new(),
            variables: HashSet::new(),
            constants: HashSet::new(),
        };
        for e in v["entries"].as_array().unwrap() {
            let name = e["name"].as_str().unwrap().to_string();
            match e["kind"].as_str() {
                Some("function") => b.functions.insert(name),
                Some("variable") => b.variables.insert(name),
                _ => b.constants.insert(name),
            };
        }
        // Language constants (gml-language.md §12.1).
        for c in ["true", "false", "pi", "self", "other", "all", "noone", "global", "local"] {
            b.constants.insert(c.into());
        }
        b
    })
}

/// A compatibility script.
pub struct CompatScript {
    pub name: &'static str,
    pub code: &'static str,
    /// Other compat scripts it calls.
    pub deps: &'static [&'static str],
}

macro_rules! s {
    ($name:literal, $deps:expr, $code:literal) => {
        CompatScript {
            name: $name,
            code: $code,
            deps: $deps,
        }
    };
}

/// Header prepended to every compat script.
pub const HEADER: &str = "// Added by gmx2gm81: GameMaker: Studio compatibility script.\n";

/// GM 8.1 functions whose Studio behaviour differs; calls are redirected to wrappers.
pub static WRAPS: &[(&str, &str)] = &[
    ("file_text_open_read", "__gmx_file_text_open_read"),
    ("file_bin_open", "__gmx_file_bin_open"),
];

pub static SCRIPTS: &[CompatScript] = &[
    // ---- Studio semantics of 8.1 functions
    s!("__gmx_file_text_open_read", &[], "// Studio returns -1 for a missing file instead of failing.\nif (!file_exists(argument0)) return -1;\nreturn file_text_open_read(argument0);"),
    s!("__gmx_file_bin_open", &[], "if (argument1 == 0 && !file_exists(argument0)) return -1;\nreturn file_bin_open(argument0, argument1);"),
    // ---- maths / geometry
    s!("point_in_rectangle", &[], "return (argument0 >= argument2 && argument0 <= argument4 && argument1 >= argument3 && argument1 <= argument5);"),
    s!("point_in_circle", &[], "return (sqr(argument0 - argument2) + sqr(argument1 - argument3) <= sqr(argument4));"),
    s!("point_in_triangle", &[], "// argument0,1 = point; argument2..7 = triangle corners\nvar d1, d2, d3, hasneg, haspos;\nd1 = (argument0 - argument4) * (argument3 - argument5) - (argument2 - argument4) * (argument1 - argument5);\nd2 = (argument0 - argument6) * (argument5 - argument7) - (argument4 - argument6) * (argument1 - argument7);\nd3 = (argument0 - argument2) * (argument7 - argument3) - (argument6 - argument2) * (argument1 - argument3);\nhasneg = (d1 < 0) || (d2 < 0) || (d3 < 0);\nhaspos = (d1 > 0) || (d2 > 0) || (d3 > 0);\nreturn !(hasneg && haspos);"),
    s!("rectangle_in_rectangle", &[], "// 0 = no overlap, 1 = inside, 2 = overlap\nvar ax1, ay1, ax2, ay2, bx1, by1, bx2, by2;\nax1 = min(argument0, argument2); ax2 = max(argument0, argument2); ay1 = min(argument1, argument3); ay2 = max(argument1, argument3);\nbx1 = min(argument4, argument6); bx2 = max(argument4, argument6); by1 = min(argument5, argument7); by2 = max(argument5, argument7);\nif (ax2 < bx1 || ax1 > bx2 || ay2 < by1 || ay1 > by2) return 0;\nif (ax1 >= bx1 && ax2 <= bx2 && ay1 >= by1 && ay2 <= by2) return 1;\nreturn 2;"),
    s!("angle_difference", &[], "return ((((argument0 - argument1) mod 360) + 540) mod 360) - 180;"),
    s!("dot_product", &[], "return argument0 * argument2 + argument1 * argument3;"),
    s!("dot_product_3d", &[], "return argument0 * argument3 + argument1 * argument4 + argument2 * argument5;"),
    s!("dot_product_normalised", &[], "var l1, l2; l1 = sqrt(sqr(argument0) + sqr(argument1)); l2 = sqrt(sqr(argument2) + sqr(argument3));\nif (l1 == 0 || l2 == 0) return 0;\nreturn (argument0 * argument2 + argument1 * argument3) / (l1 * l2);"),
    s!("int64", &[], "return floor(argument0);"),
    s!("is_undefined", &[], "// GM 8.1 has no 'undefined' value.\nreturn false;"),
    s!("is_array", &[], "return false;"),
    s!("is_bool", &[], "return false;"),
    s!("is_ptr", &[], "return false;"),
    s!("is_int32", &[], "return false;"),
    s!("is_int64", &[], "return false;"),
    s!("array_length_1d", &[], "// GM 8.1 arrays are not values; the length cannot be determined.\nreturn 0;"),
    s!("array_length_2d", &[], "return 0;"),
    s!("array_height_2d", &[], "return 0;"),
    s!("get_timer", &[], "return current_time * 1000;"),
    // ---- strings
    s!("string_byte_at", &[], "return ord(string_char_at(argument0, argument1));"),
    s!("string_byte_length", &[], "return string_length(argument0);"),
    s!("string_hash_to_newline", &[], "return argument0;"),
    s!("ansi_char", &[], "return chr(argument0);"),
    s!("string_ord_at", &[], "return ord(string_char_at(argument0, argument1));"),
    // ---- data structures
    s!("__gmx_ds_map_set", &[], "if (ds_map_exists(argument0, argument1)) ds_map_replace(argument0, argument1, argument2);\nelse ds_map_add(argument0, argument1, argument2);"),
    s!("ds_map_set", &["__gmx_ds_map_set"], "__gmx_ds_map_set(argument0, argument1, argument2);"),
    s!("ds_list_set", &[], "while (ds_list_size(argument0) <= argument1) ds_list_add(argument0, 0);\nds_list_replace(argument0, argument1, argument2);"),
    s!("ds_map_find_value_safe", &[], "if (ds_map_exists(argument0, argument1)) return ds_map_find_value(argument0, argument1);\nreturn 0;"),
    s!("ds_exists", &[], "// Assumed to exist (GM 8.1 has no generic check).\nreturn argument0 >= 0;"),
    // ---- instances / variables
    s!("variable_instance_exists", &[], "var r; r = false;\nwith (argument0) r = variable_local_exists(argument1);\nreturn r;"),
    s!("variable_instance_get", &[], "var r; r = 0;\nwith (argument0) r = variable_local_get(argument1);\nreturn r;"),
    s!("variable_instance_set", &[], "with (argument0) variable_local_set(argument1, argument2);"),
    s!("asset_get_index", &[], "// Resource names are constants in GM 8.1.\nif (string_length(argument0) == 0) return -1;\nreturn execute_string(\"return \" + argument0 + \";\");"),
    s!("instance_create_layer", &[], "return instance_create(argument0, argument1, argument3);"),
    // ---- drawing
    s!("draw_sprite_pos", &[], "// Approximation: GM 8.1 has no free quad sprite drawing; use a textured primitive.\nvar tex; tex = sprite_get_texture(argument0, argument1);\ndraw_set_alpha(argument10);\ndraw_primitive_begin_texture(pr_trianglestrip, tex);\ndraw_vertex_texture(argument2, argument3, 0, 0);\ndraw_vertex_texture(argument4, argument5, 1, 0);\ndraw_vertex_texture(argument8, argument9, 0, 1);\ndraw_vertex_texture(argument6, argument7, 1, 1);\ndraw_primitive_end();\ndraw_set_alpha(1);"),
    s!("draw_enable_alphablend", &[], "// No effect in GM 8.1.\nreturn 0;"),
    s!("draw_set_colour_write_enable", &[], "return 0;"),
    s!("draw_set_color_write_enable", &[], "return 0;"),
    s!("draw_texture_flush", &[], "return 0;"),
    s!("application_surface_enable", &[], "return 0;"),
    s!("application_surface_draw_enable", &[], "return 0;"),
    s!("surface_resize", &[], "// Not possible in GM 8.1 (ids cannot be kept); ignored.\nreturn 0;"),
    s!("surface_set_target_ext", &[], "return surface_set_target(argument1);"),
    s!("texture_global_scale", &[], "return 0;"),
    s!("__gmx_view_current", &[], "// Studio leaves view_current at the last view (7) while Draw GUI runs.\nif (variable_global_exists(\"__gmx_in_gui\")) { if (global.__gmx_in_gui) { if (view_enabled) return 7; return 0; } }\nreturn view_current;"),
    s!("display_set_gui_size", &[], "global.__gmx_gui_w = argument0;\nglobal.__gmx_gui_h = argument1;"),
    s!("display_get_gui_width", &[], "if (variable_global_exists(\"__gmx_gui_w\")) { if (global.__gmx_gui_w > 0) return global.__gmx_gui_w; }\nif (view_enabled) return view_wport[0];\nreturn room_width;"),
    s!("display_get_gui_height", &[], "if (variable_global_exists(\"__gmx_gui_h\")) { if (global.__gmx_gui_h > 0) return global.__gmx_gui_h; }\nif (view_enabled) return view_hport[0];\nreturn room_height;"),
    s!("display_set_gui_maximise", &[], "global.__gmx_gui_w = 0; global.__gmx_gui_h = 0;"),
    s!("display_set_gui_maximize", &[], "global.__gmx_gui_w = 0; global.__gmx_gui_h = 0;"),
    s!("display_get_dpi_x", &[], "return 96;"),
    s!("display_get_dpi_y", &[], "return 96;"),
    s!("display_set_timing_method", &[], "return 0;"),
    s!("display_set_windows_vertex_buffer_method", &[], "return 0;"),
    s!("display_get_frequency", &[], "return 60;"),
    s!("window_has_focus", &[], "return true;"),
    s!("window_set_min_width", &[], "return 0;"),
    s!("window_set_min_height", &[], "return 0;"),
    s!("window_set_max_width", &[], "return 0;"),
    s!("window_set_max_height", &[], "return 0;"),
    s!("window_get_visible_rects", &[], "return 0;"),
    s!("device_mouse_x", &[], "return mouse_x;"),
    s!("device_mouse_y", &[], "return mouse_y;"),
    s!("device_mouse_raw_x", &[], "return window_mouse_get_x();"),
    s!("device_mouse_raw_y", &[], "return window_mouse_get_y();"),
    s!("device_mouse_x_to_gui", &["display_get_gui_width"], "return window_mouse_get_x() * display_get_gui_width() / max(1, window_get_width());"),
    s!("device_mouse_y_to_gui", &["display_get_gui_height"], "return window_mouse_get_y() * display_get_gui_height() / max(1, window_get_height());"),
    s!("device_mouse_check_button", &[], "return mouse_check_button(argument1);"),
    s!("device_mouse_check_button_pressed", &[], "return mouse_check_button_pressed(argument1);"),
    s!("device_mouse_check_button_released", &[], "return mouse_check_button_released(argument1);"),
    s!("device_mouse_dbclick_enable", &[], "return 0;"),
    s!("device_is_keypad_open", &[], "return false;"),
    s!("os_get_language", &[], "return \"en\";"),
    s!("os_get_region", &[], "return \"GB\";"),
    s!("os_is_paused", &[], "return false;"),
    s!("os_is_network_connected", &[], "return false;"),
    s!("os_powersave_enable", &[], "return 0;"),
    s!("os_lock_orientation", &[], "return 0;"),
    s!("os_get_config", &[], "return \"Default\";"),
    s!("code_is_compiled", &[], "return false;"),
    s!("game_set_speed", &[], "room_speed = argument0;"),
    s!("game_get_speed", &[], "return room_speed;"),
    // ---- audio (new audio engine → GM 8.1 sound functions)
    s!("audio_play_sound", &[], "// Returns the sound index as the 'voice' id.\nif (argument2) sound_loop(argument0); else sound_play(argument0);\nreturn argument0;"),
    s!("audio_play_sound_at", &[], "if (argument6) sound_loop(argument0); else sound_play(argument0);\nreturn argument0;"),
    s!("audio_play_sound_on", &[], "if (argument2) sound_loop(argument1); else sound_play(argument1);\nreturn argument1;"),
    s!("audio_play_music", &[], "sound_loop(argument0);\nreturn argument0;"),
    s!("audio_stop_sound", &[], "sound_stop(argument0);"),
    s!("audio_stop_music", &[], "sound_stop_all();"),
    s!("audio_stop_all", &[], "sound_stop_all();"),
    s!("audio_pause_sound", &[], "// GM 8.1 cannot pause; stop instead.\nsound_stop(argument0);"),
    s!("audio_pause_all", &[], "sound_stop_all();"),
    s!("audio_resume_sound", &[], "return 0;"),
    s!("audio_resume_all", &[], "return 0;"),
    s!("audio_is_playing", &[], "return sound_isplaying(argument0);"),
    s!("audio_is_paused", &[], "return false;"),
    s!("audio_exists", &[], "return sound_exists(argument0);"),
    s!("audio_sound_gain", &[], "if (argument2 > 0) sound_fade(argument0, argument1, argument2); else sound_volume(argument0, argument1);"),
    s!("audio_sound_get_gain", &[], "return 1;"),
    s!("audio_sound_pitch", &[], "return 0;"),
    s!("audio_sound_get_pitch", &[], "return 1;"),
    s!("audio_sound_get_track_position", &[], "return 0;"),
    s!("audio_sound_set_track_position", &[], "return 0;"),
    s!("audio_sound_length", &[], "return 0;"),
    s!("audio_master_gain", &[], "sound_global_volume(argument0);"),
    s!("audio_music_gain", &[], "return 0;"),
    s!("audio_channel_num", &[], "return 0;"),
    s!("audio_falloff_set_model", &[], "return 0;"),
    s!("audio_listener_position", &[], "return 0;"),
    s!("audio_listener_orientation", &[], "return 0;"),
    s!("audio_emitter_create", &[], "return 0;"),
    s!("audio_emitter_position", &[], "return 0;"),
    s!("audio_emitter_free", &[], "return 0;"),
    s!("audio_emitter_gain", &[], "return 0;"),
    s!("audio_emitter_falloff", &[], "return 0;"),
    s!("audio_group_load", &[], "return true;"),
    s!("audio_group_unload", &[], "return true;"),
    s!("audio_group_is_loaded", &[], "return true;"),
    s!("audio_group_load_progress", &[], "return 100;"),
    s!("audio_group_set_gain", &[], "return 0;"),
    s!("audio_group_stop_all", &[], "sound_stop_all();"),
    s!("audio_system", &[], "return 1;"),
    // ---- gamepads → joysticks (device 0 = joystick 1)
    s!("__gmx_gp_button", &[], "// argument0 = device, argument1 = gp_* constant; returns held state.\nvar j, b; j = argument0 + 1;\nif (!joystick_exists(j)) return false;\nb = argument1 - 32768;\nswitch (b) {\ncase 13: return joystick_ypos(j) < -0.5 || (joystick_pov(j) >= 0 && (joystick_pov(j) >= 315 || joystick_pov(j) <= 45));\ncase 14: return joystick_ypos(j) > 0.5 || (joystick_pov(j) >= 135 && joystick_pov(j) <= 225);\ncase 15: return joystick_xpos(j) < -0.5 || (joystick_pov(j) >= 225 && joystick_pov(j) <= 315);\ncase 16: return joystick_xpos(j) > 0.5 || (joystick_pov(j) >= 45 && joystick_pov(j) <= 135);\n}\nif (b >= 1 && b <= 12) return joystick_check_button(j, b);\nreturn false;"),
    s!("gamepad_is_supported", &[], "return true;"),
    s!("gamepad_get_device_count", &[], "return 2;"),
    s!("gamepad_is_connected", &[], "return joystick_exists(argument0 + 1);"),
    s!("gamepad_get_description", &[], "return joystick_name(argument0 + 1);"),
    s!("gamepad_button_check", &["__gmx_gp_button"], "return __gmx_gp_button(argument0, argument1);"),
    s!("gamepad_button_check_pressed", &["__gmx_gp_button"], "var k, cur, prev; k = \"__gmx_gpp_\" + string(argument0) + \"_\" + string(argument1);\ncur = __gmx_gp_button(argument0, argument1);\nprev = false; if (variable_global_exists(k)) prev = variable_global_get(k);\nvariable_global_set(k, cur);\nreturn cur && !prev;"),
    s!("gamepad_button_check_released", &["__gmx_gp_button"], "var k, cur, prev; k = \"__gmx_gpr_\" + string(argument0) + \"_\" + string(argument1);\ncur = __gmx_gp_button(argument0, argument1);\nprev = false; if (variable_global_exists(k)) prev = variable_global_get(k);\nvariable_global_set(k, cur);\nreturn prev && !cur;"),
    s!("gamepad_button_value", &["__gmx_gp_button"], "return __gmx_gp_button(argument0, argument1);"),
    s!("gamepad_axis_value", &[], "var j; j = argument0 + 1;\nif (!joystick_exists(j)) return 0;\nswitch (argument1 - 32768) {\ncase 17: return joystick_xpos(j);\ncase 18: return joystick_ypos(j);\ncase 19: return joystick_upos(j);\ncase 20: return joystick_rpos(j);\n}\nreturn 0;"),
    s!("gamepad_set_axis_deadzone", &[], "return 0;"),
    s!("gamepad_set_button_threshold", &[], "return 0;"),
    s!("gamepad_set_vibration", &[], "return 0;"),
    s!("gamepad_axis_count", &[], "return joystick_axes(argument0 + 1);"),
    s!("gamepad_button_count", &[], "return joystick_buttons(argument0 + 1);"),
    // ---- shaders (dropped)
    s!("shader_set", &[], "// Shaders are not supported in GM 8.1; ignored.\nreturn 0;"),
    s!("shader_reset", &[], "return 0;"),
    s!("shader_is_compiled", &[], "return false;"),
    s!("shaders_are_supported", &[], "return false;"),
    s!("shader_get_uniform", &[], "return -1;"),
    s!("shader_get_sampler_index", &[], "return -1;"),
    s!("shader_set_uniform_f", &[], "return 0;"),
    s!("shader_set_uniform_i", &[], "return 0;"),
    s!("shader_set_uniform_f_array", &[], "return 0;"),
    s!("shader_set_uniform_matrix", &[], "return 0;"),
    s!("texture_set_stage", &[], "return 0;"),
    // ---- misc async / platform
    s!("show_message_async", &[], "show_message(argument0);\nreturn -1;"),
    s!("show_debug_overlay", &[], "return 0;"),
    s!("url_open", &[], "return 0;"),
    s!("http_get", &[], "return -1;"),
    s!("http_post_string", &[], "return -1;"),
    s!("json_encode", &[], "return \"{}\";"),
    s!("json_decode", &[], "return -1;"),
    s!("steam_initialised", &[], "return false;"),
    s!("achievement_available", &[], "return false;"),
];

/// Studio constants missing from GM 8.1, added as user constants when referenced.
pub static CONSTANTS: &[(&str, &str)] = &[
    ("gp_face1", "32769"),
    ("gp_face2", "32770"),
    ("gp_face3", "32771"),
    ("gp_face4", "32772"),
    ("gp_shoulderl", "32773"),
    ("gp_shoulderr", "32774"),
    ("gp_shoulderlb", "32775"),
    ("gp_shoulderrb", "32776"),
    ("gp_select", "32777"),
    ("gp_start", "32778"),
    ("gp_stickl", "32779"),
    ("gp_stickr", "32780"),
    ("gp_padu", "32781"),
    ("gp_padd", "32782"),
    ("gp_padl", "32783"),
    ("gp_padr", "32784"),
    ("gp_axislh", "32785"),
    ("gp_axislv", "32786"),
    ("gp_axisrh", "32787"),
    ("gp_axisrv", "32788"),
    ("os_windows", "0"),
    ("os_linux", "6"),
    ("os_winphone", "7"),
    ("os_tizen", "8"),
    ("os_win8native", "9"),
    ("os_wiiu", "10"),
    ("os_3ds", "11"),
    ("os_psvita", "12"),
    ("os_bb10", "13"),
    ("os_ps4", "14"),
    ("os_xboxone", "15"),
    ("os_ps3", "16"),
    ("os_xbox360", "17"),
    ("os_uwp", "18"),
    ("browser_not_a_browser", "-1"),
    ("browser_unknown", "0"),
    ("browser_ie", "1"),
    ("browser_firefox", "2"),
    ("browser_chrome", "3"),
    ("browser_safari", "4"),
    ("browser_safari_mobile", "5"),
    ("browser_opera", "6"),
    ("browser_tizen", "7"),
    ("browser_windows_store", "8"),
    ("browser_ie_mobile", "9"),
    ("ev_gui", "64"),
    ("ev_draw_begin", "72"),
    ("ev_draw_end", "73"),
    ("ev_gui_begin", "74"),
    ("ev_gui_end", "75"),
    ("ev_draw_pre", "76"),
    ("ev_draw_post", "77"),
    ("ev_user_resize", "65"),
    ("ev_global_left_button", "50"),
    ("audio_falloff_none", "0"),
    ("audio_falloff_inverse_distance", "1"),
    ("audio_falloff_inverse_distance_clamped", "2"),
    ("audio_falloff_linear_distance", "3"),
    ("audio_falloff_linear_distance_clamped", "4"),
    ("audio_falloff_exponent_distance", "5"),
    ("audio_falloff_exponent_distance_clamped", "6"),
    ("audio_old_system", "0"),
    ("audio_new_system", "1"),
    ("pointer_null", "-1"),
    ("pointer_invalid", "-1"),
    ("undefined", "0"),
    ("tm_countvsyncs", "1"),
    ("tm_sleep", "0"),
    ("display_landscape", "0"),
    ("display_landscape_flipped", "1"),
    ("display_portrait", "2"),
    ("display_portrait_flipped", "3"),
    ("mb_any", "-1"),
    ("ds_type_map", "1"),
    ("ds_type_list", "2"),
    ("ds_type_stack", "3"),
    ("ds_type_queue", "4"),
    ("ds_type_grid", "5"),
    ("ds_type_priority", "6"),
    ("buffer_fixed", "0"),
    ("buffer_grow", "1"),
    ("buffer_wrap", "2"),
    ("buffer_fast", "3"),
];

/// Studio built-in variables with no GM 8.1 equivalent, replaced on read by an
/// expression.
pub static VAR_RENAMES: &[(&str, &str)] = &[
    ("application_surface", "(-1)"),
    ("delta_time", "(1000000 / room_speed)"),
    ("browser_width", "window_get_width()"),
    ("browser_height", "window_get_height()"),
    ("os_browser", "(-1)"),
    ("os_version", "(0)"),
    ("fps_real", "fps"),
    ("async_load", "(-1)"),
    ("event_data", "(-1)"),
    ("iap_data", "(-1)"),
    ("webgl_enabled", "(false)"),
    ("gamemaker_pro", "(true)"),
    ("gamemaker_registered", "(true)"),
    ("gamemaker_version", "(810)"),
    ("view_surface_id", "(-1)"),
    ("cursor_sprite", "(-1)"),
];

pub fn script(name: &str) -> Option<&'static CompatScript> {
    SCRIPTS.iter().find(|s| s.name == name)
}

pub fn constant(name: &str) -> Option<&'static str> {
    CONSTANTS.iter().find(|c| c.0 == name).map(|c| c.1)
}

/// `name` with British spellings replaced, if that names a GM 8.1 function.
pub fn colour_rename(name: &str) -> Option<String> {
    if name.contains("colour") {
        let n = name.replace("colour", "color");
        if builtins().functions.contains(&n) {
            return Some(n);
        }
    }
    None
}

/// Close `names` under compat-script dependencies; returns the scripts in a stable order.
pub fn with_deps(names: &BTreeMap<String, ()>) -> Vec<&'static CompatScript> {
    let mut want: Vec<&'static str> = Vec::new();
    let mut stack: Vec<&'static CompatScript> = names.keys().filter_map(|n| script(n)).collect();
    while let Some(s) = stack.pop() {
        if want.contains(&s.name) {
            continue;
        }
        want.push(s.name);
        for d in s.deps {
            if let Some(ds) = script(d) {
                stack.push(ds);
            }
        }
    }
    SCRIPTS.iter().filter(|s| want.contains(&s.name)).collect()
}
