# Version 0.2.1

## Features
- Added `log` subcommand output colorization for log flags (e.g., INFO, WARNING, ERROR)
- Added support for negative priority values in `modify` subcommand, allowing mod developers to set lower priority
- Added `.gitignore` file generation to `init` subcommand if the user doesn't specify `--no-git`

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