#![no_main]

use chrono::DateTime;

use exports::wasi::logging::logging::{Guest, Level};
use wasi::clocks::wall_clock::now;

#[macro_export]
macro_rules! println {
    () => {
        wasi::cli::stdout::get_stdout().blocking_write_and_flush("\n".as_bytes())
            .expect("failed writing to stdout")
    };
    ($($arg:tt)*) => {{
        wasi::cli::stdout::get_stdout().blocking_write_and_flush((std::format!($($arg)*) + "\n").as_bytes())
            .expect("failed writing to stdout")
    }};
}

pub(crate) struct LoggingToStdout;

impl Guest for LoggingToStdout {
    fn log(level: Level, context: String, message: String) {
        let timestamp = {
            let now = now();
            DateTime::from_timestamp(now.seconds as i64, now.nanoseconds)
                .unwrap()
                .format("%Y-%m-%d %H:%M:%S.%3fZ")
        };
        let level = match level {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Critical => "CRIT",
        };

        println!("{timestamp} {level:5} [{context}]: {message}");
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "to-stdout",
    generate_all
});

export!(LoggingToStdout);
