use rodio::{Decoder, OutputStream, Source};
use std::fs::File;
use std::io::{BufReader, Error};
use std::path::PathBuf;
use std::thread;
use std::thread::JoinHandle;

/// Emit a beep sound in a dedicated thread.
///
/// # Arguments
///
/// * `is_positive` - A boolean indicating whether the beep sound is positive or negative.
///
/// # Returns
///
/// A `JoinHandle` to the spawned thread.
pub fn emit_beep(is_positive: bool) -> JoinHandle<()> {
    // Emit a beep sound in a separate thread and get the handle
    thread::spawn(move || {
        beep(is_positive).expect("Failed to play beep sound");
    })
}

/// Plays a beep sound using the `rodio` crate.
///
/// # Arguments
///
/// * `is_positive` - A boolean indicating whether the beep sound is positive or negative.
///
/// # Returns
///
/// A `Result` which is `Ok` if the sound was played successfully, or an `Error` if it failed.
pub fn beep(is_positive: bool) -> Result<(), Error> {
    // Create an output stream
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    let path_buf = retrieve_path_wav().clone();
    let wav_buf;
    if is_positive {
        wav_buf = path_buf.join("positive-beep.wav");
    } else {
        wav_buf = path_buf.join("negative-beep.wav");
    }
    let wav: &str = wav_buf.to_str().unwrap();

    // Load the sound file
    let file = BufReader::new(File::open(wav)?);

    let source = Decoder::new(file).unwrap();

    // Play the sound
    stream_handle.play_raw(source.convert_samples()).unwrap();

    // Keep the program running long enough to hear the sound
    thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

#[cfg(not(debug_assertions))]
fn retrieve_path_wav() -> PathBuf {
    use std::env;

    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let exe_dir = exe_path.parent().expect("Failed to get executable directory");

    exe_dir.join("resources/")
}

#[cfg(debug_assertions)]
fn retrieve_path_wav() -> PathBuf {
    PathBuf::from("resources/")
}