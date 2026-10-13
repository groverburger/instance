# 3D graphics (`d3d_*`) — behavioural specification

Source: manual chapter 415 (`415_00` … `415_09`), texture functions from `405_06`,
cross-references to `runtime.md` §13–14. Everything here is paraphrased. Items the
manual does not settle are marked **UNSPECIFIED – recommended:** with the behaviour
an implementation should adopt until black-box observation says otherwise. Open
items are also listed in `OPEN_QUESTIONS.md` under `[D3D]`.

Tags: `[415_xx]` = stated in that manual page; `[I]` = inferred.

---

## 0. Scope and model

* 3D support is purely a rendering feature `[415_00]`. Collisions, mouse
  coordinates, views' room mapping and instance logic stay 2D. Nothing in this
  document changes the game loop.
* The renderer gets a **depth buffer**, a **projection** (camera + lens), a **world
  transform** (applied to every vertex drawn, 2D or 3D), optional **fog**,
  **lighting** and **back-face culling**.
* Implementation note (not behaviour): the current runner uses a software
  rasteriser (`OPEN_QUESTIONS.md` R-15). 3D needs perspective-correct texture/colour
  interpolation, a per-pixel depth buffer (24-bit in GM `[415_01]`; an `f32` buffer is
  an acceptable superset) and clip-space clipping against the near plane. Either
  extend the software rasteriser or move drawing to a GPU backend (e.g. wgpu); the
  2D results must remain pixel-compatible with §13 of runtime.md when 3D mode is off.

### 0.1 Coordinate conventions

* World space: `x` grows to the right in the room, `y` grows downward in the room
  (same numbers as 2D room coordinates), `z` is "depth". For anything drawn with the
  2D functions in 3D mode, `z = current depth` (§2).
* A larger depth/z is **further from the default camera** and appears smaller
  `[415_01]`; depth 0 appears at its 2D size `[415_01]`.
* All matrices in this document use **row vectors** (`v' = v · M`), as Direct3D
  does; "add" operations post-multiply (§6).
* Angles are in degrees everywhere.

---

## 1. Entering and leaving 3D mode `[415_01]`

| Function | Semantics |
|---|---|
| `d3d_start()` | Switch the renderer to 3D mode. Returns true on success. **Recommended:** always succeeds (returns 1) once 3D is implemented; returns 0 only if the backend cannot create a depth buffer. |
| `d3d_end()` | Leave 3D mode. Returns true on success. |
| `d3d_set_hidden(enable)` | Hidden-surface removal (depth testing) on/off. |
| `d3d_set_perspective(enable)` | Whether the *default* projection (§5.1) is perspective (true) or orthographic (false). |
| `d3d_set_zwriteenable(on_off)` | Not documented in this manual (identifier from `fnames`). **Recommended:** enables/disables writing to the depth buffer while depth testing stays as set by `d3d_set_hidden`. Default true. |
| `d3d_set_depth(depth)` | §2. |

Effects of `d3d_start()`:
1. Depth testing is switched on, using a 24-bit depth buffer `[415_01]`.
   Test = "draw if new z ≤ stored z" (**UNSPECIFIED – recommended** `LESSEQUAL`, so
   equal depths let the later drawing win; equal depths still z-fight after
   transformation, as the manual warns). Depth writes on.
2. The default projection becomes a **perspective** projection (§5.1) `[415_01]`.
3. The vertical axis appears reversed relative to 2D: with the default
   perspective projection room position (0,0) is at the **bottom-left** of the view
   `[415_01]`. (See §5.1 and open question D3D-1.)
4. **UNSPECIFIED – recommended** for the other state: `d3d_start` sets
   `hidden = true`, `perspective = true`, `zwrite = true`; culling, lighting, fog,
   shading, lights, the world transform and the transform stack keep their current
   values (initial values at game start: culling off, lighting off, fog off, smooth
   shading on, ambient black, no lights enabled, identity transform, empty stack).
5. The depth buffer is cleared to "far" at the start of every view pass (runtime.md
   §13.1 step 3) while in 3D mode, and on `d3d_start` itself.

