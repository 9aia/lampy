# Getting started

The golden path: depend on lampy, resolve the env contract, and
emit spans — in that order.

## 1. Depend on the crates

Pin the git deps in your `Cargo.toml`:

```toml
[dependencies]
lampy-telemetry = { git = "https://github.com/9aia/lampy" }
lampy-telemetry-otel = { git = "https://github.com/9aia/lampy" }
```

`lampy-telemetry` is the pure port (specs, env contract,
`provider:dest` flags); `lampy-telemetry-otel` is the OTLP
implementation. `lampy-metrics-file` is the Prometheus textfile
sink for services scraped by node_exporter.

## 2. Initialize tracing once, at startup

```rust
let _guard = lampy_telemetry_otel::init("my-service", &["MYLEGACY_OTEL"])?;
```

`init` installs the fmt layer, `RUST_LOG` filtering, and the OTLP
span layer **only when an endpoint resolves** — with no
`LAMPY_OTEL_ENDPOINT` (or an `OTEL_EXPORTER_OTLP_*` equivalent)
configured it is a no-op, so dev and tests stay quiet for free.

## 3. Let env do the rest

Set `LAMPY_OTEL_ENDPOINT=http://localhost:4318` (the collector
shipped in `libraries/lampy`) and every span flows over OTLP/HTTP
with W3C `traceparent` propagation across your HTTP hops. See
[concepts](../explanation/concepts.md) for the full resolution
order — canonical `LAMPY_OTEL_*`, then your aliases, then the
standard `OTEL_*` vars.

## Next steps

- [Architecture](../explanation/architecture.md) — why the port/impl
  split exists.
- [Crates](../reference/crates.md) — the generated crate map.
- [How-to guides](../how-to/) — flag-driven signals, metrics
  sinks, and aliases.
