# IDE functional specification (GameMaker 8.1 compatible editor)

Status: draft v1. Written by a spec writer from the user manual (chapters 1xx, 2xx,
301-302), the bundled tutorials (text only), the shipped action-library files and
the bundled screenshots (described in words only). Everything here is paraphrased.
Items marked **[unverified]** are not stated in the manual and must be confirmed by
black-box observation before being treated as authoritative; they are also filed in
`OPEN_QUESTIONS.md`.

Scope: the *editor* only. Runtime semantics (event order, action behaviour, GML) and
the binary layout of `.gm81` / `.gmk` / `.lib` / `.ged` files belong to other specs.
This document refers to project data through the logical data model in section 2;
the file-format spec is responsible for mapping that model to bytes.

Conventions:
- "Form" = a property window for one resource or one global setting block.
- "Commit" = copy the editor's working copy back into the project model.
- "Std-only" = feature the manual marks as Standard-Edition-only. **Our
  implementation has no editions; treat every feature as available.** (The Lite/
  Standard split, licence activation, upgrade dialogs and the "TV logo" preference
  are out of scope.)

---------------------------------------------------------------------------------

## 1. Overall architecture of the IDE

1. One main window containing: menu bar, toolbar, resource tree (left dock), and a
   workspace area (right) in which resource forms open as child windows (MDI style).
2. Many forms can be open at the same time, each editing a different resource.
   Opening a resource whose form is already open brings that form to front instead
   of opening a second one.
3. Each form edits a **working copy** of its resource. Pressing the form's OK button
   (green check mark, always first item of a form's toolbar or bottom-left button)
   commits and closes. Closing via the window's X button, when the working copy
   differs from the model, asks "save changes?" (Yes = commit+close, No = discard+
   close, Cancel = stay open).
4. Modal dialogs (event selector, action argument dialog, file dialogs, transform
   parameter dialogs, etc.) block their parent form only.
5. The project has a "modified" flag. Commits, tree edits (create/delete/rename/
   reorder/group changes) and global-setting commits set it. Save clears it.
   New/Open/Exit with a modified project ask whether to save.
   (Manual history note: one combined question is asked, not one per open form.)
6. There is no global undo. Deleting resources is irreversible (a confirmation is
   shown). Individual editors have their own undo (room, path, sprite editor, image
   editor, code editor).

### 1.1 Simple mode vs advanced mode

A persistent IDE preference (`advanced_mode: bool`) toggled by File > Advanced Mode
(check-mark menu item). The manual notes that a restart or reload gives the full
effect; our implementation may apply it immediately (re-layout all forms).

Simple mode hides — but never deletes from the project — the following:

| Area | Hidden in simple mode |
|---|---|
| Resource tree | Paths, Scripts, Fonts, Time Lines, Extension Packages |
| File menu | Publish your Game, Import Resources, Export Resources, Preferences |
| Edit menu | Add Group, Find Resource, Expand/Collapse Resource Tree, Show Object Information |
| Resources menu | Create Path/Script/Font/Time Line, Define Constants, Define Triggers, Included Files |
| Scripts menu | entire menu |
| Keyboard shortcuts | shortcuts of hidden items are disabled too |
| Toolbar | buttons for hidden resource types / commands |
| Sprite form | Save Sprite, origin box, collision checking group, Modify Mask |
| Sound form | Save Sound(?) **[unverified]**, kind, effects, volume, pan, preload, Edit Sound |
| Background form | Save Background, "Use as tile set" and tile properties |
| Object form | depth, persistent, parent, mask, Show Information |
| Room form | tabs *tiles* and *views*; settings "Persistent" and "Creation code"; toolbar sort/lock/isometric buttons; instance creation code menu item |
| Action palette | actions flagged advanced-only in their library; tabs whose every action is advanced-only (the shipped `extra` tab) |
| Event selector | Trigger button |
| Global Game Settings | "some options" (manual does not list which) **[unverified]** |

Advanced mode adds everything listed in chapter 2xx. All behaviour below assumes
advanced mode unless noted.

---------------------------------------------------------------------------------

## 2. Logical project data model (what the IDE edits)

Every resource of the nine "list" kinds has: `id` (index within its kind, stable;
deleted slots stay empty so references by index stay valid), `name: String`,
`last_changed: timestamp` (needed by Import "keep last changed"), and a position in
the kind's resource tree. Names should be identifiers (letter first, then letters,
digits, `_`); the IDE does not enforce this but Check Resource Names reports
violations.

```
Project
  sprites:      Vec<Option<Sprite>>
  sounds:       Vec<Option<Sound>>
  backgrounds:  Vec<Option<Background>>
  paths:        Vec<Option<Path>>
  scripts:      Vec<Option<Script>>
  fonts:        Vec<Option<Font>>
  timelines:    Vec<Option<Timeline>>
  objects:      Vec<Option<Object>>
  rooms:        Vec<Option<Room>>
  game_info:    GameInfo
  settings:     GlobalGameSettings
  triggers:     Vec<Option<Trigger>>          + triggers_last_changed
  constants:    Vec<Constant>                  + constants_last_changed
  included_files: Vec<IncludedFile>
  extensions:   Vec<String>                    // names of used packages
  library_init_code / action-library refs      // owned by format spec
  resource_tree: per-kind tree of Group{name, children} | Leaf(id)
  room_order:   derived from the room tree order (first room = start room)
  last_instance_id, last_tile_id               // id allocators for room contents
```

Field lists per resource are given with each editor below (sections 5-15) and
summarised in the mapping table, section 22.

Default names for new resources: `sprite<N>`, `sound<N>`, `background<N>`,
`path<N>`, `script<N>`, `font<N>`, `timeline<N>`, `object<N>`, `room<N>`
(screenshots show `object0`, `room0`, `path0`, … for the first one). How N is
chosen (new id vs. first unused number) is **[unverified]**; recommended: N = new
resource id.

---------------------------------------------------------------------------------

## 3. Main window

### 3.1 Title bar
Shows the project file name (or `<new game>` when unsaved/new) followed by the
product name; simple mode is indicated in the caption (the tutorial tells users to
look at the caption to see the mode). Exact format is cosmetic.

### 3.2 File menu
| Command | Behaviour |
|---|---|
| New | Close current project (ask to save if modified), start an empty project. Toolbar button. |
| Open… | File dialog. Accepts `.gm81` (native), `.gmk`, `.gm6`; `.gmd` (v5) selectable via file-type filter, best effort. Toolbar button. Dropping a project file on the main window also opens it. |
| Recent Files ▸ | Up to 8 recently opened projects (if the preference is on). |
| Save | Save to current path; if none, behaves as Save As. Enabled only when modified. Toolbar button. Creates backups per preferences (section 19). |
| Save As… | Choose new path, then save. Saves in `.gm81` format. |
| Open Folder… / Save As Folder… | **Instance addition** (not in browser builds): open a project folder (`docs/project-folder.md`; must contain `project.toml`), or save the project as one. Save As Folder writes into the chosen folder when it is empty or already a project folder, else into a new subfolder named after the project. Save on a project opened from a folder writes back into it, keeping the folder's recorded `format_version` and any files outside the resource directories. Dropping a project folder on the main window opens it. |
| Create Executable… | See section 20.2. Toolbar button. |
| Publish your Game | Opened a web upload page. **Out of scope.** Instance has **Export Web Page…** in its place (section 20.5); the Publish toolbar button runs it. |
| Import Resources… | Section 21.2. |
| Export Resources… | Section 21.1. |
| Advanced Mode | Toggle (check mark). |
| Preferences… | Section 19. |
| Exit | Quit, asking to save if modified. |

### 3.3 Edit menu
Acts on the selected tree node; the same commands appear in the tree's right-click
menu (only the ones meaningful for that node are shown/enabled).

