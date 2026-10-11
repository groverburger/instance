//! Minimal example: build a GameMaker 8.1 project in code and write it as `.gm81`.
//!
//! ```text
//! cargo run -p instance-format --example make_game -- out.gm81
//! instance-runner out.gm81 --headless --frames 60 --screenshot out.png
//! gmk-info out.gm81 --tree
//! ```
//!
//! The project has one sprite, one object (Create / Step / Draw code actions, moved
//! with the arrow keys), one room with one placed instance, a resource tree and a
//! room order.

use instance_format::*;

/// GM 8.1 stores code with CR LF line endings (specs/file-formats.md, "line endings").
fn crlf(code: &str) -> String {
    code.replace("\r\n", "\n").replace('\n', "\r\n")
}

/// An "Execute Code" action (library 1, action 603) holding `code`.
fn code_action(code: &str) -> Action {
    let mut a = builtin_action(1, 603)
        .expect("built-in Execute Code action")
        .instantiate(1);
    a.args[0] = crlf(code);
    a
}

/// A 32x32, 32-bit Windows .ico (the settings store a complete .ico file).
fn make_ico() -> Vec<u8> {
    let (w, h) = (32u32, 32u32);
    let xor_len = w * h * 4;
    let and_len = (w / 8).next_multiple_of(4) * h; // 1-bit mask, rows padded to 32 bits
    let img_len = 40 + xor_len + and_len;
    let mut v = vec![0, 0, 1, 0, 1, 0]; // ICONDIR: reserved, type 1 (icon), 1 image
    v.extend_from_slice(&[w as u8, h as u8, 0, 0, 1, 0, 32, 0]);
    v.extend_from_slice(&img_len.to_le_bytes());
    v.extend_from_slice(&22u32.to_le_bytes()); // offset of the image data
    // BITMAPINFOHEADER (height is doubled: XOR image + AND mask)
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&(w as i32).to_le_bytes());
    v.extend_from_slice(&((h * 2) as i32).to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&32u16.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&(xor_len + and_len).to_le_bytes());
    v.extend_from_slice(&[0; 16]);
    for _ in 0..w * h {
        v.extend_from_slice(&[40, 160, 40, 255]); // BGRA, bottom-up rows
    }
    v.extend(std::iter::repeat(0u8).take(and_len as usize));
    v
}

fn main() -> Result<()> {
    let out = std::env::args().nth(1).unwrap_or_else(|| "make_game.gm81".into());

    let mut p = Project {
        game_id: 424242,
        guid: *b"agent-guide-demo",
        ..Project::default()
    };
    p.settings.icon = make_ico();
    p.settings.author = "instance-format example".into();
    // Throw an error when a script is called with the wrong number of arguments
    // (settings flag bit 1, which the GM 8.1 IDE stored in most user projects).
    p.settings.argument_errors = true;

    // Sprite 0: 32x32 opaque blue square, origin in the centre, rectangle mask.
    let mut img = RgbaImage::new(32, 32);
    for px in img.data.chunks_mut(4) {
        px.copy_from_slice(&[40, 90, 220, 255]); // R, G, B, A
    }
    p.sprites.push(Some(Sprite {
        name: "spr_player".into(),
        origin_x: 16,
        origin_y: 16,
        frames: vec![img],
        collision_shape: 1, // rectangle
        bbox_mode: 0,       // automatic
        bbox_left: 0,
        bbox_right: 31,
        bbox_top: 0,
        bbox_bottom: 31,
        ..Sprite::default()
    }));

    // Object 0: the player.
    let events = vec![
        Event {
            event_type: event_type::CREATE,
            number: 0,
            actions: vec![code_action("move_speed = 4;\nsteps = 0;")],
        },
        Event {
            event_type: event_type::STEP,
            number: 0, // 0 normal, 1 begin, 2 end
            actions: vec![code_action(
                "steps += 1;\n\
                 if (keyboard_check(vk_left))  x -= move_speed;\n\
                 if (keyboard_check(vk_right)) x += move_speed;\n\
                 if (keyboard_check(vk_up))    y -= move_speed;\n\
                 if (keyboard_check(vk_down))  y += move_speed;\n\
                 x = median(16, x, room_width - 16);\n\
                 y = median(16, y, room_height - 16);",
            )],
        },
        Event {
            event_type: event_type::DRAW,
            number: 0,
            // A Draw event replaces the default sprite drawing, so draw it here.
            actions: vec![code_action(
                "draw_sprite(sprite_index, image_index, x, y);\n\
                 draw_set_color(c_white);\n\
                 draw_text(8, 8, 'steps: ' + string(steps));",
            )],
        },
    ];
    p.objects.push(Some(Object {
        name: "obj_player".into(),
        sprite: 0,
        events, // grouped by event type in ascending order
        ..Object::default()
    }));

    // Room 0 with one instance. Placed-instance ids start above 100000.
    p.last_instance_id += 1;
    p.rooms.push(Some(Room {
        name: "rm_main".into(),
        caption: "Agent guide demo".into(),
        width: 320,
        height: 240,
        speed: 30,
        background_color: 0x402020, // 0x00BBGGRR
        instances: vec![Instance {
            x: 160,
            y: 120,
            object: 0,
            id: p.last_instance_id,
            creation_code: String::new(),
            locked: false,
        }],
        ..Room::default()
    }));
    p.room_order = vec![0]; // first entry = start room

    // Resource tree: the 12 standard roots, with leaves for each resource.
    p.resource_tree = TreeNode::standard_roots();
    p.resource_tree[0]
        .children
        .push(TreeNode::leaf(ResourceKind::Sprite, 0, "spr_player"));
    p.resource_tree[7]
        .children
        .push(TreeNode::leaf(ResourceKind::Object, 0, "obj_player"));
    p.resource_tree[8]
        .children
        .push(TreeNode::leaf(ResourceKind::Room, 0, "rm_main"));

    let bytes = write_project(&p, FormatVersion::V810)?;
    // Sanity check: the file reads back to the same model.
    assert!(read_project(&bytes)? == p, "read-back differs");
    std::fs::write(&out, &bytes)?;
    println!("wrote {out} ({} bytes)", bytes.len());
    Ok(())
}
