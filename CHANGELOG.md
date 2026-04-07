# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `init` command now shows progress while it creates files and runs final setup commands

## [0.3.0] - 2026-04-06

### Added

- `init` command now supports `mise` as a toolchain manager and adds `--no-toolchain` as an explicit escape hatch

### Changed

- `init` command now only offers installed package managers and toolchain managers
- `init` script template now generates `pack` and `pack:watch` npm scripts instead of `build:pack` and `watch:pack`

### Fixed

- `pack` command now names Rojo project outputs from the project's `name` field instead of the `.project.json` filename; raw `.rbxm` inputs still use the model filename

## [0.2.0] - 2026-04-06

### Changed

- `pack` command now colorizes errors, warnings, timestamps, and error counts when the terminal supports color
- `pack --watch` now inserts a blank line after each watch status line to make rebuild cycles easier to scan
- `init` command now pins generated Rojo toolchain entries to `7.6.1`
- `init` command now runs the selected toolchain manager's install command after writing its config so the scaffolded CLIs are fetched immediately

### Fixed

- `init` command now suppresses package manager and git command chatter on success, and reports subprocess failures through the CLI's styled error output

## [0.1.1] - 2026-04-06

### Changed

- `pack` command now prints a single summary line (`Packed X/Y targets successfully in Xs with N errors.`) instead of verbose per-step log output
- `pack --watch` now shows tsc-style timestamped status lines (`[H:MM:SS AM/PM] Found N errors. Watching for file changes.`) instead of raw log lines

## [0.1.0] - 2026-04-05

### Added

- `pack` command to compile a Roblox/Luau project into a bundled Lua script
  - Accepts a directory, `.rbxm` file, or Rojo `.project.json` as input; defaults to the current directory
  - `-t`/`--target` flag selects one or more build targets (comma-separated); defaults to `dev,rel`
  - Four build targets: `dev` (debug with source maps), `dev-compat` (debug + Lua 5.1 compatibility), `rel` (minified), `rel-compat` (minified + Lua 5.1 compatibility)
  - `-o`/`--out-dir` flag specifies the output directory for generated bundles
  - `--header` flag prepends a custom header file to every bundle output
  - `-w`/`--watch` flag watches for file changes and rebuilds automatically
- `init` command to scaffold a new project in any directory
  - Two templates: `package` (an `@executor-ts/` npm package) and `script` (a standalone Luau script)
  - `-y`/`--yes` skips all interactive prompts and uses defaults
  - `-f`/`--force` allows overwriting existing files or non-empty directories
  - `--name` sets the project name (defaults to the directory name)
  - `--toolchain-manager` configures a Roblox toolchain manager: `rokit` (default), `aftman`, `foreman`, or `none`
  - `--package-manager` selects a Node package manager: `npm` (default), `pnpm`, or `yarn`
  - `--no-git`, `--no-eslint`, `--no-prettier`, `--no-vscode` flags opt out of individual scaffolded features
- `--verbose` global flag for detailed diagnostic output

[Unreleased]: https://github.com/TerminalVibes/rbxex/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/TerminalVibes/rbxex/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/TerminalVibes/rbxex/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/TerminalVibes/rbxex/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/TerminalVibes/rbxex/releases/tag/v0.1.0
