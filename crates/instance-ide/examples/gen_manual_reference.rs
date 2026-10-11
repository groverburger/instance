//! Generates the function reference of the user manual (`docs/manual/reference/`)
//! from `specs/builtins.json`:
//!
//! ```text
//! cargo run -p instance-ide --example gen_manual_reference
//! ```
//!
//! One Markdown file per group of the manual's "Function reference" (see
//! `docs/manual/README.md`), a `README.md` listing them and an A–Z `index.md`.
//! Every function and variable gets a `### name` heading (GitHub anchor `#name`);
//! constants are listed in a table per section, whose heading the index links to.
//! A test of instance-ide (`help::tests`) checks that the committed files equal the
//! output of [`generate`].

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use serde::Deserialize;

/// The reference files: (group file, title, intro).
pub const GROUPS: &[(&str, &str, &str)] = &[
    ("computing.md", "Computing", "Functions for numbers, strings, dates and times, script arguments and the special instance references."),
    ("game-play.md", "Game play", "Moving instances around, paths, motion planning, collisions, creating and destroying instances, timing, rooms, score, and generating events."),
    ("user-interaction.md", "User interaction", "Reading the keyboard, the mouse and joysticks."),
    ("game-graphics.md", "Game graphics", "Drawing sprites, backgrounds, shapes and text, blending, surfaces, tiles, the display, the game window and views."),
    ("sound-and-music.md", "Sound and music", "Playing sounds and music, sound effects, 3D sound and CD audio."),
    ("splash-and-messages.md", "Splash screens, messages and highscores", "Showing splash screens and information, asking the player questions, and keeping a highscore list."),
    ("resources.md", "Resources", "Reading the properties of the game's resources, and creating, changing and deleting resources while the game runs."),
    ("files-and-system.md", "Files, registry, INI files and programs", "Reading and writing files, the registry and INI files, running other programs, and information about the device."),
    ("data-structures.md", "Data structures", "Stacks, queues, lists, maps, priority queues and grids."),
    ("particles.md", "Particles", "Simple effects, and particle systems with types, emitters, attractors, destroyers, deflectors and changers."),
    ("multiplayer.md", "Multiplayer", "Connections, sessions, players, shared data and messages for multiplayer games."),
    ("dlls.md", "DLLs", "Calling functions in external DLLs."),
    ("3d-graphics.md", "3D graphics", "3D mode, drawing primitives and shapes, projections, transformations, fog, lighting and models."),
];

