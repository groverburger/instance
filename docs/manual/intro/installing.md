# Installing and running Instance

Instance runs on Windows, macOS and Linux, and in a web browser. There is no
installer: unpack a release and start the IDE.

## Downloading a release

Release builds are published on the project's releases page:

| Download | Contents |
|---|---|
| `Instance-<version>-windows-x64.zip` | The IDE (`instance-ide.exe`), the runner (`instance-runner.exe`), command-line tools, the helper for 32-bit DLLs and a SoundFont for MIDI music |
| `Instance-<version>-macos-arm64.zip`, `-macos-x64.zip` | `Instance.app` (the IDE, with the runner inside) |
| `Instance-Runner-<version>-macos-….zip` | `Instance Runner.app`, the runner on its own |
| `Instance-<version>-web.zip` | The browser version, for hosting on your own web server |

### Windows

Unzip the archive into a folder of your choice and double-click `instance-ide.exe`.
Keep `instance-runner.exe` in the same folder: the IDE looks for it there when you run
a game or create an executable.

### macOS

Unzip and move `Instance.app` to your Applications folder. The builds are not signed
by a registered developer, so the first time you open it, right-click the app and
choose **Open**, then confirm.

### Linux

Instance is built from source on Linux (see below). You need the ALSA, udev,
X11/Wayland and OpenGL development packages installed first.

## Using the browser version

The IDE also runs in a web browser, with nothing to install. It can do almost
everything the desktop version can; [Instance in the browser](../ide/web.md) lists the
differences (for example, you save by downloading the project file, and you can't
create executables).

## Building from source

If you have [Rust](https://rustup.rs) installed, you can build Instance yourself:

```sh
git clone https://github.com/groverburger/instance
cd instance
cargo build --release
```

The programs end up in `target/release/`. Start the IDE with
`target/release/instance-ide`. When the IDE runs from a build folder like this, it
finds the runner in the same folder automatically.

## Starting the IDE

Start the IDE on its own to begin with an empty project, or give it a project to open:

```sh
instance-ide mygame.gm81
instance-ide mygame/          # a project folder
```

You can also drag a `.gm81` or `.gmk` file onto the IDE window to open it.

## If the IDE can't find the runner

Running a game needs the runner program. If the IDE reports that it can't find it,
open **File > Preferences…** and enter the full path of `instance-runner` (or
`instance-runner.exe`) in **Runner executable**, or pick it with the **…** button.
Leave the field empty to let the IDE search next to itself. The line under the field
shows which runner it found.

## Where Instance keeps its settings

Your preferences, the list of recent files and installed extension packages are
stored per user, not in your projects:

| System | Folder |
|---|---|
| Windows | `%APPDATA%\Instance` |
| macOS | `~/Library/Application Support/Instance` |
| Linux | `~/.config/Instance` |

Games that save data with the registry or highscore functions also keep it in this
folder, one entry per game.

See also: [A tour of the IDE](tour.md) · [Instance in the browser](../ide/web.md) · [Running and debugging games](../ide/running-and-debugging.md)
