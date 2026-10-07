//! The multiplayer demo project, built with gm-format (shared by the example and
//! the integration test).
//!
//! One object runs everything in its Create event. Started with the game parameter
//! `host` it creates a session and waits for a player; otherwise it finds the
//! session on the LAN and joins. The two exchange a guaranteed message each way and
//! one shared-data slot each, report what they received with
//! `show_debug_message` (printed as `[debug] ...` by a headless runner), then the
//! client leaves and the host ends the session.

use gm_format as fmt;

const CREATE: &str = r#"
var i, t, n;
role = 'client';
for (i = 1; i <= parameter_count(); i += 1) {
    if (parameter_string(i) == 'host') role = 'host';
}
mplay_init_tcpip('');
show_debug_message(role + ': tcp/ip status ' + string(mplay_connect_status()) + ', address ' + mplay_ipaddress());
if (role == 'host') {
    if (!mplay_session_create('gm81 demo', 4, 'HostPlayer')) {
        show_debug_message('host: cannot create session');
        game_end();
        exit;
    }
    show_debug_message('host: session created, waiting for a player');
    t = current_time;
    while (mplay_player_find() < 2 && current_time < t + 30000) {}
    if (mplay_player_find() < 2) {
        show_debug_message('host: nobody joined');
        game_end();
        exit;
    }
    show_debug_message('host: player joined: ' + mplay_player_name(1) + ' (id ' + string(mplay_player_id(1)) + ')');
    mplay_data_write(0, 'hello from host');
    mplay_message_send_guaranteed(0, 1, 'ping');
    t = current_time;
    while (!mplay_message_receive(0) && current_time < t + 10000) {}
    show_debug_message('host: got message ' + string(mplay_message_id()) + ' "' + string(mplay_message_value()) + '" from ' + mplay_message_name());
    show_debug_message('host: shared slot 1 = ' + string(mplay_data_read(1)));
    t = current_time;
    while (mplay_player_find() > 1 && current_time < t + 10000) {}
    show_debug_message('host: players now ' + string(mplay_player_find()) + '; ending session');
    mplay_end();
} else {
    t = current_time;
    n = 0;
    while (n == 0 && current_time < t + 20000) n = mplay_session_find();
    if (n == 0) {
        show_debug_message('client: no session found');
        game_end();
        exit;
    }
    show_debug_message('client: found session "' + mplay_session_name(0) + '"');
    if (!mplay_session_join(0, 'ClientPlayer')) {
        show_debug_message('client: join failed');
        game_end();
        exit;
    }
    show_debug_message('client: joined, status ' + string(mplay_session_status()) + ', players ' + string(mplay_player_find()));
    t = current_time;
    while (!mplay_message_receive(0) && current_time < t + 10000) {}
    show_debug_message('client: got message ' + string(mplay_message_id()) + ' "' + string(mplay_message_value()) + '" from ' + mplay_message_name());
    t = current_time;
    while (!is_string(mplay_data_read(0)) && current_time < t + 10000) {}
    show_debug_message('client: shared slot 0 = ' + string(mplay_data_read(0)));
    mplay_data_write(1, 42);
    mplay_message_send_guaranteed(mplay_player_id(1), 2, 'pong');
    t = current_time;
    while (current_time < t + 300) {}
    mplay_session_end();
    show_debug_message('client: left, status ' + string(mplay_session_status()));
    mplay_end();
}
game_end();
"#;

/// Builds the demo project.
pub fn project() -> fmt::Project {
    let mut p = fmt::Project::default();
    p.game_id = 810_042;
    p.settings.display_errors = true;
    let mut act = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    act.args[0] = CREATE.to_string();
    p.objects.push(Some(fmt::Object {
        name: "obj_net".into(),
        sprite: -1,
        visible: true,
        events: vec![fmt::Event { event_type: 0, number: 0, actions: vec![act] }],
        ..Default::default()
    }));
    p.rooms.push(Some(fmt::Room {
        name: "rm_demo".into(),
        instances: vec![fmt::Instance { x: 0, y: 0, object: 0, id: 100001, creation_code: String::new(), locked: false }],
        ..Default::default()
    }));
    p.last_instance_id = 100002;
    p.room_order.push(0);
    p
}
