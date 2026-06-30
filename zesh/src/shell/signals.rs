// Signal handling

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

pub static G_SIGINT_RECEIVED: AtomicBool = AtomicBool::new(false);
pub static G_INTERRUPT_LOOP: AtomicBool = AtomicBool::new(false);
pub static G_PENDING_SIGNAL: AtomicI32 = AtomicI32::new(-1);  // signal number, -1 = none
pub static G_FOREGROUND_PID: AtomicI32 = AtomicI32::new(-1);
pub static G_EXIT_TRAP_RUNNING: AtomicBool = AtomicBool::new(false);
pub static G_SIGNAL_TRAP_RUNNING: AtomicBool = AtomicBool::new(false);
pub static G_PENDING_EXIT_STATUS: AtomicI32 = AtomicI32::new(0);
pub static G_SHELL_PGID: AtomicI32 = AtomicI32::new(-1);

// Trap actions: index = signal number
pub static G_TRAP_ACTIONS: Mutex<[Option<String>; 32]> = Mutex::new([
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
]);
pub static G_TRAP_EXIT: Mutex<Option<String>> = Mutex::new(None);

extern "C" fn handle_sigint(sig: libc::c_int) {
    G_SIGINT_RECEIVED.store(true, Ordering::SeqCst);
    G_INTERRUPT_LOOP.store(true, Ordering::SeqCst);
    G_PENDING_SIGNAL.store(sig, Ordering::SeqCst);

    let fgpgid = G_FOREGROUND_PID.load(Ordering::SeqCst);
    if fgpgid > 0 {
        // fgpgid is actually a process group id (pgid). Send SIGINT to the entire process group
        // by using the negative pgid. SAFETY: sending SIGINT to a process group is valid
        unsafe { libc::kill(-fgpgid, sig); }
    }
}

extern "C" fn handle_sigchld(_sig: libc::c_int) {
    // Reap any zombie children and detect stops
    loop {
        let mut wstatus: libc::c_int = 0;
        let pid = unsafe { libc::waitpid(-1, &mut wstatus, libc::WNOHANG | libc::WUNTRACED) };
        if pid <= 0 { break; }

        // Try to detect and record stopped children
        // Use best-effort locking - if we can't get the lock, skip the update
        if libc::WIFSTOPPED(wstatus) {
            if let Some(mut jobs_table) = crate::shell::jobs::try_get_jobs() {
                if let Some(job) = jobs_table.find_by_pid_mut(pid) {
                    job.status = crate::shell::jobs::JobStatus::Stopped;
                }
            }
        }
    }
}

pub fn init_shell_pgid() {
    // Claim the controlling terminal for the shell (pgid already initialized in setup_signals)
    // SAFETY: tcsetpgrp is safe to call when interactive
    unsafe {
        let pgid = G_SHELL_PGID.load(Ordering::SeqCst);
        if pgid > 0 {
            let _ = libc::tcsetpgrp(0, pgid);
        }
    }
}

pub fn setup_signals() {
    // Initialize shell's process group (for both interactive and non-interactive modes)
    // In non-interactive mode, we won't call tcsetpgrp, but we still need the pgid for signal handling
    unsafe {
        let _ = libc::setpgid(0, 0);
        let pgid = libc::getpgrp();
        G_SHELL_PGID.store(pgid, Ordering::SeqCst);
    }

    // SAFETY: setting up signal handlers with valid function pointers
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handle_sigint as usize;
        libc::sigemptyset(&mut sa.sa_mask);
        sa.sa_flags = 0;  // Don't use SA_RESTART for SIGINT - we need waitpid to return EINTR
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());

        // SIGTERM - use same handler as SIGINT, also without SA_RESTART
        let mut sa_term: libc::sigaction = std::mem::zeroed();
        sa_term.sa_sigaction = handle_sigint as usize;
        libc::sigemptyset(&mut sa_term.sa_mask);
        sa_term.sa_flags = 0;  // Don't use SA_RESTART for SIGTERM - we need waitpid to return EINTR
        libc::sigaction(libc::SIGTERM, &sa_term, std::ptr::null_mut());

        // SIGCHLD
        let mut sa2: libc::sigaction = std::mem::zeroed();
        sa2.sa_sigaction = handle_sigchld as usize;
        libc::sigemptyset(&mut sa2.sa_mask);
        sa2.sa_flags = libc::SA_RESTART;
        libc::sigaction(libc::SIGCHLD, &sa2, std::ptr::null_mut());

        // Ignore SIGPIPE
        let mut sa3: libc::sigaction = std::mem::zeroed();
        sa3.sa_sigaction = libc::SIG_IGN;
        libc::sigemptyset(&mut sa3.sa_mask);
        libc::sigaction(libc::SIGPIPE, &sa3, std::ptr::null_mut());

        // Ignore SIGTTOU/SIGTTIN in interactive mode
        let mut sa4: libc::sigaction = std::mem::zeroed();
        sa4.sa_sigaction = libc::SIG_IGN;
        libc::sigemptyset(&mut sa4.sa_mask);
        libc::sigaction(libc::SIGTTOU, &sa4, std::ptr::null_mut());
        libc::sigaction(libc::SIGTTIN, &sa4, std::ptr::null_mut());
    }
}

pub fn reset_signals_for_child() {
    // SAFETY: resetting signals to default in child process
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = libc::SIG_DFL;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGPIPE, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGTTOU, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGTTIN, &sa, std::ptr::null_mut());
    }
}

pub fn run_exit_trap(action: &str, vars: &crate::shell::vars::VarStore, script_file: &str) {
    let tokens = crate::shell::lexer::lex(action);
    let nodes = crate::shell::parser::parse(tokens);
    let mut ctx = crate::shell::executor::ExecContext::new_subshell();
    ctx.script_file = script_file.to_string();
    ctx.exit_status = G_PENDING_EXIT_STATUS.load(Ordering::SeqCst);
    crate::shell::executor::execute_list_with_vars(&nodes, &mut ctx, vars);
}

pub fn check_and_run_trap(vars: &crate::shell::vars::VarStore, script_file: &str) -> bool {
    // Re-entrancy guard: block recursive trap execution (e.g. kill inside a trap handler
    // re-delivers the signal while the handler is still running)
    if G_SIGNAL_TRAP_RUNNING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return false;
    }

    let sig_num = G_PENDING_SIGNAL.swap(-1, Ordering::SeqCst);
    if sig_num < 0 || sig_num >= 32 {
        G_SIGNAL_TRAP_RUNNING.store(false, Ordering::SeqCst);
        return false;
    }

    let action = if let Ok(traps) = G_TRAP_ACTIONS.lock() {
        traps[sig_num as usize].clone()
    } else {
        G_SIGNAL_TRAP_RUNNING.store(false, Ordering::SeqCst);
        return false;
    };

    if let Some(action_str) = action {
        let tokens = crate::shell::lexer::lex(&action_str);
        let nodes = crate::shell::parser::parse(tokens);
        let mut ctx = crate::shell::executor::ExecContext::new_subshell();
        ctx.script_file = script_file.to_string();
        ctx.exit_status = 0;
        crate::shell::executor::execute_list_with_vars(&nodes, &mut ctx, vars);
        G_SIGNAL_TRAP_RUNNING.store(false, Ordering::SeqCst);
        return true;
    }
    G_SIGNAL_TRAP_RUNNING.store(false, Ordering::SeqCst);
    false
}
