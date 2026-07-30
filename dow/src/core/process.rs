// dow/src/core/
// ├── process.rs  -- Cross-platform process tree traversal for agent identity
//
// Related Docs:
// - [Claim Core](./claim.rs)
// - [Guard Hook](../hooks/guard.rs)

/// Walk up the process tree from the current process to find a stable ancestor
/// (skipping ephemeral shell processes). Returns the PID of the first non-shell
/// ancestor, which is typically the agent runtime (e.g., Kiro, Claude Code).
///
/// Used by `detect_agent_id()` to record a stable identity at claim time.
pub fn find_stable_ancestor() -> Option<u32> {
    let current_pid = std::process::id();
    let mut pid = get_parent_pid(current_pid)?;

    // Walk up, skipping shell processes, with a depth limit to avoid infinite loops
    let max_depth = 5;
    for _ in 0..max_depth {
        if pid <= 1 {
            return None; // Reached init/launchd/System — no stable ancestor found
        }
        if !is_shell_process(pid) {
            return Some(pid);
        }
        match get_parent_pid(pid) {
            Some(ppid) if ppid != pid && ppid > 1 => pid = ppid,
            _ => return Some(pid), // Can't go further, use current
        }
    }

    Some(pid)
}

/// Check if `target_pid` is an ancestor of the current process.
/// Used by the guard hook to verify the current command belongs to the same
/// agent session that created the claim.
pub fn is_ancestor_of_current(target_pid: u32) -> bool {
    if target_pid <= 1 {
        return false;
    }

    let mut pid = std::process::id();
    let max_depth = 10; // Generous depth for complex process trees

    for _ in 0..max_depth {
        if pid == target_pid {
            return true;
        }
        if pid <= 1 {
            return false;
        }
        match get_parent_pid(pid) {
            Some(ppid) if ppid != pid => pid = ppid,
            _ => return false,
        }
    }

    false
}

/// Parse a "pid:<number>" string and return the numeric PID.
pub fn parse_pid_agent_id(agent_id: &str) -> Option<u32> {
    agent_id.strip_prefix("pid:").and_then(|s| s.parse().ok())
}

/// Check if a given PID is still alive.
pub fn is_process_alive(pid: u32) -> bool {
    platform::is_process_alive(pid)
}

// ─── Platform-specific implementations ─────────────────────────────────────────

/// Get the parent PID of a given process.
fn get_parent_pid(pid: u32) -> Option<u32> {
    platform::get_parent_pid(pid)
}

/// Determine if a process is an ephemeral shell (bash, sh, zsh, etc.).
fn is_shell_process(pid: u32) -> bool {
    let name = match platform::get_process_name(pid) {
        Some(n) => n,
        None => return false, // Can't determine — assume not a shell
    };

    let shell_names: &[&str] = &[
        "sh", "bash", "zsh", "fish", "dash", "ksh", "csh", "tcsh", "ash",
        // Windows shells
        "cmd", "cmd.exe", "powershell", "powershell.exe", "pwsh", "pwsh.exe",
    ];

    let lower = name.to_lowercase();
    shell_names.iter().any(|s| lower == *s)
}

