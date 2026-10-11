# Arrays

Any variable — instance, global or local — can hold an array: a numbered list of values.
Write the index in square brackets after the name. Indices start at 0.

```gml
names[0] = "Ada";
names[1] = "Grace";
names[2] = "Linus";

var i;
for (i = 0; i < 3; i += 1) {
    draw_text(32, 32 + i * 24, names[i]);
}
```

## Creating and growing

There is no declaration and no size to give. Assigning to an element creates the array
and grows it as needed. Each element has its own type, so one array can mix reals and
strings.

Elements that you skipped over read as `0`:

```gml
b[3] = 7;
show_message(string(b[1]));   // 0
```

Reading past the end of what was ever assigned is an error, *"Unknown variable b or
array index out of bounds"* (or `0`, if the game treats uninitialised variables as 0).

## Two-dimensional arrays

Use two indices separated by a comma for a grid:

```gml
var cx, cy;
for (cx = 0; cx < 8; cx += 1) {
    for (cy = 0; cy < 8; cy += 1) {
        board[cx, cy] = 0;
    }
}
board[3, 4] = 1;
```

Each row grows on its own: it is as long as the largest column written in that row,
and a row you never wrote to doesn't exist — after only `g[2, 4] = 1`, reading
`g[2, 0]` gives 0 but reading `g[1, 0]` is an error. A one-dimensional array is just
row 0: `a[5]` and `a[0, 5]` are the same element.

## Limits

- Indices must be 0 or more. A negative index is the error *"Negative array index"*.
- Each index must be below 32000 (*"Array index >= 32000"*), and an array holds at most
  1 000 000 elements in total.
- Indices are rounded to whole numbers (halves to even). A string index is the error
  *"Wrong type of array index"*.

## Arrays are not values

An array belongs to the variable that holds it; it can't be copied or passed around as
a whole.

- The plain variable and element 0 are the same storage: after `a = 5`, `a[0]` is 5,
  and after `a[0] = 7`, `a` is 7.
- `c = a` copies only `a[0]` into `c`.
- You can't pass an array to a script or return one from it. To share a list between
  objects or scripts, keep the array in one instance (or in `global`) and access it
  there — `global.names[i]`, `obj_controller.names[i]` — or use a
  [data structure](../reference/data-structures.md) such as a list or grid, whose id is
  an ordinary number you can pass anywhere.

## Arrays of other instances

The index applies to the variable right before it, so you can reach another instance's
array:

```gml
other.inventory[2] = "key";
global.high[0] = score;
```

## Built-in arrays

Some built-in variables are arrays: `alarm[0..11]`, the `view_*[0..7]` and
`background_*[0..7]` variables, and `instance_id[]`. Writing a built-in array without an
index means index 0: `alarm = 30` sets `alarm[0]`.

See also: [Variables and scope](variables.md) · [Data structures](../reference/data-structures.md) ·
[Common pitfalls](pitfalls.md)
