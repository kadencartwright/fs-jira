# fs-jira

Rust bootstrap for a read-only FUSE filesystem.

## Prerequisites

- Linux with FUSE support
- Rust toolchain (`rustup`, `cargo`)
- FUSE userspace library headers (for example, `libfuse3-dev` on Debian/Ubuntu)

## Build

```bash
cargo build
```

## Mount

Create a mountpoint and run:

```bash
mkdir -p /tmp/fs-jira-mnt
cargo run -- /tmp/fs-jira-mnt
```

In another terminal:

```bash
ls -la /tmp/fs-jira-mnt
cat /tmp/fs-jira-mnt/test.md
```

Expected output for `cat`:

```text
Hello World!
```

The filesystem is mounted read-only. Writes should fail.

## Unmount

```bash
fusermount3 -u /tmp/fs-jira-mnt
```

If your distro provides `fusermount` instead of `fusermount3`, use that command.
