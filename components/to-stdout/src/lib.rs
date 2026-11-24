use jiff::{tz::TimeZone, Timestamp};

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
        let now = now();
        println!(
            "{}",
            format_line(now.seconds, now.nanoseconds, level, &context, &message)
        );
    }
}

/// Formats a log statement as `YYYY-MM-DD HH:MM:SS.mmmZ LEVEL [context]: message`, the timestamp in
/// UTC from seconds and nanoseconds since the Unix epoch.
fn format_line(
    seconds: i64,
    nanoseconds: u32,
    level: Level,
    context: &str,
    message: &str,
) -> String {
    let timestamp = {
        // a timestamp out of jiff's range falls back to the epoch, `unwrap` would pull jiff's error
        // formatting into the component
        let dt = TimeZone::UTC
            .to_datetime(Timestamp::new(seconds, nanoseconds as i32).unwrap_or_default());
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}Z",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second(),
            dt.millisecond(),
        )
    };
    let level = match level {
        Level::Trace => "TRACE",
        Level::Debug => "DEBUG",
        Level::Info => "INFO",
        Level::Warn => "WARN",
        Level::Error => "ERROR",
        Level::Critical => "CRIT",
    };
    format!("{timestamp} {level:5} [{context}]: {message}",)
}

wit_bindgen::generate!({
    path: "../wit",
    world: "to-stdout",
    features: ["clocks-timezone"],
    generate_all
});

export!(LoggingToStdout);

#[cfg(test)]
mod tests {
    use super::*;

    // 2025-02-10 18:04:35 UTC
    const SECONDS: i64 = 1_739_210_675;

    #[test]
    fn formats_line() {
        assert_eq!(
            format_line(SECONDS, 925_000_000, Level::Info, "component", "message"),
            "2025-02-10 18:04:35.925Z INFO  [component]: message"
        );
    }

    #[test]
    fn formats_each_level_padded_to_five() {
        for (level, expected) in [
            (Level::Trace, "TRACE"),
            (Level::Debug, "DEBUG"),
            (Level::Info, "INFO "),
            (Level::Warn, "WARN "),
            (Level::Error, "ERROR"),
            (Level::Critical, "CRIT "),
        ] {
            assert_eq!(
                format_line(SECONDS, 0, level, "c", "m"),
                format!("2025-02-10 18:04:35.000Z {expected} [c]: m")
            );
        }
    }

    #[test]
    fn truncates_to_milliseconds() {
        assert_eq!(
            format_line(SECONDS, 999_999_999, Level::Info, "c", "m"),
            "2025-02-10 18:04:35.999Z INFO  [c]: m"
        );
        assert_eq!(
            format_line(SECONDS, 1_000_000, Level::Info, "c", "m"),
            "2025-02-10 18:04:35.001Z INFO  [c]: m"
        );
    }

    #[test]
    fn pads_date_and_time_fields() {
        // 2001-02-03 04:05:06 UTC
        assert_eq!(
            format_line(981_173_106, 7_000_000, Level::Info, "c", "m"),
            "2001-02-03 04:05:06.007Z INFO  [c]: m"
        );
    }

    #[test]
    fn formats_epoch() {
        assert_eq!(
            format_line(0, 0, Level::Info, "c", "m"),
            "1970-01-01 00:00:00.000Z INFO  [c]: m"
        );
    }

    #[test]
    fn formats_before_epoch() {
        assert_eq!(
            format_line(-1, 500_000_000, Level::Info, "c", "m"),
            "1969-12-31 23:59:59.500Z INFO  [c]: m"
        );
    }

    #[test]
    fn falls_back_to_epoch_when_out_of_range() {
        assert_eq!(
            format_line(i64::MAX, 0, Level::Info, "c", "m"),
            "1970-01-01 00:00:00.000Z INFO  [c]: m"
        );
        assert_eq!(
            format_line(SECONDS, 1_000_000_000, Level::Info, "c", "m"),
            "1970-01-01 00:00:00.000Z INFO  [c]: m"
        );
    }

    #[test]
    fn keeps_context_and_message_verbatim() {
        assert_eq!(
            format_line(SECONDS, 0, Level::Warn, "", "multi\nline {} [x]"),
            "2025-02-10 18:04:35.000Z WARN  []: multi\nline {} [x]"
        );
    }
}
