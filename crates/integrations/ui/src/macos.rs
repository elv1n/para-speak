#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSWindow, NSWindowCollectionBehavior};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSInteger, MainThreadMarker};
#[cfg(target_os = "macos")]
use objc2::{msg_send, runtime::AnyObject};

#[cfg(target_os = "macos")]
pub fn setup_window_for_all_spaces(window_handle: &dyn raw_window_handle::HasWindowHandle) {
    use raw_window_handle::{RawWindowHandle};

    if let Ok(RawWindowHandle::AppKit(handle)) = window_handle.window_handle().map(|h| h.as_raw()) {
        unsafe {
            let ns_view = handle.ns_view.as_ptr() as *mut AnyObject;
            if !ns_view.is_null() {
                let ns_view_obj = &*ns_view;
                
                let window_ptr = msg_send![ns_view_obj, window];
                let window: *mut NSWindow = window_ptr;
                
                if !window.is_null() {
                    let window_obj = &*window;
                    
                    let current_behavior = window_obj.collectionBehavior();
                    
                    let can_join_all_spaces = NSWindowCollectionBehavior(1 << 0); // NSWindowCollectionBehaviorCanJoinAllSpaces
                    let stationary = NSWindowCollectionBehavior(1 << 4); // NSWindowCollectionBehaviorStationary
                    let fullscreen_auxiliary = NSWindowCollectionBehavior(1 << 7); // NSWindowCollectionBehaviorFullScreenAuxiliary
                    let ignores_cycle = NSWindowCollectionBehavior(1 << 6); // NSWindowCollectionBehaviorIgnoresCycle
                    
                    let new_behavior = NSWindowCollectionBehavior(
                        current_behavior.0 | can_join_all_spaces.0 | stationary.0 | fullscreen_auxiliary.0 | ignores_cycle.0
                    );
                    window_obj.setCollectionBehavior(new_behavior);
                    
                    let screen_saver_level: NSInteger = 1000; // NSScreenSaverWindowLevel
                    window_obj.setLevel(screen_saver_level);
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub fn setup_all_app_windows_for_spaces() {
    unsafe {
        let mtm = MainThreadMarker::new().expect("Must be called from main thread");
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.windows();
        
        for i in 0..windows.count() {
            let window = windows.objectAtIndex(i);
            let current_behavior = window.collectionBehavior();
            
            let can_join_all_spaces = NSWindowCollectionBehavior(1 << 0); // NSWindowCollectionBehaviorCanJoinAllSpaces
            let stationary = NSWindowCollectionBehavior(1 << 4); // NSWindowCollectionBehaviorStationary
            let fullscreen_auxiliary = NSWindowCollectionBehavior(1 << 7); // NSWindowCollectionBehaviorFullScreenAuxiliary
            let ignores_cycle = NSWindowCollectionBehavior(1 << 6); // NSWindowCollectionBehaviorIgnoresCycle
            
            let new_behavior = NSWindowCollectionBehavior(
                current_behavior.0 | can_join_all_spaces.0 | stationary.0 | fullscreen_auxiliary.0 | ignores_cycle.0
            );
            window.setCollectionBehavior(new_behavior);
            
            let screen_saver_level: NSInteger = 1000; // NSScreenSaverWindowLevel
            window.setLevel(screen_saver_level);
            
            log::info!("Set up window {} for all spaces", i);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn setup_window_for_all_spaces(_window_handle: &dyn raw_window_handle::HasWindowHandle) {
}

#[cfg(not(target_os = "macos"))]
pub fn setup_all_app_windows_for_spaces() {
}