/// Sub-pages: (page code, group file, section title, one-line intro).
pub const SECTIONS: &[(&str, &str, &str, &str)] = &[
    ("401_06_addressing", "computing.md", "Special instance references", "Values that stand for particular instances wherever an instance id is expected."),
    ("401_18_scripts", "computing.md", "Script arguments", "The arguments a script receives from its caller."),
    ("402_01_constants", "computing.md", "Basic constants", "Truth values and the number pi."),
    ("402_02_real", "computing.md", "Real-valued functions", "Arithmetic, rounding, random numbers, trigonometry and other functions on real numbers."),
    ("402_03_string", "computing.md", "String handling", "Building, searching, cutting and converting strings."),
    ("402_04_dates", "computing.md", "Dates and times", "Date-time values: creating them, reading their parts, comparing and adding to them."),
    ("403_01_moving", "game-play.md", "Moving around", "The motion variables of an instance and functions that move it."),
    ("403_02_paths", "game-play.md", "Paths", "Letting an instance follow a path."),
    ("403_03_planning", "game-play.md", "Motion planning", "Finding a way towards a goal around obstacles, step by step or as a whole path."),
    ("403_04_collision", "game-play.md", "Collision checking", "Testing whether shapes collide with instances."),
    ("403_05_instances", "game-play.md", "Instances", "Instance properties, and creating, finding, changing and destroying instances."),
    ("403_06_deactive", "game-play.md", "Deactivating instances", "Taking instances out of the game temporarily, and bringing them back."),
    ("403_07_timing", "game-play.md", "Timing", "The room speed, alarms, time lines and the system clock."),
    ("403_08_rooms", "game-play.md", "Rooms", "The current room, moving between rooms, and saving and loading the game."),
    ("403_09_score", "game-play.md", "Score, lives and health", "The built-in score, lives and health values and how they are shown."),
    ("403_10_events", "game-play.md", "Generating events", "Running events of instances from code, and the event type and number constants."),
    ("403_11_misc", "game-play.md", "Miscellaneous variables and functions", "Error handling, debug output, accessing variables by name and other functions."),
    ("404_01_keyboard", "user-interaction.md", "The keyboard", "Reading and simulating keys, and the virtual key constants."),
    ("404_02_mouse", "user-interaction.md", "The mouse", "The mouse position, its buttons and the cursor."),
    ("404_03_joystick", "user-interaction.md", "The joystick", "Joystick axes, buttons and directions."),
    ("405_01_sprites", "game-graphics.md", "Sprites and images", "The sprite of an instance and how it is drawn."),
    ("405_02_backgrounds", "game-graphics.md", "Backgrounds", "The background images of the current room."),
    ("405_03_drawing", "game-graphics.md", "Drawing sprites and backgrounds", "Drawing sprites and backgrounds in a draw event, plainly or scaled, rotated, blended and cut."),
    ("405_04_shapes", "game-graphics.md", "Drawing shapes", "Points, lines, rectangles, circles, arrows, health bars and other shapes, and the color constants."),
    ("405_05_text", "game-graphics.md", "Fonts and text", "Choosing a font and alignment, drawing text and measuring it."),
    ("405_06_advanced", "game-graphics.md", "Advanced drawing", "Colored shapes, primitives, textures and blend modes."),
    ("405_07_surfaces", "game-graphics.md", "Surfaces", "Drawing into images in memory and using them afterwards."),
    ("405_08_tiles", "game-graphics.md", "Tiles", "Adding, finding, changing and removing tiles."),
    ("405_09_display", "game-graphics.md", "The display", "The screen's size, color depth and refresh rate."),
    ("405_10_window", "game-graphics.md", "The window", "The game window: full screen, caption, size, position, cursor and scaling."),
    ("405_11_views", "game-graphics.md", "Views", "The parts of the room that are shown on the screen."),
    ("405_12_repaint", "game-graphics.md", "Repainting the screen", "Drawing and showing the screen from code."),
    ("406_01_basic", "sound-and-music.md", "Basic sound functions", "Playing, looping, stopping and mixing sounds."),
    ("406_02_effects", "sound-and-music.md", "Sound effects", "Chorus, echo, flanger, gargle, reverb, compressor and equalizer effects."),
    ("406_03_3d", "sound-and-music.md", "3D sound", "Positioning sounds in space."),
    ("406_04_cd", "sound-and-music.md", "CD music", "Playing audio CDs, and sending commands to the media control interface."),
    ("407_01_splash", "splash-and-messages.md", "Splash screens", "Showing videos, text, web pages and images, and the game information."),
    ("407_02_messages", "splash-and-messages.md", "Pop-up messages and questions", "Messages, questions, menus and file dialogs, and how they look."),
    ("407_03_highscore", "splash-and-messages.md", "The highscore list", "Keeping, showing and drawing the highscore list."),
    ("408_01_sprites", "resources.md", "Sprite information", "Reading the properties of sprites."),
    ("408_02_sounds", "resources.md", "Sound information", "Reading the properties of sounds."),
    ("408_03_backgrounds", "resources.md", "Background information", "Reading the properties of backgrounds."),
    ("408_04_fonts", "resources.md", "Font information", "Reading the properties of fonts."),
    ("408_05_paths", "resources.md", "Path information", "Reading the properties and points of paths."),
    ("408_06_scripts", "resources.md", "Script information", "Reading the properties of scripts."),
    ("408_07_timelines", "resources.md", "Time line information", "Reading the properties of time lines."),
    ("408_08_objects", "resources.md", "Object information", "Reading the properties of objects."),
    ("408_09_rooms", "resources.md", "Room information", "Reading the properties of rooms."),
    ("409_01_sprites", "resources.md", "Changing sprites", "Creating, loading, changing and deleting sprites while the game runs."),
    ("409_02_sounds", "resources.md", "Changing sounds", "Loading, replacing and deleting sounds while the game runs."),
    ("409_03_backgrounds", "resources.md", "Changing backgrounds", "Creating, loading, changing and deleting backgrounds while the game runs."),
    ("409_04_fonts", "resources.md", "Changing fonts", "Adding, replacing and deleting fonts while the game runs."),
    ("409_05_paths", "resources.md", "Changing paths", "Creating paths and changing their points while the game runs."),
    ("409_06_scripts", "resources.md", "Running code", "Running scripts by index, and running code from strings and files."),
    ("409_07_timelines", "resources.md", "Changing time lines", "Creating time lines and changing their moments while the game runs."),
    ("409_08_objects", "resources.md", "Changing objects", "Creating objects and changing their properties and events while the game runs."),
    ("409_09_rooms", "resources.md", "Changing rooms", "Creating rooms and changing their settings, instances and tiles while the game runs."),
    ("410_01_files", "files-and-system.md", "Files", "Text and binary files, file names and directories, and the game's parameters and environment."),
    ("410_02_registry", "files-and-system.md", "The registry", "Storing settings in the system registry."),
    ("410_03_ini", "files-and-system.md", "INI files", "Storing settings in INI files."),
    ("410_04_programs", "files-and-system.md", "Running programs", "Starting other programs and opening files with them."),
    ("410_05_device", "files-and-system.md", "Device information", "Which kind of system and device the game runs on."),
    ("411_00_datastructures", "data-structures.md", "General", "Settings shared by all data structures."),
    ("411_01_stacks", "data-structures.md", "Stacks", "Last in, first out collections."),
    ("411_02_queues", "data-structures.md", "Queues", "First in, first out collections."),
    ("411_03_lists", "data-structures.md", "Lists", "Ordered collections that can be indexed, sorted and shuffled."),
    ("411_04_maps", "data-structures.md", "Maps", "Collections of key-value pairs."),
    ("411_05_priority", "data-structures.md", "Priority queues", "Collections that hand out values in order of their priority."),
    ("411_06_grid", "data-structures.md", "Grids", "Two-dimensional arrays with operations on regions."),
    ("412_01_1explosions", "particles.md", "Simple effects", "Ready-made effects such as explosions, smoke, rain and snow."),
    ("412_01_types", "particles.md", "Particle types", "The shape, size, color, motion and life of particles."),
    ("412_02_systems", "particles.md", "Particle systems", "Creating systems that hold particles, and drawing and updating them."),
    ("412_03_emitters", "particles.md", "Emitters", "Creating particles in a region, in bursts or in streams."),
    ("412_04_attractors", "particles.md", "Attractors", "Pulling particles towards a point or pushing them away."),
    ("412_05_destroyers", "particles.md", "Destroyers", "Removing particles that enter a region."),
    ("412_06_deflectors", "particles.md", "Deflectors", "Making particles bounce off a region."),
    ("412_07_changers", "particles.md", "Changers", "Turning particles of one type into another type inside a region."),
    ("413_01_connection", "multiplayer.md", "Setting up a connection", "Choosing how the computers of the players are connected."),
    ("413_02_sessions", "multiplayer.md", "Sessions", "Creating, finding and joining game sessions."),
    ("413_03_players", "multiplayer.md", "Players", "The players in a session."),
    ("413_04_shared", "multiplayer.md", "Shared data", "Values that all players of a session share."),
    ("413_05_messages", "multiplayer.md", "Messages", "Sending messages to other players and receiving them."),
    ("414_00_dlls", "dlls.md", "Calling DLL functions", "Defining functions in external DLLs and calling them."),
    ("415_01_mode", "3d-graphics.md", "3D mode", "Turning 3D mode on and setting how things are drawn."),
    ("415_02_drawing", "3d-graphics.md", "Easy drawing in 3D", "Giving ordinary 2D drawing a depth."),
    ("415_03_primitives", "3d-graphics.md", "Drawing polygons", "Primitives built from vertices, with colors, textures and normals."),
    ("415_04_shapes", "3d-graphics.md", "Drawing basic shapes", "Blocks, cylinders, cones, ellipsoids, walls and floors."),
    ("415_05_projection", "3d-graphics.md", "Viewing the world", "Projections: where the camera is and where it looks."),
    ("415_06_transform", "3d-graphics.md", "Transformations", "Translating, rotating and scaling what is drawn, and the transformation stack."),
    ("415_07_fog", "3d-graphics.md", "Fog", "Fading distant things into a color."),
    ("415_08_light", "3d-graphics.md", "Lighting", "Light sources and shading."),
    ("415_09_1models", "3d-graphics.md", "Models", "Building, saving, loading and drawing 3D models."),
];