| Command | Behaviour |
|---|---|
| Insert *resource* | Create a new resource of the selected node's kind, placed **before** the selected node, and open its form. |
| Duplicate | Copy selected resource (all data), new name/id, insert after it **[placement unverified]**, open its form. |
| Delete | Delete selected resource or group (and everything inside). Confirmation required. Cannot be undone. Close any open form of deleted resources. |
| Rename | Inline rename in the tree (also F2 in 8.1; also slow second click on a selected name). Top-level kind nodes cannot be renamed. |
| Properties | Open the form (same as double-click). |
| Add Group | Ask for name, create a group under the selected node's kind (inside the selected group if a group is selected **[unverified]**). Groups nest. |
| Find Resource… | Ask for a name; open that resource's form. |
| Expand / Collapse Resource Tree | Fully expand / collapse. |
| Show Object Information | Read-only, printable text report of all objects (properties, events, actions). |
| Set Transparency Background… | Global IDE preference: how transparent pixels are shown in sprite/image/room editors: checkerboard (block size + two user colours) or a single solid colour. |

### 3.4 Resources menu
Create Sprite, Create Sound, Create Background, Create Path, Create Script, Create
Font, Create Time Line, Create Object, Create Room; Change Game Information; Change
Global Game Settings; Select Extension Packages; Define Constants; Define Triggers;
Included Files.
- "Create X" appends a new resource of that kind (at the end of the kind's tree
  **[or into the selected group — unverified]**) and opens its form.
- Each has a toolbar button and a keyboard shortcut (shortcut keys not listed in
  the manual; the manual history says Ctrl+Alt combinations were replaced by
  Shift+Ctrl). Proposed: Shift+Ctrl+{S,U,B,P,C,F,T,O,R} **[unverified]**.

### 3.5 Scripts menu (advanced only)
| Command | Behaviour |
|---|---|
| Import Scripts… | Load a `.gml` script library file; each script in it becomes a new script resource. |
| Export Scripts… | Save scripts to a `.gml` file: the selected script; or all scripts of a selected group; or all scripts if the root or another kind is selected. Also in the script tree context menu. |
| Show Built-in Variables | Sorted list (local and global). Source: the `fnames` identifier list. |
| Show Built-in Functions | Sorted list. Source: `fnames`. |
| Show Extension Functions | Sorted list of functions of the used extension packages. |
| Show Constants | Sorted list of built-in constants plus user constants. |
| Show Resource Names | Sorted list of all resource names; clicking one opens it. |
| Search in Scripts… | Ask for a string; list every occurrence. In 8.1 the search covers all code: scripts, code actions in objects and time lines, room creation code, instance creation code, trigger conditions. Clicking a hit opens the containing editor at that position. |
| Check Resource Names | Report names that are not valid identifiers, duplicate names, and names equal to a built-in variable, function or constant. Clicking opens the resource. |
| Check All Scripts | Syntax-check all scripts (proposed: all code locations as for Search). Report errors; clicking a report jumps to the line/column. |

`.gml` library format: plain text, **[unverified, confirm by observation]** each
script introduced by a line `#define <name>` followed by its body.

### 3.6 Run menu
- Run normally (toolbar green triangle) — section 20.1.
- Run in Debug mode (toolbar red triangle) — section 20.3.

### 3.7 Window menu
Cascade; Arrange Icons (arrange minimised forms); Close All (closes every form,
asking per changed form whether to commit).

### 3.8 Help menu
Contents (open manual), Tutorials ▸ (list of tutorial folders), News, More
Tutorials, Website, Forum, Wiki, Report a bug, About; plus one help entry per used
extension package (opens the package's help file). Licence-related items (Upgrade,
Enter Activation Code) are out of scope. Web links are optional.

### 3.9 Toolbar (left to right, as seen in the advanced-mode screenshot)
New, Open, Save | Create Executable, Publish | Run normally, Run in debug mode |
Create Sprite, Sound, Background, Path, Script, Font, Time Line, Object, Room |
Game Information, Global Game Settings, Extension Packages | Help contents.
Each button has a tooltip; hidden-in-simple-mode rules apply.

### 3.10 Resource tree (resource explorer)
- Fixed top-level nodes (advanced order): Sprites, Sounds, Backgrounds, Paths,
  Scripts, Fonts, Time Lines, Objects, Rooms (folder icons), then leaf items Game
  Information, Global Game Settings, Extension Packages. Simple mode shows Sprites,
  Sounds, Backgrounds, Objects, Rooms, Game Information, Global Game Settings.
- Leaf resources show a small icon (sprites/backgrounds show their image thumbnail,
  objects show their sprite) **[icon details cosmetic]**.
- Expand/collapse with +/-; empty groups get a closed-folder icon.
- Single click selects; click on selected name renames (also F2); double-click opens
  the form; Enter opens **[unverified]**; right-click shows the Edit menu commands
  relevant to the node, plus Create/Add Group, and for scripts Export Scripts.
- Drag-and-drop reordering: drag a resource or group to another position or into a
  group **of the same kind only** (cannot drop a sound among sprites). Order is
  saved. Order matters semantically for rooms (first room in tree order is the
  start room; next/previous room actions follow tree order).
- Dropping image files on the main window offers to create a Sprite or a Background
  from them (tutorial 2 relies on this).

### 3.11 Workspace
Area to the right of the tree where forms float. Optional decorative background and
a "news/website" panel (preference) — cosmetic, out of scope. An optional tutorial
side panel (HTML pages `page01.html…` from tutorial folders) — out of scope/P2.

---------------------------------------------------------------------------------

## 4. Common widgets

- **Resource picker**: text field showing the selected resource name or a
  placeholder (`<no sprite>`, `<no object>`, `<no background>` …) plus a menu
  button. Clicking either pops up a menu with "none" and all resources of that kind
  arranged as in the tree (groups as submenus). Stores the resource id or -1.
  Object form sprite picker additionally offers New (create sprite and open it) and
  Edit (open selected sprite).
- **Colour box**: shows a colour; click opens a colour chooser. Stores a 24-bit BGR
  colour (format detail owned by format spec).
- **Image preview with transparency background**: uses the global transparency
  background preference.
- **Number fields** accept integers unless stated.

---------------------------------------------------------------------------------

## 5. Sprite form

Fields (data → `Sprite`):
| UI | Data | Notes |
|---|---|---|
| Name | `name` | |
| Load Sprite | replaces `frames` | Opens the sprite file dialog (5.1). |
| Save Sprite | — | Saves to `.gmspr` (images + origin + collision settings). Advanced. |
| Edit Sprite | `frames` (+size) | Opens the sprite editor (section 6). |
| Width/Height, Number of subimages | read-only | All frames share one size. |
| Show: [n] + arrow | — | Cycles the preview subimage. |
| Origin X, Y; Center button | `origin_x`, `origin_y` | Clicking in the preview image sets origin; origin may be outside the image; shown as a cross if preference "show origin" is on. |
| Precise collision checking | `mask.shape == Precise` (unchecked → Rectangle) **[mapping unverified]** | |
| Separate collision masks | `mask.separate` | |
| Modify Mask | mask fields | Opens the mask form (5.2). A "Modified" marker appears in the sprite form when the mask was customised. |
| Preview pane | — | Shows current subimage over the transparency background. |

### 5.1 Sprite file dialog
Standard file selector plus a right-hand pane with an animated preview, image info
and three options: **Make Opaque** (drop alpha), **Remove Background** (default
on: the colour of the bottom-left pixel becomes transparent **[which pixel is
unverified for sprites; manual states bottom-left for loading images]**),
**Smooth Edges**. Supported formats: common image formats incl. PNG, BMP, GIF
(animated GIFs give one frame per GIF frame). A file name ending in `_stripN`
(not for GIFs) is split horizontally into N equal frames. Multi-select loads several
images as frames (sizes adapted to a common canvas).

