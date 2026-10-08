# GML language specification (GameMaker 8.1 compatible)

Status: draft v1. Clean-room spec written from the user manual (chapters 401, 402,
207, 214, 112_04/112_07, 302_05) and the `fnames` identifier list. Nothing here is
copied from the manual verbatim; it is a paraphrased, implementation-oriented
restatement. Where the manual is silent, the text says
**UNSPECIFIED – implementation choice: …** and gives a recommendation. Such points
should be confirmed by black-box testing against GM 8.1 and are tracked in
`specs/OPEN_QUESTIONS.md` (tag `[GML]`).

The companion file `specs/builtins.json` catalogues every built-in function,
variable and constant (see §15).

---

## 1. Where GML appears

GML source text shows up in four places. All use the same lexer/parser.

| Context | Entry rule | Notes |
|---|---|---|
| Script resource | `program` | Has a name, can receive up to 16 arguments, may `return` a value. |
| "Execute code" action in an event | `program` | No name; receives no arguments (argument variables read as 0 – see §11.4). Runs once per instance selected by the action's "applies to" field. |
| Room creation code (and instance creation code) | `program` | |
| Any action parameter field that takes a value | `expression` | Only an expression, not statements. |
| User-defined constant values (§12.3) | `expression` | Evaluated once at game start. |
| `execute_string(str, …)` / `execute_file(fname, …)` | `program` | Compiled at run time; see §11.6. |

A first line starting with `///` in a code action is used by the IDE as the
action's description; for the language it is an ordinary comment.

**Compilation time.** UNSPECIFIED – implementation choice: compile all code
(scripts, code actions, creation code, action-argument expressions, constants)
when the game is loaded, before any event runs. A syntax error is reported with
the resource/object, event, action number, line and column, and the faulty piece
of code is then never executed (treated as empty).

---

## 2. Lexical grammar

