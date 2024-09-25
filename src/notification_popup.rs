#[cfg(target_os = "windows")]
use native_dialog::{MessageDialog, MessageType};
use std::cmp::PartialEq;
#[cfg(target_os = "windows")]
use std::ffi::CStr;
#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::{BOOL, LPARAM};
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{EnumChildWindows, SendMessageA, BM_CLICK};
#[cfg(target_os = "windows")]
use winapi::um::winuser::{EnumWindows, GetClassNameA, GetWindowTextA, IsWindowVisible};


/// Enum representing different types of notifications.
#[derive(PartialEq, Clone, Copy)]
pub enum NotificationType {
    FirstStepDone,
    FirstStepDoneBC,
    BackupCanceled,
    BackupStarted,
    BackupDone,
    GenericError,
    ConfigError,
}

/// Shows a popup notification based on the notification type and an optional message.
///
/// # Arguments
///
/// * `notification_type` - The type of notification to show.
/// * `msg` - An optional message to display in the popup.
#[cfg(target_os = "windows")]
pub fn show_popup(notification_type: NotificationType, msg: Option<String>) {
    close_related_popups(notification_type);
    show_notification_popup(notification_type, msg);
}

#[cfg(not(target_os = "windows"))]
pub fn show_popup(notification_type: NotificationType, msg: Option<String>) {
    use notify_rust::Notification;

    let n = match notification_type {
        NotificationType::BackupDone => ("Backup done", "face-smile"),
        NotificationType::BackupStarted => ("Backup started", "dialog-information"),
        NotificationType::BackupCanceled => ("Backup canceled", "dialog-warning"),
        NotificationType::FirstStepDoneBC => ("Emergency backup software was activated. By making 3 consecutive quick clicks:\n- left clicks you will confirm\n- right clicks you will cancel", "dialog-information"),
        NotificationType::FirstStepDone => ("Emergency backup software was activated. By drawing a:\n- clockwise rectangle you will confirm\n- counterclockwise rectangle you will cancel", "dialog-information"),
        _ => (msg.as_deref().unwrap_or("An error occurred"), "dialog-error"),
    };

    Notification::new()
        .summary("Emergency backup")
        .body(n.0)
        .icon(n.1)
        .show()
        .unwrap();
}

/// Closes related popups based on the notification type.
///
/// # Arguments
///
/// * `notification_type` - The type of notification to handle.
#[cfg(target_os = "windows")]
fn close_related_popups(notification_type: NotificationType) {
    match notification_type {
        NotificationType::BackupStarted | NotificationType::BackupCanceled => {
            close_popup("Backup di Emergenza - FirstStepDone");
        }
        NotificationType::BackupDone => {
            close_popup("Backup di Emergenza - BackupStarted");
        }
        NotificationType::FirstStepDone | NotificationType::FirstStepDoneBC => {
            close_popup("Backup di Emergenza - BackupCanceled");
        }
        _ => {}
    }
}

/// Shows a notification popup in a separate thread based on the notification type and an optional message used in case of errors.
///
/// # Arguments
///
/// * `notification_type` - The type of notification to show.
/// * `msg` - An optional message to display in the popup.
#[cfg(target_os = "windows")]
fn show_notification_popup(notification_type: NotificationType, msg: Option<String>) {
    thread::spawn(move || {
        match notification_type {
            NotificationType::FirstStepDone => show_popup_without_btn(
                MessageType::Warning,
                "FirstStepDone",
                "  Emergency backup software was activated. By drawing a:\n  - clockwise rectangle you will confirm\n  - counterclockwise rectangle you will cancel",
            ),
            NotificationType::FirstStepDoneBC => show_popup_without_btn(
                MessageType::Warning,
                "FirstStepDone",
                "  Emergency backup software was activated. By making 3 consecutive quick clicks:\n  - left clicks you will confirm\n  - right clicks you will cancel",
            ),
            NotificationType::BackupDone => show_popup_without_btn(
                MessageType::Info,
                "BackupDone",
                "  Backup done",
            ),
            NotificationType::BackupStarted => show_popup_without_btn(
                MessageType::Info,
                "BackupStarted",
                "  Backup started",
            ),
            NotificationType::BackupCanceled => show_popup_without_btn(
                MessageType::Info,
                "BackupCanceled",
                "  Backup canceled",
            ),
            _ => show_popup_without_btn(
                MessageType::Error,
                "Error",
                &msg.unwrap(),
            )
        }
    });
}