### 5.2 Mask properties form
| UI | Data |
|---|---|
| Image info, Show subimage, "Show collision mask" (default on, mask drawn darkened), zoom in/out/1:1 | view only |
| Bounding box: Automatic / Full image / Manual | `mask.bbox_mode` |
| Left, Right, Top, Bottom (editable only when Manual; in manual mode left-drag draws the box, right-drag moves it) | `mask.bbox` |
| Shape: Precise / Rectangle / Disk / Diamond | `mask.shape` |
| Separate collision masks | `mask.separate` (ignored when bbox is manual) |
| Alpha tolerance 0-255 (slider + field) | `mask.alpha_tolerance` |

Automatic bbox = tight box around pixels whose alpha exceeds the tolerance (per
frame if separate masks). The IDE should compute and display it live; whether the
computed mask bitmaps are stored in the project or regenerated is a format/runtime
concern.

---------------------------------------------------------------------------------

## 6. Sprite editor (frame-level editor)

Opened from Edit Sprite. Edits a working copy of the frame list; "Close Saving
Changes" / OK commits back to the sprite form's working copy.

Layout: menu (File, Edit, Transform, Images, Animation), toolbar, left panel with
animation preview (checkbox Show Preview, preview speed — preview-only —,
Background Color button, background resource picker + Stretch background
checkbox), right area with all frames as thumbnails labelled `image 0`, `image 1`, …
(one selected). Status bar: frame count, size, memory estimate.

**File**: New (ask size; empty sprite) · Create from File (replace, options opaque/
remove background/smooth edges; multi-select; strips and animated GIFs split) ·
Add from File (append; if size differs choose placement or stretch) · Save as PNG
File (horizontal strip; `_stripN` appended when N>1) · Create from Strip / Add from
Strip (6.1) · Close Saving Changes.

**Edit**: Undo / Redo (16 levels) · Cut / Copy / Paste (selected frame, via
clipboard; pasted image becomes a new frame, size mismatch prompts) · Erase (fill
current or all frames with chosen colour+alpha, preview) · Delete · Move Left ·
Move Right · Add Empty (append) · Insert Empty (before selected) · Edit (image
editor on selected frame; also double-click) · Set Transparency Background.

**Transform** (each with "current frame / all frames" choice): Shift (dx, dy) ·
Mirror/Flip · Rotate (90/180/270/arbitrary) · Scale (factor, horiz/vert; canvas
unchanged) · Skew (h/v amount) · Resize Canvas (new size + anchor) · Stretch (new
size + quality) · Crop (shrink to content with a border; negative border trims).

**Images**: Cycle Left · Cycle Right · Black and White · Colorize (hue slider, or
shift hue) · Colorize Partial (source hue + tolerance → new hue; pick by clicking
preview) · Intensity (saturation, value) · Invert · Make Opaque · Erase a Color
(colour + tolerance → transparent) · Smooth Edges · Opacity (relative/absolute;
fully transparent stays transparent) · Set Alpha from File (lightness of another
image → alpha; stretched to fit) · Fade (towards colour by amount) · Blur (small/
medium/large; colour and/or alpha) · Sharpen (subtle/strong/special; colour and/or
alpha) · Outline (colour, thickness, inside, remove image, smooth) · Shadow (colour,
opacity, offset, soft/hard) · Glow (colour, opacity, thickness, inside) · Buttonize
(colour, opacity, border thickness, smooth) · Gradient Fill (two colours, opacity,
kind, Replace, Change Transparency).

**Animation** (produce new frame sequences): Set Length (repeat to N frames) ·
Stretch (duplicate/drop frames to N) · Reverse · Add Reverse · Translation Sequence
(frames, total dx, dy) · Rotation Sequence (frames, angle, cw/ccw) · Colorize ·
Fade to Color · Disappear · Shrink (direction) · Grow · Flatten (direction) ·
Raise · Overlay (with images from a file, stretched) · Morph (towards images from a
file).

Exact pixel algorithms of effects are not specified by the manual; implementers may
choose reasonable algorithms (they only affect authoring, not compatibility).

### 6.1 Strip dialog (Create/Add from Strip)
Shows the chosen bitmap with rectangles marking selected cells. Parameters:
Number of images, Images per row, Image width, Image height, Horizontal/Vertical
cell offset (cells to skip), Horizontal/Vertical pixel offset (can also be set by
clicking in the image), Horizontal/Vertical separation (pixels between cells).
Frames are taken row-major.

---------------------------------------------------------------------------------

## 7. Image editor (single image)

Used for one sprite frame or a background image (or replaced by an external editor
via preferences: image saved as `.png`, editor launched with the path, file re-read
when the editor exits).

Layout: menu (File, Edit, View, Transform, Image), toolbar, tool palette on the
left with a tool-options area below it, canvas centre (zoomable; Ctrl+wheel or
wheel zooms, middle mouse pans), right: left/right colour swatches, palette grid,
colour field/spectrum, Opacity (0-255), Color Mode (Blend / Replace). Status bar:
tool hint, mouse position, zoom, size, memory.

Left mouse button draws with the left colour, right button with the right colour.

Tools:
| Tool | Options / modifiers |
|---|---|
| Paint | pen size; Shift = constrain to horizontal/vertical; Ctrl = pick colour |
| Spray | brush size, hardness; Shift, Ctrl as above |
| Erase | size, hardness; amount by Opacity; Shift constrains |
| Color Picker | sets left/right colour incl. opacity |
| Line | width, arrow heads (none/end/start/both), anti-alias; Shift = 0/45/90° |
| Polygon | click/drag vertices, Esc ends; width, filled, anti-alias; Shift constrains |
| Rectangle / Ellipse / Rounded Rectangle | width, filled, anti-alias; Shift = square/circle; Ctrl = from centre |
| Select Region | rectangle; Shift adds, Ctrl subtracts; drag moves, right-drag copies; Del deletes; Esc ends |
| Magic Wand | tolerance, match colour only or also alpha; Shift/Ctrl add/subtract |
| Select by Spraying | brush size |
| Text | click to place; dialog for text (`#` = newline); movable box; right-click to edit; font and alignment options |
| Fill | flood fill connected pixels; tolerance; colour-only vs alpha; respects Color Mode |
| Change Color | replace all matching pixels globally; tolerance; colour-only vs alpha |

Menus:
- File: New (size; disabled for multi-frame sprites) · Open (disabled for
  multi-frame sprites) · Previous Image / Next Image (multi-frame only; commits and
  moves) · Save as PNG File · Close Saving Changes.
- Edit: Undo, Redo, Delete, Cut, Copy, Paste (as floating selection), Select All,
  Erase to Left Color, Paste from File.
- View: Zoom Out, No Zoom, Zoom In, Toggle Grid (visible only when zoomed in),
  Grid Options (size, colour, XOR drawing, snap to grid), Show Preview (1:1 preview
  bottom-right; draggable; double-click opens separate window), Set Transparency
  Background.
- Transform: as sprite editor; Resize Canvas, Stretch, Crop disabled for
  multi-frame sprites.
- Image: as sprite editor's Images menu.

---------------------------------------------------------------------------------

## 8. Sound form

