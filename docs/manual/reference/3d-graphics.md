# 3D graphics

3D mode, drawing primitives and shapes, projections, transformations, fog, lighting and models.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [3D mode](#3d-mode)
- [Easy drawing in 3D](#easy-drawing-in-3d)
- [Drawing polygons](#drawing-polygons)
- [Drawing basic shapes](#drawing-basic-shapes)
- [Viewing the world](#viewing-the-world)
- [Transformations](#transformations)
- [Fog](#fog)
- [Lighting](#lighting)
- [Models](#models)

## 3D mode

Turning 3D mode on and setting how things are drawn.

### d3d_start

`d3d_start()`

Switches to 3D mode; returns whether it succeeded.

### d3d_end

`d3d_end()`

Leaves 3D mode; returns whether it succeeded.

### d3d_set_hidden

`d3d_set_hidden(enable)`

Turns depth testing (hidden surface removal) on or off.

### d3d_set_perspective

`d3d_set_perspective(enable)`

Turns perspective projection on or off.

### d3d_set_zwriteenable

`d3d_set_zwriteenable(on_off)`

Undocumented; turns writing to the depth buffer on or off.

## Easy drawing in 3D

Giving ordinary 2D drawing a depth.

### d3d_set_depth

`d3d_set_depth(depth)`

Sets the depth value used for subsequent 2D drawing.

## Drawing polygons

Primitives built from vertices, with colors, textures and normals.

### d3d_primitive_begin

`d3d_primitive_begin(kind)`

Begins a 3D primitive of the given pr_ kind (points, lines, strips, triangles, fan).

### d3d_vertex

`d3d_vertex(x, y, z)`

Adds a vertex at (x,y,z) using the currently set color and alpha.

### d3d_vertex_color

`d3d_vertex_color(x, y, z, color, alpha)`

Adds a vertex with its own color and alpha, allowing gradients.

### d3d_primitive_end

`d3d_primitive_end()`

Finishes the primitive and draws it.

### d3d_primitive_begin_texture

`d3d_primitive_begin_texture(kind, texture)`

Begins a 3D primitive of the given kind that uses the given texture.

### d3d_vertex_texture

`d3d_vertex_texture(x, y, z, xtex, ytex)`

Adds a vertex with texture coordinates, blended with the current color and alpha.

### d3d_vertex_texture_color

`d3d_vertex_texture_color(x, y, z, xtex, ytex, color, alpha)`

Adds a vertex with texture coordinates and its own color and alpha.

### d3d_set_culling

`d3d_set_culling(cull)`

Turns backface culling on or off.

## Drawing basic shapes

Blocks, cylinders, cones, ellipsoids, walls and floors.

### d3d_draw_block

`d3d_draw_block(x1, y1, z1, x2, y2, z2, texture, hrepeat, vrepeat)`

Draws a block between two opposite corners in the current color; texture -1 means no texture; hrepeat and vrepeat set texture tiling per face.

### d3d_draw_cylinder

`d3d_draw_cylinder(x1, y1, z1, x2, y2, z2, texture, hrepeat, vrepeat, closed, steps)`

Draws a vertical cylinder in a bounding box; closed caps the ends, steps sets rotational detail (about 24); texture -1 means no texture; hrepeat and vrepeat set texture tiling per face.

### d3d_draw_cone

`d3d_draw_cone(x1, y1, z1, x2, y2, z2, texture, hrepeat, vrepeat, closed, steps)`

Draws a vertical cone in a bounding box; closed caps the base, steps sets detail (about 24); texture -1 means no texture; hrepeat and vrepeat set texture tiling per face.

### d3d_draw_ellipsoid

`d3d_draw_ellipsoid(x1, y1, z1, x2, y2, z2, texture, hrepeat, vrepeat, steps)`

Draws an ellipsoid in a bounding box with steps rotational detail (about 24); texture -1 means no texture; hrepeat and vrepeat set texture tiling per face.

### d3d_draw_wall

`d3d_draw_wall(x1, y1, z1, x2, y2, z2, texture, hrepeat, vrepeat)`

Draws a vertical wall between two corners; texture -1 means no texture; hrepeat and vrepeat set texture tiling per face.

### d3d_draw_floor

`d3d_draw_floor(x1, y1, z1, x2, y2, z2, texture, hrepeat, vrepeat)`

Draws a possibly slanted floor between two corners; texture -1 means no texture; hrepeat and vrepeat set texture tiling per face.

## Viewing the world

Projections: where the camera is and where it looks.

### d3d_set_projection

`d3d_set_projection(xfrom, yfrom, zfrom, xto, yto, zto, xup, yup, zup)`

Sets the camera from a position, a target point and an up vector.

### d3d_set_projection_ext

`d3d_set_projection_ext(xfrom, yfrom, zfrom, xto, yto, zto, xup, yup, zup, angle, aspect, znear, zfar)`

Like d3d_set_projection but also takes field-of-view angle, aspect ratio and near/far clipping planes.

### d3d_set_projection_ortho

`d3d_set_projection_ortho(x, y, width, height, angle)`

Sets an orthographic projection of a room area, rotated by the given angle.

### d3d_set_projection_perspective

`d3d_set_projection_perspective(x, y, width, height, angle)`

Sets a perspective projection of a room area, rotated by the given angle.

## Transformations

Translating, rotating and scaling what is drawn, and the transformation stack.

### d3d_transform_set_identity

`d3d_transform_set_identity()`

Resets the world transformation to identity.

### d3d_transform_set_translation

`d3d_transform_set_translation(xt, yt, zt)`

Replaces the transformation with a translation by the given vector.

### d3d_transform_set_scaling

`d3d_transform_set_scaling(xs, ys, zs)`

Replaces the transformation with a scaling by the given factors.

### d3d_transform_set_rotation_x

`d3d_transform_set_rotation_x(angle)`

Replaces the transformation with a rotation about the x-axis by the given angle.

### d3d_transform_set_rotation_y

`d3d_transform_set_rotation_y(angle)`

Replaces the transformation with a rotation about the y-axis by the given angle.

### d3d_transform_set_rotation_z

`d3d_transform_set_rotation_z(angle)`

Replaces the transformation with a rotation about the z-axis by the given angle.

### d3d_transform_set_rotation_axis

`d3d_transform_set_rotation_axis(xa, ya, za, angle)`

Replaces the transformation with a rotation about an arbitrary axis vector by the given angle.

### d3d_transform_add_translation

`d3d_transform_add_translation(xt, yt, zt)`

Appends a translation by the given vector to the transformation.

### d3d_transform_add_scaling

`d3d_transform_add_scaling(xs, ys, zs)`

Appends a scaling by the given factors to the transformation.

### d3d_transform_add_rotation_x

`d3d_transform_add_rotation_x(angle)`

Appends a rotation about the x-axis by the given angle to the transformation.

### d3d_transform_add_rotation_y

`d3d_transform_add_rotation_y(angle)`

Appends a rotation about the y-axis by the given angle to the transformation.

### d3d_transform_add_rotation_z

`d3d_transform_add_rotation_z(angle)`

Appends a rotation about the z-axis by the given angle to the transformation.

### d3d_transform_add_rotation_axis

`d3d_transform_add_rotation_axis(xa, ya, za, angle)`

Appends a rotation about an arbitrary axis vector by the given angle.

### d3d_transform_stack_clear

`d3d_transform_stack_clear()`

Empties the transformation stack.

### d3d_transform_stack_empty

`d3d_transform_stack_empty()`

Returns whether the transformation stack has no entries.

### d3d_transform_stack_push

`d3d_transform_stack_push()`

Pushes the current transformation onto the stack; returns whether there was room.

### d3d_transform_stack_pop

`d3d_transform_stack_pop()`

Pops the top transformation and makes it current; returns whether the stack had one.

### d3d_transform_stack_top

`d3d_transform_stack_top()`

Makes the top stack transformation current without removing it; returns whether one existed.

### d3d_transform_stack_discard

`d3d_transform_stack_discard()`

Removes the top stack transformation without making it current; returns whether one existed.

## Fog

Fading distant things into a color.

### d3d_set_fog

`d3d_set_fog(enable, color, start, end)`

Enables or disables fog with a color, a start distance and an end distance where visibility is fully lost.

## Lighting

Light sources and shading.

### d3d_set_lighting

`d3d_set_lighting(enable)`

Enables or disables lighting.

### d3d_set_shading

`d3d_set_shading(smooth)`

Chooses smooth (true) or flat shading.

### d3d_light_define_ambient

`d3d_light_define_ambient(color)`

Sets the ambient light color, the baseline brightness of unlit areas (default black).

### d3d_light_define_direction

`d3d_light_define_direction(light, dx, dy, dz, color)`

Defines directional light light with direction vector and color; does not enable it.

### d3d_light_define_point

`d3d_light_define_point(light, x, y, z, range, color)`

Defines point light light at a position with a range over which intensity falls off, and a color; does not enable it.

### d3d_light_enable

`d3d_light_enable(light, enable)`

Turns light number light on or off.

### d3d_vertex_normal

`d3d_vertex_normal(x, y, z, nx, ny, nz)`

Adds a vertex with a normal vector for lighting.

### d3d_vertex_normal_color

`d3d_vertex_normal_color(x, y, z, nx, ny, nz, color, alpha)`

Adds a vertex with normal vector and its own color and alpha.

### d3d_vertex_normal_texture

`d3d_vertex_normal_texture(x, y, z, nx, ny, nz, xtex, ytex)`

Adds a vertex with normal and texture coordinates, blended with the current color and alpha.

### d3d_vertex_normal_texture_color

`d3d_vertex_normal_texture_color(x, y, z, nx, ny, nz, xtex, ytex, color, alpha)`

Adds a vertex with normal, texture coordinates and its own color and alpha.

## Models

Building, saving, loading and drawing 3D models.

### d3d_model_create

`d3d_model_create()`

Creates a new 3D model and returns its index.

### d3d_model_destroy

`d3d_model_destroy(model)`

Destroys the model and frees its memory.

### d3d_model_clear

`d3d_model_clear(model)`

Removes all primitives from the model.

### d3d_model_save

`d3d_model_save(model, filename)`

Saves the model to a file.

### d3d_model_load

`d3d_model_load(model, filename)`

Loads the model from a file.

### d3d_model_draw

`d3d_model_draw(model, x, y, z, texture)`

Draws the model at (x,y,z) with the given texture (-1 for none); use transforms to rotate or scale.

### d3d_model_primitive_begin

`d3d_model_primitive_begin(model, kind)`

Starts a primitive of the given pr_ kind inside the model.

### d3d_model_vertex

`d3d_model_vertex(model, x, y, z)`

Adds a vertex to the model.

### d3d_model_vertex_color

`d3d_model_vertex_color(model, x, y, z, color, alpha)`

Adds a vertex with its own color and alpha to the model.

### d3d_model_vertex_texture

`d3d_model_vertex_texture(model, x, y, z, xtex, ytex)`

Adds a vertex with texture coordinates to the model.

### d3d_model_vertex_texture_color

`d3d_model_vertex_texture_color(model, x, y, z, xtex, ytex, color, alpha)`

Adds a vertex with texture coordinates, color and alpha to the model.

### d3d_model_vertex_normal

`d3d_model_vertex_normal(model, x, y, z, nx, ny, nz)`

Adds a vertex with a normal vector to the model.

### d3d_model_vertex_normal_color

`d3d_model_vertex_normal_color(model, x, y, z, nx, ny, nz, color, alpha)`

Adds a vertex with normal, color and alpha to the model.

### d3d_model_vertex_normal_texture

`d3d_model_vertex_normal_texture(model, x, y, z, nx, ny, nz, xtex, ytex)`

Adds a vertex with normal and texture coordinates to the model.

### d3d_model_vertex_normal_texture_color

`d3d_model_vertex_normal_texture_color(model, x, y, z, nx, ny, nz, xtex, ytex, color, alpha)`

Adds a vertex with normal, texture coordinates, color and alpha to the model.

### d3d_model_primitive_end

`d3d_model_primitive_end(model)`

Ends the current primitive in the model.

### d3d_model_block

`d3d_model_block(model, x1, y1, z1, x2, y2, z2, hrepeat, vrepeat)`

Adds a block shape to the model.

### d3d_model_cylinder

`d3d_model_cylinder(model, x1, y1, z1, x2, y2, z2, hrepeat, vrepeat, closed, steps)`

Adds a cylinder shape to the model.

### d3d_model_cone

`d3d_model_cone(model, x1, y1, z1, x2, y2, z2, hrepeat, vrepeat, closed, steps)`

Adds a cone shape to the model.

### d3d_model_ellipsoid

`d3d_model_ellipsoid(model, x1, y1, z1, x2, y2, z2, hrepeat, vrepeat, steps)`

Adds an ellipsoid shape to the model.

### d3d_model_wall

`d3d_model_wall(model, x1, y1, z1, x2, y2, z2, hrepeat, vrepeat)`

Adds a wall shape to the model.

### d3d_model_floor

`d3d_model_floor(model, x1, y1, z1, x2, y2, z2, hrepeat, vrepeat)`

Adds a floor shape to the model.
