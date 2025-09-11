
#[derive(Debug, Clone)]
pub struct FocusInfo {
    pub app_name: String,
    pub window_title: String,
}


pub fn current_input_target() -> FocusInfo {
    platform::current_input_target()
}

#[cfg(target_os = "windows")]
mod platform {
    use super::FocusInfo;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::{
        Foundation::{HWND, PWSTR},
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        },
    };

    pub fn current_input_target() -> FocusInfo {
        let mut app_name = String::new();
        let mut window_title = String::new();

        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.0 != 0 {
                let len = GetWindowTextLengthW(hwnd);
                if len > 0 {
                    let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
                    let written = GetWindowTextW(hwnd, PWSTR(buf.as_mut_ptr()), len + 1);
                    if written > 0 {
                        window_title = OsString::from_wide(&buf[..written as usize])
                            .to_string_lossy()
                            .to_string();
                    }
                }
            }
        }

        FocusInfo {
            app_name,
            window_title,
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::FocusInfo;

    fn get_app_info() -> (String, String) {
        use std::process::Command;
        
        let output = Command::new("osascript")
            .args(["-e", r#"tell application "System Events" to get name of first application process whose frontmost is true"#])
            .output();
        
        let app_name = if let Ok(output) = output {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        let window_output = Command::new("osascript")
            .args(["-e", r#"tell application "System Events" to get title of front window of (first application process whose frontmost is true)"#])
            .output();
        
        let window_title = if let Ok(output) = window_output {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        (app_name, window_title)
    }

    pub fn current_input_target() -> FocusInfo {
        let (app_name, window_title) = get_app_info();
        
        FocusInfo {
            app_name,
            window_title,
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    use super::FocusInfo;

    pub fn current_input_target() -> FocusInfo {
        FocusInfo {
            app_name: String::new(),
            window_title: String::new(),
        }
    }
}
