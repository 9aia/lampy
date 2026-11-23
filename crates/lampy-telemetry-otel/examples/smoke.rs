//! OTEL smoke path — emit one span through the full
//! `init` → tracing-opentelemetry → OTLP/HTTP exporter chain and let
//! the guard flush on exit.
//!
//! Run against any OTLP/HTTP collector (or the stub in the PL-023
//! handoff):
//!
//! ```sh
//! LAMPY_OTEL_ENDPOINT=http://127.0.0.1:4318 \
//!   cargo run -p lampy-telemetry-otel --example smoke
//! ```
//!
//! With no endpoint configured the example still runs — tracing-only,
//! nothing exported (the noop path).

use lampy_telemetry::TelemetrySpec;
use lampy_telemetry_otel as lampy;

fn main() {
    let spec = TelemetrySpec::service("lampy-smoke");
    let guard = lampy::init(&spec, "lampy-smoke");
    {
        let span = tracing::info_span!(
            "smoke.request",
            http.request.method = "GET",
            url.path = "/smoke"
        );
        let _entered = span.enter();
        tracing::info!("smoke span emitted");
    }
    // The batch exporter drains on its own schedule; the guard's Drop
    // flushes + shuts the provider down on exit either way.
    std::thread::sleep(std::time::Duration::from_secs(6));
    drop(guard);
}
