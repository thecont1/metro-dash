# Topcoat dev-loop crash — root cause and fixes

## Symptom

```
$ topcoat dev
  topcoat dev server
  watching for file changes...
  press r to reload

Loaded 443 ridership records
  ready on http://127.0.0.1:3000

  application exited (exit status: 0)

  waiting for changes...
```

The child `target/debug/metro-dash` exits cleanly with status 0 shortly after reporting `ready`. The dev CLI is left in `waiting for changes...` with no port 3000 listener and no child to respawn. Sometimes the child loads the dataset twice before exiting; this is the dev CLI's own restart cycle (see below).

## Root cause

The child exits 0 because that's the Tokio future's normal completion path. In `vendor/topcoat/src/serve.rs`:

```rust
// serve.rs:107-128
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect(...);
    };
    let terminate = async {
        tokio::signal::unix::signal(SIGTERM).expect(...).recv().await;
    };
    tokio::select! { () = ctrl_c => {}, () = terminate => {} }
}
```

`serve_until` returns Ok(()) when `shutdown_signal()` resolves — on **either** SIGINT or SIGTERM. Both signals are wired into the child:

```rust
// app_server.rs:18-26 (CLI)
let child = Command::new(exe)
    .env("TOPCOAT_DEV_URL", dev_url)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .stdin(Stdio::inherit())   // ← shared with dev CLI's tty
    .kill_on_drop(true)
    .spawn()?;
```

`stdin: Stdio::inherit()` puts the child in the dev CLI's foreground process group (`TPGID=<dev CLI pgrp>`). macOS delivers terminal-generated signals to the **whole** foreground process group.

The console crate's keyboard reader at `vendor/topcoat-cli/src/dev/keyboard.rs` uses `libc::raise(SIGINT)` in `console-0.16.3/src/unix_term.rs:358-368` to re-arm SIGINT to itself on `EINTR`. Warp's terminal activity (focus changes, ANSI sequence processing) can fire stray ^C into the foreground group. Each fire causes BOTH the dev CLI's `ctrl_c()` future to resolve AND the child's `ctrl_c()` future to resolve, because they're in the same process group at the kernel level.

When the child's `shutdown_signal()` resolves, `internal_serve` returns Ok(()) → `tokio::start` returns Ok(()) → `main` returns → **clean exit 0**. The dev CLI's handler at `vendor/topcoat-cli/src/dev.rs:145-155` then prints `application exited (exit status: 0)`.

The "double `Loaded`" you see is the dev CLI's restart path at `vendor/topcoat-cli/src/dev.rs:128-136`: when the initial build's `BuildStamp` differs from `last_build` (e.g., a stale `cargo build` from another shell bumped the binary mtime), it kills the old child and spawns a fresh one. Both load cycles completed; both exited 0 on the same SIGINT-foreground-group condition.

## Fixes, ordered by cost/benefit

### 1. Run the binary directly while developing (zero code change)

```sh
cargo build
./target/debug/metro-dash
```

This is what works today and what the production `fly deploy` does. Keep `topcoat dev` only when you actually need hot-reload of the embedded JS or the SSR HTML; for the Rust side, direct invocation has no signal-group issue.

### 2. Detach the dev CLI's child from the foreground process group (1-line patch)

Edit `vendor/topcoat-cli/src/dev/app_server.rs:18`:

```diff
 let child = Command::new(exe)
     .env("TOPCOAT_DEV_URL", dev_url)
+    .process_group(std::process::Command::process_group(0))  // = setsid()
     .stdout(Stdio::inherit())
     .stderr(Stdio::inherit())
     .stdin(Stdio::null())   // do not share tty stdin
     .kill_on_drop(true)
     .spawn()?;
```

or, since `tokio::process::Command` has `process_group(pgid)`:

```diff
 let child = Command::new(exe)
     .env("TOPCOAT_DEV_URL", dev_url)
+    .process_group(0)
     .stdout(Stdio::inherit())
     .stderr(Stdio::inherit())
-    .stdin(Stdio::inherit())
+    .stdin(Stdio::null())   // detach the stdin so the keyboard thread doesn't reach it
     .kill_on_drop(true)
     .spawn()?;
```

After this change, the child runs in its own session leader with no controlling tty, so terminal signals stop reaching it. The dev CLI still catches Ctrl-C via its own `tokio::signal::ctrl_c()` future. **The patched file lives under `vendor/topcoat-cli/`** — you'd need to upstream the change to Topcoat, not maintain a local diff.

### 3. Patch metro-dash to ignore SIGTERM/SIGINT under `topcoat dev` (heuristic, not robust)

In `src/main.rs`, before calling `topcoat::start`, set `signal(SIGTERM, SIG_IGN)` and `signal(SIGINT, SIG_IGN)` when the `TOPCOAT_DEV_URL` env var is set. The dev CLI's `start_app` calls `old.shutdown().await` before spawning the new child, which SIGKILLs if `kill_on_drop` fires; you still keep the explicit lifecycle control. **This branch works in practice but is fragile** — the day Topcoat upgrades its shutdown logic, you may end up with a child that won't die when you expect.

## What's NOT a fix

- **Disabling Topcoat's signal handler at runtime** would require topcoat's internal API for that, which doesn't exist publicly.
- **Running `topcoat dev` in its own tab/process group** (`setsid topcoat dev` from outside) **does work** but is fragile — depends on the shell session you ran it from, and the user's IDE integration may re-attach.
- **A wrapper script** that intercepts Ctrl-C and forwards only to a chosen pid is workable but adds another layer. Not worth it for a dev-only tool.

## Recommendation

Use option 1 for routine development. Apply option 2 in a fork if the dev-loop symptom becomes a daily annoyance, and upstream the change to Topcoat. Don't ship option 3 to a production binary.
