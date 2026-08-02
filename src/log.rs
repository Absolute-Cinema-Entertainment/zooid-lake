//! Logging & panic handling.

use std::{panic::PanicHookInfo, thread};

use bevy::{
    log::{Level, tracing},
    prelude::*,
};

#[cfg(not(any(feature = "debug", target_family = "wasm")))]
use std::{fs::OpenOptions, io::BufWriter};

#[cfg(not(any(feature = "debug", target_family = "wasm")))]
use bevy::log::{BoxedFmtLayer, tracing_subscriber::fmt::Layer};

/// Creates or clears the log file (except if the `debug` feature is enabled) and registers [`panic_hook`] as panic hook.
#[inline]
pub fn setup() {
    #[cfg(not(any(feature = "debug", target_family = "wasm")))]
    {
        if let Err(err) = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(crate::consts::LOG_NAME)
        {
            // This is too early to be able to log properly. It's hopefully fine to let the program keep running.
            eprintln!(
                "Failed to clear or create '{}': {err}",
                crate::consts::LOG_NAME
            );
        }
    }

    std::panic::set_hook(Box::new(panic_hook));
}

/// Logging format layer that appends to the log file.
#[expect(clippy::unnecessary_wraps)]
#[must_use]
#[cfg(not(any(feature = "debug", target_family = "wasm")))]
pub fn fmt_layer_to_file(_: &mut App) -> Option<BoxedFmtLayer> {
    Some(Box::new(Layer::default().with_ansi(false).with_writer(
        move || {
            BufWriter::new(
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(crate::consts::LOG_NAME)
                    .unwrap(),
            )
        },
    )))
}

/// Panic hook that outputs to the error level,
/// falling back to directly writing to the log file or standard error.
///
/// Activates right after a panic but before the program crashes.
#[cold]
fn panic_hook(panic_info: &PanicHookInfo) {
    #[allow(unused_mut)]
    let mut msg = if let Some(name) = thread::current().name() {
        format!("Thread '{name}' {panic_info}")
    } else {
        format!("An unnamed thread {panic_info}")
    };

    #[cfg(not(feature = "debug"))]
    msg.push_str(crate::consts::PANIC_FOOTER);

    // The logging seems to be blocking,
    // so there is probably no risk of a race condition between the program exiting and the log being written.
    //
    // If that turns out to be false, we could choose to always write directly to the log file or standard error.
    if tracing::enabled!(Level::ERROR) {
        error!("{msg}");
    } else {
        let msg = format!("Panic occured while logging was uninitialized. {msg}");

        cfg_select! {
            any(feature = "debug", target_family = "wasm") => {
                // Fall back to standard error.
                eprintln!("{msg}");
            },
            _ => {
                // Fall back to directly writing to the log file, falling back to standard error if that fails.
                match OpenOptions::new().append(true).open(crate::consts::LOG_NAME) {
                    Ok(mut file) => {
                        use std::io::Write;

                        if let Err(err) = file.write_all(msg.as_bytes()) {
                            eprintln!("Could not write to '{}' during panic: {err}\n{msg}", crate::consts::LOG_NAME);
                        }
                    }
                    Err(err) => {
                        eprintln!("Could not open '{}' during panic: {err}\n{msg}", crate::consts::LOG_NAME);
                    }
                }
            }
        }
    }
}
