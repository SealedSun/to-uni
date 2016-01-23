
use std::process::exit;
use std::sync::{Once,ONCE_INIT};
use std::io::{stderr,Write};

use log::LogLevel;
use ::env_logger;

use ::error::UniError;

/// Version of the to-uni crate.
pub const TO_UNI_VERSION : &'static str = env!("CARGO_PKG_VERSION");

pub type UniResult<T> = Result<T, UniError>;

/// Make sure errors are displayed in some form at the end of the program.
pub fn handle_program_exit(result: UniResult<()>) {
  match result {
    Ok(_) => {
      exit(0);
    },
    Err(e) => {
        // We need erros to be shown to the user. If we can, we use the error logging mechanism.
        // Otherwise, we just print to stderr. 
        if log_enabled!(LogLevel::Error) {
          error!("Fatal error: {}", e);
        } else {
          match writeln!(&mut stderr(), "Fatal error: {}", e) {
            Err(_) => (), // ignore, nothing left to do
            Ok(_) => ()
          }
        }
        exit(e.error_code() as i32);
    }
  }
}

static PROGRESSD_INIT: Once = ONCE_INIT;

/// Initialize subsystems required by to-uni.
pub fn init() {
  PROGRESSD_INIT.call_once(|| {
    env_logger::init().unwrap();
  });
}
