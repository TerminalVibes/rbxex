# rbxex

A bundler and scaffolding tool for Roblox/Luau projects.

## Installation

**Download a prebuilt binary** from the [Releases](https://github.com/TerminalVibes/rbxex/releases) page and place it on your `PATH`.

**Or build from source:**
```sh
cargo install --git https://github.com/TerminalVibes/rbxex
```

## Commands

### `rbxex pack`

Compiles a Roblox project into a bundled Lua script.

```sh
rbxex pack                        # pack default.project.json in current dir
rbxex pack path/to/project        # pack a specific .project.json or .rbxm
rbxex pack -o dist                # write output to dist/
rbxex pack -t rel                 # release target (minified)
rbxex pack -w                     # watch mode
```

**Targets:** `dev` (default), `dev-compat`, `rel`, `rel-compat`

### `rbxex init`

Scaffolds a new project in the current directory (or a given path).

```sh
rbxex init                        # interactive prompts
rbxex init my-project --yes       # non-interactive, all defaults
rbxex init --template package     # package template
rbxex init --no-eslint --no-git   # skip specific features
```

## License

MIT