| UI | Data (`Sound`) |
|---|---|
| Name | `name` |
| Load Sound | `data`, `file_name`, `extension` (wave `.wav`, MIDI `.mid`, `.mp3`, others for multimedia player) |
| Play / Stop buttons | preview only. Instance plays the preview with the game's sound engine (`instance-audio`) using the form's current kind, effects, volume and pan; one preview at a time; closing the form stops it. A new sound has volume 1 and preload on. |
| Save Sound | write `data` back to a file |
| Filename label | original file name (read-only) |
| Kind: Normal sound / Background music / 3D sound / Use multimedia player | `kind` (0..3) |
| Effects: Chorus, Echo, Flanger, Gargle, Reverb | `effects` bit set |
| Volume slider | `volume` (0..1) |
| Pan slider | `pan` (-1..1, 0 centre) |
| Preload | `preload` (default on) |
| Edit Sound | launch the external editor configured for this file type (wav/mid/mp3) on a temp file, hide the IDE, reload when editor exits |

On load, MIDI files default to Background music kind **[behaviour per manual:
"midi files are default background music"]**. Changing effects should be audible in
preview (P2).

---------------------------------------------------------------------------------

## 9. Background form

| UI | Data (`Background`) |
|---|---|
| Name | `name` |
| Load Background | `image` (file dialog with Opaque / Remove Background (default off) / Smooth Edges) |
| Save Background | `.gmbck` file (image + tile settings). Advanced. |
| Edit Background | image editor (section 7) |
| Width, Height | read-only |
| Use as tile set | `tileset.enabled` |
| tile width, tile height | `tileset.tile_w`, `tile_h` (default 16) |
| horizontal offset, vertical offset | `tileset.offset_x`, `offset_y` |
| horizontal sep, vertical sep | `tileset.sep_x`, `sep_y` |
| Preview | image; when tile set is on, grid lines overlay the tiles |

Backgrounds have one image (no animation).

---------------------------------------------------------------------------------

## 10. Path form

Toolbar: OK · Undo | Clear · Reverse · Shift · Mirror horizontally · Flip
vertically · Rotate · Scale | shift view left/right/up/down · Center view | Snap X,
Snap Y fields · Toggle grid | Room background picker (show a room as backdrop; does
not change the path).

Left panel: Name; point list (`(x,y)  sp: s`); X, Y, sp fields for the selected
point; buttons Add (append copy of selected point), Insert (insert before selected),
Delete; Connection kind: Straight lines / Smooth curve; Closed checkbox; Precision
field.

Canvas: grid; selected point red, others blue, start marked green. Mouse: click
empty space adds a point; drag a point moves it; Shift+click on a point inserts;
right-click deletes; Alt disables snapping; middle mouse pans. Status bar: mouse x,
y and visible area.

| Data (`Path`) | Source |
|---|---|
| `points: Vec<{x: f64, y: f64, speed: f64}>` (speed in %, 100 = nominal) | list |
| `connection: Straight | Smooth` | radio |
| `closed: bool` | checkbox |
| `precision: int` (range **[unverified]**, screenshot shows 4) | field |
| editor-only: `backdrop_room`, `snap_x`, `snap_y` (stored in project per path **[unverified]**) | toolbar |

---------------------------------------------------------------------------------

## 11. Script form and code editor

The same editor is used for: script resources, the "Execute Code" action, room
creation code, instance creation code, trigger conditions ("Use code editor").

In Instance, the Execute Code action and room / instance creation code open in a
**code window**: a non-modal child window with the form chrome (move, resize,
maximise, close = Cancel) that keeps its placement, stays above its owner form, and
holds the editor plus OK / Cancel. The owner form is disabled and ignores Enter /
Escape while it is open; the rest of the IDE stays usable.

Toolbar: OK · Load from text file · Save to text file · Print | Undo · Redo | Cut ·
Copy · Paste | Find/Replace toggle | error-checking toggle · code-completion toggle |
Name field (script only). For code actions, an **Applies to** selector (Self /
Other / Object [picker]) replaces/accompanies the name.

Text area: line-number gutter (F9 toggles), current line emphasised, modified lines
marked in the gutter, free cursor beyond end of line (preference), status bar with
`line/total: column`, INS/OVR, font point size (F7/F8 smaller/larger).

Editing behaviour:
- Undo/redo (Ctrl+Z, Ctrl+Shift+Z), grouped or per keystroke, depth configurable.
- Arrow keys; Ctrl+Left/Right word jump; Ctrl+Up/Down scroll; Home/End,
  Ctrl+Home/End; Shift extends selection; double-click line number selects line;
  wheel scroll.
- Auto-indent on Enter/Backspace; smart tabs (align to first non-space of the
  previous line); Tab/Shift+Tab indent/unindent selected lines.
- Ctrl+Backspace / Ctrl+Del delete word-wise.
- Drag selection to move (Ctrl to copy).
- F12 opens the script or resource whose name is under the cursor.
- AltGr composes accented characters.
- **Syntax colouring** (F10 toggles): distinct colours for keywords, comments,
  strings, numbers, built-in functions, built-in variables, built-in constants,
  resource names, script names, extension functions, user constants. Colours, the
  editor background, current-line and selection colours are configurable; optional
  bold keywords; only fixed-pitch fonts.
- **Auto-completion**: after ≥2 identifier characters and a configurable delay, a
  popup lists matching functions/variables/constants/resources; Up/Down + Enter
  inserts; Ctrl+Space forces; Esc cancels.
- **Function argument hint**: while typing inside a call, the status bar shows the
  function signature with the current argument in bold. Signatures come from
  `fnames` for built-ins, from extension packages, and for scripts **[unverified:
  scripts likely show `script(arg0, …)` without names]**.
- **Bracket matching**: bracket at cursor highlights its partner; red if unmatched.
- **Find / Replace** (Ctrl+F): side panel; find text (history), case-sensitive,
  whole word; all matches highlighted (preference); buttons previous / next / first
  / last; replace previous / next / first / last / all.
- **Snippets** (F2): popup menu from `snippets.txt` (one per line:
  `name:text`, `#` = newline, `|placeholder|` = first selection; uppercase
  placeholders replaced by double-click).
- **Live error checking**: when on, the code is continuously parsed; the first error
  is shown in a bar under the text and its line number turns red; clicking the
  message jumps to it. Checks syntax and existence of called functions.
- Code action list label: if the first line starts with `///`, the rest of that line
  is shown as the action's description in action lists.
- External editor option (preferences): text saved to `.txt`, editor launched,
  re-read after it exits.

Data: `Script { name, source: String }`; code action argument 0 holds the source;
room/instance creation code are strings on `Room` / `RoomInstance`.

---------------------------------------------------------------------------------

## 12. Font form

| UI | Data (`Font`) |
|---|---|
| Name | `name` |
| Font (dropdown of installed fonts) | `font_name` |
| Size | `size` (points) |
| Bold, Italic | `bold`, `italic` |
| Anti-Aliasing 0-3 | `antialias` |
| Character range first … till last; buttons Normal (32-127), All (0-255), Digits (48-57), Letters (65-122) **[Letters exact range unverified]** | `range_first`, `range_last` |
| Character Set (dropdown, e.g. DEFAULT_CHARSET) | `charset` |
| Sample text box (editable) + preview | editor-only |

The IDE only previews the font; rasterising glyphs happens at executable creation
or game start (runtime/format concern). Fonts are embedded only in executables;
the editable project only stores the font name.

---------------------------------------------------------------------------------

## 13. Object form

Left panel:
| UI | Data (`Object`) |
|---|---|
| Name | `name` |
| Sprite picker + New + Edit | `sprite` (-1 none) |
| Visible (default on) | `visible` |
| Solid | `solid` |
| Depth | `depth` (int, default 0) |
| Persistent | `persistent` |
| Parent picker (`<no parent>`) | `parent` (-1 none; IDE must reject cycles) |
| Mask picker (`<same as sprite>`) | `mask_sprite` (-1 = same as sprite) |
| Show Information | printable text summary of this object |
| OK | commit |

Middle: **Events** list, then buttons Add Event, Delete, Change. Right of it:
**Actions** list for the selected event. Far right: **action palette**.