Source is a sequence of 8-bit characters (the game's ANSI code page).
UNSPECIFIED – implementation choice: read source as bytes; treat any byte ≥ 0x80
as an "other" character that is only legal inside strings and comments.

### 2.1 Whitespace and comments
- Whitespace: space, tab, CR, LF (and any other byte ≤ 0x20). It separates tokens
  and is otherwise ignored. Newlines have no syntactic meaning.
- Line comment: `//` up to (not including) the next LF or end of input.
- Block comment: `/*` up to and including the next `*/`. Block comments do not nest.
  UNSPECIFIED – implementation choice: an unterminated block comment silently runs to
  end of input (no error).
- Comment openers inside string literals are part of the string.

### 2.2 Identifiers
```
identifier = (letter | "_") { letter | digit | "_" } ;
letter     = "A".."Z" | "a".."z" ;
digit      = "0".."9" ;
```
- Identifiers are **case-sensitive** (`Score` ≠ `score`).
- The manual states a maximum variable-name length of 64 characters.
  UNSPECIFIED – implementation choice: accept longer names without error and treat
  all characters as significant.
- An identifier that equals a keyword (§2.6) is the keyword.

### 2.3 Numeric literals
```
decimal_literal = digit { digit } [ "." { digit } ]
                | "." digit { digit } ;          (* leading-dot form: see note *)
hex_literal     = "$" hexdigit { hexdigit } ;
hexdigit        = digit | "A".."F" | "a".."f" ;
```
- All numeric literals produce a **real** (IEEE-754 double).
- A `.` that directly follows the digits of a decimal literal is always consumed as
  the decimal point. Consequence (documented): `100032.speed` does not mean "field
  `speed` of instance 100032"; the programmer must write `(100032).speed`.
  A literal such as `12.` (dot, no fraction digits) is legal and equals 12.
  If the dot is followed by a letter (`12.x`) – UNSPECIFIED – implementation choice:
  lex as `12.` followed by identifier `x`, which is then a syntax error at the
  parser level (two adjacent operands).
- Leading-dot form `.5`: UNSPECIFIED – implementation choice: accept, value 0.5.
- Exponent notation (`1e5`) is **not** part of the literal syntax. `1e5` lexes as
  `1` followed by identifier `e5` → syntax error. (The `real()` function does accept
  exponents in strings – that is a library matter.)
- Hex literal value is the unsigned integer formed by the digits, e.g.
  `$00FFAA` = 65450, `$FFAA00` = 16755200. A lone `$` is a lexical error.
  UNSPECIFIED – implementation choice: more than 16 hex digits is an error; values
  above 2^53 lose precision as normal for doubles.
- There are no negative literals; `-5` is unary minus applied to `5`.

### 2.4 String literals
```
string_literal = "'" { any char except "'" } "'"
               | '"' { any char except '"' } '"' ;
```
- Two delimiters are allowed, `'` and `"`; the other one may appear freely inside.
- **There are no escape sequences.** A backslash is an ordinary character. It is
  impossible to put both quote characters in a single literal; build such strings
  with `+` and `chr(34)` / `chr(39)`.
- String literals **may span several lines**; embedded CR/LF bytes become part of the
  value.
- (Library note, not lexer: the text-drawing functions interpret `#` as a line break
  and `\#` as a literal `#`. The lexer does not.)
- An unterminated string is a compile error.

### 2.5 Operators and punctuation
Longest match wins. Token set:

```
{  }  (  )  [  ]  ,  ;  :  .
=  :=  +=  -=  *=  /=  |=  &=  ^=
==  !=  <>  <  <=  >  >=
&&  ||  ^^  !
&  |  ^  ~  <<  >>
+  -  *  /  %
```
- `<>` is a synonym of `!=`.
- `%` – not described in the manual. UNSPECIFIED – implementation choice: accept as
  a synonym of `mod`.
- There are **no** `++`, `--`, `?:`, `<<=`, `>>=`, `%=` operators.
  UNSPECIFIED – implementation choice: `++`/`--` are lexed as two separate `+`/`-`
  tokens (so `a = --b` means `a = -(-b)`, and a statement `i++` is a syntax error).

### 2.6 Keywords
Reserved words (cannot be used as variable names):

```
if then else  begin end  repeat while do until for
switch case default  break continue exit return  with
var globalvar
and or xor not div mod
```
- `begin` ≡ `{`, `end` ≡ `}`.
- `and` ≡ `&&`, `or` ≡ `||`, `xor` ≡ `^^`, `not` ≡ `!`.
- `div`, `mod` are binary operators (§4).
- `self other all noone global local true false pi` are **not** keywords but
  built-in constants (§12.1). They still may not be assigned to.

---

## 3. Program and statement grammar

EBNF. `[x]` optional, `{x}` zero or more. `expr` is defined in §4.

```
program     = { statement } EOF ;           (* outer braces optional, see note *)

statement   = ";"                                   (* empty statement *)
            | block
            | var_decl | globalvar_decl
            | if_stmt | repeat_stmt | while_stmt | do_stmt | for_stmt
            | switch_stmt | with_stmt
            | "break" | "continue" | "exit"
            | "return" expr
            | assignment
            | call ;                                (* function or script call *)

block       = ("{" | "begin") { statement } ("}" | "end") ;

var_decl       = "var"       identifier { "," identifier } ;
globalvar_decl = "globalvar" identifier { "," identifier } ;

if_stmt     = "if" expr [ "then" ] statement [ [";"] "else" statement ] ;
repeat_stmt = "repeat" expr statement ;
while_stmt  = "while" expr statement ;
do_stmt     = "do" statement "until" expr ;
for_stmt    = "for" "(" simple_stmt [";"] expr [";"] simple_stmt ")" statement ;
simple_stmt = assignment | call | block | var_decl ;     (* see note *)
with_stmt   = "with" expr statement ;

switch_stmt = "switch" expr "{" { case_label | statement } "}" ;   (* "begin"/"end" also allowed *)
case_label  = "case" expr ":" | "default" ":" ;

assignment  = lvalue assign_op expr ;
assign_op   = "=" | ":=" | "+=" | "-=" | "*=" | "/=" | "|=" | "&=" | "^=" ;
call        = identifier "(" [ expr { "," expr } ] ")" ;
```

Notes:
1. **Program braces.** The manual describes a program as `{ … }`, but code without
   outer braces is accepted (the manual's own examples omit them). A program is
   simply a statement list; an outer block is just a block.
2. **Semicolons are optional.** Statements may be separated by `;` or by nothing.
   Extra `;` are empty statements. `;` may appear before `else` (`if a b=1; else b=2`).
   The parser must therefore determine statement ends from the grammar alone; this
   works because every statement starts with a keyword, `{`/`begin`, `;`, or an
   identifier/`(` beginning an lvalue or call, and expressions end where the next
   token cannot continue them.
3. **Parentheses around conditions are optional.** `if x > 3 y = 1` is legal; the
   condition is the longest expression starting after `if`. The manual examples use
   unparenthesised conditions (`if variable_global_exists('a') global.a += 1 else …`).
   Same for `while`, `repeat`, `until`, `with`, `switch`. Parentheses are just part
   of the expression. Caveat for implementers: in `if (a).x = 1` the expression is
   `(a).x`, then `= 1`… is not a statement start — UNSPECIFIED – implementation
   choice: greedy expression parsing; such code is a syntax error and real code
   always parenthesises in a way that avoids this.
4. **`then`** is an optional noise word after the `if` condition.
5. **Only assignments and calls are expression-statements.** `5;`, `a == b;`, `a;`
   are syntax errors. A call statement discards the returned value.
6. **`for` header.** The manual gives `for (<statement1> ; <expression> ; <statement2>)`.
   UNSPECIFIED – implementation choice: statement1/statement2 may be any
   `simple_stmt`; the separating `;` are required in practice (recommend: require
   them; accept a missing one only if unambiguous is not worth it).
7. **`var` / `globalvar`** take bare names only; no initialiser (`var a = 1;` is a
   syntax error). The manual asks for `var` at the start of the code; the
   implementation should accept it anywhere a statement is allowed (§8.2).
8. **`return`** requires an expression. UNSPECIFIED – implementation choice: a bare
   `return` followed by `;`/`}`/end is a syntax error.
9. **Dangling else** binds to the nearest unmatched `if`.
10. **`case` labels** may appear only at the top level of a switch body.
    `case`/`default` outside a switch is a syntax error.
11. **Functions as dot-targets.** A call MAY be the left operand of `.`: its result
    is used as the owner, exactly like a parenthesised expression, for reads and for
    assignments (`instance_nearest(x,y,o).speed = 0` is valid). VERIFIED black-box in
    GM 8.1: `show_message(string(instance_nearest(x,y,all).x))` runs. A call result
    still cannot be indexed with `[ ]`. See §4.1.

12. **Constants after `.`.** The identifier after `.` must be a variable name. If it
    names a constant — a resource name (object, sprite, room, …), a user constant or
    a built-in constant — compilation fails with "Variable name expected." VERIFIED
    black-box in GM 8.1 Lite: a script containing `global.bb_out[...] = ...` while an
    object named `bb_out` exists fails to compile with exactly that message.

13. **Constant folding type errors.** GM 8.1 evaluates operations on literal
    constants while compiling, so a literal type mix such as `'a' + 1` is a
    *compile* error ("Wrong type of arguments to +."), and so is a literal division
    by zero (`1 / 0`: "Division by 0."). Function calls are not folded
    (`chr(97) + 1` compiles and fails at run time). VERIFIED black-box: a script
    containing it makes the game fail at startup (with "Display errors" off it just
    closes). The same operation through variables or function results is a runtime
    error. Scripts are compiled at game start; code actions are compiled when first
    executed, where a compile error is FATAL.

### 3.1 Lvalues
```
lvalue      = var_ref ;
var_ref     = [ owner "." ] var_name [ index ] ;     (* applied left-to-right *)
owner       = var_ref | "(" expr ")" ;               (* chained:  a.b.c,  obj[0].id.x *)
var_name    = identifier ;                            (* must not be a constant, see §8 *)
index       = "[" expr [ "," expr ] "]" ;
```
Precisely: an lvalue is a primary that is an identifier or parenthesised expression
followed by zero or more `.identifier` / `[…]` suffixes, ending in an identifier
(optionally indexed). Examples: `x`, `a[i]`, `b[4,6]`, `global.score_tbl[3]`,
`other.alarm[0]`, `(100032).speed`, `obj[1].id.x`.

---

## 4. Expressions

### 4.1 Grammar and precedence

Binary operators, **lowest to highest** precedence. All binary operators are
left-associative.

| Level | Operators | Meaning |
|---|---|---|
| 1 (lowest) | `&&` `and`, `\|\|` `or`, `^^` `xor` | logical and/or/xor |
| 2 | `<` `<=` `==` `=` `:=`(?) `!=` `<>` `>=` `>` | comparisons → 1 or 0 |
| 3 | `\|` `&` `^` | bitwise or/and/xor |
| 4 | `<<` `>>` | bit shift |
| 5 | `+` `-` | add (concat for strings), subtract |
| 6 | `*` `/` `div` `mod` `%` | multiply, divide, integer divide, modulo |
| 7 | unary `!` `not`, `-`, `+`, `~` | prefix, right-to-left (`- - a`, `!-a`) |
| 8 (highest) | `.` field access, `[…]` indexing, `f(…)` call | postfix |

The manual lists levels 1–6 exactly in this grouping (all three logical operators
share one level, all six comparison operators share one level, all three bitwise
operators share one level). Consequences that differ from C: `a & b == c` is
`(a & b) == c`; `a || b && c` is `(a || b) && c`; `x == 1 or x == 2` works as expected.

Comparison operators are left-associative at the same level: `a < b < c` is
`(a < b) < c`.

```
expr        = logic ;
logic       = compare { ("&&"|"and"|"||"|"or"|"^^"|"xor") compare } ;
compare     = bitwise { ("<"|"<="|"=="|"="|"!="|"<>"|">="|">") bitwise } ;
bitwise     = shift   { ("|"|"&"|"^") shift } ;
shift       = additive { ("<<"|">>") additive } ;
additive    = mult    { ("+"|"-") mult } ;
mult        = unary   { ("*"|"/"|"div"|"mod"|"%") unary } ;
unary       = ("!"|"not"|"-"|"+"|"~") unary | postfix ;
postfix     = call [ "." identifier [ index ] { "." identifier [ index ] } ]
            | primary { "." identifier | index } ;
primary     = number | string | identifier | "(" expr ")" ;
call        = identifier "(" [ expr { "," expr } ] ")" ;
```
- A call may be followed by `.` (then the usual `.`/index chain), but not by `[` (§3 note 11).
- `.` binds tighter than unary minus: `-a.x` = `-(a.x)`.
- The right side of `.` must be an identifier naming a variable (built-in or user);
  it may be followed by an index.
- An index applies to the variable name immediately before it.
- **Dual meaning of `=`.** At the top of a statement, after an lvalue, `=` (and `:=`)
  is assignment. Anywhere inside an expression `=` is the equality operator, same as
  `==`. Thus `a = b = c;` assigns to `a` the result of `b == c`, and
  `if (x = 3)` tests equality. UNSPECIFIED – implementation choice: `:=` inside an
  expression is also treated as `==` (it is the same token class as `=`).

### 4.2 Values

Every value is exactly one of:
- **real** – IEEE-754 64-bit double;
- **string** – an immutable byte string (no length limit specified;
  UNSPECIFIED – implementation choice: no limit other than memory).

There is no boolean, integer, array, or null type at the value level. "true" is the
real 1 and "false" the real 0. Resource indices, instance ids, colours, key codes,
etc. are all reals. Arrays are a property of variables, not values (§9).

### 4.3 Truthiness

> Black-box results refine this section: see "Language and library details — VERIFIED" at the end.

When a value is used as a condition (`if`, `while`, `do…until`, `for` test, operands
of `! && || ^^`, "test expression" actions) it is interpreted as a boolean:

- real `v`: the manual documents that a value counts as true when it is **≥ 0.5**
  (for the Test Expression action) and that `if` treats a value whose **rounding is
  ≤ 0** as false. Implementation rule: **true iff `v >= 0.5`**.
  (The two descriptions agree everywhere except possibly at exactly 0.5 – see
  OPEN_QUESTIONS.) Negative numbers are false. NaN: UNSPECIFIED – implementation
  choice: false.
- string: UNSPECIFIED – implementation choice: a string condition is false (no error).

### 4.4 Arithmetic and string operators

Let `R` = real, `S` = string. "Error" means a runtime error (§13) with message
"Wrong type of arguments to <op>." (UNSPECIFIED wording).

| Op | R op R | S op S | mixed |
|---|---|---|---|
| `+` | sum | concatenation | error |
| `-` `*` `/` | difference / product / quotient | error | error |
| `div` | `trunc(a / b)` (toward zero) | error | error |
| `mod` `%` | `a - (a div b) * b` | error | error |
| `<<` `>>` `&` `\|` `^` | integer op (§4.6) | error | error |
| unary `-` `+` | negation / identity | error | – |
| unary `~` | bitwise not of `to_int(v)` | error | – |
| unary `!` | `1` if operand false else `0` | (string is false → 1) | – |
| `&&` `\|\|` `^^` | 1/0 per truthiness of each side | per §4.3 | per §4.3 |

- `div` and `mod` are defined in the manual exactly as above, so they work on
  non-integers: `7.5 div 2 = 3`, `7.5 mod 2 = 1.5`, `-7 div 2 = -3`, `-7 mod 2 = -1`.
  (i.e. `mod` behaves like C `fmod`.)
- **Division by zero** (`/`, `div`, `mod` with right operand 0): UNSPECIFIED –
  implementation choice: runtime error "Division by 0."; if the error is ignored the
  expression yields 0.
- **No short-circuit.** Both operands of `&&`, `||`, `^^` (and `and/or/xor`) are
  always evaluated, left first, even when the left operand already decides the
  result (documented). This matters for side-effecting calls.
- Result of every comparison and logical operator is the real `1` or `0`.
- Evaluation order of binary operands: left then right. Function arguments: left to
  right. UNSPECIFIED – implementation choice: as stated.
- There is **no implicit real↔string conversion** anywhere. Use `string(v)` and
  `real(s)`.

### 4.5 Comparison semantics

> Black-box results refine this section: see "Language and library details — VERIFIED" at the end.

- R vs R: numeric comparison. UNSPECIFIED – implementation choice: exact IEEE
  comparison (no epsilon). See OPEN_QUESTIONS.
- S vs S: documented to work for all six operators. Equality is exact, byte-for-byte,
  case-sensitive. Ordering is lexicographic by unsigned character code
  (ordinal; a proper prefix is smaller: `"ab" < "abc"`). UNSPECIFIED – implementation
  choice: ordinal byte comparison, not locale collation.
- R vs S (mixed): UNSPECIFIED – implementation choice: no error; `==`/`=` give 0,
  `!=`/`<>` give 1, and `< <= > >=` give 0. (Needs black-box confirmation; GM may
  instead raise an error.)

### 4.6 Integer conversion

> Black-box results refine this section: see "Language and library details — VERIFIED" at the end.

Several operations need an integer from a real. Define
`to_int(v)` = round `v` to the nearest integer, ties to even
(UNSPECIFIED – implementation choice; the manual only says "rounded").

Used by:
- the `if` rounding wording (but use the §4.3 rule for conditions);
- `repeat` count (documented: "rounded value");
- bitwise operators `& | ^ ~ << >>` on both operands; result converted back to real.
  UNSPECIFIED – implementation choice: operate on 64-bit signed integers
  (`i64`); shift counts outside 0..63 give 0 for `<<` and sign-fill for `>>`
  (arithmetic shift). Out-of-range reals saturate.
- array indices (§9), the left operand of `.` (§10), `with` target (§10.3),
  `argument[i]` index.

Documented conversions elsewhere: `div` truncates toward zero; `irandom_range` rounds
non-integer bounds down; date-time values store whole days in the integral part.

### 4.7 Converting reals to strings

> Black-box results refine this section: see "Language and library details — VERIFIED" at the end.
Defined by the library function `string(v)` (documented): integral values are
written without decimals; non-integral values are written with exactly two
decimals (e.g. `string(0.5)` = `"0.50"`, `string(1/3)` = `"0.33"`).
UNSPECIFIED – implementation choice: "integral" means `frac(v) == 0` exactly;
non-integral values use round-half-away-from-zero to two places; negative zero
prints `"0"`; very large magnitudes print all integer digits (no exponent).
The debugger, `show_message(string(v))`, and implicit displays use the same rule.

---

## 5. Statements – semantics

### 5.1 Block
Executes its statements in order. A block does **not** introduce a variable scope.

### 5.2 Assignment
`lvalue = expr` evaluates `expr` then stores into the lvalue (§8, §9, §10).
Compound assignments: `a op= e` is **exactly** `a = a op e`, where the lvalue
expression is re-resolved for the read and for the write (documented for the
`obj.y += 8` case, where the read uses the first instance and the write goes to all
instances – see §10.2).
- `+=` on strings concatenates. `-= *= /=` on strings: error.
- `|= &= ^=` are the bitwise operators.
- Reading an undefined variable in a compound assignment follows §8.5.
- Assigning to a read-only built-in variable, a constant, or a resource name:
  compile-time error for constants/resource names; runtime (or compile-time when
  statically known) error "Cannot assign to the variable" for read-only built-ins.
  UNSPECIFIED wording.
- Type of built-in variables: built-ins have a fixed type (all reals except a few
  strings such as `keyboard_string`, `keyboard_lastchar`, `room_caption`,
  `error_last`, `caption_*`, `working_directory` etc.). Assigning the wrong type:
  UNSPECIFIED – implementation choice: runtime error "Wrong type of value assigned".
- User variables are dynamically typed; any variable may hold a real or string and
  may change type.

### 5.3 if
Evaluate the condition (§4.3); execute the then-branch if true, else the else-branch
if present.

### 5.4 repeat
Evaluate the expression **once**; let `n = to_int(value)`. Execute the body `n`
times (zero times if `n ≤ 0`). String count: UNSPECIFIED – implementation choice:
runtime error. `break` leaves the loop; `continue` starts the next iteration.

### 5.5 while
Loop: evaluate condition; if false stop; execute body; repeat. No iteration limit
(an infinite loop hangs the game – documented).

### 5.6 do … until
Execute the body, then evaluate the condition; stop when it is **true**; otherwise
repeat. The body executes at least once. `continue` jumps to the `until` test.

### 5.7 for
`for (init; cond; step) body`: execute `init` once; then loop: evaluate `cond`, stop
if false; execute `body`; execute `step`. `continue` in the body jumps to `step`.
`break` leaves the loop without executing `step`.

### 5.8 switch
1. Evaluate the switch expression once → `v`.
2. Walk the `case` labels in source order; for each evaluate its expression
   (case expressions are general expressions, evaluated at run time) and compare with
   `v` using `==` semantics (§4.5; strings match strings, mixed types never match).
   UNSPECIFIED – implementation choice: evaluate case expressions lazily, in order,
   stopping at the first match.
3. Execution continues with the statement following the first matching label (so
   several labels may share statements). If none matches, execution continues after
   `default:` if present; otherwise the switch does nothing. `default` may appear
   anywhere in the body.
4. Fall-through: there is no implicit break; execution runs on through subsequent
   labels until `break` or the end of the switch body.
5. `break` inside a switch leaves the switch. `continue` inside a switch that is
   inside a loop/with applies to that enclosing loop (UNSPECIFIED – implementation
   choice). Duplicate case values: the first wins.

### 5.9 break
Inside `for`, `while`, `do`, `repeat`, `switch`, or `with`: terminate the innermost
such construct. Outside all of them: terminate the current program (code
action/script), like `exit` (documented).

### 5.10 continue
Inside `for`, `while`, `do`, `repeat`, or `with`: proceed with the next iteration
(for `with`: the next instance). Outside: UNSPECIFIED – implementation choice:
behave like `exit`.

### 5.11 exit
Ends execution of the current program (the current script invocation or code
action). It does not end the game or the event. Inside a script, the script returns
0 (UNSPECIFIED value – implementation choice: real 0) to its caller.
Inside a `with`, `exit` ends the whole program, not just the with-iteration.

### 5.12 return
`return e` evaluates `e` and ends the current script, making `e` the value of the
script call. In a code action, `return e` ends the code; the value is discarded.
A script that finishes without `return` yields real 0 (UNSPECIFIED – implementation
choice).

### 5.13 with
See §10.3.

### 5.14 var / globalvar
See §8.2, §8.3. They are executable statements but have no runtime effect except
declaring names.

---

## 6. Calls

### 6.1 Built-in functions
- Identified by name in `builtins.json` (`kind: "function"`).
- Brackets are required even with zero arguments (`instance_destroy()`).
  A function name without brackets is a compile error (UNSPECIFIED – implementation
  choice).
- Fixed-arity functions must receive exactly the listed number of arguments;
  otherwise compile error "Wrong number of arguments to function or script."
  (UNSPECIFIED wording). Variadic functions (`variadic: true`, e.g. `choose`, `min`,
  `max`, `mean`, `median`, `script_execute`, `execute_string`, `external_call`)
  accept the listed minimum and up to 16 arguments in total.
- Some functions return no meaningful value; used in an expression they yield 0
  (UNSPECIFIED – implementation choice).
- Argument type mismatches (e.g. string passed where a real is required) are runtime
  errors reported by the function (library specs define each).

### 6.2 Script calls
- A script resource's name may be called like a function: `scr_name(a, b)`.
- 0 to 16 arguments; more is a compile error. No arity checking against the
  script body.
- The callee runs with the **same `self` and `other`** as the caller (scripts are not
  methods of another instance).
- Each invocation gets fresh argument variables and a fresh `var`-local scope.
- Recursion is allowed. UNSPECIFIED – implementation choice: limit recursion depth
  (e.g. 10 000 nested calls / or stack-based) and report a runtime error instead of
  crashing.
- Resolution order when a called name matches both a script and a built-in function:
  UNSPECIFIED – implementation choice: built-in wins (scripts named like built-ins
  should be rejected at load with a warning).

### 6.3 Script arguments

> Black-box results refine this section: see "Language and library details — VERIFIED" at the end.
Inside a script, the passed values are available as:
- `argument0` … `argument15` – read/write variables;
- `argument[0]` … `argument[15]` – the same storage, array form; the index is any
  expression;
- `argument_count` – number of arguments actually passed (read-only in practice;
  UNSPECIFIED whether writable – recommend read-only);
- `argument_relative` – read-only; 1 if the script was invoked by an action whose
  "Relative" box was checked, else 0. 0 for calls from code.

Semantics:
- `argumentN` and `argument[N]` alias the same slot.
- Slots ≥ `argument_count` (not passed): UNSPECIFIED – implementation choice:
  read as real 0, no error.
- Index outside 0..15 for `argument[i]`: runtime error.
- Arguments are passed by value; assigning to `argument0` does not affect the caller.
- The argument slots belong to the script invocation, so inside a `with` block in a
  script they still refer to the script's arguments.
- When a script is run from the "Execute Script" action, at most 5 arguments are
  supplied (the others are absent).
- In a code action (not a script) the argument variables exist with value 0 and
  `argument_count` is 0.

---

## 7. Identifier classification (compile time)

When the compiler sees an identifier `n` in an expression or as an lvalue head, it
classifies it in this order:

1. keyword → syntax;
2. followed by `(` → function call: built-in function, else script name, else compile
   error "Unknown function or script: n";
3. name declared by `var` earlier in the same program (§8.2) → **local** variable;
4. built-in variable name (from `builtins.json`, `kind: "variable"`) → built-in
   variable with its `scope` (`instance`, `global`, or `script` for the argument
   family);
5. built-in constant (`kind: "constant"`) → constant value;
6. user-defined constant (§12.3) → its value;
7. resource name (sprite, sound, background, path, script, font, timeline, object,
   room) → the resource's index;
8. otherwise → a **user variable**, resolved at run time: if the name is in the
   global `globalvar` set → global variable; else instance variable of `self`.

Names after `.` are always variable names (steps 4 and 8 only; the owner decides the
scope, §10). Constants and resource names cannot be assigned to (compile error).
UNSPECIFIED – implementation choice for order of 5/6/7 among themselves, and whether
`var` names shadow built-ins (recommend: step order as listed; `var` shadows
everything except keywords).

---

## 8. Variables

### 8.1 Kinds of storage
| Kind | Lifetime | Created by |
|---|---|---|
| local (`var`) | one execution of one program (code action, script invocation, execute_string) | `var` declaration |
| instance | lifetime of the instance | first assignment through that instance |
| global | whole game | assignment via `global.` or to a `globalvar` name |
| built-in instance | instance | always exists |
| built-in global | game | always exists |
| arguments | one script invocation | the call |

Variables are created by assignment; no declaration is needed except for `var`.
A new plain variable is stored on the current (`self`) instance only, and is not
visible from other instances (even of the same object).

### 8.2 `var`
`var a, b, c;` declares names as local to the current program execution.
- From the point of declaration to the end of the program text, uses of those names
  refer to the local slots. UNSPECIFIED – implementation choice: compile-time
  (lexical) effect from the declaration to the end of the program, independent of
  whether the `var` statement is executed at run time.
- Locals start uninitialised (§8.5).
- Locals are shared by all code in the same program, including the bodies of `with`
  statements (so `with (o) x = localvar;` reads the outer local).
- Each script invocation has its own locals; a script cannot see its caller's locals.
- A `var` name may hold an array (`var a; a[3] = 1;`).
- Redeclaring a name is harmless.

### 8.3 `globalvar`
`globalvar a, b;` marks names as global. Documented behaviour: once the declaration
has **executed**, the plain name refers to the global variable everywhere (in all
programs, for all instances), without the `global.` prefix. `a` and `global.a` are
then the same storage.
- Implementation: a run-time set of global-declared names; checked at step 8 of §7.
  Declaring does not assign a value.
- UNSPECIFIED – implementation choice: once added, a name is never removed; a
  `globalvar` name that equals a built-in variable has no effect.

### 8.4 `global.`
`global.n` always names global variable `n`. `global` is the constant -5 and acts as
the owner "global scope" for `.` (§10.1).

### 8.5 Uninitialised variables
Reading a variable that has never been assigned (instance, global, local, or an array
element beyond the variable's current extent) is a runtime error:
"Unknown variable n" (UNSPECIFIED wording; GM uses messages of this form). The
Global Game Settings → Errors option **"Treat uninitialized variables as 0"**
changes this: such reads silently yield real 0. The setting is per game and fixed at
build time. It does not create the variable.

`variable_local_exists(name)` / `variable_global_exists(name)` test existence by
string name; companion get/set functions access variables by name (library).

Compound assignment to an undefined variable reads it first and so follows the same
rule.

### 8.6 Built-in variables
- `builtins.json` lists each with `scope` (`instance` or `global`), `readonly`, and
  `array` bounds where applicable (`alarm[0..11]`, `view_*[0..7]`,
  `background_*[0..7]`, `instance_id[0..instance_count-1]`).
- Instance-scope built-ins exist on every instance (`x`, `y`, `speed`, `alarm`, …);
  `other.x` etc. access them on another instance.
- Global built-ins (`room`, `score`, `mouse_x`, `view_xview[i]`, …) exist once.
  UNSPECIFIED – implementation choice: `inst.score` (global built-in via an instance
  owner) refers to the same global value.
- Index out of declared range: runtime error.
- Using a built-in array variable without index means index 0 (§9).

---

## 9. Arrays

- Any user variable (instance, global, local) may be used as a 1-D or 2-D array by
  indexing it: `a[i]`, `b[i, j]`. Indexes start at 0.
- **Auto-creation/growth.** Assigning to an element creates the array and grows it as
  needed; no declaration or size is given.
- Limits (documented): each index must be < 32000, total number of elements ≤ 1 000 000.
  Violations are runtime errors. **Negative indices** are invalid → runtime error.
- Indices are converted with `to_int` (§4.6). String index → runtime error.
- UNSPECIFIED – implementation choice (matches the known storage model; confirm by
  black-box test):
  - A plain variable and element `[0]` / `[0,0]` are the same storage:
    `a = 5` ⇒ `a[0] == 5` and `a[0,0] == 5`; `a[0] = 7` ⇒ `a == 7`.
  - A 1-D access `a[i]` is the same as `a[0, i]`.
  - Storage is rectangular per row: writing `b[4,6]` makes row 4 at least 7 long;
    unwritten elements inside the allocated extent read as real 0; reading outside
    the extent is an uninitialised read (§8.5).
  - Arrays are not values: `c = a` copies only `a[0]`; arrays cannot be passed to or
    returned from scripts; `argument0[1]` is not an array of the caller.
- Elements are independently typed (each may be real or string).

---

## 10. Instances, owners and `with`

### 10.1 Special instance constants
| Name | Value | Meaning as owner / with-target |
|---|---|---|
| `self` | -1 | the current instance |
| `other` | -2 | the "other" instance (collision partner, or the outer `self` inside `with`) |
| `all` | -3 | all (active) instances |
| `noone` | -4 | no instance |
| `global` | -5 | not an instance: the global variable container |
| `local` | -7 | listed in `fnames`; undocumented. UNSPECIFIED – implementation choice: as an owner it designates the current program's `var` locals; as a `with` target it selects nothing |

Values marked here are not given in the manual; they are inferred and recorded with
`source: "inferred"` in `builtins.json`.

Object names evaluate to the object index (0, 1, 2, …). Instance ids are
**≥ 100000** (documented). Object names, special constants and ids are ordinary
reals and can be stored, compared and computed with.

### 10.2 The `.` operator
`owner.name` — the manual describes `.` as an operator taking a value on the left
and a variable on the right. Let `o = to_int(owner value)` (string → runtime error).

| `o` | Read `o.name` | Write `o.name = v` |
|---|---|---|
| `self` (-1) | variable of current instance | same |
| `other` (-2) | variable of the other instance; error if there is none | same; no-op if none |
| `all` (-3) | first instance (in instance order) that exists | **all** instances |
| `noone` (-4) | runtime error | no-op |
| `global` (-5) | global variable | global variable |
| `local` (-7) | UNSPECIFIED: current locals | same |
| ≥ 100000 | that instance; runtime error if it does not exist | that instance; no-op if it does not exist |
| 0 ≤ o < 100000, an object index | first instance of that object (or of a descendant, see below) | **all** instances of that object (and descendants) |
| other values | runtime error "Unknown instance/object" | no-op (UNSPECIFIED) |

- "Instance order" = the order instances are iterated by `with` (§10.3).
- UNSPECIFIED – implementation choice: object references include instances of child
  objects (consistent with `with` and `instance_number`); deactivated instances are
  excluded.
- Compound assignment through a multi-instance owner reads from the first instance
  and writes the resulting single value to all instances (documented example:
  `ball.y += 8` sets every ball to the first ball's y + 8).
- Chaining: `obj[0].alarm[4] = 12` (owner taken from an array element),
  `obj[1].id.x = 12` (read `obj[1].id` → first instance's id → owner of `x`).
- Inside `o.name`, `name` is resolved only as a variable (built-in by its scope, or
  user variable). Built-in global variables via an instance owner: see §8.6.
- The owner of a plain identifier is implicit (`self`, or global for globalvar names
  and global built-ins).

### 10.3 `with (expr) statement`
1. Evaluate `expr` once, `o = to_int(value)`.
2. Build the target list:
   - `self` → [current instance]; `other` → [other instance] (empty if none);
   - `all` → all active instances; `noone` → [];
   - object index → all active instances of that object and its descendants
     (UNSPECIFIED: descendants included – recommend yes);
   - instance id → [that instance] if it exists and is active, else [];
   - `global`, `local`, unknown values → [] (UNSPECIFIED).
   UNSPECIFIED – implementation choice: the list is a **snapshot** taken before the
   first iteration; instances created during the loop are not visited; instances
   destroyed or deactivated before their turn are skipped.
3. For each target `t`: set `other := old self`, `self := t`; execute `statement`.
   `break` ends the whole with; `continue` moves to the next target.
4. After the loop (or on `break`), restore the previous `self` and `other`.
   `exit`/`return` inside a with leave the program entirely (restoring self/other
   for the caller).
- Nested `with`: inside the inner one, `other` is the outer one's current target.
- `var` locals and script arguments remain accessible inside the with body.
- Destroying `self` inside the body (e.g. `with (ball) instance_destroy();`) is
  allowed; the instance's variables remain readable until the end of the current
  event (UNSPECIFIED – implementation choice).

### 10.4 What `other` means outside `with`
- In a collision event: the colliding instance (documented).
- UNSPECIFIED – implementation choice for all other events: `other` = `self`, except
  in an instance's Create event triggered by `instance_create` from code, where
  `other` is the instance that called `instance_create` (and likewise for events run
  by `event_perform`, `instance_change`, `instance_copy` – `other` is the caller).
- Scripts inherit `self`/`other` from the caller.

---

## 11. Program execution model

### 11.1 Contexts
Each executing program has a context: `self` instance, `other` instance, a local
variable table, the argument slots (16 + count + relative flag), and the current
event (`event_type`, `event_number`, `event_object`, `event_action`).

### 11.2 Code action "applies to"
A code action can apply to self, other, or an object; for an object, the code is run
once per instance of that object with `self` set to that instance — the same as
wrapping the whole code in `with (object) { … }`.

### 11.3 Action argument expressions
Action fields are compiled as expressions and evaluated with the action's
applies-to instance as `self`. String-typed action fields are taken literally
unless the parameter is declared as an expression (action-library concern).

### 11.4 Arguments in non-script programs
See §6.3.

### 11.5 Scripts as values
A script name is a constant (its resource index); `script_execute(ind, args…)` calls
the script with that index. Same semantics as a direct call.

### 11.6 Dynamic code
`execute_string(str, args…)` compiles `str` as a program and runs it with the
caller's `self`/`other`, a fresh local scope and the given arguments; the result is
the program's `return` value (0 if none). Compile errors are runtime errors of the
caller. `execute_file` is the same with source from a file.

---

## 12. Constants

### 12.1 Language constants
`true` = 1, `false` = 0, `pi` ≈ 3.141592653589793 (documented as 3.1415…; use the
full double value). `self`, `other`, `all`, `noone`, `global`, `local` per §10.1.

### 12.2 Library constants
All other `#` entries of `fnames` (event types, key codes, colours, blend modes, …)
are listed in `builtins.json` with values where known. Values with
`source: "inferred"` are not stated in the manual; they come from public Windows/
DirectX conventions or file-format observation and must be verified.

### 12.3 User-defined constants (resource "Constants")
- Name rules: letter or underscore, then letters/digits/underscores.
- Value: a constant expression (number, string literal, or expression over built-in
  constants, resource names and previously defined user constants). Evaluated once
  before anything else in the game runs; cannot reference rooms, instances,
  variables or scripts.
- Text export format: one `NAME=VALUE` per line.
- UNSPECIFIED – implementation choice: constants are evaluated in list order; a
  forward reference is an error; a user constant that shadows a built-in constant or
  resource name is an error at load.

### 12.4 Resource names
Resource names (letters, digits, underscore; not starting with a digit) are constants
holding the resource index. The manual warns against naming resources `self`,
`other`, `global`, `all` or any keyword; UNSPECIFIED – implementation choice: such
resources are inaccessible from code (the built-in meaning wins).

---

## 13. Runtime errors

> Black-box results refine this section: see "Language and library details — VERIFIED" at the end.

- Error options (Global Game Settings): display error messages; write them to
  `game_errors.log`; abort on all errors; treat uninitialised variables as 0.
- Errors are either fatal (always abort) or non-fatal (player may ignore).
  With "abort on all error messages", every error is fatal.
- UNSPECIFIED – implementation choice:
  - Language-level errors (type errors, unknown variable, bad index, division by
    zero, unknown instance) are non-fatal. If ignored, the **remainder of the current
    program** (code action / script call chain up to the action) is abandoned and the
    game continues with the next action.
  - Every error sets `error_occurred = 1` and `error_last` = message (both writable
    built-ins, documented). When error display is off, execution continues as if the
    error had been ignored.
  - Message format: `ERROR in action number N of <Event> for object <obj>: <message>`
    with line/position for code. Exact wording is a UI concern (see runner spec).

---

## 14. Implementation notes (non-normative)

- Recommended pipeline: lexer → recursive-descent parser (statement start set:
  keywords, `{`, `begin`, `;`, identifier, `(`) → AST → compile to a bytecode/tree with
  resolved identifiers (§7) → interpreter.
- The value type is `enum Value { Real(f64), Str(Rc<str>) }`.
- Instance variable tables: hash map name→array-capable slot. Arrays: per-variable
  `Vec<Vec<Value>>` (row 0 used for 1-D).
- Keep `self`/`other` as instance ids, not references, because instances can be
  destroyed mid-program.

---

## 15. `builtins.json` format

```json
{
  "meta": {...},
  "entries": [
    {
      "name": "draw_text",
      "kind": "function" | "variable" | "constant",
      "readonly": false,                // variables only; constants are always read-only
      "args": ["x","y","string"],       // functions only, names from fnames
      "variadic": true,                 // only when the signature ends with ",..."
      "array": {"min":0,"max":11},      // array variables only (bounds from fnames)
      "scope": "instance" | "global" | "script",   // variables only
      "chapter": "405", "section": "5.5", "page": "405_05_text.html",
      "description": "paraphrased one-liner",
      "value": 0, "value_source": "manual" | "inferred"   // constants only; null if unknown
    }
  ]
}
```
- `scope: "script"` is used for the argument family (`argument`, `argument0..15`,
  `argument_count`, `argument_relative`), which belong to the current program
  invocation rather than to an instance or the game.
- `chapter: "yoyo"` marks identifiers in the trailing "YoYo extension stuff" block of
  `fnames`; they are undocumented.
- Order of entries follows `fnames`.

## RNG — VERIFIED black-box (GM 8.1 Lite, `bb_random`, 191 values reproduced exactly)

- State: a signed 32-bit integer `s`. One step: `s = s * 134775813 + 1` (wrapping
  mod 2^32). Every random function performs exactly one step per call.
- `random_set_seed(x)`: `s = x` rounded half-to-even, wrapped to 32 bits
  (0.5 -> 0, 1.5 -> 2, 2.7 -> 3, 2^31 -> -2^31, 2^32 -> 0).
- `random_get_seed()`: the current state `s` (not the last seed set).
- `random(x)`: step, then `u / 2^32 * x` where `u` is `s` read as unsigned.
  Negative and zero `x` are allowed (no error).
- `irandom(n)`: step, then `(u * m) >> 32` in unsigned 64-bit arithmetic with
  `m = (trunc(n) + 1)` taken as unsigned 32-bit (so `irandom(-5)` after seed 4 gives
  `u - 1`, a huge value). `irandom(2.7)` behaves like `irandom(2)`.
- `random_range(a, b)`: `a + random(1) * (b - a)`.
- `irandom_range(a, b)`: truncate both, `lo + irandom(hi - lo)` with `lo <= hi`
  (argument order doesn't matter).
- `choose(v0..vk-1)`: step, index `(u * k) >> 32`.
- The seed at game start differs per run (the game is randomized at start).
- Values >= 2^31 printed via `floor(...)` came out negative because `floor` returns
  a 32-bit integer (see the bb_lang section below); `random` itself is unsigned.

## Script argument count and name priority — VERIFIED black-box (GM 8.1 Lite, `bb_clash`)

- A user script whose name equals a built-in function (`abs`) takes priority:
  `abs(-3)` called the script.
- GM 8.1 checks script argument counts at run time. A script whose body is
  `return 42;` (no `argumentN` used) called with one argument raised the non-fatal
  error: `Illegal argument count calling script "abs".` / `Script requires 0
  arguments, 1 have been supplied.` So a script has a *required* argument count
  derived from its body (presumably the highest `argumentN` used + 1).
- The full rule (gated by settings flag bit 1) is in the bb_lang section below.

## `var` / `globalvar` of a built-in name — VERIFIED black-box (GM 8.1 Lite, `bb_perf`)

`var i, t, n, x, ...` fails to compile at `x` with "Cannot redeclare a builtin
variable." Locals (and, by the same rule, globalvars) cannot use the name of a
built-in variable; they never shadow built-ins.

## Language and library details — VERIFIED black-box (GM 8.1 Lite, `bb_lang`, `bb_args0`, `bb_random`, `bb_clash`)

These rules come from the results of the gm-blackbox projects (key prefixes in
brackets). Where they conflict with earlier sections of this document, they
take priority. Implemented in `crates/gml` (regression tests in
`crates/gml/tests/verified.rs`).

**Numeric literals** [syn.double_dot, strfmt.1_23456_0_20, div0.tiny]
- The value is built in double precision: integer digits as `v = v*10 + d`,
  then each fraction digit adds `d*p` with `p = 0.1`, divided by 10 after each
  digit. So `1.23456` comes out one ulp below the nearest double, and `0.3`
  equals `0.1 + 0.2` exactly.
- Dots after the first one are skipped: `1.2.3` is 1.23.
- `1e3` is not a number literal (it gives "Assignment operator expected").
- `%` is not an operator. It is a compile error, "Unexpected symbol." [syn.percent]

**Comparison** [eq.*, math.sin_pi_is0, math.lengthdir_x_90_is0]
- Reals are equal when `|a-b| <= max(min(|a|,|b|), 1) * 1e-14`.
  VERIFIED bounds [eq.*, eq2.*, math.sin_pi_is0, math.lengthdir_x_90_is0]:
  - `sin(pi) == 0` (difference 1.2e-16) and `lengthdir_x(10,90) == 0` (6.1e-16)
    are true, so values that close to 0 compare equal.
  - `0 == 1e-13`, `0 == 5e-13`, `0 == 2e-12`, `1 == 1 + 5e-13`,
    `1 == 1.0000000001` and `1000000 == 1000000 + 1e-7` are all false. The
    tolerance is below 1e-13 near 0 and 1, and a relative tolerance would have
    to be below 1e-13 (the `1e6` case). Delphi's `SameValue` default (1e-12) is
    ruled out.
  - The constant 1e-14 (relative above 1) is a choice inside these bounds;
    bb_lang3 `eq3.*` narrows it.
- Examples: `0.1+0.2 == 0.3` is true (exactly equal, see the literal rule above).
- `<` means "less and not equal", `<=` means "less or equal", and so on.
  So `0.3 < 0.1+0.2` is false and `0.3 >= 0.1+0.2` is true.
- `switch` cases use the same equality. A case of the other type simply doesn't
  match; there is no error [syn.switch_mixed].
- Open: whether the tolerance is relative for large values. The data only shows
  that it is below 1e-12 near 1 and near 0.

**Type errors** [mix.*, truth.str_*]
- Comparing a string with a real (`== != < <= > >=`) is an error,
  "Cannot compare arguments.".
- Arithmetic on mixed or string operands is an error, "Wrong type of arguments to +."
  (the same wording for `-`, `*`, and so on).
- Bitwise and logical operators with a string operand give
  "Wrong type of arguments for |." or "Wrong type of arguments for &&.".
- Unary `-` and `!` on a string give "Wrong type of arguments to unary operator.".
- A string used as the condition of `if`, `while`, `do … until` or `for` is a
  run-time error, "Expression expected".
- `repeat` with a string count gives "Repeat count must be a number".
- These errors abandon the rest of the code action (see Errors below).

**Integer conversion and bitwise operators** [bit.*, idx.*, repeat.*]
- Bitwise operands are rounded half to even into a 64-bit integer.
- `>>` is a logical (unsigned) shift: `-1 >> 1` is 2^63-1.
- Shift counts follow Delphi's 64-bit helpers. The low byte of the count is
  taken as a signed value. Below 32, including negative counts, the shift is by
  `count & 31`. 32..63 shift across the word. 64 and above give 0.
  Examples: `1 << -1` is 2^31, `1 << 63` is -2^63, `1 << 64` is 0.
- The 64-bit result is converted back to a real.
- `floor` and `ceil` return a **32-bit** integer: the value is truncated to 64
  bits and wrapped to 32, so `floor(3698175007)` is -596792289 [bb_random raw.*].
- `round` does not wrap: it rounds half to even.
- `div` truncates. `mod` keeps the dividend's sign and works with fractions.
- Division by zero is an error for `/` and `div` ("Division by 0.") and for
  `mod` ("Modulo by 0.").

**`string(x)` and `string_format`** [str.*, strfmt.*]
- Integral reals print as integers. Outside the 64-bit range they print "ERROR"
  (`string(power(10,20))`).
- Other reals print with 2 decimals and at most 10 significant digits:
  `4294967296.5` gives "4294967297.00".
- A value with more than 10 integer digits switches to a scientific form:
  "1.23456789E11". There is no `+` and no leading zeros in the exponent, and
  trailing zeros are dropped.
- `string_format(v, tot, dec)` uses up to 18 significant digits. Beyond that it
  uses the scientific form: `string_format(1e20,0,2)` is "1E20". The result is
  padded on the left to `tot`.
- If `dec` is outside 0..18, the result shows 18 decimals but rounds with the
  signed `dec`: `string_format(1.5,0,-1)` is "0.000000000000000000".
- A string argument is returned unchanged.
- Digit generation: the value is scaled by a power of ten in double precision to
  an 18-digit integer, rounded half to even. The digits are then rounded half up
  to the requested count.
  - This explains "2.67" for `2.675` and "1.00" for `0.995`.
  - It also explains `2^63` → "9.22337203685477632E18" and `$FFFFFFFFFFFF` →
    "281474976710655.008".
- A result whose digits are all zero gets no minus sign.

**Scripts and arguments** [args.*, bb_args0 args.*]
- Settings flag bit 1 is the Errors-tab option "Throw an error when arguments
  aren't initialised correctly." With it set, a script call must supply exactly
  `1 + highest argumentN index used` arguments; a script that uses none needs 0.
  - A script that also uses `argument[i]` accepts more arguments.
  - The call fails before the script runs with
    `Illegal argument count calling script "<name>".` + CR +
    `Script requires N arguments, M have been supplied.`.
  - `argument[i]` with `i >= argument_count` gives "Unknown variable argument or
    array index out of bounds".
  - The message says `Script requires at least N arguments, ...` when the script
    also uses `argument[i]` [bb_lang2 args.mixed_fewer].
  - `script_execute` is checked the same way. Too few arguments: the error is
    reported and the call yields 0. Too many: the error is reported as a
    non-aborting error and the script still runs with all the arguments
    [bb_lang2 args.script_execute_more = 1 with err = 1]. A direct call with too
    many arguments is an ordinary (aborting) error.
- With bit 1 clear, any count is accepted, `argument_count` is the supplied
  count, and missing arguments read 0.
- Arguments live in **one global array**:
  - A script call saves the array, installs its own arguments (the rest become 0),
    and restores the old array on return, also after an ordinary error.
  - Code actions don't touch the array; they see whatever it holds.
  - `execute_string(code, args…)` writes `argument0..` but keeps `argument_count`
    [args.exec_string_args = "1/b"].
- Reading `argument[i]` with `i >= 16` abandons the whole event silently, like
  a crash [args.arr_index_16.aborted]. On that path the script's arguments are
  *not* restored: the next code actions read `argument0 = 16`,
  `argument_count = 1` [args.code_action_*].
- A script that ends without `return` (or with `exit`) returns the value of the
  last assignment or call statement it ran [args.noreturn = 1, args.exit = 2;
  bb_lang2 ret.call_last: `global.q = 1; abs(5);` returns 5, ret.var_last:
  `var a; a = 6;` returns 6]. An `if` condition doesn't count
  [ret.if_last: `global.q = 3; if (global.q == 0) ...` returns 3].
- Writing `argument_count` (or any read-only built-in, such as `id`) compiles,
  but at run time it is the error "Cannot assign to the variable"
  [args.write_count, type.readonly_id].
- More than 16 call arguments is a compile error, "Wrong number of arguments to
  function or script.".
- `min()`, `max()`, `choose()`, `mean()` and `median()` accept no arguments and
  return 0 [math.*_none].
- `mean()` counts a string argument as 0, with no error
  [bb_lang2 mm.mean_str: `mean(1, 'a')` = 0.5].

**Errors and recovery** [errcont.*, math.*, real.*, misc.*, type.readonly_id, var.shadow_x]
- An error raised by an operator, a variable access or a script call abandons
  the rest of the code action. The next action runs. Inside a script, the script
  and its caller's action are both abandoned.
- An error raised by a **built-in function itself** doesn't abandon anything:
  the error is reported (`error_occurred`, `error_last`), the function returns
  0, and the code continues. Examples: `sqrt(-1)`, `ln(0)`, `real('abc')`,
  `sound_play(99)`.
  - The message is "Error in function ln()."; `sqrt(-1)` gives "Cannot apply
    sqrt to negative number."; `sound_play(99)` gives "Sound does not exist.".
- Errors inside `execute_string` code, both compile and run-time, are reported,
  and `execute_string` returns 0 to a caller that keeps running.
- Message layout (the full text is in `error_last`):
  - Header lines end with CR LF: `ERROR in` / `action number N` /
    `of <Event> Event` / `for object <name>:` / empty line.
  - Then, for errors in code, CR-separated lines: an optional
    `In script <name>:`, then `Error in code at line L:`, three spaces plus the
    source line, a caret line (P+1 spaces, then `^`), and
    `at position P: <message>`.
  - Function errors show only their message.
  - A compile error in `execute_string` has no header:
    `COMPILATION ERROR in string to be executed` + CR + the code part.
- Positions: for an operator error, the start of the right operand (after `(`
  for a parenthesised condition); for an index error, the start of the index
  expression.
- Message texts include "Unknown variable x" (also for `noone.x`, `obj.x` with
  no instance, and a bad instance id); "Unknown variable a or array index out of
  bounds" (an element that is not set); "Negative array index"; "Array index >=
  32000"; "Wrong type of array index"; "Cannot assign to the variable" (also for
  `noone.x = 5`).
- `obj.x = 5` with no instance of `obj` is a silent no-op.

**Built-in variables** [type.*, arr.builtin_x_index1]
- Assigning a string to a real built-in (`x`, `speed`, `score`, `image_index`)
  stores 0, with no error.
- Boolean built-ins are set when the value is above 0.5: `solid = 0.5` reads 0
  and `visible = 2` reads 1.
- `x[1]` reads `x`: the index is ignored for non-array built-ins.

**Library** [sf.*, real.*, math.max_mixed]
- String functions given a real treat it as "" and raise no error:
  `string_length(12.5)` is 0, `string_upper(5)` is "".
- `string_delete` with an index below 1 deletes nothing.
- Strings hold one byte per character [bb_lang2 case.*]:
  `string_length(chr(192))` = 1, `string_length(string_lower(chr(192)))` = 1,
  `string_length(string_upper('a' + chr(233)))` = 2,
  `string_length(string_lower(chr(195) + chr(137)))` = 2.
- `ord(s)` returns the first byte of the UTF-8 encoding of the first character:
  `ord(chr(192))` = 195, `ord(string_upper(chr(233)))` = 195. gm-runner takes
  bytes 0x80-0xFF as Latin-1 code points (0xC2 / 0xC3). Unverified: bytes
  0x80-0x9F (code page 1252 maps them elsewhere) and whether `string_lower/upper`
  map Latin-1 letters (gm-runner maps 0xC0-0xDE <-> 0xE0-0xFE except 0xD7/0xF7).
  This corrects the earlier reading that the case functions return UTF-8.
- `real(str)` ignores surrounding whitespace. The rest must be a complete
  `[+-]digits[.digits][e[+-]digits]`; `""` and `"."` give 0. Anything else
  (`"12abc"`, `"0x10"`, `"$10"`, `"1,5"`, `"inf"`, `"-"`) is the function error
  "Error in function real()." (non-aborting, result 0).
- `min`/`max` skip arguments whose type differs from the first one:
  `max('a', 1)` is "a".

**Name resolution** [bb_clash]
- A script named like a built-in function wins over the function.
- A resource name wins over a user constant of the same name.
- A user constant wins over a built-in constant (`c_red` defined as 5 gives 5).
