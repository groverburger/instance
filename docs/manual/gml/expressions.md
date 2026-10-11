# Expressions and operators

An expression computes a value: `x + 16`, `point_distance(x, y, mouse_x, mouse_y) < 64`,
`"Level " + string(global.level)`. Expressions are made of values, variables, function
calls and operators.

## Operators and precedence

GML has fewer precedence levels than most C-like languages. From **lowest** to
**highest**:

| Level | Operators | Meaning |
|---|---|---|
| 1 (lowest) | `&&` `and`, `\|\|` `or`, `^^` `xor` | logical and, or, exclusive or |
| 2 | `<` `<=` `==` `=` `!=` `<>` `>=` `>` | comparisons |
| 3 | `\|` `&` `^` | bitwise or, and, exclusive or |
| 4 | `<<` `>>` | bit shifts |
| 5 | `+` `-` | addition (joins strings), subtraction |
| 6 | `*` `/` `div` `mod` | multiplication, division, integer division, remainder |
| 7 | `!` `not`, `-`, `+`, `~` (prefix) | logical not, negation, plus, bitwise not |
| 8 (highest) | `.` `[ ]` `( )` | member access, array index, function call |

All binary operators are left-associative: `a - b - c` is `(a - b) - c`.

Two consequences catch people coming from other languages:

- **`&&`, `||` and `^^` share one level.** They are simply applied left to right, so
  `a || b && c` means `(a || b) && c`. Use parentheses whenever you mix them.
- **Bitwise operators bind tighter than comparisons.** `flags & 4 == 4` means
  `(flags & 4) == 4`, which is usually what you wanted anyway.

`x == 1 or x == 2` works as expected, because comparisons bind tighter than the logical
operators.

There is no `++`, `--`, `%`, `?:` or `<<=` in GML. `%` is a syntax error (use `mod`),
and `i++` is not a valid statement (write `i += 1`).

## Arithmetic

```gml
a = 7 / 2;       // 3.5
b = 7 div 2;     // 3   (divide, then drop the fraction toward zero)
c = 7 mod 2;     // 1   (remainder)
d = -7 div 2;    // -3
e = -7 mod 2;    // -1  (the sign follows the left operand)
f = 7.5 mod 2;   // 1.5 (works with fractions)
```

`div` and `mod` are defined so that `a == (a div b) * b + a mod b`.

Dividing by zero — with `/`, `div` or `mod` — is a run-time error (*"Division by 0."*
or *"Modulo by 0."*). With literal operands (`1 / 0`) it is already a compile error.

`+` on two strings joins them: `"abc" + "def"` is `"abcdef"`. Every other arithmetic
operator on a string is an error, and so is mixing a string and a real:

```gml
lbl = "Score: " + score;            // ERROR: Wrong type of arguments to +.
lbl = "Score: " + string(score);    // right
```

## Comparisons

Comparisons give `1` (true) or `0` (false).

**Numbers** are compared with a tiny tolerance: two reals count as equal when they
differ by less than 10<sup>-13</sup>. This hides most floating-point noise —
`0.1 + 0.2 == 0.3`, `sin(pi) == 0` and `lengthdir_x(10, 90) == 0` are all true — but it
is an absolute tolerance, not a relative one: `1000000 == 1000000.0000001` is false.
The ordering operators follow the same rule: `<` means "smaller and not equal", so
`0.3 < 0.1 + 0.2` is false.

**Strings** can be compared with all six operators. Equality is exact and
case-sensitive. Ordering compares character codes one by one, and a prefix is smaller
than the longer string: `"Apple" < "apple"` and `"ab" < "abc"` are both true.

**A string and a real** can't be compared: `"5" == 5` is the run-time error *"Cannot
compare arguments."* (In a `switch`, a case of the other type simply doesn't match.)

### `=` inside an expression

Inside an expression, `=` means the same as `==`. Only at the start of a statement,
right after a variable, is it an assignment. So

```gml
if (lives = 0) show_message("Game over");   // compares, does not assign
same = a = b;                                // same becomes 1 if a equals b, else 0
```

This makes the classic C bug (assigning inside a condition) impossible, but it also
means `a = b = 0` does **not** set both variables to 0. `:=` is accepted as another way
to write assignment.

## Logical operators and truth

Wherever GML needs a yes/no — in `if`, `while`, `until`, `for`, and as operands of
`!`, `&&`, `||`, `^^` — a real counts as **true when it is 0.5 or more**. So `1`, `2`
and `0.5` are true; `0`, `0.49` and every negative number (including `-1` and
`noone`) are false. Logical operators always give exactly `1` or `0`.

A string is not allowed as a condition: `if ("yes")` is the run-time error
*"Expression expected"*, and `!"yes"` or `"a" && 1` are type errors too.

**Both sides are always evaluated.** GML has no short-circuit evaluation: in `a && b`,
`b` is evaluated even when `a` is false. That matters when the right side would fail
or has side effects:

```gml
// Wrong: target.hp is read even when target is noone, which is an error
if (target != noone && target.hp > 0) {
    target.hp -= 1;
}

// Right: test in two steps
if (target != noone) {
    if (target.hp > 0) {
        target.hp -= 1;
    }
}
```

Operands are evaluated left to right, and so are function arguments.

## Bitwise operators

`&`, `|`, `^`, `~`, `<<` and `>>` work on whole numbers. Each operand is first rounded
to the nearest integer (halves round to the even neighbour) and treated as a 64-bit
signed integer; the result is converted back to a real.

```gml
flags = 1 | 4;        // 5
has4 = flags & 4;     // 4
n = 1 << 10;          // 1024
m = ~0;               // -1
```

`>>` is a *logical* shift: it fills with zeros, so `-1 >> 1` is a huge positive
number, not `-1`.

## Where numbers become whole numbers

Several places need an integer and round the value for you, halves going to the even
neighbour (`2.5` → 2, `3.5` → 4): array indices, `repeat` counts, bitwise operands,
alarms, and instance ids or object indices on the left of `.`. The functions
[`round()`](../reference/computing.md#round), [`floor()`](../reference/computing.md#floor)
and [`ceil()`](../reference/computing.md#ceil) do it explicitly; `round()` also rounds
halves to even.

## Converting between strings and reals

[`string(x)`](../reference/computing.md#string) turns a real into text:

- Whole numbers print without decimals: `string(42)` is `"42"`.
- Other numbers print with exactly two decimals: `string(0.5)` is `"0.50"`,
  `string(1/3)` is `"0.33"`. Very small values print as `"0.00"` (without a minus sign).
- Very large non-whole numbers switch to a scientific form such as `"1.23456789E11"`.

Need a different number of decimals? Use
[`string_format(x, total, decimals)`](../reference/computing.md#string_format):
`string_format(3.14159, 0, 3)` is `"3.142"`.

[`real(s)`](../reference/computing.md#real) goes the other way. It ignores spaces around
the number and accepts a sign, decimals and an exponent (`"-12.5"`, `"3e2"`). Anything
else — `"12abc"`, `"$10"`, `"1,5"` — is reported as an error by the function, which then
returns 0 and lets your code continue. An empty string gives 0.

## Assignment operators

Besides `=`, you can write `+=`, `-=`, `*=`, `/=`, `|=`, `&=` and `^=`. `a += b` means
exactly `a = a + b`, so `+=` also appends to a string:

```gml
log = "";
log += "started; ";
```

See also: [Statements and control flow](statements.md) · [Variables and scope](variables.md) ·
[Common pitfalls](pitfalls.md) · [Computing functions](../reference/computing.md)
