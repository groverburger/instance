# Extensions and DLLs

Instance can be extended in two ways. **Extension packages** add ready-made functions
and constants to a game, written in GML or backed by a native library. **DLLs** can also
be called directly from GML with `external_define` and `external_call`.

## Extension packages

A package is chosen per game in the IDE ([Extension packages](../ide/extensions.md)).
Once a game uses a package, its functions and constants are available everywhere in
the game's code, exactly like built-in ones:

- **Functions** are called by name. Those written in GML behave like scripts — they get
  their arguments in `argument0…`, run with the caller's `self` and `other`, and return
  a value. Those backed by a DLL call into the library.
- **Constants** are evaluated once when the game starts.
- Calling a package function with the wrong number of arguments is a compile error, as
  for built-in functions.
- Functions marked *hidden* in the package are left out of the IDE's lists but can still
  be called.

A package can name an **init function**, called once when the game starts (before the
first room's Create and Game Start events), and a **final function**, called when the
game ends (after the Game End events). They are not called again on `game_restart()`.

Instance looks for installed packages, first match first, in:

1. the `extensions` folder next to the IDE or runner;
2. the folders listed in the environment variable `INSTANCE_EXTENSIONS`;
3. the `extensions` folder in the per-user Instance data folder (where the IDE installs
   packages);
4. the `extensions` folder next to the project.

A standalone executable made with *Create Executable* carries the packages it uses, so
players don't need to install anything.

Packages written entirely in GML work everywhere, including the browser. Packages
that contain a DLL have the same limits as DLLs, below.

## Calling a DLL from GML

[`external_define()`](../reference/dlls.md#external_define) describes a function in a DLL
and returns an id; [`external_call()`](../reference/dlls.md#external_call) calls it.

```gml
// Game Start: define the function once
global.fn_add = external_define("mathlib.dll", "add_numbers",
    dll_cdecl, ty_real, 2, ty_real, ty_real);
```

```gml
// later
result = external_call(global.fn_add, 3, 4);
```

The arguments of `external_define(dll, name, calltype, restype, argcount, argtypes...)`:

- `dll` — the file name. It is looked for in the game's working directory, the program
  directory, the temporary directory, and then wherever the system finds libraries.
- `name` — the exported function's exact name.
- `calltype` — `dll_cdecl` or `dll_stdcall`, the calling convention the DLL uses.
- `restype` — `ty_real` or `ty_string`, what the function returns.
- `argcount` and one type per argument, again `ty_real` or `ty_string`. Up to 16
  arguments, reals and strings in any combination.

If the DLL or the function can't be found, or the types are wrong, `external_define`
reports *"Error defining an external function."*

Reals are passed as 64-bit floating point numbers (`double`) and strings as pointers to
zero-terminated text. A string returned by the DLL is copied immediately.

`external_call` must get exactly the defined number of arguments with the defined
types. [`external_free(dll)`](../reference/dlls.md#external_free) unloads a DLL, after which
its ids can't be used. Define each function once (in a Game Start or Create event), not
every step.

## Which DLLs work

| Platform | What can be loaded |
|---|---|
| Windows | 64-bit DLLs directly. 32-bit DLLs — which most older game DLLs are — run in a small helper program, `instance-dllhost32.exe`, which must be next to the runner (it is included in the Windows release). |
| macOS, Linux | native libraries (`.dylib`, `.so`) only; Windows DLLs can't run there |
| Browser | no DLLs |

Because a 32-bit DLL runs in a separate helper process, DLLs that need to reach *into*
the game — subclassing the game window, drawing with the game's graphics device, or
calling back into the runner — don't work. DLLs that compute something, read files,
open their own windows or talk to hardware usually do. If a DLL crashes, only the
helper stops; later calls report an error.

[`window_handle()`](../reference/dlls.md#window_handle) returns the game window's handle
for DLLs that need it. [`get_function_address()`](../reference/dlls.md#get_function_address)
is not supported and always returns 0.

A DLL runs native code with the player's rights, so only use DLLs you trust.

## Running GML from a DLL

A common technique is for a DLL to return a piece of GML as a string, which the game
then runs with [`execute_string()`](../reference/resources.md#execute_string) — see
[Functions and scripts](../gml/scripts.md#running-code-from-a-string).

See also: [Extension packages in the IDE](../ide/extensions.md) · [DLL functions](../reference/dlls.md) ·
[Functions and scripts](../gml/scripts.md)
