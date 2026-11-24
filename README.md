# lampy

Telemetry expertise for the 9aia ecosystem — one OTEL port, one OTLP
impl, one env contract. Named for *Lampyridae*, the firefly family.

## Install

Consumers pin git deps:

```bash
cargo add lampy-telemetry --git https://github.com/9aia/lampy
```

Lampy consolidates the near-verbatim `*-telemetry` crates that grew
inside hypha, sepia, bupha, and peregrin (P-042 / PL-023): the
`tracing` → OTLP span layer, W3C `traceparent` propagation, the
`--tracing`/`--metrics` signal-spec model, and a Prometheus textfile
metrics sink.

## Crates

- **`lampy-telemetry`** — the port. `TelemetrySpec` resolves the env
  contract (`LAMPY_OTEL_*` canonical, `<ALIAS>_OTEL_*` legacy
  aliases, standard `OTEL_*`); `Observability`/`SignalSpec` model
  flag-driven tracing/metrics destinations. Pure — no OTEL deps.
- **`lampy-telemetry-otel`** — the OTEL impl: `init()` installs the
  global `tracing` subscriber (fmt + `RUST_LOG` + OTLP span layer
  when an endpoint is configured), `TelemetryGuard` owns provider
  lifetime, `trace_headers()`/`server_span()`/`record_response()`
  carry W3C context across HTTP hops, `tracer_provider()`/
  `meter_provider()` feed flag-driven observer bridges.
- **`lampy-metrics-file`** — `TextfileMetrics`, an atomic Prometheus
  textfile writer for node_exporter scraping.

## Quickstart

```rust
use lampy_telemetry::TelemetrySpec;
use lampy_telemetry_otel as otel;

// In a daemon main — spec names the service + legacy env aliases.
let spec = TelemetrySpec::service("myservice").with_alias("MYSERVICE");
let _guard = otel::init(&spec, env!("CARGO_PKG_NAME"));
```

Env contract: `LAMPY_OTEL_ENDPOINT` (OTLP/HTTP base URL) enables
export; `LAMPY_OTEL_SERVICE_NAME` / `OTEL_SERVICE_NAME` set
`service.name`; the standard `OTEL_EXPORTER_OTLP_*` vars work too.

Consumers pin git deps — see `AGENTS.md` for the boundary and
commands. Design history: org ledger P-042.
