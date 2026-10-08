//! Readable one-line descriptions of built-in actions for the action list.
//!
//! The library list templates are not bundled (specs/actions.json only lists which
//! placeholders they use), so these lines are written for this project. Tokens:
//! `{0}`..`{7}` argument values, `{N}` "not " when a question is negated, `{r}`
//! "relative " when the action is relative.

/// Template of built-in action `id`, if there is one.
pub fn template(id: i32) -> Option<&'static str> {
    Some(match id {
        // move
        101 => "Start moving in a direction with {r}speed {1}",
        102 => "Start moving in direction {0} with {r}speed {1}",
        105 => "Start moving towards {r}position ({0},{1}) with speed {2}",
        103 => "Set the horizontal speed {r}to {0}",
        104 => "Set the vertical speed {r}to {0}",
        107 => "Set the gravity {r}to {1} in direction {0}",
        113 => "Reverse the horizontal direction",
        114 => "Reverse the vertical direction",
        108 => "Set the friction {r}to {0}",
        109 => "Jump {r}to position ({0},{1})",
        110 => "Jump to the start position",
        111 => "Jump to a random position",
        117 => "Align the position to a grid of {0} by {1}",
        112 => "Wrap {0} when an instance moves outside the room",
        116 => "Move in direction {0} at most {1} until a contact with {2}",
        115 => "Bounce {0} against {1}",
        119 => "Set the path to {0} with speed {1}",
        124 => "End the path",
        122 => "Set the path position {r}to {0}",
        123 => "Set the path speed {r}to {0}",
        120 => "Step towards {r}position ({0},{1}) with speed {2}",
        121 => "Step towards {r}position ({0},{1}) avoiding {3}",
        // main1
        201 => "Create an instance of object {0} at {r}position ({1},{2})",
        206 => "Create an instance of object {0} at {r}position ({1},{2}) with speed {3} in direction {4}",
        207 => "Create an instance of {0}, {1}, {2} or {3} at {r}position ({4},{5})",
        202 => "Change the instance into {0}",
        203 => "Destroy the instance",
        204 => "Destroy the instances at {r}position ({0},{1})",
        541 => "Set the sprite to {0} with subimage {1} and speed {2}",
        542 => "Scale the sprite by {0} by {1}, angle {2}, {3}",
        543 => "Blend the sprite with {0}, alpha {1}",
        205 => "Set the sprite to {0} with scale {1}",
        211 => "Play sound {0}; looping: {1}",
        212 => "Stop sound {0}",
        213 => "If sound {0} is {N}playing",
        221 => "Go to the previous room",
        222 => "Go to the next room",
        223 => "Restart the current room",
        224 => "Go to room {0}",
        225 => "If there is {N}a previous room",
        226 => "If there is {N}a next room",
        // main2
        301 => "Set {1} {r}to {0}",
        302 => "Sleep {0} milliseconds",
        303 => "Set the time line to {0} at position {1}",
        305 => "Set the time line to {0} at position {1}",
        304 => "Set the time line position {r}to {0}",
        309 => "Set the time line speed {r}to {0}",
        306 => "Start the time line",
        307 => "Pause the time line",
        308 => "Stop and reset the time line",
        321 => "Display message: {0}",
        322 => "Show the game information",
        323 => "Show video {0}",
        324 => "Show the text in {0}",
        325 => "Show the image in {0}",
        326 => "Show the webpage {0}",
        327 => "Show the video {0}",
        328 => "Change the splash settings",
        331 => "Restart the game",
        332 => "End the game",
        333 => "Save the game in file {0}",
        334 => "Load the game from file {0}",
        803 => "Replace sprite {0} with the images in {1}",
        804 => "Replace sound {0} with the file {1}",
        805 => "Replace background {0} with the image in {1}",
        // control
        401 => "If {r}position ({0},{1}) is {N}collision free for {2}",
        402 => "If there is {N}a collision at {r}position ({0},{1}) with {2}",
        403 => "If at {r}position ({1},{2}) there is {N}object {0}",
        404 => "If the number of instances of {0} is {N}{2} {1}",
        405 => "With a chance of 1 out of {0} {N}do the next action",
        407 => "If the player does {N}say yes to: {0}",
        408 => "If expression {0} is {N}true",
        409 => "If mouse button {0} is {N}pressed",
        410 => "If the instance is {N}aligned with a grid of {0} by {1}",
        422 => "Start of a block",
        421 => "Else",
        425 => "Exit this event",
        424 => "End of a block",
        423 => "Repeat the next action {0} times",
        604 => "Call the inherited event",
        603 => "Execute a piece of code",
        601 => "Execute script: {0}",
        605 => "{0}",
        611 => "Set variable {0} {r}to {1}",
        612 => "If {0} is {N}{2} {1}",
        613 => "At {r}position ({1},{2}) draw the value of: {0}",
        // score
        701 => "Set the score {r}to {0}",
        702 => "If the score is {N}{1} {0}",
        703 => "At {r}position ({0},{1}) draw the score with caption {2}",
        709 => "Show the highscore table",
        707 => "Clear the highscore table",
        711 => "Set the lives {r}to {0}",
        712 => "If the lives are {N}{1} {0}",
        713 => "At {r}position ({0},{1}) draw the lives with caption {2}",
        714 => "At {r}position ({0},{1}) draw the lives with image {2}",
        721 => "Set the health {r}to {0}",
        722 => "If the health is {N}{1} {0}",
        723 => "Draw a health bar from {r}({0},{1}) to ({2},{3})",
        731 => "Set the information shown in the window caption",
        // extra
        820 => "Create the particle system at depth {0}",
        821 => "Destroy the particle system",
        822 => "Clear all particles of the system",
        825 => "Create particle {0} with shape {1}",
        823 => "Create particle {0} with shape {1}",
        824 => "Set the color of particle {0}",
        826 => "Set the life of particle {0} to {1}-{2} steps",
        827 => "Set the speed of particle {0}",
        828 => "Set the gravity of particle {0} to {1} in direction {2}",
        829 => "Set the secondary particles of {0}",
        831 => "Create emitter {0} with shape {1}",
        832 => "Destroy emitter {0}",
        833 => "Burst {2} particles of {1} from emitter {0}",
        834 => "Stream {2} particles of {1} from emitter {0}",
        808 => "Play the CD from track {0} to {1}",
        809 => "Stop the CD",
        810 => "Pause the CD",
        811 => "Resume the CD",
        812 => "If there is {N}a CD in the drive",
        813 => "If the CD is {N}playing",
        801 => "Set the mouse cursor to {0}",
        807 => "Open webpage {0}",
        // draw
        501 => "Draw sprite {0} at {r}position ({1},{2}) with subimage {3}",
        502 => "Draw background {0} at {r}position ({1},{2})",
        514 => "At {r}position ({1},{2}) draw text: {0}",
        519 => "At {r}position ({1},{2}) draw scaled text: {0}",
        511 => "Draw a {r}rectangle from ({0},{1}) to ({2},{3})",
        516 => "Draw a {r}horizontal gradient from ({0},{1}) to ({2},{3})",
        517 => "Draw a {r}vertical gradient from ({0},{1}) to ({2},{3})",
        512 => "Draw an {r}ellipse from ({0},{1}) to ({2},{3})",
        518 => "Draw a {r}gradient ellipse from ({0},{1}) to ({2},{3})",
        513 => "Draw a {r}line from ({0},{1}) to ({2},{3})",
        515 => "Draw an {r}arrow from ({0},{1}) to ({2},{3})",
        524 => "Set the drawing color to {0}",
        526 => "Set the font for drawing text to {0}, align {1}",
        531 => "Full screen mode: {0}",
        802 => "Take a snapshot image into {0}",
        532 => "Create an effect ({0}) at {r}position ({1},{2})",
        _ => return None,
    })
}

