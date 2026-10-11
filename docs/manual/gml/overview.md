# GML overview

GML is the scripting language built into Instance. Every
drag-and-drop action has a GML equivalent, and many things — loops, arrays, string
handling, working with many instances at once — are only practical in code.

This chapter describes the language itself. What happens *when* your code runs (event
order, room changes, collisions) is covered in [How games run](../runtime/game-loop.md),
and the individual functions are listed in the [function reference](../reference/README.md).

## Where GML appears

| Place | What you write |
|---|---|
| A **script** resource | A program with a name. Call it like a function: `scr_fire(8)`. It receives up to 16 arguments and can `return` a value. See [Functions and scripts](scripts.md). |
| The **Execute Code** action in an event | A program that runs when the event happens, for the instance(s) the action applies to. |
| **Room creation code** and **instance creation code** (room editor) | A program that runs once when the room is entered or the instance is placed. |
| Any **action parameter** field | A single *expression*, such as `x + 16` or `random(100)`. |
| **Constants** (Global Game Settings) | An expression evaluated once when the game starts. See [Constants](constants.md). |
| **Trigger** conditions | A program that ends with `return <condition>;`. |
| `execute_string()` / `execute_file()` | A program built or loaded while the game runs. |

## A first look

```gml
// Create event of obj_ball: start moving in a random direction
speed = 4;
direction = random(360);
bounces = 0;
```

```gml
// Step event of obj_ball: slow down a little after every bounce
if (bounces > 10) {
    speed = max(speed - 0.05, 0);
}
```

Code is a list of *statements*. The common ones are assignments (`speed = 4`),
function calls (`instance_destroy()`), `if`, the loops (`repeat`, `while`, `do`,
`for`), `switch`, `with`, and `var` declarations. They are described in
[Statements and control flow](statements.md).

## The basic rules

- **Names are case-sensitive.** `Score`, `score` and `SCORE` are three different names.
  A name starts with a letter or `_` and continues with letters, digits and `_`.
- **Semicolons are optional.** Statements may be separated by `;` or by nothing at all.
  Writing them anyway is a good habit: it makes the code easier to read.
- **Line breaks mean nothing.** You can split a long expression over several lines.
- **Blocks** group statements: `{ ... }` or, equivalently, `begin ... end`.
- **Comments** run from `//` to the end of the line, or from `/*` to `*/`. A first line
  starting with `///` in a code action is shown as the action's description in the
  object editor; to the language it is just a comment.
- **Parentheses around conditions are optional**: `if x > 3 y = 1` works. Using them
  (`if (x > 3) y = 1`) avoids surprises.

These words are reserved and cannot be used as variable names:

```text
if then else begin end repeat while do until for switch case default
break continue exit return with var globalvar and or xor not div mod
```

`true`, `false`, `pi`, `self`, `other`, `all`, `noone`, `global` and `local` are
built-in constants. You can't assign to them either.

## Two kinds of values

Every value in GML is either a **real** (a double-precision floating point number) or a
**string**. There is no separate integer, boolean or null type:

- `true` is the real `1` and `false` is the real `0`.
- Instance ids, object indices, colours, key codes and resource indices are all reals.
- Arrays are not values; a variable can *contain* an array ([Arrays](arrays.md)).

GML never converts between the two types on its own. `"Score: " + 10` is an error;
write `"Score: " + string(10)`. Use [`string()`](../reference/computing.md#string) and
[`real()`](../reference/computing.md#real) to convert, and
[`is_real()`](../reference/computing.md#is_real) /
[`is_string()`](../reference/computing.md#is_string) to test.

## Literals

Numbers are written in decimal (`12`, `0.5`, `.5` and `12.` all work) or in
hexadecimal with a `$` prefix (`$FF` is 255, `$0000FF` is pure red as a colour).
There is no exponent notation in code: `1e3` is a syntax error. (The `real()`
function *does* accept `"1e3"` in a string.) There are no negative literals either:
`-5` is the minus operator applied to `5`.

Strings are enclosed in double or single quotes. Use one kind to include the other:

```gml
msg = 'She said "hello"';
msg2 = "It's late";
```

**There are no escape sequences.** A backslash is an ordinary character. To get both
quote characters into one string, glue pieces together with `chr(34)` (`"`) and
`chr(39)` (`'`). A string literal may span several lines; the line breaks become part
of the string. (The text-drawing functions treat `#` as a line break; that is a feature
of [drawing](../runtime/drawing.md#text), not of the language.)

Strings hold one byte per character, using the Windows-1252 character set, so
[`chr()`](../reference/computing.md#chr) takes values 0–255 and
[`string_length()`](../reference/computing.md#string_length) counts characters.

## How a program starts and ends

A program is simply a list of statements, with or without surrounding braces. One
rule is worth knowing: if the **very first token** of the code is `{`, that block is
taken to be the whole program, and anything after its closing `}` (even a lone `;`) is
the compile error *"Program ends before end of the code."* Comments after it are fine.

```gml
{
    x = 0;
}
y = 0;   // error: the program already ended at the }
```

Remove the outer braces, or start the code with something other than `{`, and
any number of blocks is fine.

## When code is compiled

Instance compiles all code — scripts, code actions, action arguments, creation code,
trigger conditions and constants — when the game loads, and reports any syntax errors
at that moment. A piece of code that failed to compile is never executed; each time
it would have run, its error is reported again. The IDE's code editor and the
`gmk-check` tool show the same errors while you edit.

Some mistakes are caught at compile time even though they look like run-time problems:
operations on literal values are worked out while compiling, so `'a' + 1` or `1 / 0`
written with literals is a compile error. The same operation on variables is a
run-time error ([Errors and debugging](../runtime/errors.md)).

See also: [Variables and scope](variables.md) · [Expressions and operators](expressions.md) ·
[Statements and control flow](statements.md) · [Common pitfalls](pitfalls.md)
