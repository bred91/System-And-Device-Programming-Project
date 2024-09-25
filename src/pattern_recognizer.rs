use crate::beeper;
use emath::Pos2;
use group_39::notification_popup;
use group_39::notification_popup::NotificationType;
use rdev::{listen, EventType};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

/// Define a struct to recognize and handle mouse patterns
pub struct PatternRecognizer {
    path_points: Vec<Pos2>,
    rectangle_corners: [Pos2; 4],
    tolerance: f32,
    sampling: f32,
    mouse_pos: Arc<Mutex<Option<Pos2>>>,
    side: i32,
    direction: i32,
    mouse_command_done: bool,
    movement_threshold: f32
}

/// Implement default initialization for PatternRecognizer
impl Default for PatternRecognizer {
    fn default() -> Self {
        Self {
            path_points: Vec::new(),
            rectangle_corners: [
                emath::pos2(0.0, 0.0),
                emath::pos2(0.0, 0.0),
                emath::pos2(0.0, 0.0),
                emath::pos2(0.0, 0.0),
            ],
            tolerance: 70.0,
            sampling: 10.0,
            mouse_pos: Arc::new(Mutex::new(None)),
            side: 0,
            direction: 0,
            mouse_command_done: false,
            movement_threshold: 4.0     // Soglia di movimento in pixel
        }
    }
}

impl PatternRecognizer {
    /// Helper function to clamp a value between min and max
    fn clamp(x: f32, min: i32, max: i32) -> f32 {
        if x < min as f32 {
            min as f32
        } else if x > max as f32 {
            max as f32
        } else {
            x
        }
    }

    /// Initializes the PatternRecognizer and sets up mouse tracking
    pub fn new() -> Self {
        let mut pr: PatternRecognizer = Default::default();

        // Calculate the expected rectangle corners based on screen size
        let (width, height) = get_screen_size(); //(1920,1080);
        pr.rectangle_corners = [
            emath::pos2(0.0, 0.0),
            emath::pos2(width as f32, 0.0),
            emath::pos2(width as f32, height as f32),
            emath::pos2(0.0, height as f32),
        ];

        pr.side = 0;
        pr.path_points.clear();

        let mouse_pos = Arc::new(Mutex::new(None)); // Create a shared mouse position
        let mouse_pos_clone = mouse_pos.clone(); // Clone the mouse position for use in a separate thread

        // Set up global mouse tracking using rdev crate
        let (tx, rx) = channel(); // Create a channel for communication between threads
        let tx_clone = Arc::new(Mutex::new(tx)); // Wrap the transmitter in a mutex
        let tx_clone2 = tx_clone.clone(); // Clone the transmitter for use in the listener thread

        // Spawn a thread to listen to mouse events
        thread::spawn(move || {
            listen(move |event| {
                match event.event_type {
                    EventType::MouseMove { x, y } => {
                        tx_clone2.lock().unwrap().send((x as f32, y as f32)).ok();
                    }
                    _ => {}
                }
            }).unwrap();
        });

        // Spawn another thread to update the mouse position based on received events
        thread::spawn(move || {
            while let Ok((x, y)) = rx.recv() {
                let mut pos = mouse_pos_clone.lock().unwrap();
                *pos = Some(emath::pos2(Self::clamp(x, 0, width.try_into().unwrap()), Self::clamp(y, 0, height.try_into().unwrap())));
            }
        });
        pr.mouse_pos = mouse_pos;
        pr
    }

    /// Main method to recognize the pattern
    pub fn recognize_pattern(&mut self) {
        let mut prev_mouse_pos: Option<Pos2> = None;

        loop {
            let mouse_pos = {
                let pos = self.mouse_pos.lock().unwrap();
                *pos
            };

            if let Some(pos) = mouse_pos {
                if let Some(prev_pos) = prev_mouse_pos {
                    if pos.distance(prev_pos) > self.movement_threshold {
                        if self.pattern_recognition(pos) {
                            return;
                        }
                    }
                }
                prev_mouse_pos = Some(pos);
            }
        }
    }

