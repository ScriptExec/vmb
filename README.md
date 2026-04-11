# Vostok Mod Builder
This tool provides a simple CLI for mod developers, used to package and install mods for [Road to Vostok](https://store.steampowered.com/app/1963610/Road_to_Vostok/).

### This tool supports:
- Initializing a directory with mod boilerplate files
- Packing files into a `.vmz` archive
- Installing mods from either a .`zip` archive or a `.vmz` archive or a directory

## Overview
```shell
Usage:
    vmb <COMMAND>

Commands:
  init     Initialize the given path with mod boilerplate
  pack     Package one or more files/directories into a .vmz archive
  install  Install a [.zip|.vmz] archive or a mod root directory into an auto-detected or provided directory
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Command Usage

- Use `vmb init [--no-git] [--update-id=1234] ./mod/path ` to create boilerplate files such as: `mod.txt` and `mods/<MOD_NAME>/Main.gd` from embedded templates, optionally skipping git repository intitialization.
- Use `vmb pack -o "./path/output.vmz" file1 file2 ...` to package files/directories into a `.vmz` zip archive.
- Use `vmb install [source] [path]` to copy an archive or mod root directory (source) to the detected mods directory; mod roots are packed to a temporary `.vmz` first. `[source]` defaults to `.` and `[path]` is used if the default install path cannot be detected or is not desired.

For more help on a specific command, run `vmb <COMMAND> --help` or `vmb help <COMMAND>`.

## Install path resolution

`install` subcommand uses this path resolution order:

1. `VOSTOK_PATH` (expects the game installation path; installs to `<VOSTOK_PATH>/mods`)
1. Optional `path` argument
1. On Windows, `C:\Program Files (x86)\Steam\steamapps\common\Road to Vostok\` when present

## Building

To build the tool and install it globally, run:
```shell
cargo install --path <PATH_TO_INSTALL>
```
if you are in the root directory of the project, you can run:
```shell
cargo install --path .
```