### 13.1 Event list
- Shows each defined event with its icon and display name, e.g. `Create`,
  `Alarm 0`, `Step`, `Begin Step`, `Collision with obj_wall`, `press <Left>`,
  `release <Space>`, `Left Button`, `Draw`, `User Defined 3`, trigger name, etc.,
  sorted by event type then subtype **[sort order and exact labels unverified]**.
- Exactly one event is selected (current). Clicking selects.
- Add Event opens the event selector (13.2); the new event becomes selected. If it
  already exists, it is just selected.
- Delete removes the selected event with all its actions (confirmation if it has
  actions **[unverified]**).
- Change opens the event selector and moves the actions to the chosen event, which
  must not already exist.
- Right-click menu additionally offers Duplicate (copy all actions into a new event
  chosen via the selector).
- Events without actions are removed automatically when the form is committed.

Data: `events: Map<(EventType, subtype), Vec<ActionInstance>>`. EventType/subtype
numbering is defined in the runtime spec.

### 13.2 Event selector ("Choose the Event to Add")
Modal grid of buttons: Create, Destroy, Alarm, Step, Collision, Keyboard, Mouse,
Other, Draw, Key Press, Key Release, Trigger (advanced, only meaningful if triggers
exist), Cancel. Buttons with subtypes pop up a menu:
- Alarm: Alarm 0 … Alarm 11.
- Step: Step, Begin Step, End Step.
- Collision: menu of all objects (tree structure).
- Keyboard / Key Press / Key Release: keys grouped in submenus (e.g. `<No key>`,
  `<Any key>`, arrows, Enter/Space/Shift/Ctrl/Alt, letters, digits, keypad,
  function keys, other keys) **[exact grouping unverified]**.
- Mouse: Left/Right/Middle button, No button, Left/Right/Middle pressed,
  Left/Right/Middle released, Mouse enter, Mouse leave, Mouse wheel up, Mouse
  wheel down, Global mouse ▸ (global left/right/middle button/pressed/released),
  Joystick 1 ▸ / Joystick 2 ▸ (left, right, up, down, button 1-8).
- Other: Outside room, Intersect boundary, Views ▸ (Outside view 0-7, Intersect
  boundary view 0-7), Game start, Game end, Room start, Room end, No more lives,
  No more health, Animation end, End of path, Close button, User defined ▸ (0-15).
- Trigger: menu of defined trigger names.
- Create, Destroy, Draw: no submenu.

### 13.3 Action list
- Shows the actions of the selected event top to bottom: icon + one-line
  description generated from the action's library "list text" template with
  argument values substituted (rules in the action-library spec). Questions have a
  distinct icon shape; the code action shows its `///` label; the comment action is
  shown in italics; actions between Start Block / End Block are indented (nested).
  Applies-to other than self, and Relative/NOT may be indicated in the label
  **[unverified]**.
- Hovering an action shows a longer hint (library hint text) if the preference is on.
- Drag from the palette to the list inserts at the drop point and opens the
  argument dialog (if the action has one); Cancel on that first dialog removes the
  new action **[unverified]**.
- Drag within the list reorders; Alt+drag copies; drag between action lists of
  different open forms (objects and time lines) moves/copies.
- Double-click edits the arguments.
- Multi-select with Shift/Ctrl; Ctrl+A selects all; Del deletes.
- Right-click menu: Edit values, Cut, Copy, Paste, Delete, Select All, Clear
  **[exact items unverified beyond delete/copy/paste]**. Clipboard is IDE-internal
  and works across forms.

### 13.4 Action palette
- Vertical tabs on the right edge, one per loaded action library, captioned by the
  library's tab caption. Shipped libraries in file order give: `move`, `main1`,
  `main2`, `control`, `score`, `extra`, `draw`. Extension packages / user libraries
  may add more tabs.
- Each tab shows the library's actions as icon buttons in a grid, divided by
  library-defined separators/labels (e.g. Move tab: "Move", "Jump", "Paths",
  "Steps"). Hidden actions and advanced-only actions in simple mode are skipped.
- Tooltip = action name/hint.
- The tab selected when a form opens is the "Default tab" preference.

### 13.5 Action argument dialog
Built from the action's library definition:
- Top: **Applies to** radio group — Self (default) / Other / Object + object picker
  — shown only if the action declares it.
- Up to 6 argument rows, each "label: widget", widget by argument kind:
  expression (text), string (text), both (text), boolean (true/false dropdown),
  menu (dropdown of library-defined items), sprite / sound / background / path /
  script / object / room / font / time line (resource picker), colour (colour box),
  font-string (font dialog) **[kind list from library spec; confirm there]**.
  Expressions may be any GML expression (variables, functions, constants).
- **Relative** checkbox if the action declares it.
- **NOT** checkbox for question actions.
- OK / Cancel.
- Special cases: Execute Code opens the code editor (with Applies to); Comment has
  one string; Start/End Block, Else, Exit, Call Parent Event have no dialog;
  Execute Script shows a script picker plus five argument fields; Move Fixed shows a
  3x3 grid of direction toggle buttons (8 directions + stop) plus speed
  **[how this maps to the stored argument is a library-spec question]**.

Data: `ActionInstance { lib_id, action_id, applies_to: Self|Other|Object(id),
relative: bool, not: bool, args: Vec<String> }` plus whatever the action-library
spec requires to be cached in the project (kind, function name, etc.).

---------------------------------------------------------------------------------

## 14. Time line form

Left: Name; buttons Add, Change, Delete, Clear, Shift, Duplicate, Spread, Merge;
Show Information; OK. Middle: **Moments** list (step numbers, ascending). Then
action list and action palette identical to the object form (13.3–13.5).

| Button | Behaviour |
|---|---|
| Add | ask step number; create (or select existing) moment |
| Change | ask new step for the selected moment |
| Delete | ask first..last range; delete all moments in it |
| Clear | delete all moments (confirm) |
| Shift | ask range start, end, new start; move moments; colliding moments merge their action lists |
| Duplicate | like Shift but copies |
| Spread | ask range and percentage; scale distances from range start (100 = no change; example 5,8,12 at 200% → 5,11,19); collisions merge |
| Merge | ask range; all moments merged into the first |

Negative moments are allowed but never execute (warn). Data:
`Timeline { name, moments: BTreeMap<i32, Vec<ActionInstance>> }`.

---------------------------------------------------------------------------------

## 15. Room form

### 15.1 Toolbar (left to right)
OK (commit+close) · Undo (last change) | Clear all (instances or tiles of the
current layer, depending on tab) · Shift all (ask dx, dy; may move outside the
room) · Sort by X · Sort by Y · Lock all · Unlock all | Snap X · Snap Y | Toggle
grid · Isometric grid | Show ▸ menu (toggle drawing of objects, tiles, backgrounds,
foregrounds, views, grid **[exact list unverified]**) | Zoom out · Zoom 1:1 · Zoom in.
Mouse wheel zooms; middle mouse pans. Status bar: mouse x, y (snapped) and the
object/instance under the cursor **[unverified]**.

Clear/Shift work on locked items too. Sort/lock act on instances on the objects tab
and on the current tile layer on the tiles tab.

### 15.2 Left tabs
Simple: objects, settings, backgrounds. Advanced adds: tiles, views.

**objects tab**
- Large preview of the selected object's sprite (origin cross if not top-left).
- Object picker ("Object to add with left mouse"); clicking the preview also opens
  the picker.
- Hint text (8.1): left = add; +Alt = no snap; +Shift = add multiple while dragging;
  +Ctrl = move. Right = context menu; +Shift = delete all at position; +Ctrl =
  delete.
- "Delete underlying" checkbox (8.1 default off): placing an instance removes
  instances at the same position underneath.
