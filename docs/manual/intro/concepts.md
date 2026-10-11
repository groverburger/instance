# Core ideas: resources, objects, events and rooms

A handful of ideas explain almost everything in Instance. Once they click, the rest of
the manual is detail.

## Resources

Everything your game is made of is a **resource**. Each resource has a **name**, which
you use to refer to it from actions and code, so pick names that are easy to read and
type: letters, digits and underscores, starting with a letter. A common habit is to
start names with the kind of resource: `spr_player`, `snd_jump`, `obj_enemy`,
`rm_level1`. Names should also be unique across the whole project, because code sees
all resources in one name space.

Sprites, sounds, backgrounds, paths, fonts and time lines are mostly *material*:
images, audio and data. Scripts hold *code*. Objects describe *behaviour*, and rooms
put everything *together*.

## Objects and instances

An **object** is a description: "a gem looks like `spr_gem`, starts moving when it's
created, and bounces off walls." It's a blueprint, not a thing on screen.

An **instance** is one actual copy of an object in the running game. A room can hold
any number of instances of the same object, and each instance has its own position,
speed, sprite frame and variables. When the player catches one gem, only that gem
moves away.

Instances are created when a room starts (from the instances you placed in the room
editor) or during the game (with the **Create Instance** action or the
`instance_create` function). They exist until they are destroyed or the room ends.

## Events and actions

Objects do nothing on their own. They **react to events**. An event is a moment in the
game: the instance was just created, a key was pressed, it collided with another
instance, a step of the game passed, it is time to draw.

For each event you care about, you give the object a list of **actions**. When the
event happens for an instance, its actions run in order, top to bottom. Some actions
are **questions** (tests) that decide whether the next action, or block of actions,
runs. The **Execute Code** action runs a piece of GML, so anything an action can do,
code can do too.

The most important events:

| Event | Happens… |
|---|---|
| Create | once, when the instance is created |
| Step | every step of the game (30 times a second by default) |
| Collision | every step while the instance overlaps an instance of a given object |
| Keyboard, Key Press, Key Release | while a key is held, when it goes down, when it comes up |
| Mouse | when the mouse interacts with the instance (or anywhere, for global mouse events) |
| Alarm | when one of the instance's countdown timers runs out |
| Draw | when the instance is drawn; replaces the default drawing of its sprite |
| Destroy | when the instance is destroyed |

[Events](../ide/events.md) lists them all.

## Steps

A running game advances in **steps**. In every step, Instance handles input, runs the
step and collision events, moves instances by their speed, and draws the room. The
room's **speed** setting says how many steps happen per second. Time in Instance is
usually counted in steps: an alarm set to 30 goes off after 30 steps, which is one
second in a room with speed 30. [The game loop](../runtime/game-loop.md) describes
exactly what happens in a step and in what order.

## Rooms

A **room** is a level, a screen or a menu. It has a size, a background, and the
instances placed in it. The game starts in the first room in the resource tree, and
actions such as **Next Room** move through the rooms in that order.

Rooms can also have **views**: windows onto part of a larger room, for scrolling
games.

## Variables

Every instance carries **variables**, named values that it remembers. Some are built
in: `x` and `y` (position), `speed` and `direction`, `sprite_index`, `image_index`,
`visible` and many more. Changing them changes the instance: setting `x` moves it,
setting `visible` to `false` hides it. You can also make your own (`lives_left`,
`has_key`). Games also have global values such as `score`, `lives` and `health`. See
[Variables and scope](../gml/variables.md).

## Parents

An object can have a **parent** object. It then inherits all of the parent's events
and actions, and counts as an instance of the parent in collisions and tests. Make an
`obj_enemy` parent with the shared behaviour, and give each kind of enemy its own
sprite and extra events. See [Instances, objects and parenting](../runtime/instances.md).

## Putting it together

When you press Run:

1. Instance builds the first room and creates the instances placed in it, running
   each one's Create event.
2. Step after step, it runs the events that apply, moves instances, checks
   collisions and draws everything.
3. Actions and code change variables, create and destroy instances, play sounds, and
   move to other rooms, until the game ends.

See also: [Your first game](first-game.md) · [Objects](../ide/objects.md) · [Events](../ide/events.md) · [The game loop](../runtime/game-loop.md)
