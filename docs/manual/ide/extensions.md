# Extension packages

An **extension package** adds new functions and constants to GML: for example extra
dialog boxes, printing, or room transition effects. Packages are installed once on
your computer and then switched on per project.

A package consists of a `.ged` file (its description), a `.dat` file (its contents)
and, optionally, a `.chm` help file, all with the same name.

Open the **Extension Packages** form by double-clicking **Extension Packages** in the
resource tree, with **Resources > Select Extension Packages**, or with the toolbar
button.

## Choosing packages for a project

The form has two lists:

- **Used packages**: the packages this project uses, in order.
- **Available packages**: installed packages the project doesn't use yet.

To use a package, select it under Available packages and click the left arrow, or
double-click it. To stop using one, select it under Used packages and click the right
arrow, or double-click it. Press **OK** to apply the change to the project, or
**Cancel** to keep the old list.

A used package that isn't installed on this computer stays in the list, shown in red
as *(not installed)*. The project still remembers it, so it works again on a
computer where the package is installed.

## Package information

Select a package in either list to see its details in the **Information** box: the
version, author, date, license, which other files it uses, the folder it was found
in, and its description, followed by all of its **functions** (with their arguments;
hover over one for its help text) and **constants**. Problems with the package, such as
a missing `.dat` file, are reported there too.

**Help** opens the package's help file, if it has one.

## Installing and uninstalling

| Button | What it does |
|---|---|
| Install… | Choose a package's `.ged` file. Its `.dat` (and `.chm`, if present) must be in the same folder with the same name, or you can select them together. The package is checked and copied into your personal extensions folder, and then appears under Available packages. |
| Uninstall | Removes the selected package from your personal extensions folder, after asking. Packages found elsewhere (next to Instance or next to the project) can't be uninstalled here. |

Install and Uninstall change the files on disk immediately; pressing Cancel doesn't
undo them.

Only `.ged` packages can be installed. The compressed `.gex` package format isn't
supported.

## Where Instance looks for packages

Packages are found in these folders, in this order. If two folders hold a package with
the same name, the first one wins.

1. The `extensions` folder next to the IDE (or runner).
2. The folders listed in the `INSTANCE_EXTENSIONS` environment variable.
3. The `extensions` folder in your personal settings folder (where **Install** puts
   packages).
4. The `extensions` folder next to the project file.

Putting a project's packages in an `extensions` folder beside it is a good way to
keep them together, for example in version control.

## Using a package

Once a project uses a package, call its functions from code like built-in functions,
and use its constants. Packages can also run setup code when the game starts.
Packages that contain DLLs only work on systems where those DLLs can be loaded; see
[Extensions and DLLs](../runtime/extensions-and-dlls.md).

When you [create an executable](executables.md), the used packages are built into it,
so players don't need to install anything.

> **Note:** Actions that a package may define are not added to the action palette.
> Use the package's functions in **Execute Code** instead.

See also: [Extensions and DLLs](../runtime/extensions-and-dlls.md) · [DLL functions](../reference/dlls.md) · [Creating executables](executables.md)
