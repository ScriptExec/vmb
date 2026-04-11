# Vostok Mod Builder
This tool provides a simple CLI for mod developers, used to package and install mods for [Road to Vostok](https://store.steampowered.com/app/1963610/Road_to_Vostok/).

> [!TIP]
> If you are new to modding consider visiting the [Vostok Modding Wiki](https://github.com/ametrocavich/vostok-modding-wiki/wiki) for guides and resources to get started.

### This tool supports:
- Initializing a directory with mod boilerplate files
- Packing files into a `.vmz` archive
- Installing mods from either a .`zip` archive or a `.vmz` archive or a directory
- Modifying mod parameters (e.g., name, version, priority)
- Viewing the latest output log from the game (if available)

## Overview
```shell
Usage:
    vmb <COMMAND>

Commands:
  init     Initialize the given path with mod boilerplate
  modify   Modify parameters of the mod
  pack     Package one or more files/directories into a .vmz archive
  install  Install a [.zip|.vmz] archive or a mod root directory into an auto-detected or provided directory
  log      Displays the latest output log (if available)
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

For more help on a specific command, run `vmb <COMMAND> --help` or `vmb help <COMMAND>`.

## Install path resolution

`install` subcommand uses this path resolution order:

1. `VOSTOK_PATH` (expects the game installation path; installs to `<VOSTOK_PATH>/mods`)
1. Optional `path` argument
1. On Windows, `C:\Program Files (x86)\Steam\steamapps\common\Road to Vostok\` when present
1. On Linux, `~/.steam/steam/steamapps/common/Road to Vostok/` when present

## Examples
```shell
# Initialize a mod directory with git repository
vmb init "My First Mod"
cd "My First Mod"

# Package the mod directory into a .vmz archive
vmb pack -o MyFirstMod.vmz ./mods ./mod.txt
# Modify the mod's parameters (e.g., name, version, priority)
vmb modify -n "My First Custom Mod" -i "my-first-mod-id" -p 10 -v 1.0.0 -u 12345
# Install the mod from the .vmz archive to the detected mods directory
vmb install MyFirstMod.vmz
# Alternatively, install the mod directly from the mod directory
vmb install .
```

## Example
```shell
# Initialize a mod directory with git repository
vmb init "My First Mod"
cd "My First Mod"

# Package the mod directory into a .vmz archive
vmb pack -o MyFirstMod.vmz ./mods ./mod.txt
# Modify the mod's parameters (e.g., name, version, priority)
vmb modify -n "My First Custom Mod" -i "my-first-mod-id" -p 10 -v 1.0.0 -u 12345
# Install the mod from the .vmz archive to the detected mods directory
vmb install MyFirstMod.vmz
# Alternatively, install the mod directly from the mod directory
vmb install .
```

## Building

To build the tool and install it globally, run:
```shell
cargo install --path <PATH_TO_INSTALL>
```
if you are in the root directory of the project, you can run:
```shell
cargo install --path .
```

