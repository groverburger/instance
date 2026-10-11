# Particles

Simple effects, and particle systems with types, emitters, attractors, destroyers, deflectors and changers.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Simple effects](#simple-effects)
- [Particle types](#particle-types)
- [Particle systems](#particle-systems)
- [Emitters](#emitters)
- [Attractors](#attractors)
- [Destroyers](#destroyers)
- [Deflectors](#deflectors)
- [Changers](#changers)

## Simple effects

Ready-made effects such as explosions, smoke, rain and snow.

### effect_create_below

`effect_create_below(kind, x, y, size, color)`

Creates a particle effect of a kind, size 0-2, and color at (x,y), drawn beneath instances at depth 100000.

### effect_create_above

`effect_create_above(kind, x, y, size, color)`

Like effect_create_below but drawn above instances at depth -100000.

### effect_clear

`effect_clear()`

Removes all effects.

### Constants: simple effects

| Constant | Value | Meaning |
| --- | --- | --- |
| `ef_explosion` | 0 | Effect kind constant for an explosion, used with effect_create_\*. |
| `ef_ring` | 1 | Effect kind constant for a ring, used with effect_create_\*. |
| `ef_ellipse` | 2 | Effect kind constant for an ellipse, used with effect_create_\*. |
| `ef_firework` | 3 | Effect kind constant for a firework, used with effect_create_\*. |
| `ef_smoke` | 4 | Effect kind constant for smoke, used with effect_create_\*. |
| `ef_smokeup` | 5 | Effect kind constant for rising smoke, used with effect_create_\*. |
| `ef_star` | 6 | Effect kind constant for a star, used with effect_create_\*. |
| `ef_spark` | 7 | Effect kind constant for a spark, used with effect_create_\*. |
| `ef_flare` | 8 | Effect kind constant for a flare, used with effect_create_\*. |
| `ef_cloud` | 9 | Effect kind constant for a cloud, used with effect_create_\*. |
| `ef_rain` | 10 | Effect kind constant for rain (created at room top, position ignored), used with effect_create_\*. |
| `ef_snow` | 11 | Effect kind constant for snow (created at room top, position ignored), used with effect_create_\*. |

## Particle types

The shape, size, color, motion and life of particles.

### part_type_create

`part_type_create()`

Creates a new particle type and returns its index.

### part_type_destroy

`part_type_destroy(type)`

Destroys a particle type to free memory.

### part_type_exists

`part_type_exists(type)`

Returns whether the particle type exists.

### part_type_clear

`part_type_clear(type)`

Resets the particle type to default settings.

### part_type_shape

`part_type_shape(type, shape)`

Sets the particle shape to a pt_shape_ constant (default pt_shape_pixel).

### part_type_sprite

`part_type_sprite(type, sprite, animate, stretch, random)`

Uses a sprite for the particle, with flags for animation, stretching over lifetime, and random start subimage.

### part_type_size

`part_type_size(type, size_min, size_max, size_incr, size_wiggle)`

Sets min and max starting size, per-step size change, and size wiggle (default size 1).

### part_type_scale

`part_type_scale(type, xscale, yscale)`

Sets horizontal and vertical scale factors multiplied with the size.

### part_type_orientation

`part_type_orientation(type, ang_min, ang_max, ang_incr, ang_wiggle, ang_relative)`

Sets angle min/max, per-step change, wiggle and whether the angle is relative to the motion direction.

### part_type_life

`part_type_life(type, life_min, life_max)`

Sets minimum and maximum particle lifetime in steps (default 100 each).

### part_type_step

`part_type_step(type, step_number, step_type)`

Sets how many particles of a type are spawned each step; negative values mean a 1/-number chance per step.

### part_type_death

`part_type_death(type, death_number, death_type)`

Sets how many particles of a type spawn when one dies of old age; negative means a chance.

### part_type_speed

`part_type_speed(type, speed_min, speed_max, speed_incr, speed_wiggle)`

Sets min/max starting speed, per-step speed change (never below 0), and wiggle.

### part_type_direction

`part_type_direction(type, dir_min, dir_max, dir_incr, dir_wiggle)`

Sets direction range in counterclockwise degrees (0 = right), per-step change, and wiggle.

### part_type_gravity

`part_type_gravity(type, grav_amount, grav_dir)`

Sets gravity amount added each step and its direction (270 is down).

### part_type_color1

`part_type_color1(type, color1)`

Gives the particle a single color.

### part_type_color2

`part_type_color2(type, color1, color2)`

Interpolates the particle color between two colors over its life.

### part_type_color3

`part_type_color3(type, color1, color2, color3)`

Interpolates the particle color between three colors at start, middle and end.

### part_type_color_mix

`part_type_color_mix(type, color1, color2)`

Gives each particle a fixed color randomly mixed from two colors.

### part_type_color_rgb

`part_type_color_rgb(type, rmin, rmax, gmin, gmax, bmin, bmax)`

Gives each particle a fixed color chosen from RGB ranges (0-255 each).

### part_type_color_hsv

`part_type_color_hsv(type, hmin, hmax, smin, smax, vmin, vmax)`

Gives each particle a fixed color chosen from HSV ranges (0-255 each).

### part_type_alpha1

`part_type_alpha1(type, alpha1)`

Sets a single alpha transparency (0-1) for the particle type.

### part_type_alpha2

`part_type_alpha2(type, alpha1, alpha2)`

Interpolates alpha between a start and end value.

### part_type_alpha3

`part_type_alpha3(type, alpha1, alpha2, alpha3)`

Interpolates alpha between three values over the particle life.

### part_type_blend

`part_type_blend(type, additive)`

Selects additive (1) or normal (0) blending for the particle type.

### Constants: particle types

| Constant | Value | Meaning |
| --- | --- | --- |
| `pt_shape_pixel` | 0 | Built-in 64x64 particle shape constant (pixel) for part_type_shape. |
| `pt_shape_disk` | 1 | Built-in 64x64 particle shape constant (disk) for part_type_shape. |
| `pt_shape_square` | 2 | Built-in 64x64 particle shape constant (square) for part_type_shape. |
| `pt_shape_line` | 3 | Built-in 64x64 particle shape constant (line) for part_type_shape. |
| `pt_shape_star` | 4 | Built-in 64x64 particle shape constant (star) for part_type_shape. |
| `pt_shape_circle` | 5 | Built-in 64x64 particle shape constant (circle) for part_type_shape. |
| `pt_shape_ring` | 6 | Built-in 64x64 particle shape constant (ring) for part_type_shape. |
| `pt_shape_sphere` | 7 | Built-in 64x64 particle shape constant (sphere) for part_type_shape. |
| `pt_shape_flare` | 8 | Built-in 64x64 particle shape constant (flare) for part_type_shape. |
| `pt_shape_spark` | 9 | Built-in 64x64 particle shape constant (spark) for part_type_shape. |
| `pt_shape_explosion` | 10 | Built-in 64x64 particle shape constant (explosion) for part_type_shape. |
| `pt_shape_cloud` | 11 | Built-in 64x64 particle shape constant (cloud) for part_type_shape. |
| `pt_shape_smoke` | 12 | Built-in 64x64 particle shape constant (smoke) for part_type_shape. |
| `pt_shape_snow` | 13 | Built-in 64x64 particle shape constant (snow) for part_type_shape. |

## Particle systems

Creating systems that hold particles, and drawing and updating them.

### part_system_create

`part_system_create()`

Creates a new particle system and returns its index.

### part_system_destroy

`part_system_destroy(system)`

Destroys a particle system to free memory.

### part_system_exists

`part_system_exists(system)`

Returns whether the particle system exists.

### part_system_clear

`part_system_clear(system)`

Resets the system to defaults, removing all particles, emitters and attractors.

### part_system_draw_order

`part_system_draw_order(system, oldtonew)`

Chooses draw order: true draws oldest particles first (default), false newest first.

### part_system_depth

`part_system_depth(system, depth)`

Sets the depth of the particle system relative to instances.

### part_system_position

`part_system_position(system, x, y)`

Sets the offset position at which the system is drawn, e.g. to follow an object.

### part_system_automatic_update

`part_system_automatic_update(system, automatic)`

Sets whether the system updates automatically (1, default) or not (0).

### part_system_automatic_draw

`part_system_automatic_draw(system, draw)`

Sets whether the system draws automatically (1, default) or not (0).

### part_system_update

`part_system_update(system)`

Advances particle positions and emitters one step; needed only when updating is manual.

### part_system_drawit

`part_system_drawit(system)`

Draws the system particles; needed only for manual drawing, call from a draw event.

### part_particles_create

`part_particles_create(system, x, y, parttype, number)`

Creates a number of particles of a type at (x,y) in the system.

### part_particles_create_color

`part_particles_create_color(system, x, y, parttype, color, number)`

Creates particles of a type at (x,y) with a given color; useful for single-color or colorless types.

### part_particles_clear

`part_particles_clear(system)`

Removes all particles from the system.

### part_particles_count

`part_particles_count(system)`

Returns the number of particles in the system.

## Emitters

Creating particles in a region, in bursts or in streams.

### part_emitter_create

`part_emitter_create(system)`

Creates an emitter in the particle system and returns its index.

### part_emitter_destroy

`part_emitter_destroy(system, emitter)`

Destroys an emitter in the particle system.

### part_emitter_destroy_all

`part_emitter_destroy_all(system)`

Destroys all emitters in the particle system.

### part_emitter_exists

`part_emitter_exists(system, emitter)`

Returns whether an emitter with that index exists in the system.

### part_emitter_clear

`part_emitter_clear(system, emitter)`

Resets an emitter to default settings.

### part_emitter_region

`part_emitter_region(system, emitter, xmin, xmax, ymin, ymax, shape, distribution)`

Sets the emitter region extents, shape and distribution.

### part_emitter_burst

`part_emitter_burst(system, emitter, parttype, number)`

Emits a single burst of a number of particles of a type.

### part_emitter_stream

`part_emitter_stream(system, emitter, parttype, number)`

Continuously emits a number of particles per step; negative means a 1/-number chance per step.

### Constants: emitters

| Constant | Value | Meaning |
| --- | --- | --- |
| `ps_distr_linear` | 0 | Emitter distribution constant: uniform chance across the region. |
| `ps_distr_gaussian` | 1 | Emitter distribution constant: more particles near the center. |
| `ps_distr_invgaussian` | 2 | Emitter distribution constant: more particles near the edges. |
| `ps_shape_rectangle` | 0 | Region shape constant for rectangle used by emitters, destroyers and changers. |
| `ps_shape_ellipse` | 1 | Region shape constant for ellipse used by emitters, destroyers and changers. |
| `ps_shape_diamond` | 2 | Region shape constant for diamond used by emitters, destroyers and changers. |
| `ps_shape_line` | 3 | Region shape constant for line used by emitters, destroyers and changers. |

## Attractors

Pulling particles towards a point or pushing them away.

### part_attractor_create

`part_attractor_create(system)`

Creates an attractor in the particle system and returns its index.

### part_attractor_destroy

`part_attractor_destroy(system, attractor)`

Destroys an attractor in the particle system.

### part_attractor_destroy_all

`part_attractor_destroy_all(system)`

Destroys all attractors in the particle system.

### part_attractor_exists

`part_attractor_exists(system, attractor)`

Returns whether an attractor with that index exists in the system.

### part_attractor_clear

`part_attractor_clear(system, attractor)`

Resets an attractor to default settings.

### part_attractor_position

`part_attractor_position(system, attractor, x, y)`

Sets the attractor position to (x,y).

### part_attractor_force

`part_attractor_force(system, attractor, force, dist, kind, additive)`

Sets attractor force, maximum effect distance, force kind and whether it is additive to speed.

### Constants: attractors

| Constant | Value | Meaning |
| --- | --- | --- |
| `ps_force_constant` | 0 | Attractor force kind where the force does not depend on distance. |
| `ps_force_linear` | 1 | Attractor force kind growing linearly from 0 at max distance to full force at the attractor. |
| `ps_force_quadratic` | 2 | Attractor force kind that grows quadratically with closeness. |

## Destroyers

Removing particles that enter a region.

### part_destroyer_create

`part_destroyer_create(system)`

Creates a destroyer in the particle system and returns its index.

### part_destroyer_destroy

`part_destroyer_destroy(system, destroyer)`

Destroys a destroyer in the particle system.

### part_destroyer_destroy_all

`part_destroyer_destroy_all(system)`

Destroys all destroyers in the particle system.

### part_destroyer_exists

`part_destroyer_exists(system, destroyer)`

Returns whether a destroyer with that index exists in the system.

### part_destroyer_clear

`part_destroyer_clear(system, destroyer)`

Resets a destroyer to default settings.

### part_destroyer_region

`part_destroyer_region(system, destroyer, xmin, xmax, ymin, ymax, shape)`

Sets the destroyer region extents and shape.

## Deflectors

Making particles bounce off a region.

### part_deflector_create

`part_deflector_create(system)`

Creates a deflector in the particle system and returns its index.

### part_deflector_destroy

`part_deflector_destroy(system, deflector)`

Destroys a deflector in the particle system.

### part_deflector_destroy_all

`part_deflector_destroy_all(system)`

Destroys all deflectors in the particle system.

### part_deflector_exists

`part_deflector_exists(system, deflector)`

Returns whether a deflector with that index exists in the system.

### part_deflector_clear

`part_deflector_clear(system, deflector)`

Resets a deflector to default settings.

### part_deflector_region

`part_deflector_region(system, deflector, xmin, xmax, ymin, ymax)`

Sets the rectangular region in which the deflector acts.

### part_deflector_kind

`part_deflector_kind(system, deflector, kind)`

Sets the deflector kind (horizontal or vertical deflection).

### part_deflector_friction

`part_deflector_friction(system, deflector, amount)`

Sets how much particles are slowed on impact with the deflector.

### Constants: deflectors

| Constant | Value | Meaning |
| --- | --- | --- |
| `ps_deflect_vertical` | 0 | Deflector kind deflecting particles vertically, typically for horizontal walls. |
| `ps_deflect_horizontal` | 1 | Deflector kind deflecting particles horizontally, typically for vertical walls. |

## Changers

Turning particles of one type into another type inside a region.

### part_changer_create

`part_changer_create(system)`

Creates a changer in the particle system and returns its index.

### part_changer_destroy

`part_changer_destroy(system, changer)`

Destroys a changer in the particle system.

### part_changer_destroy_all

`part_changer_destroy_all(system)`

Destroys all changers in the particle system.

### part_changer_exists

`part_changer_exists(system, changer)`

Returns whether a changer with that index exists in the system.

### part_changer_clear

`part_changer_clear(system, changer)`

Resets a changer to default settings.

### part_changer_region

`part_changer_region(system, changer, xmin, xmax, ymin, ymax, shape)`

Sets the changer region extents and shape.

### part_changer_kind

`part_changer_kind(system, changer, kind)`

Sets the changer kind (ps_change_motion, shape or all).

### part_changer_types

`part_changer_types(system, changer, parttype1, parttype2)`

Sets which particle type is turned into which other type.

### Constants: changers

| Constant | Value | Meaning |
| --- | --- | --- |
| `ps_change_all` | 0 | Changer kind altering all parameters, effectively replacing the particle with a new one. |
| `ps_change_shape` | 1 | Changer kind altering only shape-related parameters such as size and color. |
| `ps_change_motion` | 2 | Changer kind altering only motion parameters. |