/// Words that must not appear in the manual (case-insensitive), apart from built-in
/// identifiers that contain them.
pub const FORBIDDEN_WORDS: &[&str] = &["gamemaker", "game maker", "yoyo"];

/// The first forbidden word in `text` (see [`FORBIDDEN_WORDS`]) that is not part of
/// an identifier for which `allowed` holds, with its byte offset.
pub fn forbidden_word(text: &str, allowed: &dyn Fn(&str) -> bool) -> Option<(usize, String)> {
    let lower = text.to_ascii_lowercase();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let b = text.as_bytes();
    for w in FORBIDDEN_WORDS {
        for (at, _) in lower.match_indices(w) {
            let mut s = at;
            while s > 0 && ident(b[s - 1]) {
                s -= 1;
            }
            let mut e = at + w.len();
            while e < b.len() && ident(b[e]) {
                e += 1;
            }
            if !allowed(&text[s..e]) {
                return Some((at, text[s..e].to_string()));
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct JArray {
    min: Option<i64>,
    max: Option<i64>,
    note: Option<String>,
}

#[derive(Deserialize)]
struct JEntry {
    name: String,
    kind: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    variadic: bool,
    #[serde(default)]
    readonly: bool,
    scope: Option<String>,
    array: Option<JArray>,
    chapter: String,
    page: Option<String>,
    description: String,
    value: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct JFile {
    entries: Vec<JEntry>,
}

/// Rewritten descriptions (identifiers and descriptions that name the original
/// product, or refer to its documentation).
fn description(e: &JEntry) -> String {
    let fixed = match e.name.as_str() {
        "local" => "Special value listed alongside self, other and global; its meaning is not documented.",
        "gamemaker_pro" => "Read-only flag telling whether the game was made with the full (paid) edition of the original tool rather than the free one.",
        "gamemaker_registered" => "Same value as gamemaker_pro.",
        "gamemaker_version" => "Read-only version number of the engine (810 to 819 for version 8.1); compare it against ranges rather than exact values.",
        "YoYo_GetPlatform" => "Undocumented extension function that reports the platform the game runs on.",
        "YoYo_EnableAlphaBlend" => "Undocumented extension function that turns alpha blending on or off.",
        "get_function_address" => "Returns the address of a built-in function so that DLLs can call it directly.",
        "d3d_set_zwriteenable" => "Undocumented; turns writing to the depth buffer on or off.",
        "path_scale" if e.kind == "variable" => "Scale of the path the instance follows (1 is the original size).",
        _ => "",
    };
    let mut d = if fixed.is_empty() {
        e.description.clone()
    } else {
        fixed.to_string()
    };
    if e.kind == "variable" && e.readonly {
        // (the kind line says so already)
        for p in ["Read-only flag: ", "Read-only ", "read-only "] {
            if let Some(rest) = d.strip_prefix(p) {
                d = upper_first(rest);
            }
        }
        for p in ["; read-only.", ", read-only."] {
            if let Some(rest) = d.strip_suffix(p) {
                d = format!("{rest}.");
            }
        }
    }
    // references to the specs
    while let Some(i) = d.find(" (see ") {
        let Some(j) = d[i..].find(')') else { break };
        d.replace_range(i..i + j + 1, "");
    }
    assert!(
        forbidden_word(&d, &|w| w.starts_with("gamemaker_")
            || w.starts_with("YoYo_"))
        .is_none(),
        "the description of {} names a third-party product: {d}",
        e.name
    );
    escape_md(&d)
}

/// Escapes Markdown characters that would otherwise change the text (`*`, `_` inside
/// words is fine on GitHub, `#` at the start of a line cannot happen here).
fn escape_md(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '*' | '\\' | '`' | '<' | '>' | '[' | ']' | '|' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Readable argument names.
fn arg_names(e: &JEntry) -> Vec<String> {
    let n = e.name.as_str();
    let ind = if n.starts_with("part_type_") {
        "type"
    } else if n.starts_with("part_system_") || n.starts_with("part_particles_") {
        "system"
    } else if n.starts_with("part_emitter_") {
        "emitter"
    } else if n.starts_with("part_attractor_") {
        "attractor"
    } else if n.starts_with("part_destroyer_") {
        "destroyer"
    } else if n.starts_with("part_deflector_") {
        "deflector"
    } else if n.starts_with("part_changer_") {
        "changer"
    } else if n.starts_with("d3d_model_") {
        "model"
    } else if n.starts_with("d3d_light_") {
        "light"
    } else if n.starts_with("sprite_") {
        "sprite"
    } else if n.starts_with("sound_") {
        "sound"
    } else if n.starts_with("background_") {
        "background"
    } else if n.starts_with("font_") {
        "font"
    } else if n.starts_with("path_") {
        "path"
    } else if n.starts_with("script_") {
        "script"
    } else if n.starts_with("timeline_") {
        "timeline"
    } else if n.starts_with("object_") {
        "obj"
    } else if n.starts_with("room_") {
        "room"
    } else {
        "index"
    };
    let special: Option<&[&str]> = match n {
        "mp_grid_clear_cell" | "mp_grid_add_cell" => Some(&["id", "h", "v"]),
        "highscore_set_colors" => Some(&["back", "new", "other"]),
        _ => None,
    };
    if let Some(s) = special {
        return s.iter().map(|s| s.to_string()).collect();
    }
    let renamed: Vec<String> = e
        .args
        .iter()
        .map(|a| {
            match a.as_str() {
                "ind" => ind,
                "numb" => "number",
                "fname" => "filename",
                "col" => "color",
                "w" => "width",
                "h" => "height",
                "ps" => "system",
                "snd" => "sound",
                "val" => "value",
                "prec" => "precision",
                "sep" => "separation",
                "dir" => "direction",
                "spr" => "sprite",
                "back" => "background",
                "vis" => "visible",
                "rot" => "rotation",
                "prio" => "priority",
                "curs" => "cursor",
                "capt" => "caption",
                "animat" => "animate",
                "def" => "default",
                "dname" => "dirname",
                "evnumb" => "event_number",
                "evtype" => "event_type",
                "imgnumb" => "image_count",
                "argnumb" => "arg_count",
                "millisec" => "milliseconds",
                "texid" => "texture",
                "pers" => "persistent",
                "xm" => "xcenter",
                "ym" => "ycenter",
                "xg" => "xgoal",
                "yg" => "ygoal",
                other => other,
            }
            .to_string()
        })
        .collect();
    let unique: HashSet<&String> = renamed.iter().collect();
    if unique.len() == renamed.len() {
        renamed
    } else {
        e.args.clone()
    }
}

/// Argument names that are renamed in the description too (when they appear there
/// as whole words they always mean the argument).
const RENAME_IN_TEXT: &[&str] = &[
    "ind", "numb", "fname", "col", "w", "h", "ps", "snd", "val", "prec", "spr", "texid", "xm",
    "ym", "xg", "yg", "evnumb", "evtype", "imgnumb", "argnumb", "dname", "pers", "curs", "capt",
    "millisec",
];

/// Replaces the original argument names `from` by `to` in `text` (whole words).
fn rename_in_description(text: &str, from: &[String], to: &[String]) -> String {
    let mut out = text.to_string();
    for (a, b) in from.iter().zip(to) {
        if a == b || !RENAME_IN_TEXT.contains(&a.as_str()) {
            continue;
        }
        let mut res = String::new();
        let bytes = out.as_bytes();
        let mut i = 0;
        let is_id = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
        while i < out.len() {
            if out[i..].starts_with(a.as_str())
                && (i == 0 || !is_id(bytes[i - 1]))
                && bytes.get(i + a.len()).is_none_or(|&c| !is_id(c))
            {
                res.push_str(b);
                i += a.len();
            } else {
                let ch = out[i..].chars().next().unwrap();
                res.push(ch);
                i += ch.len_utf8();
            }
        }
        out = res;
    }
    out
}

/// GitHub's heading anchor of `text`.
pub fn slug(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c == ' ' {
                Some('-')
            } else if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

fn constants_title(section: &str) -> String {
    format!("Constants: {}", section.to_lowercase())
}

fn value_text(v: &Option<serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => format!("`\"{s}\"`"),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => "unknown".to_string(),
    }
}

fn variable_line(e: &JEntry) -> String {
    let mut parts = vec![match e.scope.as_deref() {
        Some("instance") => "Instance variable".to_string(),
        Some("script") => "Script variable".to_string(),
        _ => "Global variable".to_string(),
    }];
    if e.readonly {
        parts.push("read-only".into());
    }
    if let Some(a) = &e.array {
        let range = match (a.min, a.max) {
            (Some(lo), Some(hi)) => format!("array, indices {lo} to {hi}"),
            _ => match &a.note {
                Some(n) => format!("array, indices {}", n.replace("..", " to ")),
                None => "array".to_string(),
            },
        };
        parts.push(range);
    }
    format!("*{}.*", parts.join(", "))
}

/// The page code of an entry ("405_04_shapes"); entries without a page go to a
/// fitting page of their chapter.
fn page_of(e: &JEntry) -> String {
    if e.name == "path_scale" && e.kind == "variable" {
        // (listed with the path functions of chapter 409 by mistake)
        return "403_02_paths".into();
    }
    match &e.page {
        Some(p) => p.trim_end_matches(".html").to_string(),
        None => match e.chapter.as_str() {
            "405" => "405_06_advanced".into(),
            "415" => "415_01_mode".into(),
            c => panic!("{}: no page and no fallback for chapter {c}", e.name),
        },
    }
}

/// All reference files: file name (inside `docs/manual/reference/`) → content.
pub fn generate(builtins_json: &str) -> BTreeMap<String, String> {
    let file: JFile = serde_json::from_str(builtins_json).expect("parse builtins.json");
    let mut by_page: BTreeMap<String, Vec<&JEntry>> = BTreeMap::new();
    for e in &file.entries {
        let p = page_of(e);
        assert!(
            SECTIONS.iter().any(|s| s.0 == p),
            "{}: page {p} has no section",
            e.name
        );
        by_page.entry(p).or_default().push(e);
    }
    let mut out = BTreeMap::new();
    // name → (file, anchor, kind)
    let mut index: Vec<(String, String, String, &str)> = Vec::new();
    for &(gfile, gtitle, gintro) in GROUPS {
        let mut s = String::new();
        let mut anchors: HashSet<String> = HashSet::new();
        let add_anchor = |a: String, anchors: &mut HashSet<String>| {
            assert!(anchors.insert(a.clone()), "{gfile}: duplicate anchor #{a}");
            a
        };
        let _ = writeln!(s, "# {gtitle}\n");
        let _ = writeln!(s, "{gintro}\n");
        let _ = writeln!(
            s,
            "This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.\n"
        );
        add_anchor(slug(gtitle), &mut anchors);
        let sections: Vec<_> = SECTIONS.iter().filter(|s| s.1 == gfile).collect();
        for &&(code, _, title, _) in &sections {
            if by_page.contains_key(code) {
                let _ = writeln!(s, "- [{title}](#{})", slug(title));
            }
        }
        s.push('\n');
        for &&(code, _, title, intro) in &sections {
            let Some(entries) = by_page.get(code) else {
                continue;
            };
            let _ = writeln!(s, "## {title}\n");
            add_anchor(slug(title), &mut anchors);
            let _ = writeln!(s, "{intro}\n");
            for e in entries.iter().filter(|e| e.kind != "constant") {
                let _ = writeln!(s, "### {}\n", e.name);
                let a = add_anchor(slug(&e.name), &mut anchors);
                if e.kind == "function" {
                    let mut args = arg_names(e);
                    let desc = rename_in_description(&description(e), &e.args, &args);
                    if e.variadic {
                        args.push("...".into());
                    }
                    let _ = writeln!(s, "`{}({})`\n", e.name, args.join(", "));
                    let _ = writeln!(s, "{desc}\n");
                    index.push((e.name.clone(), gfile.to_string(), a, "function"));
                } else {
                    let _ = writeln!(s, "{} {}\n", variable_line(e), description(e));
                    index.push((e.name.clone(), gfile.to_string(), a, "variable"));
                }
            }
            let consts: Vec<_> = entries.iter().filter(|e| e.kind == "constant").collect();
            if !consts.is_empty() {
                let ct = constants_title(title);
                let _ = writeln!(s, "### {ct}\n");
                let a = add_anchor(slug(&ct), &mut anchors);
                let _ = writeln!(s, "| Constant | Value | Meaning |");
                let _ = writeln!(s, "| --- | --- | --- |");
                for e in consts {
                    let _ = writeln!(
                        s,
                        "| `{}` | {} | {} |",
                        e.name,
                        value_text(&e.value),
                        description(e)
                    );
                    index.push((e.name.clone(), gfile.to_string(), a.clone(), "constant"));
                }
                s.push('\n');
            }
        }
        out.insert(gfile.to_string(), s.trim_end().to_string() + "\n");
    }
    // README
    let mut r = String::new();
    r.push_str("# Function reference\n\n");
    r.push_str(
        "Every built-in function, variable and constant of GML, grouped by topic. Each group \
         page starts with a list of its sections; inside a section, functions and variables \
         each have their own heading, and constants are listed in a table at the end of the \
         section.\n\n",
    );
    r.push_str(
        "To find a name quickly, use the [A–Z index](index.md), or type it into the search \
         box of the help window. In the code editor, put the text cursor on a name and press \
         **F1** to open its entry.\n\n",
    );
    r.push_str("## Groups\n\n");
    for &(gfile, gtitle, gintro) in GROUPS {
        let _ = writeln!(r, "- [{gtitle}]({gfile}): {}", lower_first(gintro));
    }
    r.push_str("- [All functions, variables and constants (A–Z)](index.md)\n");
    out.insert("README.md".to_string(), r);
    // index
    index.sort_by(|a, b| {
        a.0.to_lowercase()
            .cmp(&b.0.to_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
    let mut x = String::new();
    x.push_str("# All functions, variables and constants (A–Z)\n\n");
    x.push_str(
        "Every built-in name in alphabetical order, with a link to its entry in the \
         [function reference](README.md).\n\n",
    );
    let mut letter = None;
    let mut letters = Vec::new();
    let mut body = String::new();
    for (name, f, a, kind) in &index {
        let l = name.chars().next().unwrap().to_ascii_uppercase();
        if letter != Some(l) {
            letter = Some(l);
            letters.push(l);
            let _ = writeln!(body, "\n## {l}\n");
        }
        let _ = writeln!(body, "- [{name}]({f}#{a}) ({kind})");
    }
    let nav: Vec<String> = letters
        .iter()
        .map(|l| format!("[{l}](#{})", l.to_ascii_lowercase()))
        .collect();
    let _ = writeln!(x, "{}", nav.join(" · "));
    x.push_str(&body);
    out.insert("index.md".to_string(), x);
    out
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// `<workspace>/specs/builtins.json` and `<workspace>/docs/manual/reference`.
pub fn paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    (
        root.join("specs/builtins.json"),
        root.join("docs/manual/reference"),
    )
}

#[allow(dead_code)]
fn main() {
    let (json, dir) = paths();
    let src = std::fs::read_to_string(&json).expect("read specs/builtins.json");
    std::fs::create_dir_all(&dir).expect("create docs/manual/reference");
    for (name, text) in generate(&src) {
        std::fs::write(dir.join(&name), text).expect("write reference file");
        println!("wrote {}", dir.join(&name).display());
    }
}