    /// Function to recognize the pattern by analyzing the mouse movements
    fn pattern_recognition(&mut self, mouse_pos: Pos2) -> bool {
        println!("Mouse pos: {:?}", mouse_pos);

        if self.side == 0 {
            // Check if the mouse is near the first corner of the rectangle
            if self.is_near(mouse_pos, self.rectangle_corners[0], self.tolerance) {
                self.path_points.clear();
                self.path_points.push(mouse_pos);
            }

            // Check if the mouse has moved significantly
            if !self.path_points.is_empty() && !self.is_near(mouse_pos, *self.path_points.last().unwrap(), self.sampling) {
                self.path_points.push(mouse_pos);
                if self.path_points.len() > 1000 {
                    self.path_points.clear();
                }
            }

            if let Some(last_point) = self.path_points.last() {
                // Check if the path is moving towards the top-right corner
                if self.is_near(*last_point, self.rectangle_corners[1], self.tolerance) {
                    let mut invalid_side = false;
                    let mut prev_x = self.path_points[0].x;

                    for point in &self.path_points {
                        // Check if the current point's y-coordinate exceeds the tolerance
                        // or if the x-coordinate is less than the previous x-coordinate minus the sampling value (this ensures we do not come back in the path while drawing the rectangle)
                        if point.y >= self.tolerance || point.x < prev_x - self.sampling {
                            invalid_side = true;
                            self.path_points.clear();
                            break;
                        }
                        prev_x = point.x;
                    }
                    if !invalid_side {
                        self.direction = 0; //clockwise
                        self.side = 1;
                    }
                } else if self.is_near(*last_point, self.rectangle_corners[3], self.tolerance) {
                    let mut invalid_side = false;
                    let mut prev_y = self.path_points[0].y;

                    for point in &self.path_points {
                        // Check if the current point's x-coordinate exceeds the tolerance
                        // or if the y-coordinate is less than the previous y-coordinate minus the sampling value (this ensures we do not come back in the path while drawing the rectangle)
                        if point.x >= self.tolerance || point.y < prev_y - self.sampling {
                            invalid_side = true;
                            self.path_points.clear();
                            break;
                        }
                        prev_y = point.y;
                    }
                    if !invalid_side {
                        self.direction = 1; //counter-clockwise
                        self.side = 1;
                    }
                }
            }
        }

        // Check if a valid rectangle gesture (the first one, to start the backup)  has been made
        if !self.mouse_command_done {
            //If it is the first command, it has to be clockwise
            if self.check_rectangle_gesture_clockwise(mouse_pos) {
                self.mouse_command_done = true;
                self.path_points.clear();
                self.side = 0;
                beeper::emit_beep(true);
                notification_popup::show_popup(NotificationType::FirstStepDone, None);
                return false;
            }
        } else {
            // Depending on the direction, we confirm or cancel the backup operation
            if self.direction == 0 {
                if self.check_rectangle_gesture_clockwise(mouse_pos) {
                    println!("STARTING BACKUP...");
                    self.mouse_command_done = false;
                    self.path_points.clear();
                    self.side = 0;
                    //todo: opInizioBackup
                    beeper::emit_beep(true);
                    notification_popup::show_popup(NotificationType::BackupStarted, None);
                    return true;
                }
            } else if self.direction == 1 {
                if self.check_rectangle_gesture_counterclockwise(mouse_pos) {
                    println!("CANCELLING OPERATION...");
                    self.mouse_command_done = false;
                    self.path_points.clear();
                    self.side = 0;
                    //todo: opCancellata
                    beeper::emit_beep(false);
                    notification_popup::show_popup(NotificationType::BackupCanceled, None);
                    return false;
                }
            }
        }
        false
    }

    /// Check if a point is close to another point with a certain tolerance
    fn is_near(&self, point: Pos2, target: Pos2, tolerance: f32) -> bool {
        point.distance(target) <= tolerance
    }

    /// Check if the path is valid for a given side and update state
    fn check_path_validity(&mut self, pointer_pos: Pos2, invalid: bool, rect_corner: Pos2, next_side: i32) -> bool {
        if invalid {
            self.path_points.clear();
            println!("INVALID PATH");
            self.side = 0;
        } else {
            self.path_points.push(pointer_pos);
        }

        //When I reach the bottom right corner, I check that the path on the right side is valid
        //When I reach the bottom left corner, I check that the path on the bottom side is valid
        //When I reach the top left corner, I check that the path on the left side is valid
        if let Some(last_point) = self.path_points.last() {
            if self.is_near(*last_point, rect_corner, self.tolerance) {
                if self.path_points.len() > 0 {
                    if next_side != 4 {
                        self.side = next_side;
                    } else {  //Rectangle completed
                        println!("VALID PATH");
                        self.path_points.clear();
                        self.side = 0;
                        return true;
                    }
                } else {
                    println!("INVALID PATH");
                    self.side = 0;
                }
            }
        }
        false
    }

