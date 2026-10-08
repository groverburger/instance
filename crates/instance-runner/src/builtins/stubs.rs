//! Functions accepted without real effect (counted as "stubbed" in coverage):
//! multiplayer (413), DLLs (414), 3D (415), CD audio, sound effects, joysticks,
//! dialog styling, splash-window settings and video splash.

use super::{bo, r, st, zero, Reg};

const ZERO: &[&str] = &[
    // 406
    "sound_pan",
    "sound_background_tempo",
    "sound_effect_chorus",
    "sound_effect_echo",
    "sound_effect_flanger",
    "sound_effect_gargle",
    "sound_effect_reverb",
    "sound_effect_compressor",
    "sound_effect_equalizer",
    "sound_3d_set_sound_position",
    "sound_3d_set_sound_velocity",
    "sound_3d_set_sound_distance",
    "sound_3d_set_sound_cone",
    "cd_init",
    "cd_play",
    "cd_stop",
    "cd_pause",
    "cd_resume",
    "cd_set_position",
    "cd_set_track_position",
    "cd_open_door",
    "cd_close_door",
    "cd_present",
    "cd_number",
    "cd_playing",
    "cd_paused",
    "cd_track",
    "cd_length",
    "cd_track_length",
    "cd_position",
    "cd_track_position",
    // 404 joysticks (none present)
    "joystick_exists",
    "joystick_axes",
    "joystick_buttons",
    "joystick_has_pov",
    "joystick_check_button",
    "joystick_xpos",
    "joystick_ypos",
    "joystick_zpos",
    "joystick_rpos",
    "joystick_upos",
    "joystick_vpos",
    // 407 styling / splash
    "message_background",
    "message_button",
    "message_alpha",
    "message_text_font",
    "message_button_font",
    "message_input_font",
    "message_text_charset",
    "message_mouse_color",
    "message_input_color",
    "message_position",
    "message_size",
    "message_caption",
    "splash_show_video",
    "splash_show_web",
    "splash_set_caption",
    "splash_set_fullscreen",
    "splash_set_border",
    "splash_set_size",
    "splash_set_position",
    "splash_set_adapt",
    "splash_set_top",
    "splash_set_color",
    "splash_set_main",
    "splash_set_scale",
    "splash_set_cursor",
    "splash_set_interrupt",
    "splash_set_stop_key",
    "splash_set_stop_mouse",
    "splash_set_close_button",
    // 413 multiplayer (no connection possible)
    "mplay_init_ipx",
    "mplay_init_tcpip",
    "mplay_init_modem",
    "mplay_init_serial",
    "mplay_connect_status",
    "mplay_end",
    "mplay_session_mode",
    "mplay_session_create",
    "mplay_session_find",
    "mplay_session_join",
    "mplay_session_status",
    "mplay_session_end",
    "mplay_player_find",
    "mplay_player_id",
    "mplay_data_write",
    "mplay_data_read",
    "mplay_data_mode",
    "mplay_message_send",
    "mplay_message_send_guaranteed",
    "mplay_message_receive",
    "mplay_message_id",
    "mplay_message_value",
    "mplay_message_player",
    "mplay_message_count",
    "mplay_message_clear",
    // 414
    "external_call",
    "external_define",
    "external_free",
    "get_function_address",
    "window_handle",
    "YoYo_GetPlatform",
];

const EMPTY_STRING: &[&str] = &[
    "joystick_name",
    "MCI_command",
    "mplay_ipaddress",
    "mplay_session_name",
    "mplay_player_name",
    "mplay_message_name",
];

pub fn register(l: &mut Reg) {
    for n in ZERO {
        l.push((n, |_, _, _, _| zero()));
    }
    for n in EMPTY_STRING {
        l.push((n, |_, _, _, _| st("")));
    }
    l.push(("joystick_direction", |_, _, _, _| r(101.0)));
    l.push(("joystick_pov", |_, _, _, _| r(-1.0)));
    // 415: 3D mode is not supported; d3d_start reports failure, the rest are no-ops.
    l.push(("d3d_start", |_, _, _, _| bo(false)));
    l.push(("d3d_transform_stack_empty", |_, _, _, _| bo(true)));
    for i in 0..gml::builtins::FUNCTIONS.len() {
        let f = &gml::builtins::FUNCTIONS[i];
        if (f.chapter == "415" || f.chapter == "yoyo") && f.name != "d3d_start" && f.name != "d3d_transform_stack_empty" {
            l.push((f.name, |_, _, _, _| zero()));
        }
    }
}