Effects of `d3d_end()`: depth testing and writing off, default projection back to
the 2D orthographic view mapping of runtime.md §12.2. Fog and lighting are not
applied in 2D mode (their settings are remembered). **Recommended:** the world
transform keeps applying in 2D mode too (§6.4).

Flags set while not in 3D mode (`d3d_set_hidden`, `_perspective`, `_culling`,
`_fog`, `_lighting`, `_shading`, `_zwriteenable`) are stored and take effect when 3D
mode is active; `d3d_start` then overrides `hidden`/`perspective`/`zwrite` as above.

Restrictions while in 3D mode (manual guidance, not enforced): backgrounds,
foregrounds and tiles do not tile/line up correctly under perspective; mouse
coordinates are not transformed; collisions remain 2D; room transitions are not
performed (runtime.md §3.4) `[415_00]`.

---

## 2. 2D drawing in 3D mode ("easy drawing") `[415_02]`

* Every 2D drawing function (sprites, backgrounds drawn by code, shapes, text,
  `draw_primitive_*`, surfaces) emits geometry in the plane `z = current depth`.
* Before each instance's Draw event (and before its default draw) the current
  depth is set to that instance's `depth` `[415_02]`. `d3d_set_depth(depth)` changes
  it for the following drawing until the next instance starts drawing.
* **UNSPECIFIED – recommended:** tiles are drawn at `z = layer depth`; particle
  systems at `z = system depth`; the cursor sprite at `z = 0` with depth testing off.
* Backgrounds/foregrounds (room settings) in 3D mode: **UNSPECIFIED – recommended:**
  draw them as in 2D, before (backgrounds) / after (foregrounds) the depth-sorted
  list, at `z = 0` with depth testing and depth writes temporarily disabled, so they
  act as a backdrop/overlay without occluding by depth. The room background colour
  fill is the colour clear of the pass.
* The depth-sorted draw order of runtime.md §13.2 is unchanged; with depth testing
  on, order only matters for equal z and for blending.
* All 2D-drawn geometry has **no normal** (treated as the zero vector), so with
  lighting enabled it receives only ambient light (§8.4).
* Transparency: there is no alpha test. Pixels with alpha 0 still write depth when
  depth writes are on, so draw transparent/translucent things after the opaque
  things behind them. **UNSPECIFIED** — see D3D-6.

---

## 3. Primitives in 3D `[415_03]`, `[415_08]`

### 3.1 Functions

| Function | Vertex attributes |
|---|---|
| `d3d_primitive_begin(kind)` | start, untextured |
| `d3d_primitive_begin_texture(kind,texid)` | start, with texture `texid` (−1 = none) |
| `d3d_vertex(x,y,z)` | position; colour/alpha = current draw colour/alpha |
| `d3d_vertex_color(x,y,z,col,alpha)` | position, own colour/alpha |
| `d3d_vertex_texture(x,y,z,xtex,ytex)` | position, texcoord; colour = draw colour/alpha |
| `d3d_vertex_texture_color(x,y,z,xtex,ytex,col,alpha)` | position, texcoord, own colour/alpha |
| `d3d_vertex_normal(x,y,z,nx,ny,nz)` | position, normal; draw colour/alpha |
| `d3d_vertex_normal_color(x,y,z,nx,ny,nz,col,alpha)` | position, normal, colour |
| `d3d_vertex_normal_texture(x,y,z,nx,ny,nz,xtex,ytex)` | position, normal, texcoord; draw colour/alpha |
| `d3d_vertex_normal_texture_color(x,y,z,nx,ny,nz,xtex,ytex,col,alpha)` | all |
| `d3d_primitive_end()` | draw the primitive |

* `kind` is one of `pr_pointlist, pr_linelist, pr_linestrip, pr_trianglelist,
  pr_trianglestrip, pr_trianglefan` (assembly rules as for 2D primitives `[405_06]`).
* "Draw colour/alpha" is sampled **when the vertex call is made** (so
  `draw_set_color` between vertices gives per-vertex colours `[415_03]`).
