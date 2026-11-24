//! Runs the to-stdout component, built by `make components`, with wasmtime, calling
//! wasi:logging/logging#log and checking what it writes to stdout.

use std::path::Path;
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};

use wasmtime::component::{Component, Linker, ResourceTable, Val};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{HostWallClock, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

const COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/components/to-stdout/to-stdout.wasm"
);

// 2025-02-10 18:04:35.925 UTC
const NOW: Duration = Duration::new(1_739_210_675, 925_000_000);

struct Host {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

/// The system clock the component reads, always at the same time since the Unix epoch.
struct FixedClock(Duration);

impl HostWallClock for FixedClock {
    fn resolution(&self) -> Duration {
        Duration::from_nanos(1)
    }

    fn now(&self) -> Duration {
        self.0
    }
}

/// Instantiates the component with the system clock at `now`, calls `log` with each of `calls`, and
/// returns what the component wrote to stdout.
async fn log(now: Option<Duration>, calls: &[(&str, &str, &str)]) -> String {
    assert!(
        Path::new(COMPONENT).exists(),
        "{COMPONENT} not found, build it with `make components`"
    );

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config).unwrap();
    let component = Component::from_file(&engine, COMPONENT).unwrap();

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p3::add_to_linker(&mut linker).unwrap();

    let stdout = MemoryOutputPipe::new(1 << 20);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    if let Some(now) = now {
        builder.wall_clock(FixedClock(now));
    }
    let mut store = Store::new(
        &engine,
        Host {
            ctx: builder.build(),
            table: ResourceTable::new(),
        },
    );

    let instance = linker
        .instantiate_async(&mut store, &component)
        .await
        .unwrap();
    let interface = instance
        .get_export_index(&mut store, None, "wasi:logging/logging@0.1.0-draft")
        .expect("exports wasi:logging/logging");
    let func = instance
        .get_export_index(&mut store, Some(&interface), "log")
        .and_then(|index| instance.get_func(&mut store, index))
        .expect("exports wasi:logging/logging#log");

    for (level, context, message) in calls {
        func.call_async(
            &mut store,
            &[
                Val::Enum(level.to_string()),
                Val::String(context.to_string()),
                Val::String(message.to_string()),
            ],
            &mut [],
        )
        .await
        .unwrap();
    }

    drop(store);
    String::from_utf8(stdout.contents().to_vec()).unwrap()
}

#[tokio::test]
async fn logs_each_level() {
    for (level, expected) in [
        ("trace", "TRACE"),
        ("debug", "DEBUG"),
        ("info", "INFO "),
        ("warn", "WARN "),
        ("error", "ERROR"),
        ("critical", "CRIT "),
    ] {
        assert_eq!(
            log(Some(NOW), &[(level, "component", "message")]).await,
            format!("2025-02-10 18:04:35.925Z {expected} [component]: message\n"),
        );
    }
}

#[tokio::test]
async fn logs_each_call_on_its_own_line() {
    assert_eq!(
        log(
            Some(NOW),
            &[
                ("info", "first", "one"),
                ("warn", "second", "two"),
                ("error", "third", "three"),
            ]
        )
        .await,
        "2025-02-10 18:04:35.925Z INFO  [first]: one\n\
         2025-02-10 18:04:35.925Z WARN  [second]: two\n\
         2025-02-10 18:04:35.925Z ERROR [third]: three\n",
    );
}

#[tokio::test]
async fn logs_context_and_message_verbatim() {
    assert_eq!(
        log(Some(NOW), &[("info", "", "a {} [b] ✓ c")]).await,
        "2025-02-10 18:04:35.925Z INFO  []: a {} [b] ✓ c\n",
    );
}

#[tokio::test]
async fn truncates_timestamp_to_milliseconds() {
    assert_eq!(
        log(
            Some(Duration::new(1_739_210_675, 999_999_999)),
            &[("info", "c", "m")]
        )
        .await,
        "2025-02-10 18:04:35.999Z INFO  [c]: m\n",
    );
}

#[tokio::test]
async fn logs_at_epoch() {
    assert_eq!(
        log(Some(Duration::ZERO), &[("info", "c", "m")]).await,
        "1970-01-01 00:00:00.000Z INFO  [c]: m\n",
    );
}

#[tokio::test]
async fn timestamps_from_host_system_clock() {
    let before = Timestamp::now();
    let line = log(None, &[("info", "c", "m")]).await;
    let after = Timestamp::now();

    let logged: Timestamp = format!("{}Z", line[..23].replace(' ', "T"))
        .parse()
        .unwrap();
    // the logged timestamp is truncated to milliseconds
    assert!(
        logged > before - SignedDuration::from_millis(1) && logged <= after,
        "logged {line:?} outside {before}..{after}"
    );
}
