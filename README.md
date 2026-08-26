# Bashx

<p align="center"><img src=".github/ax.svg" width="96" height="96" alt="AX ecosystem"></p>

Bashx runs non-interactive Bash commands and implements the Unix tool convention.

```sh
bashx describe
printf '%s\n' '{"command":"printf hello"}' | bashx run bash
```

Commands run with host filesystem and network access. Bashx inherits the environment it was launched with, including credentials visible to it. It does not load Bash profiles or accept interactive input.

Commands time out after 120 seconds by default and cannot request more than 600 seconds. Timeout kills the command process group. Combined stdout and stderr output is limited to the last 16 KiB.

Bashx uses the current directory as its working directory. Set `BASHX_WORKSPACE` to override it:

```sh
AX_TOOLS=bashx ax
```

The working directory is not a filesystem boundary. Commands can read and change files elsewhere when OS permissions allow it. Security is the caller's job: run Bashx inside a container or VM when you need isolation, and launch it from an environment whose variables you are comfortable exposing to the model.

The model calls the tool named `bash`. AX has no built-in fallback.

## Build

```sh
cargo build --release
```

## Install

```sh
curl -fsSL https://ax.3lines.studio/install.sh | sh -s -- bashx
```
