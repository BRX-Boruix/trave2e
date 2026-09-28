# trave2e

A BORUIX acceptance test for directory traversal permission: without the execute bit, files under a directory are unreachable.

[简体中文](README.md)

Spawned by a shell builtin; not used standalone.

## What it tests

The program first prepares two control directories as the owner: one with the execute bit (0755),
one without (0700), both holding a 0644 file. It then downgrades to an ordinary user and checks:

- The ordinary user can read the file under the execute-permitted directory — the control holds
- Reading the same file under the no-execute directory is denied with a permission error
- Both checks run at the same moment under the same identity; the directory execute bit is the only variable

The criterion: **without the execute bit, a file under a directory is unreachable even at 0644**.

The program finally attempts to raise its own identity and expects refusal — identity can only be lowered, never raised.

## Exit codes

- `0` — all passed
- non-zero — the number of the failed check, matching the `FAIL(n)` position; number ranges distinguish the owner side from the downgraded side

## Building

```bash
cargo build --release
```

## Repository layout

```
trave2e/
├── Cargo.toml    # package manifest
├── build.rs      # injects the linker script
├── linker.ld     # user-space segment layout
└── src/
    └── main.rs   # fixture setup and downgraded checks
```

## Related projects

- [`libsys`](https://github.com/BRX-Boruix/libsys) — identity switching and file status interfaces
- [`shell`](https://github.com/BRX-Boruix/shell) — the coordinating builtin that spawns this program

## License

MIT License, copyright Yang Borui. See [LICENSE](LICENSE).
