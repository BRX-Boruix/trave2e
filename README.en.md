# trave2e

An **acceptance program** for BORUIX verifying a directory traversal permission rule that is easy to overlook.

[简体中文](README.md)

## What it tests

The rule: **if a directory lacks the "enter" permission, files inside it are unreachable even when they are "readable by everyone".**

Reaching a file takes two steps — **enter** each directory along the path, then access the file itself. The first step needs the enter permission on a directory; if some level will not let you in, the later check never happens at all, and the file's own permissive mode is irrelevant.

The program builds two **control fixtures** and tests both under one non-owner identity:

| Fixture | Directory mode | File mode | Expected |
| --- | --- | --- | --- |
| Control | `0755` (enterable) | `0644` | Accessible |
| Subject | `0700` (not enterable) | `0644` | **Refused** |

The only variable is **whether the parent directory has the enter permission**; the file modes are identical. So the subject being refused while the control succeeds proves the refusal comes from the directory check. The error must be "permission denied", not "no such file".

## A design that was overturned

The program was originally designed as: drop privilege, then have the owner adjust permissions, then drop privilege again. **That necessarily fails**, because it runs into an important security property:

**Once privilege is dropped, it cannot be raised again.** A process without sufficient privilege may keep lowering its own, but cannot raise it — otherwise any program could promote itself to administrator. So "drop privilege first, then use high privilege to change something" cannot work. That is not a defect but evidence the permission model is working correctly.

The corrected approach drops privilege once, prepares every fixture beforehand, and then verifies both sides. That also makes the comparison cleaner: the two checks happen under **the same identity, in the same process, at the same moment**.

The program finally **attempts to raise privilege and expects refusal** — an expected refusal, not a failure.

## Exit codes

| Exit code | Meaning |
| --- | --- |
| `0` | Everything passed |
| Non-zero | Number of the first failing step |

## Test fixtures

```
/scratch/trave2e_dir/    # 0755, the control
/scratch/trave2e_nx/     # 0700, the subject
```

> The program finally prints that the identity cannot be restored from that point — that is **by design**, not a fault.

## Building

```bash
cargo build --release
```

Started by the shell builtin `trave2e`.

## Layout

```
trave2e/
├── Cargo.toml    # package definition
├── build.rs      # injects the linker script
├── linker.ld     # user-space section layout
└── src/
    └── main.rs   # fixture construction and both-sided verification
```

## Related projects

- [`libsys`](https://github.com/BRX-Boruix/libsys) — provides identity query, change, and file attribute interfaces
- [`login`](https://github.com/BRX-Boruix/login) — privilege dropping at login
- [`shell`](https://github.com/BRX-Boruix/shell) — provides the command that starts it

## License

MIT License, copyright Yang Borui. See [LICENSE](LICENSE).