    /// Check if the path drawn is a rectangle (clockwise direction)
    fn check_rectangle_gesture_clockwise(&mut self, pointer_pos: Pos2) -> bool {
        let mut invalid = false;
        if self.side == 1 { //RIGHT
            if !self.path_points.is_empty() && !self.is_near(pointer_pos, *self.path_points.last().unwrap(), self.sampling) {
                if pointer_pos.x < self.rectangle_corners[1].x - self.tolerance || pointer_pos.y < self.path_points.last().unwrap().y - self.sampling {
                    invalid = true;
                }
            }
            self.check_path_validity(pointer_pos, invalid, self.rectangle_corners[2], 2);
        }

        if self.side == 2 { //BOTTOM
            if !self.path_points.is_empty() && !self.is_near(pointer_pos, *self.path_points.last().unwrap(), self.sampling) {
                if pointer_pos.y < self.rectangle_corners[2].y - self.tolerance || pointer_pos.x > self.path_points.last().unwrap().x + self.sampling {
                    invalid = true;
                }
            }

            self.check_path_validity(pointer_pos, invalid, self.rectangle_corners[3], 3);
        }

        if self.side == 3 { //LEFT
            if !self.path_points.is_empty() && !self.is_near(pointer_pos, *self.path_points.last().unwrap(), self.sampling) {
                if pointer_pos.x > self.tolerance || pointer_pos.y > self.path_points.last().unwrap().y + self.sampling {
                    invalid = true;
                }
            }
            return self.check_path_validity(pointer_pos, invalid, self.rectangle_corners[0], 4);
        }
        false
    }

    /// Check if the path drawn is a rectangle (counter-clockwise direction)
    fn check_rectangle_gesture_counterclockwise(&mut self, pointer_pos: Pos2) -> bool {
        let mut invalid = false;
        if self.side == 1 { //BOTTOM
            if !self.path_points.is_empty() && !self.is_near(pointer_pos, *self.path_points.last().unwrap(), self.sampling) {
                if pointer_pos.y < self.rectangle_corners[2].y - self.tolerance || pointer_pos.x < self.path_points.last().unwrap().x - self.sampling {
                    invalid = true;
                }
            }
            self.check_path_validity(pointer_pos, invalid, self.rectangle_corners[2], 2);
        }

        if self.side == 2 { //RIGHT
            if !self.path_points.is_empty() && !self.is_near(pointer_pos, *self.path_points.last().unwrap(), self.sampling) {
                if pointer_pos.x < self.rectangle_corners[1].x - self.tolerance || pointer_pos.y > self.path_points.last().unwrap().y + self.sampling {
                    invalid = true;
                }
            }
            self.check_path_validity(pointer_pos, invalid, self.rectangle_corners[1], 3);
        }

        if self.side == 3 { //TOP
            if !self.path_points.is_empty() && !self.is_near(pointer_pos, *self.path_points.last().unwrap(), self.sampling) {
                if pointer_pos.y > self.tolerance || pointer_pos.x > self.path_points.last().unwrap().x + self.sampling {
                    invalid = true;
                }
            }
            return self.check_path_validity(pointer_pos, invalid, self.rectangle_corners[0], 4);
        }
        false
    }
}

/// Calculating the physical screen dimensions
#[cfg(target_os = "windows")]
fn get_screen_size() -> (u32, u32) {
    // The Windows function GetSystemMetrics may return values smaller than the actual screen size
    // if the operating system is configured to use display scaling (DPI scaling).
    // This happens because GetSystemMetrics returns dimensions in logical pixels, not physical pixels.
    // To obtain the screen size in physical pixels, the DPI scaling factor must be taken into account.
    // We used the function GetDpiForWindow or GetDpiForSystem to obtain the DPI scaling factor
    // and then calculate the physical screen size.
    use winapi::um::winuser::{GetDpiForWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    use winapi::um::winuser::GetDesktopWindow;

    // Get the logical width and height of the screen
    let width_logical = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height_logical = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    // Get the handle to the desktop window
    let hwnd = unsafe { GetDesktopWindow() };
    // Get the DPI for the desktop window
    let dpi = unsafe { GetDpiForWindow(hwnd) };

    // Calculate the physical width and height of the screen
    let width_physical = (width_logical as f32 * dpi as f32 / 96.0) as u32;
    let height_physical = (height_logical as f32 * dpi as f32 / 96.0) as u32;

    (width_physical, height_physical)
}

#[cfg(target_os = "macos")]
fn get_screen_size() -> (f64, f64) {
    use cocoa::appkit::{NSMainScreen, NSScreen};
    use cocoa::base::id;
    use cocoa::foundation::NSRect;
    use objc::runtime::Nil;

    unsafe {
        let screen: id = NSScreen::mainScreen(Nil);
        let frame: NSRect = msg_send![screen, frame];
        (frame.size.width, frame.size.height)
    }
}

#[cfg(target_os = "linux")]
fn get_screen_size() -> (i32, i32) {
    use x11::xlib::*;
    use std::ptr;

    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            panic!("Unable to open X display");
        }

        let screen = XDefaultScreen(display);
        let width = XDisplayWidth(display, screen);
        let height = XDisplayHeight(display, screen);

        XCloseDisplay(display);

        (width, height)
    }
}