* Vertices without a normal get the zero normal.
* Nothing is drawn until `d3d_primitive_end()`; the world transform, projection and
  render state in effect **at end** are used.
* Texture coordinates: (0,0) is the top-left of the texture image; values in 0..1
  span the texture (see §10 for non-power-of-two sizes). The manual notes that with
  y-up projections textures may need flipping by the user `[415_03]`.
* Textured primitives are modulated by vertex colour/alpha (if
  `texture_set_blending(true)`, runtime.md §13.4).
* **UNSPECIFIED – recommended:** incomplete trailing vertices (e.g. 2 vertices left
  over in a triangle list) are ignored; a new `begin` without `end` discards the
  pending primitive; no vertex limit.
* 2D `draw_primitive_*` / `draw_vertex*` behave identically with `z = current depth`.

### 3.2 Back-face culling `[415_03]`
* `d3d_set_culling(cull)`: when true, triangles seen from their back are not drawn.
* The **front** of a triangle is the side from which its vertices appear in
  **clockwise** order on screen (y down). Default: both sides drawn. (An earlier
  revision of this spec said counter-clockwise; games written against real GM 8.1,
  such as the gamekit's Emberwood, showed that to be backwards — see D3D-3.)
* Implementation: compute the signed area of the projected triangle in screen
  coordinates and drop it when it appears counter-clockwise to the viewer. **UNSPECIFIED**
  whether GM's projections mirror the image (D3D-1/D3D-3); built-in shapes (§4) must
  be generated so that their outward faces are front faces under the same rule
  `[415_04]`.
* Culling applies to 3D and 2D-emitted triangles alike while in 3D mode. Points and
  lines are never culled.

---

## 4. Basic shapes `[415_04]`

Common rules:
* Drawn in the current draw colour and alpha; `texid = -1` means untextured.
* `hrepeat`/`vrepeat`: number of texture repetitions along the horizontal and
  vertical edge of **each face**. Texture coordinates run from 0 to `hrepeat`
  horizontally and 0 to `vrepeat` vertically, **unscaled** by `texture_get_width/height`
  (so non-power-of-two images show their padding, §10). Repetition beyond 1 is only
  visible with `texture_set_repeat(true)` (**UNSPECIFIED**, D3D-4).
* Correct outward normals are generated automatically `[415_08]`, and the shapes
  work with culling on `[415_04]`.
* Coordinates are used as given (no sorting of corners); swapped corners simply
  mirror the shape (and its winding).
* `steps` = number of segments around the vertical axis (manual suggests 24).
  **Recommended:** clamp to 3..128.
* **UNSPECIFIED – recommended tessellation and texture mapping** (exact GM
  geometry not documented):

Let `cx=(x1+x2)/2, cy=(y1+y2)/2, rx=(x2-x1)/2, ry=(y2-y1)/2`.
Texture "top" (v = 0) is at the `z2` edge and v = `vrepeat` at the `z1` edge for all
side faces (D3D-5).

| Shape | Geometry |
|---|---|
| `d3d_draw_block(x1,y1,z1,x2,y2,z2,texid,hrepeat,vrepeat)` | Axis-aligned box with the given opposite corners; 6 quads (12 triangles). Side faces: u along the face's horizontal edge (from the x1/y1 end to the x2/y2 end), v from z2 to z1. Faces z=z1 and z=z2: u along x (x1→x2), v along y (y1→y2). Normals ±x, ±y, ±z outward. |
| `d3d_draw_cylinder(x1,y1,z1,x2,y2,z2,texid,hrepeat,vrepeat,closed,steps)` | Vertical (z-axis) elliptic cylinder inscribed in the box. Ring points `P_i = (cx + rx·cos a_i, cy + ry·sin a_i)`, `a_i = 2π·i/steps`, i = 0..steps (seam duplicated). Side: quad strip between z2 (v=0) and z1 (v=vrepeat), u = hrepeat·i/steps. Normals radial (`(cos a/rx, sin a/ry, 0)` normalised). If `closed`: caps at z1 and z2 as triangle fans around (cx,cy) with normals −z / +z and planar texture mapping `u = hrepeat·(px−x1)/(x2−x1)`, `v = vrepeat·(py−y1)/(y2−y1)`. |
| `d3d_draw_cone(…same args…)` | Base ellipse at z1, apex at (cx,cy,z2). Side: triangles from each ring segment to the apex; u as cylinder, v = 0 at apex, vrepeat at base. Normals: perpendicular to the slant (normalise `(cos a/rx, sin a/ry, k)` with `k = r̄/|z2−z1|`, r̄ = mean radius). If `closed`: base cap at z1 (the apex needs none). |
| `d3d_draw_ellipsoid(x1,y1,z1,x2,y2,z2,texid,hrepeat,vrepeat,steps)` | Ellipsoid inscribed in the box, `rz = (z2−z1)/2`, `cz` midpoint. `steps` longitudinal segments, `max(2, steps/2)` latitudinal rings; latitude from the z2 pole (v=0) to the z1 pole (v=vrepeat), u = hrepeat·i/steps. Normals = normalised `((px−cx)/rx², (py−cy)/ry², (pz−cz)/rz²)`. |
| `d3d_draw_wall(x1,y1,z1,x2,y2,z2,texid,hrepeat,vrepeat)` | Vertical quad with corners (x1,y1,z1), (x2,y2,z1), (x2,y2,z2), (x1,y1,z2). u: 0 at (x1,y1) → hrepeat at (x2,y2); v: 0 at z2 → vrepeat at z1. Single-sided for culling; normal horizontal, perpendicular to the wall (`(y2−y1, −(x2−x1), 0)` normalised; sign must match the front-face rule). If x1=x2 and y1=y2 nothing is drawn. |
| `d3d_draw_floor(x1,y1,z1,x2,y2,z2,texid,hrepeat,vrepeat)` | Quad with corners (x1,y1,z1), (x2,y1,z1), (x2,y2,z2), (x1,y2,z2) — a floor sloped along y when z1≠z2 `[415_04]` "(slanted) floor". u along x (x1→x2, 0→hrepeat), v along y (y1→y2, 0→vrepeat). Normal = normalised cross product giving +z for a flat floor. Single-sided. |

---

## 5. Projection (camera) `[415_05]`

The projection is per view pass. **At the start of drawing each view (or the room
when views are off) it is reset to the default** `[415_05]`; a projection set in a
Draw event stays in effect for the remainder of that pass (later instances too).

### 5.1 Default projection
Let the pass rectangle be `(vx, vy, vw, vh, vangle)` (the view, or the room
`(0,0,room_width,room_height,0)` without views).

* Perspective (default after `d3d_start`): camera placed at distance `vw` from the
  z = 0 plane on the negative-depth side, centred on the rectangle, looking toward
  +z `[415_01]` ("distance = width of the room/view"; depths below `-vw` are behind
  the camera). Field of view chosen so that the plane z = 0 maps exactly onto the
  view rectangle (`fovy = 2·atan(vh / (2·vw))`, aspect `vw/vh`); near 1, far 32000.
  Up direction: world **+y appears up on screen** (the manual's "y reversed", origin
  at bottom-left `[415_01]`), rotated by `vangle` like `view_angle`.
  Equivalent: `look_at_lh(eye=(cx,cy,−vw), at=(cx,cy,0), up=(−sin θ, cos θ, 0))` with
  `θ = vangle` (**[I]** sign of θ), `perspective_fov_lh(fovy, vw/vh, 1, 32000)`.
  **This orientation is the top black-box verification item (D3D-1).**
* If `d3d_set_perspective(false)`: the default is the orthographic projection of
  §5.3 for the rectangle (2D-oriented, y down), keeping depth testing.
* Note on the manual: it describes the default as looking "along the negative z
  axis"; that contradicts "greater depth looks smaller" with z = depth. This spec
  follows the depth statements (D3D-1).

### 5.2 `d3d_set_projection(xfrom,yfrom,zfrom, xto,yto,zto, xup,yup,zup)`
Camera at `from`, looking at `to`, with `up` giving the camera's upward direction
(need not be perpendicular to the view direction; it is orthogonalised) `[415_05]`.
Build the view matrix as Direct3D's left-handed look-at:
```
zaxis = normalize(to − from)
xaxis = normalize(cross(up, zaxis))
yaxis = cross(zaxis, xaxis)
view  = rows [xaxis.x yaxis.x zaxis.x 0; xaxis.y yaxis.y zaxis.y 0;
              xaxis.z yaxis.z zaxis.z 0; −dot(xaxis,from) −dot(yaxis,from) −dot(zaxis,from) 1]
```
This makes the FPS example of the manual work: looking from (100,100,10) toward +x
with up (0,0,1) puts room +y (south) on the right of the screen, so a camera whose
look point is `(x+cos(dir), y−sin(dir), z)` faces GM's `direction`.
Lens: **UNSPECIFIED – recommended** `fovy = 45°`, aspect = `vw/vh` of the current
pass, near 1, far 32000 (the documented defaults of the `_ext` form `[415_05]`).
Degenerate input (from = to, or up parallel to view direction): **recommended**
ignore the call.

### 5.3 `d3d_set_projection_ext(…, angle, aspect, znear, zfar)`
As §5.2 plus: `angle` = field of view in degrees (**[I]** vertical, as Direct3D's
fov projection), `aspect` = width/height ratio, `znear`/`zfar` clipping distances;
geometry nearer than znear or further than zfar is clipped `[415_05]`. `znear` must
be > 0 `[415_05]`; **recommended:** if `znear ≤ 0` use 1; if `zfar ≤ znear` ignore the
call. Projection matrix = `perspective_fov_lh(angle, aspect, znear, zfar)`:
```
ys = 1/tan(angle/2); xs = ys/aspect
[xs 0 0 0; 0 ys 0 0; 0 0 zf/(zf−zn) 1; 0 0 −zn·zf/(zf−zn) 0]
```

### 5.4 `d3d_set_projection_ortho(x,y,w,h,angle)` / `d3d_set_projection_perspective(x,y,w,h,angle)`
* `_ortho`: a plain orthographic projection of room area `(x,y,w,h)` rotated by
  `angle`, exactly like a 2D view (y down; this is what makes the manual's score
  overlay example render upright) `[415_05]`. **UNSPECIFIED – recommended** depth
  range z ∈ [−32000, 32000] (so negative depths remain visible). Depth testing is
  not changed (the manual's overlay example turns it off itself).
* `_perspective`: the default-style perspective projection (§5.1) of area
  `(x,y,w,h)` rotated by `angle`.

### 5.5 Interaction with surfaces and views
* `surface_set_target` installs an orthographic projection of the surface
  (runtime.md §14.1) even in 3D mode; `surface_reset_target` restores the pass
  projection that was active before.
* **UNSPECIFIED – recommended:** each surface has its own depth buffer, cleared when
  it becomes the target while in 3D mode (D3D-7).

---

## 6. Transformations `[415_06]`

### 6.1 Current world transform
A single global 4×4 matrix `W` (initially identity) applied to **every vertex drawn**
(3D primitives, shapes, models, and 2D drawing) before the view/projection.
Rotation and scaling are about the **world origin**, not about the drawn object
`[415_06]`.

### 6.2 Set / add
| Function | Effect |
|---|---|
| `d3d_transform_set_identity()` | `W = I` |
| `d3d_transform_set_translation(xt,yt,zt)` | `W = T(xt,yt,zt)` |
| `d3d_transform_set_scaling(xs,ys,zs)` | `W = S(xs,ys,zs)` |
| `d3d_transform_set_rotation_x/_y/_z(angle)` | `W = Rx/Ry/Rz(angle)` |
| `d3d_transform_set_rotation_axis(xa,ya,za,angle)` | `W = Raxis(a, angle)` |
| `d3d_transform_add_*` (same args) | `W = W · M` (the new transform is applied **after** the existing ones) |

Order of composition: with row vectors, `add` post-multiplies, so in
`set_identity; add_rotation_x(90); add_translation(100,100,10)` a vertex is first
rotated, then translated — the order the manual's example requires `[415_06]`.

Rotation sense (**[I]**, must be verified, D3D-2): a positive angle about +z turns
+x toward **−y**, i.e. counter-clockwise as seen in the room's 2D (y-down) top view,
matching GM's `direction`/`image_angle` convention (so
`d3d_transform_add_rotation_z(direction)` turns a model built facing +x to face
`direction`). The x/y/axis rotations use the same handedness:
```
θ = angle in radians; c = cos θ; s = sin θ        (row-vector matrices)
Rz = [ c −s 0 0;  s c 0 0;  0 0 1 0;  0 0 0 1]    maps (1,0,0) → (c, −s, 0)
Rx = [1 0 0 0; 0 c −s 0; 0 s c 0; 0 0 0 1]       maps (0,1,0) → (0, c, −s)
Ry = [ c 0 s 0; 0 1 0 0; −s 0 c 0; 0 0 0 1]      maps (0,0,1) → (−s, 0, c)  [I]
Raxis(a,θ) = Rodrigues rotation about normalize(a) by −θ (right-handed), so that
             Raxis((0,0,1),θ) = Rz(θ). Zero axis → identity.
```

### 6.3 Transform stack
A LIFO stack of matrices, independent of `W`.

| Function | Returns | Effect |
|---|---|---|
| `d3d_transform_stack_clear()` | — | empty the stack |
| `d3d_transform_stack_empty()` | bool | whether the stack is empty |
| `d3d_transform_stack_push()` | bool | push a copy of `W`; false if the stack is full (nothing pushed) |
| `d3d_transform_stack_pop()` | bool | `W = top`, remove top; false if empty (W unchanged) |
| `d3d_transform_stack_top()` | bool | `W = top`, keep it; false if empty |
| `d3d_transform_stack_discard()` | bool | remove top without changing `W`; false if empty |

**UNSPECIFIED – recommended** capacity: 32 entries (the manual only says the stack
can run out). The stack persists across steps and rooms.

### 6.4 Scope
**UNSPECIFIED – recommended:** `W` and the stack persist until changed (they are
not reset per instance, per view or per room) and apply in 2D mode as well as in 3D
mode. The manual advises always resetting to identity after use `[415_06]`.

---

## 7. Fog `[415_07]`

`d3d_set_fog(enable, color, start, end)`: enable/disable fog with colour `color`;
`start` = distance where fog begins, `end` = distance where fog is total.
* Fog factor per pixel (GM prefers per-pixel "table" fog, falling back to per-vertex
  fog `[415_07]`; implement per-pixel): `d` = view-space depth (distance along the
  camera's viewing axis), `f = clamp((end − d)/(end − start), 0, 1)`;
  `rgb = f·rgb + (1−f)·fog_rgb`; alpha unchanged. Linear fog.
* Applies to everything drawn in 3D mode while enabled, including 2D-emitted
  geometry and lit/unlit geometry alike; not applied in 2D mode.
* `end ≤ start`: **recommended** treat as a step at `start`.

---

## 8. Lighting `[415_08]`

### 8.1 Switches
* `d3d_set_lighting(enable)`: when off (default), vertex colours are used as-is.
* `d3d_set_shading(smooth)`: true (default) = Gouraud interpolation of vertex
  colours; false = flat: each triangle takes the colour of its **first** vertex
  (Direct3D convention, **[I]**). Shading applies whether or not lighting is on.

### 8.2 Lights
Only diffuse lighting exists (no specular) `[415_08]`. Usually 8 lights can be
active at once `[415_08]`.

| Function | Semantics |
|---|---|
| `d3d_light_define_direction(ind,dx,dy,dz,col)` | Light `ind` becomes directional; `(dx,dy,dz)` = direction the light travels. Does not enable it. |
| `d3d_light_define_point(ind,x,y,z,range,col)` | Light `ind` becomes a point light at (x,y,z) whose intensity falls off over `range`. Does not enable it. |
| `d3d_light_enable(ind,enable)` | Turn light `ind` on/off. Enabling an undefined light: **recommended** no effect until defined. |
| `d3d_light_define_ambient(col)` | Ambient light colour; default black (`$000000`) `[415_08]`. |

**UNSPECIFIED – recommended:** `ind` is any integer ≥ 0 (store in a map). When more
than 8 lights are enabled only the 8 lowest indices are used. Light positions and
directions are in world space and are *not* affected by the world transform.

### 8.3 Lighting equation (recommended, Direct3D fixed-function style, **[I]**)
Per vertex, with `N` = world-transformed normal (inverse-transpose of `W`'s 3×3,
then normalised; zero normal stays zero), `C` = vertex colour (r,g,b in 0..1),
`A` = ambient colour:
```
L_dir   = normalize(−direction)                         (directional)
L_point = normalize(lightpos − P); d = |lightpos − P|
att     = (d ≥ range) ? 0 : 1 − d/range                 (point; linear, D3D-8)
sum     = A + Σ_enabled light_col · max(0, N·L) · att   (att = 1 for directional)
result  = C · clamp(sum, 0, 1)         (alpha = vertex alpha, unchanged)
```
The result is then interpolated per §8.1 and textured/fogged as usual.

### 8.4 Normals
Normals come from the `*_normal*` vertex functions; built-in shapes generate them
`[415_08]`; everything else has the zero normal and therefore shows `C · A` only
(sprites drawn with lighting on appear in the ambient colour — black by default).

---

## 9. Models `[415_09_1models]`

A model is a recorded list of primitive and shape commands, drawn later in one call.
Models carry **no texture**; the texture is given when drawing, so a model uses at
most one texture `[415_09_1models]`.

| Function | Semantics |
|---|---|
| `d3d_model_create()` | New empty model; returns its id. **Recommended:** ids 0,1,2,… never reused while the game runs. |
| `d3d_model_destroy(ind)` | Free the model; id becomes invalid. |
| `d3d_model_clear(ind)` | Remove all contents. |
| `d3d_model_save(ind,fname)` | Write the model to a file (§9.2). Returns whether successful (**recommended**). |
| `d3d_model_load(ind,fname)` | Replace the model's contents with the file's (§9.2). Returns whether successful (**recommended**). |
| `d3d_model_draw(ind,x,y,z,texid)` | Draw the model offset by (x,y,z) with texture `texid` (−1 none). Rotation/scaling via the world transform `[415_09_1models]`. |
| `d3d_model_primitive_begin(ind,kind)` / `_primitive_end(ind)` | As §3 without texture. |
| `d3d_model_vertex*(ind, …)` | Same arguments as the `d3d_vertex*` counterparts, model id first, no texture id. |
| `d3d_model_block/_cylinder/_cone/_ellipsoid/_wall/_floor(ind, …)` | As the `d3d_draw_*` shapes, model id first, no texid. |

* Drawing: each model vertex `v` is transformed as `(v + (x,y,z)) · W` (the offset is
  applied in model space, before the world transform) — this makes
  `set_rotation_z(dir); add_translation(px,py,pz); d3d_model_draw(m,0,0,0,tex)` work.
* Vertices recorded **without** a colour: **UNSPECIFIED – recommended** use the draw
  colour/alpha current at `d3d_model_draw` time (the file format stores no colour
  for them, so creation-time colours could not survive save/load). Shapes in models
  likewise use the draw-time colour (D3D-9).
* Unknown model id: **recommended** runtime error "model does not exist" (or ignore
  in non-strict mode).
* Implementations may compile models into vertex buffers; shapes are tessellated at
  record time with the §4 rules.

### 9.1 Recording rules
Commands are stored in call order. A vertex outside `primitive_begin/end`:
**recommended** ignored. `primitive_end` without begin: ignored.

### 9.2 Model file format **[I — not in the manual; verify by saving a model with GM 8.1]**
Plain text, one value per field, lines separated by CRLF:
```
100                      (format version)
<n>                      (number of command lines that follow)
<n lines>: kind v1 v2 v3 v4 v5 v6 v7 v8 v9 v10
```
Each command line has the command number followed by exactly 10 numbers (unused
trailing ones written as 0), numbers written as decimal reals (recommended
`%.4f`; readers must accept any real syntax and any whitespace):

| kind | command | values |
|---|---|---|
| 0 | primitive_begin | kind |
| 1 | primitive_end | — |
| 2 | vertex | x y z |
| 3 | vertex_color | x y z col alpha |
| 4 | vertex_texture | x y z xtex ytex |
| 5 | vertex_texture_color | x y z xtex ytex col alpha |
| 6 | vertex_normal | x y z nx ny nz |
| 7 | vertex_normal_color | x y z nx ny nz col alpha |
| 8 | vertex_normal_texture | x y z nx ny nz xtex ytex |
| 9 | vertex_normal_texture_color | x y z nx ny nz xtex ytex col alpha |
| 10 | block | x1 y1 z1 x2 y2 z2 hrepeat vrepeat |
| 11 | cylinder | x1 y1 z1 x2 y2 z2 hrepeat vrepeat closed steps |
| 12 | cone | x1 y1 z1 x2 y2 z2 hrepeat vrepeat closed steps |
| 13 | ellipsoid | x1 y1 z1 x2 y2 z2 hrepeat vrepeat steps |
| 14 | wall | x1 y1 z1 x2 y2 z2 hrepeat vrepeat |
| 15 | floor | x1 y1 z1 x2 y2 z2 hrepeat vrepeat |

Loading a file whose first line is not 100, or with an unknown kind: fail
(return false, model left empty). This layout is the commonly observed one; it must
be confirmed by black-box saving (D3D-10). Because shapes are stored as commands,
models must keep commands (not only tessellated geometry) to be saved.

---

## 10. Textures `[405_06]`

* `sprite_get_texture(spr,subimg)`, `background_get_texture(back)`,
  `surface_get_texture(id)` return a texture id (−1 if the resource does not exist).
  **Recommended:** texture ids are stable for the lifetime of the image (sprite
  replace/delete invalidates them); the manual warns ids can change, which is why
  models do not store them `[415_09_1models]`. `subimg` is taken modulo the frame count
  (**recommended**, like `draw_sprite`).
* Texture sizes are powers of two in GM. An image of `w×h` occupies the top-left of a
  `W×H` texture with `W = next_pow2(w)`, `H = next_pow2(h)`; the padding is
  transparent black (**recommended**). `texture_get_width(tex) = w/W`,
  `texture_get_height(tex) = h/H` (values in 0..1) `[405_06]`. Texture coordinate
  (u,v) addresses the *padded* texture; use `texture_get_width/height` as the far
  edge to address only the image. Implementations must emulate this padding so
  that games using raw 0..1 or `hrepeat` coordinates on non-power-of-two images look
  the same.
* `texture_set_repeat(repeat)`: true → wrap addressing; false → coordinates outside
  0..1 do not repeat (**recommended** clamp-to-edge of the padded texture). Drawing
  any sprite or background resets it to false `[405_06]`.
* `texture_set_interpolation(linear)`: bilinear vs nearest sampling for all drawing.
  `texture_set_blending(blend)`: whether vertex colour/alpha modulate texels.
* `texture_preload(texid)`, `texture_set_priority(texid,prio)`: memory hints; may be
  no-ops (priority default 0).
* No mip-mapping (**recommended**).

---

## 11. Constants

| Constant | Value | Source |
|---|---|---|
| `pr_pointlist` | 1 | [I] (builtins.json; Direct3D primitive type numbering) |
| `pr_linelist` | 2 | [I] |
| `pr_linestrip` | 3 | [I] |
| `pr_trianglelist` | 4 | [I] |
| `pr_trianglestrip` | 5 | [I] |
| `pr_trianglefan` | 6 | [I] |

There are no `d3d_*` constants in `fnames`. Model command numbers (§9.2) are file
values, not GML constants.

## 12. Function return values (summary)
`d3d_start`, `d3d_end`, `d3d_transform_stack_*` (except clear), `d3d_model_create`,
`_load`, `_save`, `texture_get_*`, `*_get_texture` return values as described;
every other `d3d_*` function returns 0.
