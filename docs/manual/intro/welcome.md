# Welcome to Instance

Instance is a tool for making 2D games. It has two parts that work together:

- **The IDE**, where you build a game: you draw or import images, add sounds, design
  levels, and describe how things behave.
- **The runner**, the game engine that plays what you built, either straight from the
  IDE or as a stand-alone program you can give to other people.

You don't need to be a programmer to start. Most behaviour can be put together from
ready-made **actions** that you drag into place ("move in this direction", "play this
sound", "add 10 to the score"). When you want more control, every action has a
counterpart in **GML**, the scripting language built into Instance, and you can mix
the two freely.

![The Instance IDE with a room open](../images/ide.png)

## What a game is made of

An Instance game is a single project file (`.gm81`) holding a set of **resources**:

| Resource | What it is |
|---|---|
| Sprites | Images, often animated, that give things their look |
| Sounds | Sound effects and music |
| Backgrounds | Large images for the backdrop of a level, or tile sets |
| Paths | Routes that things can follow |
| Scripts | Reusable pieces of GML code |
| Fonts | Typefaces for drawing text |
| Time lines | Lists of actions that happen at set moments |
| Objects | The things in your game and how they behave |
| Rooms | The levels, screens and menus where objects are placed |

Objects are the heart of it all. An object reacts to **events** (it was created, a key
was pressed, it hit a wall) by running **actions**. You place copies of objects, called
**instances**, in rooms, and when the game runs, those instances come to life.
[Core ideas](concepts.md) explains this in more detail.

## Where to go from here

1. [Install Instance](installing.md), or open the browser version.
2. Take [a tour of the IDE](tour.md) to learn your way around.
3. Build [your first game](first-game.md), a small click game, step by step.
4. Read [Core ideas](concepts.md) before starting a game of your own.

After that, the [Using the IDE](../README.md#using-the-ide) chapters describe each
editor in detail, [The GML language](../gml/overview.md) teaches the scripting
language, [How games run](../runtime/game-loop.md) explains what happens while a game
plays, and the [Function reference](../reference/README.md) lists every built-in
function.

## Opening existing projects

Instance reads `.gm81` and `.gmk` project files made with older 2D game tools of the
same family, and plays them the way they were meant to play, quirks included. Open one
with **File > Open…** and press **F5** to run it.

See also: [Installing](installing.md) · [Tour](tour.md) · [Your first game](first-game.md)