- Placing: left click adds an instance of the selected object at the snapped
  position; holding the button drags it; Shift+drag paints multiple instances (one
  per grid cell).
- Ctrl+left drag moves an existing instance (Alt for unsnapped). Locked instances
  cannot be moved or deleted interactively.
- Right-click on instance menu: Delete; Change position… (exact x, y); Move to
  front (bottom-most at position to top) / Move to back; Lock / Unlock; Creation
  code… (advanced); Edit object (opens the object form, 8.1).
- Instances of objects without a sprite are drawn as a placeholder (blue ball with a
  question mark).

**settings tab**
Name; Caption for the room; Width; Height (pixels, default 640×480); Speed (steps
per second, default 30); Persistent (advanced); Creation code button (advanced;
opens code editor).

**tiles tab** (advanced)
- Tileset preview with grid; tileset picker below (backgrounds; those marked as
  tile sets are intended).
- Click selects one tile; Alt+drag selects an arbitrary pixel region; Shift+drag
  selects multiple whole tiles (only correct with zero separation); Ctrl+drag
  selects a region that is a multiple of the room grid. Modifiers can change during
  the drag.
- Placing/deleting/moving in the room works as for instances (same modifiers,
  Delete underlying checkbox, right-click menu).
- Current Tile Layer dropdown (by depth; default layer depth 1000000) with Add
  (ask depth), Delete (layer and its tiles; at least one layer must remain), Change
  (ask new depth; equal depth merges layers).

**backgrounds tab**
- Draw background color checkbox; Color box.
- List Background 0…7 (defined ones shown bold).
- For the selected background: Visible when room starts; Foreground image;
  background picker; Tile Hor.; Tile Vert.; X; Y; Stretch; Hor. Speed; Vert. Speed.

**views tab** (advanced)
- Enable the use of Views.
- List View 0…7 (visible ones bold).
- Visible when room starts; View in room X, Y, W, H; Port on screen X, Y, W, H;
  Object following picker; Hbor, Vbor; Hsp, Vsp (-1 = unlimited).

### 15.3 Closing
- OK commits. X closes and asks about saving.
- Preference "remove instances outside the room": on commit, warn about instances/
  tiles fully outside the room and offer to delete them.
- Preference "remember room settings": the editor-only state (snap, grid, iso,
  show flags, delete-underlying, current tab, zoom, scroll, window maximised) is
  stored per room and restored next time.
- Preference "Room grid ON by default" and default room fill colour / "fill room
  with colour when no background colour selected" control editor display only.

### 15.4 Data (`Room`)
```
name, caption, width, height, speed, persistent, creation_code,
draw_bg_color: bool, bg_color,
backgrounds[8]: { visible, foreground, background: id, x, y, tile_h, tile_v,
                  hspeed, vspeed, stretch },
views_enabled,
views[8]: { visible, view_x, view_y, view_w, view_h, port_x, port_y, port_w,
            port_h, hborder, vborder, hspeed, vspeed, follow: object id },
instances: Vec<{ x, y, object, id, creation_code, locked }>,   // order = creation order
tiles: Vec<{ x, y, background, src_x, src_y, w, h, depth, id, locked }>,
editor: { remember, snap_x, snap_y, show_grid, isometric, show_objects,
          show_tiles, show_backgrounds, show_foregrounds, show_views,
          delete_underlying, current_tab, zoom, scroll_x, scroll_y, ... }
```
New instance and tile ids come from the project-wide allocators
(`last_instance_id`, `last_tile_id`); ids must stay unique.

---------------------------------------------------------------------------------

## 16. Game Information editor

Small rich-text editor (stored as RTF). Menus File / Edit / Format; toolbar: OK,
Options, Print | Cut, Copy, Paste | font name, size | Bold, Italic, Underline |
text colour, background colour | align left/centre/right | bullets. Paste of rich
text from other programs is supported. Background colour applies to the whole
document.

File > Options:
| UI | Data (`GameInfo`) |
|---|---|
| Show help in a separate window | `separate_window` (default off = shown in game window, game paused) |
| Caption | `caption` (separate window only) |
| Left, Top (-1 = centred), Width, Height | `x`, `y`, `w`, `h` |
| Show the window border and caption | `show_border` |
| Allow the player to resize | `sizeable` |
| Stay always on top | `stay_on_top` |
| Stop the game while showing | `pause_game` |
plus `rtf: String`, `background_color`. (Labels approximate; fields per manual.)

---------------------------------------------------------------------------------

## 17. Global Game Settings form

Tabbed dialog, OK/Cancel. Tabs and fields (data `GlobalGameSettings`):

**Graphics**: Start in fullscreen mode · Scaling: Fixed scale (% , 100 = none) /
Keep aspect ratio / Full scale · Interpolate colors between pixels · Color outside
the room region · Allow the player to resize the game window · Let the game window
always stay on top · Don't draw a border in windowed mode · Don't show the buttons
in the window caption · Display the cursor · Freeze the game when the form loses
focus · Disable screensavers and power saving actions (default on).

**Resolution**: Set the resolution of the screen (master checkbox enabling:) ·
Color depth (no change / 16-bit / 32-bit) · Resolution (no change or a list of
standard sizes) · Frequency (no change or a list of Hz values) · Use
synchronization to avoid tearing (independent of the master checkbox
**[unverified]**). Option lists **[unverified]**.

**Other**: Default keys: Let <Esc> end the game · Treat the close button as <Esc>
key · Let <F1> show the game information · Let <F4> switch between screen modes ·
Let <F5> save the game and <F6> load a game · Let <F9> take a screenshot · Game
process priority: Normal / High / Highest · Version information: Major, Minor,
Release, Build; Company, Product, Copyright, Description (≤64 chars each).

**Loading**: Show your own image while loading (Change image / clear) · Make image
partially transparent · Make translucent with alpha value (0-255) · Loading
progress bar: No bar / Default bar / Own bar (Back image, Front image; Scale
image (default) vs clip) · Game icon (Change icon; must contain at least 32×32) ·
Game Identifier (number; Change/randomise).

**Errors**: Display error messages · Write error messages to file game_errors.log ·
Abort on all error messages · Treat uninitialized variables as 0.

**Info**: Author, Version, Last changed (read-only, maintained automatically),
Information (multi-line text). Not visible to the running game.

(Constants and Included Files are no longer tabs here since 8.0; they have their own
dialogs.)

---------------------------------------------------------------------------------

## 18. Constants, Triggers, Included files, Extension packages

### 18.1 User-Defined Constants (Resources > Define Constants)
Grid with columns Name | Value; cells edited in place. Buttons: Insert (above
selected), Add (append; also Enter), Delete, Clear, Up, Down, Sort (by name), Load
(append from text file), Save (to text file), OK. Text file format: one `NAME=VALUE`
per line. Value is a constant expression string (number, quoted string, or
expression using built-in constants and resource names). Data: `Vec<Constant{name,
value: String}>`. Non-modal: can stay open while writing code.

### 18.2 Triggers (Resources > Define Triggers)
Left: trigger list; buttons Add, Delete, Load (`.gmtrg`, appends), Save (`.gmtrg`),
Clear; OK. Right, for the selected trigger: Name; Condition (code text; must
`return` a boolean) + "Use code editor" button; Moment of checking: Begin of step /
Middle of step (default) / End of step; Constant name. Data: `Trigger { name,
condition, moment, constant_name }`. Deleting a trigger used by objects is allowed
but warned against; objects then reference a missing trigger **[editor display of
dangling trigger events unverified]**.