/// Fill `template`. `arg(i)` gives the display value of argument `i`.
pub fn fill(template: &str, arg: &dyn Fn(usize) -> String, negate: bool, relative: bool) -> String {
    let mut out = String::new();
    let mut used_r = false;
    let mut used_n = false;
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        let Some(j) = tail.find('}') else {
            out.push_str(&rest[i..]);
            rest = "";
            break;
        };
        match &tail[..j] {
            "N" => {
                used_n = true;
                if negate {
                    out.push_str("not ");
                }
            }
            "r" => {
                used_r = true;
                if relative {
                    out.push_str("relative ");
                }
            }
            t => match t.parse::<usize>() {
                Ok(n) => out.push_str(&arg(n)),
                Err(_) => {
                    out.push('{');
                    out.push_str(t);
                    out.push('}');
                }
            },
        }
        rest = &tail[j + 1..];
    }
    out.push_str(rest);
    if negate && !used_n {
        out = format!("NOT {out}");
    }
    if relative && !used_r {
        out.push_str(" (relative)");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_tokens() {
        let a = |i: usize| ["x", "3", "equal to"][i].to_string();
        assert_eq!(
            fill(template(612).unwrap(), &a, true, false),
            "If x is not equal to 3"
        );
        assert_eq!(
            fill(template(611).unwrap(), &a, false, true),
            "Set variable x relative to 3"
        );
        assert_eq!(fill("Go", &a, false, true), "Go (relative)");
    }

    #[test]
    fn every_builtin_action_has_a_template() {
        for l in gm_format::builtin_libraries() {
            for a in &l.actions {
                if !a.is_decoration() {
                    assert!(template(a.action_id).is_some(), "{}", a.palette_title);
                }
            }
        }
    }
}
