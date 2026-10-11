# Files, registry, INI files and programs

Reading and writing files, the registry and INI files, running other programs, and information about the device.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Files](#files)
- [The registry](#the-registry)
- [INI files](#ini-files)
- [Running programs](#running-programs)
- [Device information](#device-information)

## Files

Text and binary files, file names and directories, and the game's parameters and environment.

### file_text_open_read

`file_text_open_read(filename)`

Opens a text file for reading and returns a file id; up to 32 files may be open at once.

### file_text_open_write

`file_text_open_write(filename)`

Opens (creating if needed) a text file for writing and returns its file id.

### file_text_open_append

`file_text_open_append(filename)`

Opens (creating if needed) a text file for appending and returns its file id.

### file_text_close

`file_text_close(file)`

Closes the text file with the given id.

### file_text_write_string

`file_text_write_string(file, str)`

Writes a string to the open text file.

### file_text_write_real

`file_text_write_real(file, value)`

Writes a real number to the text file, always using a dot as decimal separator.

### file_text_writeln

`file_text_writeln(file)`

Writes a newline to the text file.

### file_text_read_string

`file_text_read_string(file)`

Reads and returns a string from the file up to the end of the line.

### file_text_read_real

`file_text_read_real(file)`

Reads and returns a real number from the text file.

### file_text_readln

`file_text_readln(file)`

Skips the rest of the current line and moves to the start of the next.

### file_text_eof

`file_text_eof(file)`

Returns whether the end of the file has been reached.

### file_text_eoln

`file_text_eoln(file)`

Returns whether the end of the current line has been reached.

### file_exists

`file_exists(filename)`

Returns true if a file with the given name exists.

### file_delete

`file_delete(filename)`

Deletes the named file.

### file_rename

`file_rename(oldname, newname)`

Renames a file from oldname to newname.

### file_copy

`file_copy(filename, newname)`

Copies a file to a new name.

### directory_exists

`directory_exists(dirname)`

Returns whether the directory exists; a full path is required.

### directory_create

`directory_create(dirname)`

Creates the directory (full path) if it does not already exist.

### file_find_first

`file_find_first(mask, attr)`

Returns the first file name matching a wildcard mask and extra attribute flags, or an empty string if none.

### file_find_next

`file_find_next()`

Returns the next file matching the earlier mask and attributes, or empty string when none remain.

### file_find_close

`file_find_close()`

Ends a file search, releasing the memory it used; call after processing all files.

### file_attributes

`file_attributes(filename, attr)`

Returns whether the file has all the given fa_ attribute flags.

### filename_name

`filename_name(filename)`

Returns the file name part with extension but without the path.

### filename_path

`filename_path(filename)`

Returns the path part of a file name including the trailing backslash.

### filename_dir

`filename_dir(filename)`

Returns the directory part of a file name, like the path but without the final backslash.

### filename_drive

`filename_drive(filename)`

Returns the drive part of a file name.

### filename_ext

`filename_ext(filename)`

Returns the extension of a file name including the leading dot.

### filename_change_ext

`filename_change_ext(filename, newext)`

Returns the file name with its extension replaced by newext; an empty string removes the extension.

### file_bin_open

`file_bin_open(filename, mode)`

Opens a binary file with mode 0 read, 1 write or 2 read/write, creating it if missing; returns an id (32 max open).

### file_bin_rewrite

`file_bin_rewrite(file)`

Clears the binary file and restarts writing from the beginning.

### file_bin_close

`file_bin_close(file)`

Closes the binary file with the given id.

### file_bin_position

`file_bin_position(file)`

Returns the current byte position in the binary file, starting at 0.

### file_bin_size

`file_bin_size(file)`

Returns the size of the binary file in bytes.

### file_bin_seek

`file_bin_seek(file, pos)`

Moves the current position of the binary file; seek to the size to append.

### file_bin_write_byte

`file_bin_write_byte(file, byte)`

Writes one byte to the binary file.

### file_bin_read_byte

`file_bin_read_byte(file)`

Reads and returns one byte from the binary file.

### export_include_file

`export_include_file(filename)`

Exports the named included file to the working location; the name must be a quoted string.

### export_include_file_location

`export_include_file_location(filename, location)`

Exports the named included file to the given full path and file name.

### discard_include_file

`discard_include_file(filename)`

Discards the named included file from memory.

### parameter_count

`parameter_count()`

Returns the number of command-line parameters passed to the game.

### parameter_string

`parameter_string(n)`

Returns command-line parameter n (1-based); index 0 is the path of the game executable.

### environment_get_variable

`environment_get_variable(name)`

Returns the string value of the named environment variable.

### game_id

*Global variable, read-only.* Unique identifier of the game, handy for making unique file names.

### working_directory

*Global variable, read-only.* Working directory of the game, without a trailing backslash.

### temp_directory

*Global variable, read-only.* Temporary directory for the game, no trailing backslash; contents removed when the game ends.

### program_directory

*Global variable, read-only.* Directory holding the game executable, without trailing backslash; differs from the working directory when testing from the editor.

### disk_free

`disk_free(drive)`

Returns free space in bytes on a drive letter, or the current drive if omitted.

### disk_size

`disk_size(drive)`

Returns total size in bytes of a drive letter, or the current drive if omitted.

### Constants: files

| Constant | Value | Meaning |
| --- | --- | --- |
| `fa_readonly` | 1 | File attribute flag for read-only files. |
| `fa_hidden` | 2 | File attribute flag for hidden files. |
| `fa_sysfile` | 4 | File attribute flag for system files. |
| `fa_volumeid` | 8 | File attribute flag for volume-id entries. |
| `fa_directory` | 16 | File attribute flag for directories. |
| `fa_archive` | 32 | File attribute flag for archive-marked files. |

## The registry

Storing settings in the system registry.

### registry_write_string

`registry_write_string(name, str)`

Writes a string value under the given name in the registry.

### registry_write_real

`registry_write_real(name, value)`

Writes a real value under the given name in the registry.

### registry_read_string

`registry_read_string(name)`

Returns the string stored under the name in the registry, or empty string if missing.

### registry_read_real

`registry_read_real(name)`

Returns the real stored under the name in the registry, or 0 if missing.

### registry_exists

`registry_exists(name)`

Returns whether the named registry entry exists.

### registry_write_string_ext

`registry_write_string_ext(key, name, str)`

Writes a string value under a name inside the specified registry key.

### registry_write_real_ext

`registry_write_real_ext(key, name, value)`

Writes a real value under a name inside the specified registry key.

### registry_read_string_ext

`registry_read_string_ext(key, name)`

Returns the string under a name in the given key, or empty string if missing.

### registry_read_real_ext

`registry_read_real_ext(key, name)`

Returns the real under a name in the given key, or 0 if missing.

### registry_exists_ext

`registry_exists_ext(key, name)`

Returns whether the name exists in the given registry key.

### registry_set_root

`registry_set_root(root)`

Sets which registry root the other registry functions operate on.

## INI files

Storing settings in INI files.

### ini_open

`ini_open(filename)`

Opens the named INI file, which must be in the game folder.

### ini_close

`ini_close()`

Closes the currently open INI file.

### ini_read_string

`ini_read_string(section, key, default)`

Reads a string for the key in the section, returning the default if absent.

### ini_read_real

`ini_read_real(section, key, default)`

Reads a real for the key in the section, returning the default if absent.

### ini_write_string

`ini_write_string(section, key, str)`

Writes a string value for the key in the section.

### ini_write_real

`ini_write_real(section, key, value)`

Writes a real value for the key in the section.

### ini_key_exists

`ini_key_exists(section, key)`

Returns whether the key exists in the section.

### ini_section_exists

`ini_section_exists(section)`

Returns whether the section exists in the INI file.

### ini_key_delete

`ini_key_delete(section, key)`

Deletes the key from the section.

### ini_section_delete

`ini_section_delete(section)`

Deletes the whole section from the INI file.

## Running programs

Starting other programs and opening files with them.

### execute_program

`execute_program(prog, arg, wait)`

Runs an external program with arguments, optionally waiting until it finishes.

### execute_shell

`execute_shell(prog, arg)`

Runs a program or file through the system shell with the given arguments.

### secure_mode

*Global variable, read-only.* Flag telling whether the game runs in secure mode.

## Device information

Which kind of system and device the game runs on.

### os_type

*Global variable, read-only.* Variable giving the platform the game runs on (os_win32, os_macosx, os_ios, etc.).

### os_device

*Global variable, read-only.* Variable further classifying the device, e.g. device_ios_iphone, device_ios_ipad.

### Constants: device information

| Constant | Value | Meaning |
| --- | --- | --- |
| `os_win32` | unknown | Value of os_type for a 32-bit Windows executable. |
| `os_win64` | unknown | Value of os_type for a 64-bit Windows executable. |
| `os_macosx` | unknown | Value of os_type for Mac OS X. |
| `os_psp` | unknown | Value of os_type for PlayStation Portable. |
| `os_ios` | unknown | Value of os_type for Apple iOS devices. |
| `os_android` | unknown | Value of os_type for Android devices. |
| `device_ios_iphone` | unknown | Value of os_device for a non-retina iPhone. |
| `device_ios_iphone_retina` | unknown | Value of os_device for an iPhone with retina display. |
| `device_ios_ipad` | unknown | Value of os_device for an iPad. |
