//! BORUIX \`trave2e\`：目录遍历权限（A2-3 / ADR-040 §3.4）的**真实用户态**验收。
//!
//! 由 shell 内建命令 \`trave2e\` 派生运行。与内核停机测试 \`[test-traverse]\` 互补：
//! 后者跑在内核态、直接构造 ProcessIdentity；本程序走**真实用户链路**——
//! 用户态经 libsys \`identity_set\` 真实降级身份，再经 \`stat\` 穿越 syscall 边界，
//! 验证遍历检查在真实进程上下文（真实 PCB 身份、真实路径、真实指针校验）中生效。
//!
//! 验收判据（multi-user.md A2-3）：「无 x 的目录下文件即使 0644 也不可达（EACCES）」。
//!
//! 退出码：0 = 全部通过；非零 = 首个失败项编号。

#![no_std]
#![no_main]

use libsys::{
    chmod, identity_query, identity_set, mkdir, stat, write, Error, IdentityInfo, Permissions,
    STDOUT,
};

fn say(parts: &[&[u8]]) {
    for p in parts {
        let _ = write(STDOUT, p);
    }
    let _ = write(STDOUT, b"\n");
}

fn dec(v: u64) -> ([u8; 24], usize) {
    let mut b = [0u8; 24];
    let mut i = b.len();
    let mut n = v;
    loop {
        i -= 1;
        b[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 { break; }
    }
    let mut out = [0u8; 24];
    let len = b.len() - i;
    out[..len].copy_from_slice(&b[i..]);
    (out, len)
}

#[unsafe(no_mangle)]
pub extern "C" fn user_main(_argc: isize, _argv: *const *const u8) -> i32 {
    say(&[b"[trave2e] === A2-3 user-space E2E: traversal requires Execute ==="]);

    // 起始身份（应为 system(1)，全能力）——先如实记录，退出前恢复。
    let start: IdentityInfo = match identity_query() {
        Ok(i) => i,
        Err(_) => { say(&[b"[trave2e] FAIL(1): identity_query"]); return 1; }
    };
    say(&[b"[trave2e] start uid=", &dec(start.uid as u64).0[..dec(start.uid as u64).1],
          b" caps=", &dec(start.caps as u64).0[..dec(start.caps as u64).1]]);

    // ---- 夹具：/scratch/trave2e_dir(0700) 下放 0644 文件 ----
    let _ = mkdir("/scratch", Permissions::all());
    let _ = mkdir("/scratch/trave2e_dir", Permissions::all());
    // 目录 0700：属主 rwx，other 无 x —— 非属主无法穿越。
    if chmod("/scratch/trave2e_dir", 0o700).is_err() {
        say(&[b"[trave2e] FAIL(2): chmod dir 0700"]);
        return 2;
    }
    // 写一个 0644 文件进去（属主为当前 uid=1）。
    {
        use libsys::{close, open, write as w, OpenFlags};
        let fd = match open("/scratch/trave2e_dir/f.txt", OpenFlags::CREATE_OR_TRUNCATE, Permissions::read_write()) {
            Ok(f) => f,
            Err(_) => { say(&[b"[trave2e] FAIL(3): create file"]); return 3; }
        };
        let _ = w(fd, b"hello");
        let _ = close(fd);
    }
    if chmod("/scratch/trave2e_dir/f.txt", 0o644).is_err() {
        say(&[b"[trave2e] FAIL(4): chmod file 0644"]);
        return 4;
    }
    say(&[b"[trave2e] fixture ready: dir 0700 + file 0644"]);

    // ---- 1. 属主（uid=1，有 x）能穿越，且文件确为 0644 ----
    match stat("/scratch/trave2e_dir/f.txt") {
        Ok(s) => {
            if s.perms & 0o777 != 0o644 {
                say(&[b"[trave2e] FAIL(5): fixture file perms=", &dec((s.perms & 0o777) as u64).0[..dec((s.perms & 0o777) as u64).1], b" (want 0644)"]);
                return 5;
            }
        }
        Err(_) => { say(&[b"[trave2e] FAIL(6): owner stat should succeed"]); return 6; }
    }
    say(&[b"[trave2e] owner traversal OK (file is 0644)"]);

    // ---- 4. 对照夹具（**在降权之前**建好）----
    //   **关键纪律（实测得出）**：A2-1 route-B 语义下，**降权后无法再提权**——
    //   无 CAP_SYSTEM 者**只能下调**（uid 不得改变、caps 不得新增）。故"先降级、
    //   再由属主 chmod、再降级"的设计**必然失败**（实测 FAIL(11)：restore identity）。
    //   那是内核**正确**行为（route-B 的安全属性），不是缺陷。
    //   正确形态：降权前把两个夹具都建好（有 x 目录 + 无 x 目录），**只降权一次**，
    //   对两者分别验证——全程不需要恢复特权。这也让对照更干净：两次 stat 发生在
    //   同一身份、同一进程、同一时刻，唯一变量就是"父目录有无 x"。
    if chmod("/scratch/trave2e_dir", 0o755).is_err() {
        say(&[b"[trave2e] FAIL(11): cannot pre-set control dir"]);
        return 11;
    }
    if mkdir("/scratch/trave2e_nx", Permissions::all()).is_err() {
        say(&[b"[trave2e] FAIL(12): cannot create no-x dir"]);
        return 12;
    }
    {
        use libsys::{close, open, write as w, OpenFlags};
        if let Ok(fd) = open("/scratch/trave2e_nx/f.txt", OpenFlags::CREATE_OR_TRUNCATE, Permissions::read_write()) {
            let _ = w(fd, b"hi");
            let _ = close(fd);
        }
    }
    let _ = chmod("/scratch/trave2e_nx/f.txt", 0o644);
    if chmod("/scratch/trave2e_nx", 0o700).is_err() {
        say(&[b"[trave2e] FAIL(13): chmod no-x dir"]);
        return 13;
    }
    say(&[b"[trave2e] two fixtures pre-set by owner: dir(0755) and nx(0700)"]);

    // ---- 5. 单次降权后同时验证两侧（此后不再需要提权）----
    if identity_set(2002, 2002, 0).is_err() {
        say(&[b"[trave2e] FAIL(14): downgrade"]);
        return 14;
    }
    // 5a. 有 x 的目录 → 可穿越（对照正例）。
    match stat("/scratch/trave2e_dir/f.txt") {
        Ok(_) => say(&[b"[trave2e] dir(0755): non-owner stat succeeds (control) OK"]),
        Err(e) => {
            say(&[b"[trave2e] FAIL(15): control stat failed errno=", &dec(e.to_errno() as u64).0[..dec(e.to_errno() as u64).1]]);
            return 15;
        }
    }
    // 5b. 无 x 的目录 → 必须 EACCES（核心判据；与 5a 同身份、同时刻）。
    match stat("/scratch/trave2e_nx/f.txt") {
        Ok(_) => {
            say(&[b"[trave2e] FAIL(16): non-owner stat through no-x dir SUCCEEDED"]);
            return 16;
        }
        Err(e) => {
            if e != Error::PermissionDenied {
                say(&[b"[trave2e] FAIL(17): expected EACCES, got errno=", &dec(e.to_errno() as u64).0[..dec(e.to_errno() as u64).1]]);
                return 17;
            }
        }
    }
    say(&[b"[trave2e] nx(0700): non-owner stat -> EACCES OK (same uid, same moment)"]);

    // ---- 6. 正向验证 route-B 的不可逆性（"不能再提权"本身是安全属性）----
    //   这是**预期内的拒绝**，非失败：无 CAP_SYSTEM 者自抬必须被内核拦下。
    match identity_set(1, 0, 31) {
        Ok(_) => {
            say(&[b"[trave2e] FAIL(18): unprivileged identity_set allowed to RAISE (route-B violated)"]);
            return 18;
        }
        Err(e) => {
            if e != Error::PermissionDenied {
                say(&[b"[trave2e] FAIL(19): expected EACCES on raise, got errno=", &dec(e.to_errno() as u64).0[..dec(e.to_errno() as u64).1]]);
                return 19;
            }
            say(&[b"[trave2e] unprivileged raise correctly rejected (route-B holds) OK"]);
        }
    }
    say(&[b"[trave2e] note: identity is NOT restorable from here -- by design, not a defect"]);
    say(&[b"[trave2e] === A2-3 PASS ==="]);
    0
}
