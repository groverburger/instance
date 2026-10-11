# Splash screens, messages and highscores

Showing splash screens and information, asking the player questions, and keeping a highscore list.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Splash screens](#splash-screens)
- [Pop-up messages and questions](#pop-up-messages-and-questions)
- [The highscore list](#the-highscore-list)

## Splash screens

Showing videos, text, web pages and images, and the game information.

### splash_show_video

`splash_show_video(filename, loop)`

Shows a video file as a splash screen, optionally looping; supported formats depend on installed codecs.

### splash_show_text

`splash_show_text(filename, delay)`

Shows a txt or rtf file as a splash screen for a delay in ms; zero or negative waits for Escape or click.

### splash_show_web

`splash_show_web(url, delay)`

Shows a web page (URL or full local path) as a splash screen for a delay in milliseconds.

### splash_show_image

`splash_show_image(filename, delay)`

Shows a static image file as a splash screen for a delay in milliseconds.

### splash_set_caption

`splash_set_caption(cap)`

Sets the caption of a separate splash window.

### splash_set_fullscreen

`splash_set_fullscreen(full)`

Chooses whether a separate splash window is full screen; default is normal window.

### splash_set_border

`splash_set_border(border)`

Chooses whether a separate splash window has a border; default true.

### splash_set_size

`splash_set_size(width, height)`

Sets the size of a separate splash window; default 640x480.

### splash_set_position

`splash_set_position(x, y)`

Sets the screen position of a separate splash window; default is chosen by Windows.

### splash_set_adapt

`splash_set_adapt(adapt)`

Chooses whether the splash window resizes to the scaled video or image; default true.

### splash_set_top

`splash_set_top(top)`

Chooses whether the splash window stays above other windows; default true.

### splash_set_color

`splash_set_color(color)`

Sets the color of the area surrounding the splash image or video.

### splash_set_main

`splash_set_main(main)`

Chooses whether splash shows in the main game window (true, default) or a separate window.

### splash_set_scale

`splash_set_scale(scale)`

Sets the scale for splash video or image; 0 (default) scales to fill the window.

### splash_set_cursor

`splash_set_cursor(visible)`

Sets whether the mouse cursor is visible during splash; default visible, cannot be hidden for movies.

### splash_set_interrupt

`splash_set_interrupt(interrupt)`

Chooses whether game play pauses while a separate splash window is shown; default true.

### splash_set_stop_key

`splash_set_stop_key(stop)`

Chooses whether pressing Escape ends the splash screen; default true.

### splash_set_stop_mouse

`splash_set_stop_mouse(stop)`

Chooses whether a mouse click in the splash ends it; default true.

### splash_set_close_button

`splash_set_close_button(show)`

Chooses whether a close button appears at the splash top right; default true.

### show_info

`show_info()`

Displays the game information window.

### load_info

`load_info(filename)`

Loads game information from an rtf file (no images) so different help text can be shown.

## Pop-up messages and questions

Messages, questions, menus and file dialogs, and how they look.

### show_message

`show_message(str)`

Shows a message dialog with the given text.

### show_message_ext

`show_message_ext(str, but1, but2, but3)`

Shows a message with up to three buttons (empty text hides one, & marks shortcut); returns pressed button number, 0 for Esc.

### show_question

`show_question(str)`

Shows a yes/no question; returns true if yes is chosen.

### get_integer

`get_integer(str, default)`

Prompts the player for a number with a default value and returns it.

### get_string

`get_string(str, default)`

Prompts the player for a string with a default value and returns it.

### message_background

`message_background(background)`

Sets a background resource as the image behind pop-up dialogs; alpha ignored, but transparent bottom-left pixel color is keyed out.

### message_button

`message_button(sprite)`

Sets the three-image sprite (normal, hover, pressed) used for dialog buttons.

### message_alpha

`message_alpha(alpha)`

Sets pop-up translucency from 0 (fully translucent) to 1 (opaque).

### message_text_font

`message_text_font(name, size, color, style)`

Sets font name, size, color and style (0 normal, 1 bold, 2 italic, 3 both) for dialog text.

### message_button_font

`message_button_font(name, size, color, style)`

Sets font name, size, color and style for dialog button text.

### message_input_font

`message_input_font(name, size, color, style)`

Sets font name, size, color and style for the dialog input field.

### message_text_charset

`message_text_charset(type, charset_id)`

Selects the character set used by dialogs for a given text type.

### message_mouse_color

`message_mouse_color(color)`

Sets the button font color used when the mouse is over a dialog button.

### message_input_color

`message_input_color(color)`

Sets the background color of the dialog input field.

### message_position

`message_position(x, y)`

Sets the screen position of pop-up dialogs; -1,-1 centers them.

### message_size

`message_size(width, height)`

Fixes dialog size; -1 width uses the image width, -1 height depends on line count.

### message_caption

`message_caption(show, str)`

Sets whether dialogs show a border (1/0) and the caption text.

### show_menu

`show_menu(str, default)`

Shows a popup menu from a bar-separated string; returns chosen index or the default if none.

### show_menu_pos

`show_menu_pos(x, y, str, default)`

Same as show_menu but displayed at screen position x,y.

### get_color

`get_color(defcol)`

Asks the player to pick a color starting from a default; returns -1 if cancelled.

### get_open_filename

`get_open_filename(filter, filename)`

Shows an open-file dialog with name\|mask\|... filter; returns empty string if cancelled.

### get_save_filename

`get_save_filename(filter, filename)`

Shows a save-file dialog with a filter; returns empty string if cancelled.

### get_directory

`get_directory(dirname)`

Asks for a directory starting at a default name; returns empty string if cancelled.

### get_directory_alt

`get_directory_alt(caption, root)`

Alternative directory chooser with caption and tree root (empty for all); empty string if cancelled.

### show_error

`show_error(str, abort)`

Shows a standard error message and/or logs it, optionally aborting the game.

### Constants: pop-up messages and questions

| Constant | Value | Meaning |
| --- | --- | --- |
| `text_type` | unknown | Constant selecting which dialog element (text, button or input field) message_text_charset applies to. |
| `button_type` | unknown | Constant selecting which dialog element (text, button or input field) message_text_charset applies to. |
| `input_type` | unknown | Constant selecting which dialog element (text, button or input field) message_text_charset applies to. |
| `ANSI_CHARSET` | 0 | Windows character-set constant usable with message_text_charset. |
| `DEFAULT_CHARSET` | 1 | Windows character-set constant usable with message_text_charset. |
| `EASTEUROPE_CHARSET` | 238 | Windows character-set constant usable with message_text_charset. |
| `RUSSIAN_CHARSET` | 204 | Windows character-set constant usable with message_text_charset. |
| `SYMBOL_CHARSET` | 2 | Windows character-set constant usable with message_text_charset. |
| `SHIFTJIS_CHARSET` | 128 | Windows character-set constant usable with message_text_charset. |
| `HANGEUL_CHARSET` | 129 | Windows character-set constant usable with message_text_charset. |
| `GB2312_CHARSET` | 134 | Windows character-set constant usable with message_text_charset. |
| `CHINESEBIG5_CHARSET` | 136 | Windows character-set constant usable with message_text_charset. |
| `JOHAB_CHARSET` | 130 | Windows character-set constant usable with message_text_charset. |
| `HEBREW_CHARSET` | 177 | Windows character-set constant usable with message_text_charset. |
| `ARABIC_CHARSET` | 178 | Windows character-set constant usable with message_text_charset. |
| `GREEK_CHARSET` | 161 | Windows character-set constant usable with message_text_charset. |
| `TURKISH_CHARSET` | 162 | Windows character-set constant usable with message_text_charset. |
| `VIETNAMESE_CHARSET` | 163 | Windows character-set constant usable with message_text_charset. |
| `THAI_CHARSET` | 222 | Windows character-set constant usable with message_text_charset. |
| `MAC_CHARSET` | 77 | Windows character-set constant usable with message_text_charset. |
| `BALTIC_CHARSET` | 186 | Windows character-set constant usable with message_text_charset. |
| `OEM_CHARSET` | 255 | Windows character-set constant usable with message_text_charset. |

## The highscore list

Keeping, showing and drawing the highscore list.

### highscore_show_ext

`highscore_show_ext(number, background, border, col1, col2, name, size)`

Shows the highscore table with custom background, border, colors, font name and size; number -1 only displays.

### highscore_show

`highscore_show(number)`

Shows the highscore table, letting the player enter a name if number qualifies; -1 just displays.

### highscore_set_background

`highscore_set_background(background)`

Sets which background resource is used for the highscore table.

### highscore_set_border

`highscore_set_border(show)`

Sets whether the highscore window has a border.

### highscore_set_font

`highscore_set_font(name, size, style)`

Chooses the font name, point size and style (0 regular, 1 bold, 2 italic, 3 bold+italic) of the highscore table.

### highscore_set_strings

`highscore_set_strings(caption, nobody, escape)`

Overrides the caption, empty-slot text and escape hint strings of the table, e.g. for translation.

### highscore_set_colors

`highscore_set_colors(back, new, other)`

Sets background, new-entry and other-entry colors of the table.

### highscore_clear

`highscore_clear()`

Clears all highscore entries.

### highscore_add

`highscore_add(str, number)`

Adds an entry with the given name and score to the list.

### highscore_add_current

`highscore_add_current()`

Adds the current score to the list, asking the player for a name.

### highscore_value

`highscore_value(place)`

Returns the score at the given rank (1 to 10).

### highscore_name

`highscore_name(place)`

Returns the name at the given rank (1 to 10).

### draw_highscore

`draw_highscore(x1, y1, x2, y2)`

Draws the highscore table inside the given box using the current font.
