# Executable format (Instance's own; not YoYo's)

A standalone game made by Instance's "Create Executable" is:

    [ verbatim copy of instance-runner.exe ]
    [ payload: the project serialised with instance-format as v810 .gm81 bytes ]
    [ footer, 24 bytes:
        u8[8]  magic  = "INSTNC\0\x01"   (49 4E 53 54 4E 43 00 01)
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

## Extension packages in standalone executables (resolves EXT-2)

When the project references extension packages, the payload is followed by an
extension bundle, and the footer magic becomes "INSTNC\0\x02" with an extra field:

    [ runner ][ project payload ][ bundle ][ footer, 40 bytes:
        u8[8] magic = "INSTNC\0\x02"
        u64 LE payload_offset, u64 LE payload_length
        u64 LE bundle_offset,  u64 LE bundle_length ]

bundle = u32 LE count, then per package: u32 LE name length + UTF-8 name,
u32 LE .ged length + bytes, u32 LE .dat length + bytes (the package files as installed).
The runner loads bundled packages before searching the extension search path.
Version 1 footers (24 bytes, no bundle) remain valid.

## Footers of earlier builds

Builds made before the product was named Instance wrote the same layout with the
magics "GM81CLN\x01" (v1, 24-byte footer) and "GM81CLN\x02" (v2, 40-byte footer).
Readers (the runner at start-up, the extension-bundle loader, the IDE's check that the
runner carries no game) accept both the current and these old magics; writers always
use "INSTNC\0\x01" / "INSTNC\0\x02".