// ─── Platform module ───────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod platform {
    use std::fs;

    pub fn get_parent_pid(pid: u32) -> Option<u32> {
        // /proc/<pid>/stat format: pid (comm) state ppid ...
        // The comm field may contain spaces and parentheses, so find the last ')'
        let stat = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
        let after_comm = stat.rfind(')')? + 2; // skip ") "
        let fields: Vec<&str> = stat[after_comm..].split_whitespace().collect();
        // fields[0] = state, fields[1] = ppid
        fields.get(1)?.parse().ok()
    }

    pub fn get_process_name(pid: u32) -> Option<String> {
        // /proc/<pid>/comm contains just the process name (max 15 chars)
        fs::read_to_string(format!("/proc/{}/comm", pid))
            .ok()
            .map(|s| s.trim().to_string())
    }

    pub fn is_process_alive(pid: u32) -> bool {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::Command;

    pub fn get_parent_pid(pid: u32) -> Option<u32> {
        let output = Command::new("ps")
            .args(["-o", "ppid=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .ok()
    }

    pub fn get_process_name(pid: u32) -> Option<String> {
        let output = Command::new("ps")
            .args(["-o", "comm=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let full_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // ps -o comm= may return full path on macOS; extract basename
        full_path
            .rsplit('/')
            .next()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn is_process_alive(pid: u32) -> bool {
        // Use ps to check process existence (avoids libc dependency)
        Command::new("ps")
            .args(["-p", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::mem;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct PROCESSENTRY32W {
        dwSize: u32,
        cntUsage: u32,
        th32ProcessID: u32,
        th32DefaultHeapID: usize,
        th32ModuleID: u32,
        cntThreads: u32,
        th32ParentProcessID: u32,
        pcPriClassBase: i32,
        dwFlags: u32,
        szExeFile: [u16; 260],
    }

    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    const INVALID_HANDLE_VALUE: isize = -1;

    extern "system" {
        fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
        fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn CloseHandle(hObject: isize) -> i32;
    }

    /// Take a process snapshot and find the entry matching the given PID.
    /// Returns (parent_pid, exe_name) if found.
    fn find_process_entry(pid: u32) -> Option<(u32, String)> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return None;
            }

            let mut entry: PROCESSENTRY32W = mem::zeroed();
            entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

            let mut result = None;
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    if entry.th32ProcessID == pid {
                        let len = entry
                            .szExeFile
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(260);
                        let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                        let basename = name
                            .rsplit('\\')
                            .next()
                            .unwrap_or(&name)
                            .to_string();
                        result = Some((entry.th32ParentProcessID, basename));
                        break;
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }

            CloseHandle(snapshot);
            result
        }
    }

    pub fn get_parent_pid(pid: u32) -> Option<u32> {
        find_process_entry(pid).map(|(ppid, _)| ppid)
    }

    pub fn get_process_name(pid: u32) -> Option<String> {
        find_process_entry(pid).map(|(_, name)| name)
    }

    pub fn is_process_alive(pid: u32) -> bool {
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const STILL_ACTIVE: u32 = 259;

        extern "system" {
            fn OpenProcess(
                dwDesiredAccess: u32,
                bInheritHandles: i32,
                dwProcessId: u32,
            ) -> isize;
            fn GetExitCodeProcess(hProcess: isize, lpExitCode: *mut u32) -> i32;
            fn CloseHandle(hObject: isize) -> i32;
        }

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return false;
            }
            let mut exit_code: u32 = 0;
            let alive = GetExitCodeProcess(handle, &mut exit_code) != 0
                && exit_code == STILL_ACTIVE;
            CloseHandle(handle);
            alive
        }
    }
}

// Fallback for other platforms
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    pub fn get_parent_pid(_pid: u32) -> Option<u32> {
        None
    }

    pub fn get_process_name(_pid: u32) -> Option<String> {
        None
    }

    pub fn is_process_alive(_pid: u32) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_stable_ancestor_returns_some() {
        // Current process has a parent (the test runner)
        let ancestor = find_stable_ancestor();
        assert!(ancestor.is_some());
        let pid = ancestor.unwrap();
        assert!(pid > 1);
    }

    #[test]
    fn test_is_ancestor_of_current_with_parent() {
        // Our parent PID should be an ancestor
        let ppid = get_parent_pid(std::process::id());
        if let Some(ppid) = ppid {
            if ppid > 1 {
                assert!(is_ancestor_of_current(ppid));
            }
        }
    }

    #[test]
    fn test_is_ancestor_of_current_with_self() {
        // Our own PID is in our ancestor chain (trivially)
        assert!(is_ancestor_of_current(std::process::id()));
    }

    #[test]
    fn test_is_ancestor_nonexistent_pid() {
        // A very large PID that certainly doesn't exist
        assert!(!is_ancestor_of_current(u32::MAX - 1));
    }

    #[test]
    fn test_parse_pid_agent_id() {
        assert_eq!(parse_pid_agent_id("pid:1234"), Some(1234));
        assert_eq!(parse_pid_agent_id("pid:0"), Some(0));
        assert_eq!(parse_pid_agent_id("/dev/pts/1"), None);
        assert_eq!(parse_pid_agent_id("session:abc"), None);
        assert_eq!(parse_pid_agent_id("pid:"), None);
        assert_eq!(parse_pid_agent_id("pid:abc"), None);
    }

    #[test]
    fn test_is_process_alive_self() {
        assert!(is_process_alive(std::process::id()));
    }

    #[test]
    fn test_is_process_alive_nonexistent() {
        assert!(!is_process_alive(u32::MAX - 1));
    }

    #[test]
    fn test_is_shell_process_detection() {
        // The test runner itself should not be a shell
        let self_pid = std::process::id();
        assert!(!is_shell_process(self_pid));
    }
}
