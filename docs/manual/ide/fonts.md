# Fonts

A **font** resource describes a typeface for drawing text in your game: which font,
how large, bold or italic, and which characters to include. Without a font resource,
text is drawn in the default font (Arial, 12 points).

Create one with **Resources > Create Font** and double-click it in the tree to open
the **Font Properties** form.

## The font form

| Field | What it does |
|---|---|
| Name | The name you use in actions and code. |
| Font | The name of an installed font, such as `Arial` or `Verdana`. Type any name, or choose a common one from the **…** list. |
| Size | The size in points. |
| Style | **Bold** and **Italic**. |
| Anti-aliasing | Smoothing of the letter edges, from 0 (none, crisp pixels) to 3 (smoothest). |
| Character set | The character set to use, for example `ANSI_CHARSET`, `DEFAULT_CHARSET`, `RUSSIAN_CHARSET` or `SHIFTJIS_CHARSET`. |
| Character range | The first and last character code to include ("from … till …"). |
| Normal, All, Digits, Letters | Quick ranges: 32–127 (printable ASCII), 0–255, 48–57 (digits only) and 65–122 (letters). |
| OK, Cancel | Apply or discard your changes and close the form. |

Below the settings is a sample text box. Type something to see it at the chosen size
and style.

> **Note:** The preview uses the IDE's own font, so it shows the size and style but
> not the typeface itself. Run the game to see the real font.

## How fonts are used

The project stores only the font's settings, not the font file. The game looks the
font up on the computer it runs on when it starts, and turns the characters in the
range into images. If the font isn't installed there, Instance uses its bundled
fallback font (Liberation Sans). For a game you share, stick to fonts that are widely
installed, or check that the fallback looks acceptable.

Only characters inside the range can be drawn. Keep the range small (the Normal range
covers English text) unless you need more: every character takes memory.

## Drawing text

Choose a font with the **Set Font** action (draw tab) or
[`draw_set_font`](../reference/game-graphics.md#draw_set_font), then draw text in a
Draw event:

```gml
draw_set_font(fnt_title);
draw_set_color(c_white);
draw_text(16, 16, "Level " + string(level));
```

See also: [Drawing](../runtime/drawing.md) · [Actions: draw](actions.md#draw)