/// Shows a popup without a button.
///
/// # Arguments
///
/// * `message_type` - The type of message to display.
/// * `title` - The title of the popup.
/// * `message` - The message to display in the popup.
#[cfg(target_os = "windows")]
fn show_popup_without_btn(message_type: MessageType, title: &str, message: &str) {
    MessageDialog::new()
        .set_type(message_type)
        .set_title(format!("Backup di Emergenza - {}", title).as_ref())
        .set_text(message)
        .show_alert()
        .unwrap();
}

/// Closes a popup with the specified title by simulating a button click.
/// NOTE: this is a `best effort` approach, it may not be possible to close it.
///
/// # Arguments
///
/// * `title` - The title of the popup to close.

#[cfg(target_os = "windows")]
fn close_popup(title: &str) {
    // Retrieve the window handle by its title
    let window = get_window_by_title(title);

    // If the window is found
    if let Some((hwnd_popup, _, _)) = window {
        // Retrieve the button handle by its parent window handle and title "OK"
        if let Some((btn_handle, _, _)) = get_button_by_parent_handler_and_title(hwnd_popup, "OK") {
            // Simulate a button click to close the popup
            click_button(btn_handle);
        }
    }
}

/// Retrieves a window by its title.
///
/// # Arguments
///
/// * `title` - The title of the window to retrieve.
///
/// # Returns
///
/// An `Option` containing a tuple with the window handle, title, and class name if found.
#[cfg(target_os = "windows")]
fn get_window_by_title(title: &str) -> Option<(HWND, String, String)> {
    // Vector to store the windows found during enumeration
    let mut windows: Vec<(HWND, String, String)> = Vec::new();

    // Enumerate all top-level windows and store their handles, titles, and class names
    unsafe {
        // The EnumWindows function is a Windows API function that enumerates all top-level windows
        // on the screen by passing the handle of each window, in turn,
        // to an application-defined callback function.
        EnumWindows(Some(enum_windows_proc), &mut windows as *mut _ as LPARAM);
    }

    windows.into_iter().find(|x| x.1 == title)
}

/// Callback function for enumerating top-level windows.
/// This function is called by the Windows API for each top-level window found during enumeration.
/// It retrieves the window's title and class name, and if the window is visible and has a non-empty title,
/// it adds the window's handle, title, and class name to the list of windows.
///
/// # Arguments
///
/// * `hwnd` - The handle to the window.
/// * `lparam` - A parameter passed to the callback function, which in this case is a pointer to a vector of tuples.
///
/// # Returns
///
/// A boolean value indicating whether to continue enumeration. Returning 1 continues enumeration, while returning 0 stops it.
///
/// # Safety
///
/// This function uses unsafe code to call Windows API functions and to dereference a raw pointer.
#[cfg(target_os = "windows")]
extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Buffers to store the window title and class name
    let mut title = [0i8; 256];
    let mut class_name = [0i8; 256];

    unsafe {
        // Retrieve the window title
        GetWindowTextA(hwnd, title.as_mut_ptr(), title.len() as i32);
        // Retrieve the window class name
        GetClassNameA(hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
    }

    // Convert the C strings to Rust strings
    let title = unsafe { CStr::from_ptr(title.as_ptr()) }.to_str().unwrap_or("");
    let class_name = unsafe { CStr::from_ptr(class_name.as_ptr()) }.to_str().unwrap_or("");

    // Check if the window is visible and has a non-empty title
    if !title.is_empty() && unsafe { IsWindowVisible(hwnd) } != 0 {
        // Add the window's handle, title, and class name to the list of windows
        let windows = unsafe { &mut *(lparam as *mut Vec<(HWND, String, String)>) };
        windows.push((hwnd, title.to_string(), class_name.to_string()));
    }
    1 // Continue enumeration
}

