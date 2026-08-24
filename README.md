# Bashx

Bashx runs non-interactive Bash commands in a Birdcage sandbox and implements the Unix tool convention.

```sh
bashx describe
printf '%s\n' '{"command":"printf hello"}' | bashx run bash
```

The sandbox exposes the selected workspace as writable, system programs and libraries as read-only, a minimal environment, and no network. Host home files and credentials outside the workspace are hidden. Commands time out after 120 seconds by default and cannot request more than 600 seconds. Bashx fails if the sandbox cannot start.

Bashx uses the current directory as its workspace. Set `BASHX_WORKSPACE` to override it:

```sh
AX_TOOLS=bashx ax
```

The model calls the tool named `bash`. AX has no built-in fallback.

Bashx supports Linux and macOS through Birdcage. Sandbox behavior depends on platform kernel facilities. It is not a virtual machine and does not defend against kernel vulnerabilities.

## Build

```sh
cargo build --release
```

## Install

```sh
curl -fsSL https://ax.3lines.studio/install.sh | sh -s -- bashx
```
