#![no_main]

use std::collections::HashMap;

use exports::wasi::logging::logging::{Guest, Level};
use wasi::config::store;
use wasi::logging::logging;

pub(crate) struct LoggingLevels;

impl Guest for LoggingLevels {
    fn log(level: Level, context: String, message: String) {
        Filter::initialize();

        if Filter::should_log(level, &context) {
            let level = level_map(level);
            logging::log(level, &context, &message)
        }
    }
}

struct Filter {
    initialized: bool,
    levels: Option<HashMap<String, Level>>,
    default_level: Option<Level>,
}

impl Filter {
    fn initialize() {
        if unsafe { STATE.initialized } {
            return;
        }

        let mut levels = HashMap::new();
        for (context, level) in store::get_all().unwrap_or(vec![]) {
            if let Some(level) = Filter::str_to_level(level) {
                levels.insert(context.clone(), level);
            }
        }

        let default_level: Option<Level> = levels.get("*").map(|r| *r);

        unsafe {
            STATE.levels = Some(levels);
            STATE.default_level = default_level;
            STATE.initialized = true;
        };
    }

    fn should_log(level: Level, context: &str) -> bool {
        let filter_level = Filter::level_for_context(context);
        Filter::level_ordinal(filter_level) <= Filter::level_ordinal(level)
    }

    #[allow(static_mut_refs)]
    fn level_for_context(context: &str) -> Level {
        let levels;
        let default_level;
        unsafe {
            levels = STATE.levels.as_ref();
            default_level = STATE.default_level.unwrap_or(Level::Info);
        }
        levels
            .map(|levels| levels.get(context))
            .flatten()
            .map(|r| *r)
            .unwrap_or(default_level)
    }

    fn str_to_level(level: String) -> Option<Level> {
        match level.to_lowercase().as_str() {
            "trace" => Some(Level::Trace),
            "debug" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            "critical" => Some(Level::Critical),
            _ => None,
        }
    }

    fn level_ordinal(level: Level) -> u8 {
        match level {
            Level::Trace => 0,
            Level::Debug => 1,
            Level::Info => 2,
            Level::Warn => 3,
            Level::Error => 4,
            Level::Critical => 5,
        }
    }
}

static mut STATE: Filter = Filter {
    initialized: false,
    levels: None,
    default_level: None,
};

fn level_map(level: Level) -> logging::Level {
    match level {
        Level::Trace => logging::Level::Trace,
        Level::Debug => logging::Level::Debug,
        Level::Info => logging::Level::Info,
        Level::Warn => logging::Level::Warn,
        Level::Error => logging::Level::Error,
        Level::Critical => logging::Level::Critical,
    }
}

wit_bindgen::generate!({
    path: "../../wit",
    world: "levels",
    generate_all
});

export!(LoggingLevels);