### 18.3 Included Files (Resources > Included Files)
List with Add, Change, Delete, Clear. Add/Change opens "Included File Properties":
File Name (target name; editable), Original File (read-only path), Size, Load
button; Store in the editable gm81 file; Free memory after export (default on);
Overwrite the file if it exists; Remove at game end; Export: Don't export
automatically / Export to the temporary folder / Export to the folder containing
the game (default) / Export to the following folder: [path]. Data:
`IncludedFile { file_name, original_path, original_size, stored: bool,
data: Option<bytes>, free_after_export, overwrite, remove_at_end, export_mode,
export_folder }`. When not stored, the data is read from `original_path` at
executable/run time.

### 18.4 Extension Packages (tree item or Resources menu)
Two lists: Used packages (left) / Available packages (right) with a move button
between; Information box for the selected package (Author, Version, Date, License,
Description); buttons OK, Help (package help file), Install. Install dialog: list of
installed packages, Install (choose a `.gex` file), Uninstall, Find More (web).
Data: `extensions: Vec<package name>`. Installed packages live in an IDE-level
folder (shipped ones are `.ged` + `.dat` + `.chm`).

**Instance's form** (settles details the above leaves open):
* Installed packages are scanned from the search path of `extensions.md` §2.1
  (first hit wins by package name = `.ged` file name without extension; listed
  sorted case-insensitively). Packages whose `.ged` has `hidden = true` are not
  offered in Available (they stay listed when used).
* The Used list keeps the project's order; a package is added at the end. Used
  packages that are not installed stay in the list, shown in red as
  "`<name>` (not installed)", and are saved unchanged.
* Move: the left arrow (or a double-click in Available) adds the selected
  package; the right arrow (or a double-click in Used) removes it.
* Information box for the selection: name, Version, Author, Date, License, Uses,
  the folder it was found in, Description, then the non-hidden functions as
  signatures `name(string, real)` (`name(...)` for variadic; help line as tooltip)
  and the non-hidden constants `NAME = value`. Missing / unreadable packages and a
  missing `.dat` are reported there.
