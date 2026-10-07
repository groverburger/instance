# Executable format (gm81-clean's own; not YoYo's)

A standalone game made by gm81-clean's "Create Executable" is:

    [ verbatim copy of gm-runner.exe ]
    [ payload: the project serialised with gm-format as v810 .gm81 bytes ]
    [ footer, 24 bytes:
        u8[8]  magic  = "GM81CLN\x01"
        u64 LE payload_offset   (absolute file offset of the payload)
        u64 LE payload_length ]

Runner start-up: if no project path is given on the command line, the runner opens
its own executable (`std::env::current_exe()`), reads the last 24 bytes, and if the
magic matches, loads the payload as the game. Command-line project paths still win.
The window caption and icon come from the project's settings as usual (setting the
PE icon resource is optional, P2).

Loading executables compiled by YoYo's GameMaker 8.1 is a separate, future feature:
its embedded-data layout must be derived by black-box comparison of (.gm81, .exe)
pairs compiled by the user, never by examining the runner's code.
