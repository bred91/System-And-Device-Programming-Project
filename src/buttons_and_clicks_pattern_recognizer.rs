use crate::beeper::emit_beep;
use crate::notification_popup;
use crate::notification_popup::NotificationType;
use rdev::{listen, EventType, Key};
use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
enum State {
    Waiting,
    CtrlAltBPressed(Instant),
    Activated(u8, u8), // (left clicks, right clicks)
    Sleeping
}

/// Starts a pattern recognizer for button and click events.
///
/// This function spawns a new thread that listens for specific key's combination and click events.
/// After pressing for 5 seconds ctrl+alt+b, the user can choose to confirm throughout three consecutive
/// left clicks or to cancel (throughout 3 right ones), restarting the pattern.
pub fn start_button_and_clicks_pattern_recognizer() {
    let terminate_pair = Arc::new((Mutex::new(false), Condvar::new()));
    let terminate_pair_clone = Arc::clone(&terminate_pair);

    thread::spawn(move || {
        let mut state = State::Waiting;

        listen(move |event| {
            match &mut state {
                State::Waiting => {
                    // Check for Ctrl + Alt + B key press
                    if let EventType::KeyPress(key) = event.event_type {
                        if key == Key::ControlLeft || key == Key::Alt || key == Key::KeyB {
                            state = State::CtrlAltBPressed(Instant::now());
                        }
                    }
                }
                State::CtrlAltBPressed(start_time) => {
                    // Check if 5 seconds have passed
                    if start_time.elapsed() >= Duration::from_secs(5) {
                        state = State::Activated(0, 0);
                        emit_beep(true);
                        notification_popup::show_popup(NotificationType::FirstStepDoneBC, None);
                    } else if let EventType::KeyRelease(key) = event.event_type {
                        // Reset state if any key other than Ctrl, Alt, or B is released
                        if key != Key::ControlLeft && key != Key::Alt && key != Key::KeyB {
                            state = State::Waiting;
                        }
                    }
                }
                State::Activated(left_clicks, right_clicks) => {
                    // check for clicks
                    if let EventType::ButtonPress(button) = event.event_type {
                        match button {
                            rdev::Button::Left => {
                                *left_clicks += 1;
                                *right_clicks = 0; // Reset right clicks
                            }
                            rdev::Button::Right => {
                                *right_clicks += 1;
                                *left_clicks = 0; // Reset left clicks
                            }
                            _ => {}
                        }
                        // Confirmed if 3 consecutive left clicks
                        if *left_clicks >= 3 {
                            emit_beep(true);
                            notification_popup::show_popup(NotificationType::BackupStarted, None);

                            let (lock, cvar) = &*terminate_pair_clone;
                            let mut terminated = lock.lock().unwrap();
                            *terminated = true;
                            cvar.notify_all();
                            state = State::Sleeping;
                            // Canceled if 3 consecutive right clicks
                        } else if *right_clicks >= 3 {
                            emit_beep(false);
                            notification_popup::show_popup(NotificationType::BackupCanceled, None);
                            state = State::Waiting;
                        }
                    }
                },
                State::Sleeping => {
                    let barrier = Barrier::new(2);
                    barrier.wait();     // <- NOTE:
                    // since there aren't any other instance of the barrier,
                    // it will wait until the end of the program, without consuming any cpu cycle

                    // alternative strategy
                    /*thread::sleep(Duration::from_secs(123_456));*/
                }
            }
        }).unwrap();
    });

    // Wait for the condition variable
    let (lock, cvar) = &*terminate_pair;
    let mut terminated = lock.lock().unwrap();
    while !*terminated {
        terminated = cvar.wait(terminated).unwrap();
    }
}