/// Retrieves a button by its parent window handle and title.
/// This function enumerates all child windows of the specified parent window and returns the handle,
/// title, and class name of the button that matches the given title.
///
/// # Arguments
///
/// * `parent_hwnd` - The handle to the parent window. This is a `HWND` type, which is a handle to a window.
/// * `button_title` - A string slice that holds the title of the button to retrieve.
///
/// # Returns
///
/// An `Option` containing a tuple with the button handle (`HWND`), title (`String`),
/// and class name (`String`) if a button with the specified title is found. Returns `None` otherwise.
///
/// # Safety
///
/// This function uses unsafe code to call Windows API functions and to dereference a raw pointer.
/// Ensure that the `parent_hwnd` is a valid handle to a window before calling this function.
#[cfg(target_os = "windows")]
fn get_button_by_parent_handler_and_title(parent_hwnd: HWND, button_title: &str) -> Option<(HWND, String, String)> {
    let mut buttons: Vec<(HWND, String, String)> = Vec::new();
    unsafe {
        // The EnumChildWindows function is a Windows API function that enumerates the child windows
        // that belong to the specified parent window by passing the handle of each child window,
        // in turn, to an application-defined callback function.
        EnumChildWindows(parent_hwnd, Some(enum_child_windows_proc), &mut buttons as *mut _ as LPARAM);
    }
    buttons.into_iter().find(|x| x.1 == button_title)
}

/// Callback function for enumerating child windows.
/// This function is called by the Windows API for each child window found during enumeration.
/// It retrieves the child window's title and class name, and if the window is visible and has a non-empty title,
/// it adds the window's handle, title, and class name to the list of buttons.
///
/// # Arguments
///
/// * `hwnd` - The handle to the child window.
/// * `lparam` - A parameter passed to the callback function, which in this case is a pointer to a vector of tuples.
///
/// # Returns
///
/// A boolean value indicating whether to continue enumeration. Returning 1 continues enumeration, while returning 0 stops it.
///
/// # Safety
///
/// This function uses unsafe code to call Windows API functions and to dereference a raw pointer.
#[cfg(target_os = "windows")]
extern "system" fn enum_child_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Buffers to store the child window title and class name
    let mut title = [0i8; 256];
    let mut class_name = [0i8; 256];

    unsafe {
        // Retrieve the child window title
        GetWindowTextA(hwnd, title.as_mut_ptr(), title.len() as i32);
        // Retrieve the child window class name
        GetClassNameA(hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
    }

    // Convert the C strings to Rust strings
    let title = unsafe { CStr::from_ptr(title.as_ptr()) }.to_str().unwrap_or("");
    let class_name = unsafe { CStr::from_ptr(class_name.as_ptr()) }.to_str().unwrap_or("");

    // Check if the child window is visible and has a non-empty title
    if !title.is_empty() && unsafe { IsWindowVisible(hwnd) } != 0 {
        // Add the child window's handle, title, and class name to the list of buttons
        let buttons = unsafe { &mut *(lparam as *mut Vec<(HWND, String, String)>) };
        buttons.push((hwnd, title.to_string(), class_name.to_string()));
    }
    1 // Continue enumeration
}

/// Simulates a button click.
///
/// # Arguments
///
/// * `hwnd_button` - The handle to the button to click. This is a `HWND` type, which is a handle to a window.
///
/// # Safety
///
/// This function is marked as unsafe because it calls the Windows API function `SendMessageA`.
#[cfg(target_os = "windows")]
fn click_button(hwnd_button: HWND) {
    unsafe {
        // The SendMessageA function is a Windows API function that sends the specified message to a window or windows.
        // Send a BM_CLICK message to the button to simulate a click event
        SendMessageA(hwnd_button, BM_CLICK, 0, 0);
    }
}