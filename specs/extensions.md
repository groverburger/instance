# Extension packages and DLLs — behavioural specification

Sources: manual `215_00` (extension packages in the IDE), `414_00` (DLL functions);
package file layout in `file-formats.md` §6.2 and §11 (`.ged`/`.dat`), plus
observation of the three packages shipped with GM 8.1 (`GM Printing`,
`GM Room Transitions`, `GM Windows Dialogs`). Paraphrased. Open items:
`OPEN_QUESTIONS.md` section `[EXT]`.

---

## 1. Concepts

* An **extension package** adds GML functions and constants (and possibly actions)
  to games that *use* it `[215_00]`. Each package has a unique **name** (the `name`
  field of its `.ged`, e.g. `"GM Room Transitions"`).
* A package contains **files** (file-formats §11.1). Each file has a kind:

| file_kind | Kind | Content |
|---|---|---|
| 1 | DLL | native Windows library; its listed functions are exports called like `external_call` |
| 2 | GML | text: a sequence of `#define name` sections, each a script body |
| 3 | action library [I] | `.lib`-format actions added to the palette (none shipped) |
| 4 | other [I] | data file copied alongside (e.g. images) |

* Every file can name an **init function** (called once at game start) and a
  **final function** (called once at game end), each the name of a function of that
  file (possibly a hidden one). Observed: `GM Room Transitions` file `Transitions.gml`
  init `__tr_init`; `GM Windows Dialogs` file `Windows_Dialogs.dll` init `__wd_init`.
* Each function record has: `name` (GML-visible), `external_name` (DLL export; empty =
  same as `name`), calling convention, help line, `hidden` flag, argument count,
  17 argument types, return type (file-formats §11.1).
* Each file lists **constants**: name, value string, hidden flag.

The older DLL mechanism (`external_define` / `external_call`, §5) remains available
to code; the manual recommends packages instead `[414_00]`.

---

## 2. Installation and lookup

### 2.1 IDE side
* Installed packages live in the `extensions` folder next to the IDE executable as
  `<package name>.ged` (description), `<package name>.dat` (payload: the files'
  bytes, file-formats §11.2) and `<package name>.chm` (help) — verified for the
  shipped packages.
* The IDE's package form lists "used" and "available" packages; Install adds a
  package from a distribution file (`.gex`), Uninstall removes it `[215_00]`
  (`ide.md` §18.4). The `.gex` layout is **not known** (EXT-1). gm81-clean's IDE
  additionally accepts installing a `.ged`+`.dat`(+`.chm`) set by copying it.
* **Recommended** search path for packages (first hit wins, by package name):
  1. `<IDE exe dir>/extensions/`
  2. `%APPDATA%/gm81-clean/extensions/` (where gm81-clean's IDE installs `.gex`/
     `.ged` packages when the IDE folder is not writable)
  3. `<project dir>/extensions/`

### 2.2 Project side
* A project stores only the **names** of the packages it uses, in order
  (file-formats §6.2). Nothing of the package is stored in the `.gmk/.gm81`.
* Loading a project whose packages cannot be found: the IDE keeps the names
  (lossless save) and warns; running fails with "extension package <name> not
  found" (**recommended**).
* Standalone executables must carry the package data: **proposal** for
  `exe-format.md` — after the project payload, append for each used package
  `{u32 name_len; name; u32 ged_len; ged bytes; u32 dat_len; dat bytes}` preceded by
  `u32 count`, and record its offset/length in an extended footer (EXT-2).

### 2.3 Runner side: loading a package
At game load, for every used package (project order), for every file (package
order):
1. Decode the file's bytes from `.dat` (file-formats §11.2).
2. Write DLL/other files to the game's temporary directory (`temp_directory`,
   recommended `<temp>/gm81_<game_id>_<pid>/`) under their `file_name`. Delete them at
   exit.
3. GML files: split at lines starting with `#define`; each section's name is the
   word after `#define`, its body the following lines. Compile each section as a
   script.
4. Register every function record (§3) and constant (§4).
5. Action-library files: register their actions (affects only the IDE palette and
   action execution of games built with them).

