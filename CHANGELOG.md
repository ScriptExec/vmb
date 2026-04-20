# Version 0.3.0

## Features
- Added `log` subcommand output colorization for log flags (e.g., INFO, WARNING, ERROR)
- Added support for negative priority values in `modify` subcommand, allowing mod developers to set lower priority
- Added `.gitignore` file generation to `init` subcommand if the user doesn't specify `--no-git`
- Added `run` subcommand which launches the game and streams the log output
- Added `self update` subcommand that checks for available updates and updates the tool to the latest release

## Changes
- Updated `log` subcommand with the `--watch` option to use an alternate buffer for printing
- Updated `zip` files to change the extension to `.vmz` when installing
- Updated `pack` subcommand to no longer require specifying the input files/directories (defaults to: `./mod.txt` and `./mods`)

# Version 0.2.0
This release introduces several new features and improvements

## Features
- Added `log` subcommand with `--watch` option to monitor the game's log file in real-time, providing mod developers with immediate feedback on their mods' behavior and any potential issues.
- Added `modify` subcommand to allow users to modify the mod's parameters (e.g., name, version, priority) directly from the command line without needing to edit the `mod.txt` file manually.
- Added more options to `init` subcommand

# Version 0.1.1
This release contains a small bug fix for the `install` command, ensuring that the "mods" directory is correctly appended to the installation path when using the `VOSTOK_PATH` environment variable.

## Fixes
- Fixed "mods" dir not being appended to VOSTOK_PATH

# Version 0.1.0
This is the initial release of Vostok Mod Builder, providing basic functionality