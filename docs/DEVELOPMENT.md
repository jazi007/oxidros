# Development Guide

## Prerequisites

- [Nix](https://nixos.org/download/) with flakes enabled
- (Optional) [direnv](https://direnv.net/) for automatic shell activation

Enable flakes in `~/.config/nix/nix.conf` or `/etc/nix/nix.conf`:

```
experimental-features = nix-flakes nix-command
```

## Available Dev Shells

Shells follow the pattern `ros-<distro>-<variant>`:

| Shell | ROS distro | Contents |
|---|---|---|
| `ros-jazzy-full` | Jazzy (LTS) | RCL + msgs + CLI + Zenoh RMW + test packages |
| `ros-jazzy-minimal` | Jazzy (LTS) | RCL + headers/libs to build oxidros-rcl only |
| `ros-humble-full` | Humble (LTS) | same as jazzy-full |
| `ros-humble-minimal` | Humble (LTS) | same as jazzy-minimal |
| `ros-kilted-full` | Kilted | same as jazzy-full |
| `ros-kilted-minimal` | Kilted | same as jazzy-minimal |
| `ros-lyrical-full` | Lyrical | same as jazzy-full |
| `ros-lyrical-minimal` | Lyrical | same as jazzy-minimal |

`default` → `ros-jazzy-full`.

> **Note on lyrical**: It was released May 2026. If `nix develop .#ros-lyrical-full` fails,
> run `nix flake update` to pull the latest `nix-ros-overlay`.

## Quick Start

### Option A — direnv (recommended)

```sh
# First time only
direnv allow

# From then on, cd into the repo and the shell activates automatically.
```

### Option B — manual `nix develop`

```sh
# Default (jazzy, full)
nix develop

# Specific distro and variant
nix develop .#ros-humble-full
nix develop .#ros-kilted-minimal
```

## Building

```sh
# Auto-detects backend from ROS_DISTRO (rcl when set, zenoh otherwise)
just test

# Explicit backend
cargo build --workspace --no-default-features --features rcl
cargo build --workspace --exclude oxidros-wrapper --exclude oxidros-rcl --no-default-features --features zenoh
```

## Running Tests

```sh
just test

# Or directly with nextest
cargo nextest run --workspace --no-default-features --features rcl
```

## Switching Distros

```sh
# Exit current shell, enter a different one
nix develop .#ros-humble-full
```

With direnv, edit `.envrc` and change the shell name, then run `direnv allow`.

## Binary Cache

The flake uses `ros.cachix.org` to avoid rebuilding ROS packages from source.
Add your machine as a trusted user, or run:

```sh
nix-env -iA cachix -f https://cachix.org/api/v1/install
cachix use ros
```

## Environment Variables Set by the Shell

| Variable | Value |
|---|---|
| `ROS_DISTRO` | e.g. `jazzy` |
| `ROS_VERSION` | `2` |
| `AMENT_PREFIX_PATH` | Nix store ROS env (appended, not prepended) |
| `LD_LIBRARY_PATH` | ROS libs appended |
| `LIBCLANG_PATH` | Required by `oxidros-build` / bindgen |
| `CLANG_PATH` | Required by `oxidros-build` / bindgen |
| `RUSTC_WRAPPER` | `sccache` for incremental compilation caching |
| `RUST_BACKTRACE` | `1` |

Path variables are **appended** (suffix logic): if you source a local ROS workspace
`setup.bash`, its paths stay at the front and take precedence over the Nix store paths.