Duplicate names (an extension function or constant equal to a built-in, script,
resource, or another extension's name): **recommended** load error naming both.

---

## 3. Extension functions

### 3.1 Visibility
* Every function record becomes a global GML function callable by `name` from any
  code, scripts, `execute_string`, and from other extension functions.
* `hidden = true` only hides the function from the IDE (code completion, "Show
  Extension Functions"); it is **still callable** — the shipped GML wrappers call
  hidden DLL functions (e.g. `wd_message_show` → `__wd_message_show`).
* Argument count: calling with a different number of arguments than
  `argument_count` is a compile-time error like for built-ins ("wrong number of
  arguments"). `argument_count = -1` [I] means variadic (0..16).

### 3.2 GML-kind functions (file_kind 2)
* The function `name` maps to the `#define` section with the same name (GML
  records have empty `external_name`; if non-empty, **recommended** use it as the
  section name).
* Executed exactly like a script: arguments in `argument0..argument15` /
  `argument[]`, `self`/`other` are the caller's, the value of `return` (or 0) is the
  result, local `var`s are private (gml-language.md).
* Argument and return types in the record (1 = string, 2 = real) are informational
  for GML functions: no conversion or checking.
* **Recommended: fully supported natively on all platforms.** This covers packages
  written purely in GML (e.g. `GM Room Transitions`).

### 3.3 DLL-kind functions (file_kind 1)
* Equivalent to an implicit `external_define(<file in temp dir>, external_name or
  name, conv, restype, argument_count, arg types…)` performed at load, and
  `external_call` on each call (§5).
* Type codes in the record: **1 = string, 2 = real** (note: differs from `ty_*`).
* Calling convention codes observed: 12 on all DLL functions, 11 and 2 on GML
  functions. **[I]** 12 = stdcall, 11 = cdecl; GML records ignore it (EXT-3).
* Unused argument type slots hold 2 (real).
* Unlike `external_define`, records with more than 4 arguments including strings
  are allowed only if the DLL bridge supports them (gm81-clean's bridge does).

### 3.4 Init and final functions
* **Init:** after all resources and extensions are loaded and before the first
  room is entered (before any Create/Game Start event), call each file's init
  function (if non-empty) with no arguments, in package order then file order.
  The call runs with no current instance (`self` = global/none; **recommended**
  same context as room creation code before instances exist).
* **Final:** when the game ends (after Game End events, before resources are
  freed), call each non-empty final function in **reverse** order (**recommended**).
  Not called on `game_restart` (the game is not unloaded). Called when the game is
  closed through any normal path; not on a crash.
* An init/final name that does not resolve to a function of that file: ignore
  (recommended, with a debug warning).

---

## 4. Extension constants
* Every constant becomes a global read-only GML constant, available in all code.
* The value is stored as a string; **recommended:** treat it like a user-defined
  constant (file-formats §3.2 / IDE constants): parse it as a GML expression and
  evaluate once at load (numbers like `"1"` become reals; quoted strings become
  strings; expressions may reference built-in constants and earlier constants).
  An unparsable value: use the raw string.
* `hidden` constants are usable but not listed in the IDE.

---

## 5. DLL functions in GML `[414_00]`

### 5.1 `external_define(dll,name,calltype,restype,argnumb,arg1type,arg2type,…)`
* `dll`: DLL file name (with or without path). Search: the game's working
  directory, the program directory, the game temp directory, then the system's
  normal library search `[414_00]` ("running directory … or anywhere Windows can
  find it").
* `name`: exported function name (exact).
* `calltype`: `dll_cdecl` or `dll_stdcall`.
* `restype`: `ty_real` or `ty_string`.
* `argnumb`: 0..16; followed by exactly `argnumb` argument types (`ty_real` /
  `ty_string`). With **more than 4 arguments all must be `ty_real`** `[414_00]`.
* Returns an **id** for `external_call` (**recommended** 0, 1, 2, … in definition
  order; never reused).
* Errors (DLL not found, export not found, bad type/argument count): runtime error
  "Error defining an external function." (**recommended** wording); returns −1 if
  errors are ignored.
* Defining the same function twice yields two ids for the same function (allowed).

### 5.2 `external_call(id,arg1,arg2,…)`
* Calls the function with exactly the defined number of arguments, which must have
  the defined types `[414_00]`. Mismatch in count or type: runtime error (no
  conversion).
* Argument passing (Win32 ABI): reals as IEEE-754 `double` (8 bytes on the stack);
  strings as a pointer to a NUL-terminated byte string in the system ANSI code page,
  valid only for the duration of the call.
* Return: `ty_real` → the `double` returned (x87 ST0 on 32-bit); `ty_string` → the
  returned `char*` is copied into a GML string immediately (NULL → `""`). The DLL
  owns the returned memory (the manual's example returns a static buffer).
* Calling convention: `dll_cdecl` = caller cleans the stack, `dll_stdcall` = callee
  cleans. Both are 32-bit x86 conventions; on x64 there is only one convention and
  the flag is ignored.
* Unknown or freed id: runtime error.

### 5.3 `external_free(dll)`
Unloads the named DLL (so the game may delete the file) `[414_00]`; ids defined on it
become invalid. Unknown name: no effect.

### 5.4 Related
* `window_handle()`: the main window's native handle as a real `[414_00]` (HWND on
  Windows; recommended 0 elsewhere).
* `execute_string(str,…)`, `execute_file(fname)`: run GML; the manual suggests DLLs
  return GML code to execute (gml-language.md).
* `get_function_address(function_name)`: returns the address of a runner function
  for DLLs to call directly; documented as unsupported and Delphi-specific
  (`register`/Pascal convention, Delphi strings) `[414_00]`. **gm81-clean: always
  returns 0** (cannot be supported).
* GM 8.1 has no "secure mode" setting; the manual's remark that DLLs are disabled in
  secure mode does not apply.

---

## 6. Calling native DLLs from a 64-bit runner (recommendation)

GM 8.1 DLLs are **32-bit x86** PE files; a 64-bit process cannot load them. Decide
per DLL by reading the PE header machine field (`0x014C` = x86, `0x8664` = x64).

### 6.1 Tiers
1. **Native shims (all platforms, preferred).** A registry keyed by
   `(lower-case DLL file name, export name)` with Rust implementations. Used before
   any real DLL is loaded. Candidates: the three shipped packages' DLLs and
   popular 8.1-era DLLs, implemented clean-room from their user documentation only.
2. **GML-kind extensions**: always native (§3.2).
3. **x64 DLLs**: load in-process (`libloading`) and call via `libffi`.
4. **x86 DLLs on Windows: 32-bit helper process** `gm81-dllhost32.exe` (same Rust
   workspace, target `i686-pc-windows-msvc`), started on the first need and killed
   at game end.
5. Otherwise (x86 DLL on non-Windows, or helper missing): `external_define` fails
   with the error of §5.1.

### 6.2 Helper process protocol
* Transport: the helper's stdin/stdout as binary pipes (or a named pipe). Frames:
  `u32 length; u8 op; payload`, little-endian; strings `u32 len + bytes`.
* Ops: `LOAD {path}` → `{handle | error}`; `DEFINE {handle, symbol, conv, restype,
  argtypes[]}` → `{fn_id | error}`; `CALL {fn_id, args[] (tag+f64 or str)}` →
  `{result value | error}`; `FREE {handle}`; `SET_CWD {path}`; `QUIT`.
* The helper performs calls with `libffi` (x86 `cdecl`/`stdcall` ABIs) or with
  per-arity thunks; it sets its current directory to the runner's
  `working_directory` and runs a Win32 message loop while idle so DLL-created
  windows/dialogs work.
* Calls are synchronous; the runner thread blocks until the reply.
* `window_handle()` values can be passed to the helper (window handles are valid
  system-wide).

### 6.3 Known limitations of the helper approach
DLLs that need to run *inside* the game process cannot work: those that subclass
the game window procedure, access the runner's Direct3D device or memory, or call
back into the runner via `get_function_address`. They fail silently or with errors;
document this to users. A DLL crash kills only the helper: report a runtime error
"external DLL crashed" and treat later calls as errors.

### 6.4 Security
DLLs run arbitrary native code with the user's rights — the same trust as running
the original game executable. The runner should print which DLLs a game loads in
debug mode; no further sandboxing is promised.

---

## 7. Constants

| Constant | Value | Source |
|---|---|---|
| `dll_cdecl` | 0 | [I] (builtins.json) |
| `dll_stdcall` | 1 | [I] |
| `ty_real` | 0 | [I] |
| `ty_string` | 1 | [I] |

`.ged` record codes (not GML constants): file kinds 1 DLL / 2 GML / 3 action lib [I] /
4 other [I]; argument/return types 1 string / 2 real; calling convention 12 [I
stdcall] / 11 [I cdecl] / 2 (GML, meaning unknown).
