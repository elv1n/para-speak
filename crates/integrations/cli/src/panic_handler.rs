use std::panic::{self, PanicHookInfo as PanicInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use backtrace::Backtrace;
use log::error;

static PANIC_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);
static SIGNAL_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn install_panic_handler() {
    if PANIC_HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    panic::set_hook(Box::new(|panic_info| {
        handle_panic(panic_info);
    }));
    
    log::info!("Panic handler installed");
}

fn handle_panic(panic_info: &PanicInfo) {
    let backtrace = Backtrace::new();
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("<unnamed>");
    
    let payload = panic_info.payload();
    let message = if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic payload".to_string()
    };
    
    let location = panic_info.location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "Unknown location".to_string());
    
    error!("PANIC in thread '{}': {} at {}", thread_name, message, location);
    error!("Backtrace:\n{:?}", backtrace);
    
    std::process::exit(1);
}

pub fn setup_full_backtrace_for_dev() {
    let config = config::Config::global();
    if config.debug {
        std::env::set_var("RUST_BACKTRACE", "full");
    } else {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    
    install_signal_handler();
}

#[cfg(unix)]
fn install_signal_handler() {
    if SIGNAL_HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    
    unsafe {
        use libc::{sigaction, sigemptyset, sighandler_t, SA_SIGINFO, SIGABRT, SIGSEGV};
        
        extern "C" fn signal_handler(sig: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
            let sig_name = match sig {
                SIGABRT => "SIGABRT",
                SIGSEGV => "SIGSEGV",
                _ => "UNKNOWN",
            };
            
            eprintln!("\n=== CRITICAL: Caught signal {} ===", sig_name);
            eprintln!("This is likely a native library abort/crash");
            eprintln!("Check the debug logs above for the last operation before crash");
            
            let backtrace = Backtrace::new();
            eprintln!("Native backtrace:\n{:?}", backtrace);
            
            std::process::exit(134);
        }
        
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = signal_handler as sighandler_t;
        action.sa_flags = SA_SIGINFO;
        sigemptyset(&mut action.sa_mask);
        
        sigaction(SIGABRT, &action, std::ptr::null_mut());
        sigaction(SIGSEGV, &action, std::ptr::null_mut());
    }
    
    log::info!("Signal handler installed for SIGABRT and SIGSEGV");
}

#[cfg(not(unix))]
fn install_signal_handler() {
    log::info!("Signal handler not available on non-Unix platforms");
}