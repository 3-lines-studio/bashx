# bashx

Run non-interactive Bash commands through the `ax` Unix tool convention.

```sh
bashx describe
printf '%s\n' '{"command":"printf hello"}' | bashx run bash
```

Commands run with host filesystem and network access. bashx runs them in a minimal clean environment (`PATH`, `HOME`, `LANG`, `TERM`) rather than inheriting the launching shell, so credentials exposed to `ax` are not visible to the model. It does not load Bash profiles or accept interactive input.

Commands time out after 120 seconds by default (max 600 seconds); the timeout kills the command process group. Combined stdout/stderr is capped at the last 16 KiB.

bashx uses `$BOT_ROOT/workspace` when run inside a bot, falling back to `$BOT_ROOT`, then the current directory (or a `workspace/` directory present there) when standalone:

```sh
AX_TOOLS=bashx ax
```

The working directory is not a filesystem boundary — commands can read and change files elsewhere when OS permissions allow. Isolation is the caller's job: run bashx inside a container or VM when you need it, with minimal filesystem and network access.

## Build and install

```sh
go build -trimpath -ldflags='-s -w' -o bashx .
curl -fsSL https://ax.3lines.studio/install.sh | sh -s -- bashx
```