* Install, Uninstall and Help are on the form itself (no separate install dialog).
  Help opens `<name>.chm` (or the `.ged`'s help file name in the same folder) with
  the system. Install picks a `.ged` (plus, optionally, its `.dat` / `.chm`, which
  otherwise must be beside it with the same base name), validates both files, and
  copies them to the user's extension folder named after the package's `name`
  field (falling back to the file name). `.gex` files are refused with a message
  (EXT-1). Uninstall (after a Yes/No confirmation) deletes the `.ged`/`.dat`/`.chm`
  of the selected package, only for packages in the user's extension folder;
  packages next to the IDE or in the project folder cannot be removed here.
  Install and Uninstall change the disk immediately and are not undone by Cancel;
  OK/Cancel only concern the project's list. Used packages contribute
functions/constants to code completion and colouring, and actions (if any) to the
palette.

---------------------------------------------------------------------------------

## 19. Preferences (File > Preferences)

Stored per user (our implementation: a config file in the user config directory),
never in the project.

**General**: Show recently edited games in the file menu (8) · Load last opened
file on startup · Keep backup copies of files (`.gb0`–`.gb9`) + Maximum number of
backups (1-9) · Show progress while loading and saving · At startup check for and
remove old temporary files · Don't show the website in the main window · Hide the
designer and wait while the game is running · Run games in secure mode (game may
not run external programs or write/delete files outside its folder; applies only to
games launched from the IDE) · Show news on startup / in default browser (out of
scope).

**Forms**: Sprite: Show the origin in the sprite image · Object: show hints for
actions; Default tab to select (index) — also used by time lines · Room: When
closing, remove instances outside the room; Remember room settings when closing the
form; Room grid ON by default; Fill room with colour when no background colour
selected + colour (editor only).

**Scripts and Code**: Group undo operations · Number of undo · Automatic
indentation · Indent amount · Smart tabs · Allow cursor beyond end of line · Show
auto-completion options + Delay (ms) · Show function argument help · Show find
string · Show line numbers · Show matching brackets · Show code snippets with F2 ·
Check code while typing · Enable bold keywords · Use color coding · Colors (per
token class, background, current line, selection; Reset) · Font (fixed-pitch; name
and size).

**Editors**: Code editor: built-in / external program path · Image editor:
built-in / external · Sound editors for `.wav`, `.mid`, `.mp3` (external program
paths).

Other IDE-level settings outside the dialog: temp folder override via `tempdir.txt`
next to the IDE executable (one absolute path, no trailing backslash; a unique
sub-folder per IDE instance); custom workspace background `back.bmp`;
transparency-background settings (Edit menu); `advanced_mode`; window layout.

Backup rotation **[unverified]**: proposed — on save, shift `name.gb(k)` →
`name.gb(k+1)` for k = max-2 … 0, then copy the previous file to `name.gb0`.

---------------------------------------------------------------------------------

## 20. Running, debugging, creating executables

### 20.1 Run normally
1. Require at least one room (else error message).
2. Serialize the current in-memory project (committed state) to a temporary game
   file in the IDE temp folder **[whether open, uncommitted forms are committed
   first is unverified]**.
3. Launch the runner with: the game, the working directory = folder of the project
   file (or temp folder if unsaved **[unverified]**), secure-mode flag, debug flag
   off.
4. If "Hide the designer" is on, hide the IDE until the game exits; otherwise IDE
   stays visible but editing is blocked until exit **[unverified]**.
5. Runtime errors are reported by the runner (type, resource/action, line,
   position).

### 20.2 Create Executable
File > Create Executable: require ≥1 room; ask for an `.exe` path; produce a
stand-alone game using the game icon and version info from Global Game Settings,
embedding fonts (rasterised), included files, used extension packages. Our
implementation: runner binary + appended/embedded game data in our own format
(defined by the runner/exe spec). Compatibility with YoYo's exe format is not
required unless another spec says so.

### 20.3 Run in Debug mode
Runs the game as in 20.1 with debug flag on and shows the **Debug Information**
window (separate, always on top):
- Menus: **Run** (Run/continue, Pause, Step (one step while paused), Restart game,
  End game), **Watch** (Add expression, Change, Delete, Clear, Load watches, Save
  watches), **Tools** (Show global variables, Show instances (list of all
  instances), Show local variables (by object name or instance id), Show messages
  (output of `show_debug_message`), Execute code (run a GML command in the game),
  Set game speed).
- Toolbar mirrors the main items (run, pause, step, restart, end | add, change,
  delete, clear watch).
- Grid "Expression | Value" re-evaluated each step.
- Status bar: fps, instance count, id of instance under the mouse.
Exact item list **[partly unverified]**. Requires an IPC protocol between IDE and
runner (or an in-process debug runner) — to be defined in the runner spec.

### 20.4 Error/consistency checks available before running
Scripts > Check Resource Names, Check All Scripts (section 3.5). Live code checking
in each code editor (section 11).

### 20.5 Export Web Page (Instance addition)
File > Export Web Page (also the Publish toolbar button): require ≥1 room; ask for
an `.html` path (in the browser IDE: download). The page is self-contained: the web
runner's wasm-bindgen glue inline, and the runner's wasm, the game as `.gm81` and, when
the game has MIDI sounds, a SoundFont, each gzip-compressed and base64-encoded
(`instance_format::web`). The game starts on a click (browser audio policy). Extension
packages are not included. The `gmk-web` tool makes the same page from the command line.

---------------------------------------------------------------------------------

## 21. Export / import resources (and the old "merge games")

"Merge Game" existed in older versions and was **replaced in 8.0** by export/import.
Our implementation provides only export/import.

### 21.1 Export
1. Check that all resource names are unique across the project; if not, show an
   error and abort.
2. Ask for a `.gmres` file name.
3. Dialog "Resources to Export": checkboxes Sprites, Sounds, Backgrounds, Paths,
   Scripts, Fonts, Time Lines, Objects, Rooms, Constants, Include Files, Extension
   Packages, Trigger Events, Game Information, Global Game Settings; Select All /
   Select None; OK / Cancel. Default: all.
4. Write the file. Cross-references (objects→sprites, rooms→objects, etc.) are
   stored **by name**. (`.gmres` layout is not documented; for us, define our own
   format or reverse it from black-box observation; P2.)

### 21.2 Import
1. Same unique-name check on the current project.
2. Choose a `.gmres`.
3. Close all open forms.
4. Dialog "Resources to Import": same checkboxes (types absent from the file are
   greyed out); Handling naming conflicts: Keep Original / Always Replace / Keep
   Last Changed (default, compares `last_changed`) / Keep Both; "Place new resources
   in a new group" (group named after the file).
5. Apply: new resources appended (in the file's tree order, groups preserved);
   conflicting ones handled per option; references resolved by name, unresolved →
   none (e.g. room instances of missing objects are dropped). Constants, triggers
   and global settings are all-or-nothing using their single last-changed stamp.
   Instance/tile ids are renumbered to stay unique. Not undoable.

---------------------------------------------------------------------------------

## 22. Field → data model mapping summary

| Resource | Fields edited by the IDE | Editor |
|---|---|---|
| Sprite | name, frames[RGBA, same size], origin_x/y, mask{shape, separate, bbox_mode, bbox l/r/t/b, alpha_tolerance}, last_changed | §5, §6, §7 |
| Sound | name, kind, file_name, extension, data, effects, volume, pan, preload | §8 |
| Background | name, image, tileset{enabled, tile_w/h, offset_x/y, sep_x/y} | §9, §7 |
| Path | name, points[x,y,speed], connection, closed, precision, (editor: backdrop room, snap) | §10 |
| Script | name, source | §11 |
| Font | name, font_name, size, bold, italic, antialias, charset, range_first/last | §12 |
| Timeline | name, moments{step → actions} | §14 |
| Object | name, sprite, solid, visible, depth, persistent, parent, mask_sprite, events{(type,sub) → actions} | §13 |
| Room | name, caption, size, speed, persistent, creation_code, bg colour/draw flag, 8 backgrounds, views_enabled + 8 views, instances, tiles, editor state | §15 |
| Action instance | lib_id, action_id, applies_to, relative, not, args[] | §13.5 |
| Game information | rtf, background colour, window options | §16 |
| Global game settings | graphics, resolution, keys, priority, version info, loading image/bar/icon, game id, error options, author/version/info | §17 |
| Constants | ordered name/value pairs + last_changed | §18.1 |
| Triggers | name, condition, moment, constant_name + last_changed | §18.2 |
| Included files | file_name, original path/size, stored data, flags, export mode/folder | §18.3 |
| Extensions | list of used package names | §18.4 |
| Resource tree | per-kind ordered tree of groups/leaves | §3.10 |
| IDE preferences (not in project) | §19 | §19 |

**Round-trip rule:** the IDE must preserve every field it loads, including fields it
does not (yet) expose in the UI, so that open→save of an untouched project yields an
equivalent file.

---------------------------------------------------------------------------------

## 23. MVP prioritisation for a Rust / egui implementation

egui notes: use `egui::SidePanel` for the tree, `egui::Window`s (constrained to the
central area) for forms to emulate MDI; modal dialogs via a blocking overlay window
per form; a custom painter widget for room/path/image canvases; `egui_extras`
code editor or a custom `TextEdit` layouter for colouring. Keep every form a pure
function of (working copy, UI state) → actions, so commit/cancel is trivial.

**P0 — "open, edit the basics, save, run" (round-trip safe)**
- Project load/save via the format crate; modified flag; New/Open/Save/Save As/
  Recent/Exit; save prompts.
- Resource tree: kinds, groups, select, rename, double-click open, context menu
  (Create, Insert, Duplicate, Delete with confirm, Add Group, Properties),
  drag-reorder within a kind.
- Advanced mode only (simple-mode hiding deferred).
- Forms with working-copy/commit semantics; Window > Close All.
- Sprite form: load image (PNG/BMP/GIF incl. `_stripN` and animated GIF), origin
  (+Center, click), precise/separate checkboxes; frame preview.
- Sound form: load/save, kind, volume, pan, preload, effects checkboxes (no preview
  playback required).
- Background form: load, tile-set fields.
- Script form: plain multi-line editor with line numbers, basic syntax colouring
  (keywords/strings/comments/numbers), Ctrl+F find.
- Object form: all properties, event list + full event selector, action list
  (add by drag or double-click, reorder, delete, copy/paste, block indentation),
  palette loaded from `.lib` files, argument dialog for all argument kinds incl.
  applies-to/relative/NOT, code action via script editor.
- Room form: objects / settings / backgrounds tabs; grid + snap; add (incl. Shift
  paint), move (Ctrl), delete; Delete underlying; instance creation code; room
  creation code; zoom/pan.
- Global Game Settings form (all tabs as plain widgets).
- Game Information: plain text editing that preserves RTF (edit as text, keep
  header) — acceptable stopgap.
- Run normally (launch our runner with temp file, wait, report errors).

**P1 — full advanced-mode authoring**
- Time line form (all range operations).
- Path form with canvas editing, smooth preview, transforms, room backdrop.
- Font form with live preview.
- Room tiles tab (layers, region select) and views tab; sort/lock/isometric;
  remember room settings; remove-outside check.
- Mask dialog (bbox modes, shapes, tolerance, live bbox).
- Sprite editor frame operations (File, Edit, Transform menus; strip dialog).
- Code editor: auto-completion, argument hints (from `fnames`), bracket matching,
  live error checking (needs the GML parser crate), find/replace panel, F12 goto,
  snippets.
- Constants, Triggers, Included Files dialogs; Extension Packages selector (reading
  `.ged`).
- Scripts menu: Search in Scripts (all code), Check All Scripts, Check Resource
  Names, Show built-in/extension lists, Import/Export `.gml`.
- Run in Debug mode with debug window (watches, pause/step, messages, instances).
- Create Executable.
- Preferences dialog + backups + recent files persistence.
- Simple mode toggle.

**P2 — polish and rarely used**
- Built-in image editor (all tools), sprite editor Images/Animation effect menus.
- Rich-text Game Information editor (fonts, colours, alignment) and its options
  dialog if not done.
- Export/Import Resources (`.gmres`), `.gmspr` / `.gmbck` / `.gmtrg` files,
  constants text load/save.
- Object/time-line Show Information and printing; Show Object Information.
- External editors (code/image/sound), sound preview with effects.
- Extension install/uninstall (`.gex`), extension help.
- Tutorial side panel, news, web links, Publish (likely never).
- `.gmd` / `.gm6` import if the format spec covers it.

---------------------------------------------------------------------------------

## 24. Sources consulted
Manual chapters 106-114, 201-216, 301-303, "What is new"; tutorial texts
"Your First Game" and "Scrolling Shooters"; action library file headers (tab
captions) and `lib/Readme.txt` (action id list); manual screenshots of the main
form, object, event selector, room (all tabs), sprite, mask, sound, background,
path, font, time line, script, debug, triggers, included file, constants,
import/export, game information, preferences (forms tab) and extension packages
forms.

## Global Game Settings › Errors tab — VERIFIED (screenshot of GM 8.1 Lite)

Five checkboxes, in this order and wording:
1. Display error messages
2. Write error messages to file game_errors.log
3. Abort on all error messages
4. Treat uninitialized variables as value 0
5. Throw an error when arguments aren't initialised correctly.

Option 5 is GM 8.1's strict script-argument check (gml-language.md, bb_lang vs
bb_args0). In the file it is bit 1 (value 2) of the settings word whose bit 0 is
option 4 (file-formats.md, FF-18).
