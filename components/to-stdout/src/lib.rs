#![no_main]

use chrono::DateTime;

use exports::wasi::logging::logging::{Guest, Level};
use wasi::clocks::system_clock::now;

#[macro_export]
macro_rules! println {
    () => {
        println!("\n");
    };
    ($($arg:tt)*) => {{
       wit_bindgen::block_on(async move{
            let (mut writer, reader) = wit_stream::new();
            wasi::cli::stdout::write_via_stream(reader);
            writer.write_all((std::format!($($arg)*) + "\n").as_bytes().to_vec()).await;
        });
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
    features: ["clocks-timezone"],
    generate_all
});

export!(LoggingToStdout);
