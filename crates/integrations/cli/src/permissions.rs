use anyhow::{anyhow, Result};
use log::{info, warn};
use std::env;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermissionType {
    Accessibility,
    Microphone,
}

impl PermissionType {
    fn display_name(&self) -> &str {
        match self {
            Self::Accessibility => "Accessibility",
            Self::Microphone => "Microphone",
        }
    }

    fn description(&self) -> &str {
        match self {
            Self::Accessibility => "Required for global hotkeys and clipboard access",
            Self::Microphone => "Required for audio recording features",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermissionStatus {
    Authorized,
    Denied,
    NotDetermined,
    Restricted,
}

pub struct PermissionManager;

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionManager {
    pub fn new() -> Self {
        Self
    }

    fn get_current_app_path(&self) -> String {
        env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "current application".to_string())
    }

    pub fn ensure_permissions(&self) -> Result<()> {
        let required = vec![PermissionType::Accessibility];

        for permission in &required {
            let status = self.check_permission(*permission)?;

            match status {
                PermissionStatus::Authorized => {
                    info!("✅ {} permission granted", permission.display_name());
                }
                PermissionStatus::Denied => {
                    self.show_permission_instructions(*permission);
                    return Err(anyhow!(
                        "{} permission denied. Please grant in System Preferences and restart.",
                        permission.display_name()
                    ));
                }
                PermissionStatus::NotDetermined => {
                    info!("🔐 Requesting {} access...", permission.display_name());
                    self.request_permission(*permission)?;
                }
                PermissionStatus::Restricted => {
                    return Err(anyhow!(
                        "{} permission is restricted by system policy",
                        permission.display_name()
                    ));
                }
            }
        }

        info!("✅ All required permissions are granted");
        Ok(())
    }

    fn check_permission(&self, permission: PermissionType) -> Result<PermissionStatus> {
        let script = match permission {
            PermissionType::Accessibility => {
                r#"
                ObjC.import('AppKit');
                $.AXIsProcessTrusted() ? 'authorized' : 'denied'
                "#
            }
            PermissionType::Microphone => {
                r#"
                ObjC.import('AVFoundation');
                const status = $.AVCaptureDevice.authorizationStatusForMediaType($.AVMediaTypeAudio);
                switch(status) {
                    case $.AVAuthorizationStatusAuthorized: 'authorized'; break;
                    case $.AVAuthorizationStatusDenied: 'denied'; break;
                    case $.AVAuthorizationStatusNotDetermined: 'not_determined'; break;
                    case $.AVAuthorizationStatusRestricted: 'restricted'; break;
                    default: 'unknown';
                }
                "#
            }
        };

        let output = Command::new("osascript")
            .args(["-l", "JavaScript", "-e", script])
            .output()?;

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

        match result.as_str() {
            "authorized" => Ok(PermissionStatus::Authorized),
            "denied" => Ok(PermissionStatus::Denied),
            "not_determined" => Ok(PermissionStatus::NotDetermined),
            "restricted" => Ok(PermissionStatus::Restricted),
            _ => Ok(PermissionStatus::NotDetermined),
        }
    }

    fn request_permission(&self, permission: PermissionType) -> Result<()> {
        match permission {
            PermissionType::Accessibility => {
                let script = r#"
                ObjC.import('AppKit');
                const options = $.NSDictionary.dictionaryWithObjectForKey(
                    $.kAXTrustedCheckOptionPrompt,
                    true
                );
                $.AXIsProcessTrustedWithOptions(options);
                "#;

                Command::new("osascript")
                    .args(["-l", "JavaScript", "-e", script])
                    .output()?;
            }
            _ => {
                warn!(
                    "{} permission must be granted manually in System Preferences",
                    permission.display_name()
                );
            }
        }

        Ok(())
    }

    fn show_permission_instructions(&self, permission: PermissionType) {
        let app_path = self.get_current_app_path();
        println!("\n⚠️  Manual Setup Required for: {}", app_path);
        println!(
            "Please grant {} permission in System Preferences:",
            permission.display_name()
        );
        println!(
            "System Preferences > Security & Privacy > Privacy > {}",
            permission.display_name()
        );
        println!(
            "• {}: {}",
            permission.display_name(),
            permission.description()
        );
        println!("\nAfter granting permissions, please restart the application.\n");
    }
}
