# trave2e

**简体中文** | [English](#english)

BORUIX 的**目录遍历权限验收**程序——验证一条容易被忽视但至关重要的访问控制规则。

规则是：**一个目录如果没有"可进入"权限，那么它里面的文件即使是"人人可读"的，也访问不到。**

```
[trave2e] owner traversal OK (file is 0644)
[trave2e] dir(0755): non-owner stat succeeds (control) OK
[trave2e] nx(0700): non-owner stat -> EACCES OK (same uid, same moment)
```

---

## 这条规则是什么

文件权限通常被理解成"这个文件谁能读、谁能写"。但访问一个文件其实是**两步**：

1. 沿着路径**逐级进入**每一层目录
2. 最后访问文件本身

第一步需要目录上的"进入"权限。如果中间某一层目录不让你进，那么**后面的检查根本轮不到**
——文件本身权限再宽松也没用。

```
/scratch/trave2e_nx/    ← 权限 0700，别人进不去
    └── f.txt            ← 权限 0644，人人可读
```

上例中 `f.txt` 是"人人可读"的，但对非属主来说它**完全不可达**。这符合 POSIX 语义，也是直觉上
容易忽略的一条——很多人会以为"文件是 0644，那就谁都能读"。

## 为什么这个测试重要

这条规则是**访问控制的最后一道闸**。如果实现漏掉了目录的可进入检查，那么：

- 用户把自己的目录设成"仅自己可进入"，以为里面的东西别人看不到
- 但实际上别人可以直接按路径去读里面那些"人人可读"的文件

这等于**私有目录形同虚设**——用户以为自己关上了门，其实门没关。

## 怎么验证

程序构造两个**对照夹具**，然后用同一个非属主身份分别测试：

| 夹具 | 目录权限 | 文件权限 | 期望结果 |
| --- | --- | --- | --- |
| **对照组** | 0755（可进入） | 0644 | 能访问 |
| **实验组** | 0700（不可进入） | 0644 | **被拒绝** |

两组唯一的变量就是**父目录有没有"可进入"权限**，文件权限完全相同。所以如果实验组被拒绝而对照组
成功，就证明拒绝确实来自目录检查，而不是别的原因。

**要求返回的错误必须是"权限不足"**，而不是别的错误。如果返回"文件不存在"，说明实现根本没走到
文件那一层——那也是一种错误，只是错法不同。

## 一个被推翻的设计，以及它揭示的安全属性

这个程序最初的设计是：降权 → 由属主修改权限 → 再降权验证。**这个设计必然失败**，而失败的
原因本身是一个重要的安全属性。

**一旦降低了权限，就再也提不回来。** 一个没有足够权限的进程可以继续往下降低自己的权限，但
**不能往上提**——否则权限模型就形同虚设，任何程序都能自我提升为管理员。

所以"先降权，再以高权限去改东西"这条路**走不通**。这不是缺陷，而是**内核正确工作的证据**。

改正后的做法更简单也更好：**只降权一次**，降权之前把所有夹具都准备好，之后同时验证两侧。

这样反而让对照更干净——两次检查发生在**同一身份、同一进程、同一时刻**，唯一变量就是
"父目录有没有可进入权限"。

## 程序还验证了"不能提权"这件事

除了遍历规则，程序最后还**主动尝试提权并期望被拒绝**。

这是一个**预期内的拒绝**，不是失败。如果提权竟然成功了，那才是真正的失败——说明权限模型存在
严重漏洞。

所以这个程序验证的是**两个方向**：该拒绝的要拒绝（遍历），该拦住的要求拦住（提权）。

## 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 全部通过 |
| 非零 | 首个失败项的编号，可直接定位到具体哪一步 |

失败时也会打印实际观察到的值（比如收到的错误码），便于判断是"完全放行"还是"拒绝了但理由不对"。

**注意**：程序打印的最后一行会说明"身份无法从此处恢复"——这是**设计如此**，不是故障。见上文。

## 测试夹具

程序在 `/scratch/` 下创建临时目录与文件进行测试：

```
/scratch/trave2e_dir/    # 0755，对照组
/scratch/trave2e_nx/     # 0700，实验组
```

## 构建

```bash
cargo build --release
```

编译产物部署为 BORUIX 系统中的用户态程序，由 shell 内建命令拉起。

## 文件结构

```
trave2e/
├── Cargo.toml    # 包定义
├── build.rs      # 注入链接脚本
├── linker.ld     # 用户态段布局
└── src/
    └── main.rs   # 夹具构造与两侧验证
```

## 相关项目

- [`libsys`](https://github.com/BRX-Boruix/libsys) —— 提供身份查询、变更与文件属性接口
- [`shell`](https://github.com/BRX-Boruix/shell) —— 提供拉起本程序的命令
- [`login`](https://github.com/BRX-Boruix/login) —— 登录时的权限降低

## 许可

MIT License，版权归 Yang Borui 所有。详见 [LICENSE](LICENSE)。

---

# English

[简体中文](#trave2e) | **English**

A **directory traversal permission acceptance** program for BORUIX — it verifies an easily
overlooked but critical access control rule.

The rule: **if a directory lacks the "enter" permission, files inside it are unreachable even when
they are "readable by everyone".**

```
[trave2e] owner traversal OK (file is 0644)
[trave2e] dir(0755): non-owner stat succeeds (control) OK
[trave2e] nx(0700): non-owner stat -> EACCES OK (same uid, same moment)
```

---

## What the rule is

File permissions are usually understood as "who may read or write this file". But reaching a file
takes **two steps**:

1. **Enter** each directory along the path, one level at a time
2. Then access the file itself

Step one needs the "enter" permission on a directory. If some level along the way will not let you
in, the later check **never happens at all** — the file's own permissive mode is irrelevant.

```
/scratch/trave2e_nx/    <- mode 0700, others cannot enter
    └── f.txt            <- mode 0644, readable by everyone
```

In the example above `f.txt` is "readable by everyone", yet it is **entirely unreachable** for a
non-owner. That matches POSIX semantics and is easy to overlook — many assume "the file is 0644, so
anyone can read it".

## Why this test matters

The rule is the **last gate of access control**. An implementation that skipped the directory enter
check would mean:

- A user sets a directory to "only I may enter", believing its contents are hidden
- But others could walk straight down the path and read the "readable by everyone" files inside

That makes **a private directory a fiction** — the user believes the door is shut when it is not.

## How it is verified

The program builds two **control fixtures**, then tests both under one non-owner identity:

| Fixture | Directory mode | File mode | Expected |
| --- | --- | --- | --- |
| **Control** | 0755 (enterable) | 0644 | Accessible |
| **Subject** | 0700 (not enterable) | 0644 | **Refused** |

The only variable between them is **whether the parent directory has the enter permission**; the file
modes are identical. So if the subject is refused while the control succeeds, the refusal is proven to
come from the directory check and not something else.

**The error must be "permission denied"**, not some other error. Were it "no such file", the
implementation never reached the file at all — also wrong, but wrong in a different way.

## A design that was overturned, and the security property it revealed

The program was originally designed as: drop privilege, then have the owner adjust permissions, then
drop privilege again. **That design necessarily fails**, and the reason is itself an important security
property.

**Once privilege is dropped, it cannot be raised again.** A process without sufficient privilege may
keep lowering its own privilege, but **cannot raise it** — otherwise the permission model would be a
fiction and any program could promote itself to administrator.

So "drop privilege first, then use high privilege to change something" **cannot work**. That is not a
defect but **evidence the kernel is doing the right thing**.

The corrected approach is simpler and better: **drop privilege once**, prepare every fixture before
the drop, and then verify both sides afterwards.

It also makes the comparison cleaner — the two checks happen under **the same identity, in the same
process, at the same moment**, with the parent directory's enter permission as the sole variable.

## The program also verifies that privilege cannot be raised

Beyond the traversal rule, the program finally **attempts to raise privilege and expects refusal**.

That is an **expected refusal**, not a failure. If raising it succeeded, *that* would be the real
failure — evidence of a severe hole in the permission model.

So the program verifies **both directions**: what should be refused is refused (traversal), and what
should be blocked is blocked (privilege raising).

## Exit codes

| Exit code | Meaning |
| --- | --- |
| `0` | Everything passed |
| Non-zero | The number of the first failing step, pinpointing exactly where it failed |

On failure it also prints the value it actually observed (the error code received, say), distinguishing
"let through entirely" from "refused, but for the wrong reason".

**Note**: the program's last line states that the identity cannot be restored from that point — that is
**by design**, not a fault. See above.

## Test fixtures

The program creates temporary directories and files under `/scratch/`:

```
/scratch/trave2e_dir/    # 0755, the control
/scratch/trave2e_nx/     # 0700, the subject
```

## Building

```bash
cargo build --release
```

The artifact is deployed as a user-space program in a BORUIX system, started by a shell builtin.

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
- [`shell`](https://github.com/BRX-Boruix/shell) — provides the command that starts this program
- [`login`](https://github.com/BRX-Boruix/login) — privilege dropping at login

## License

MIT License, copyright Yang Borui. See [LICENSE](LICENSE).
