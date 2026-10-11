# DLLs

Calling functions in external DLLs.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Calling DLL functions](#calling-dll-functions)

## Calling DLL functions

Defining functions in external DLLs and calling them.

### external_call

`external_call(id, arg1, arg2, ...)`

Calls a previously defined external function by id with matching real or string arguments and returns its result.

### external_define

`external_define(dll, name, calltype, restype, arg_count, arg1type, arg2type, ...)`

Defines an external DLL function: DLL name, function name, calling convention, result type, argument count (0-16) and argument types; returns an id.

### external_free

`external_free(dllname)`

Unloads the named DLL so it can be removed; best done at game end.

### get_function_address

`get_function_address(function_name)`

Returns the address of a built-in function so that DLLs can call it directly.

### window_handle

`window_handle()`

Returns the window handle of the game's main window.

### Constants: calling dll functions

| Constant | Value | Meaning |
| --- | --- | --- |
| `ty_real` | 0 | Argument/result type constant marking a real-number value in external_define. |
| `ty_string` | 1 | Argument/result type constant marking a string value in external_define. |
| `dll_cdecl` | 0 | Calling-convention constant for external_define selecting cdecl. |
| `dll_stdcall` | 1 | Calling-convention constant for external_define selecting stdcall. |
