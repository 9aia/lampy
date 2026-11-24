# Architecture

Lampy is ports-and-adapters over one question: *how does a 9aia
service emit telemetry, and who decides the env contract?* Before
P-042 every repo grew its own near-verbatim `*-telemetry` crate —
four copies of the same OTLP span layer, each hardcoding its own
`<REPO>_OTEL_*` env prefix. Lampy is the single owner.

## The port: `lampy-telemetry`

Pure — no OTEL, no I/O. Everything a consumer needs to *describe*
telemetry without importing an exporter:

- **`TelemetrySpec`** — who the consumer is (`service` name +
  legacy env `aliases`). `resolve()`/`service_name()` turn the env
  contract into a `TelemetryConfig`.
- **The env contract** — canonical `LAMPY_OTEL_*` first, then each
  `<ALIAS>_OTEL_*` in order, then the standard `OTEL_*` vars. One
  unified prefix org-wide; aliases keep pre-lampy deployments
  working.
- **`Observability`/`SignalSpec`/`Provider`** — the
  `provider:dest` flag model (`--tracing otlp:http://…`,
  `--metrics noop`) engine bridges resolve CLI flags through.
- **`TelemetryError`** — resolution/provider failures; the OTEL
  build error stays a rendered string so the port never depends on
  `opentelemetry-otlp`.

## The impls

- **`lampy-telemetry-otel`** — the tracing bridge: `init()` builds
  the global `tracing` subscriber (fmt + `EnvFilter` + OTLP span
  layer *only when an endpoint resolves*), `TelemetryGuard` owns
  the SDK providers' lifetime (flush + shutdown on drop), and
  `trace_headers()`/`server_span()`/`record_response()` carry W3C
  context across HTTP hops. `tracer_provider()`/`meter_provider()`
  build providers for flag-driven bridges — a consumer's
  `Observer`-style adapter (e.g. `peregrin-telemetry`) composes its
  own delegates over them.
- **`lampy-metrics-file`** — `TextfileMetrics`, an atomic
  tmp+rename Prometheus `.prom` writer. Absorbs
  `bupha-telemetry-otel`: domain event sinks that want scrapeable
  metrics write through this instead of hand-rolling file I/O.

<!-- crate-map -->
| crate | layer | purpose |
|---|---|---|
| `lampy-telemetry` | port | Telemetry port — the org env contract, signal specs, and provider vocabulary consumers resolve against |
| `lampy-metrics-file` | adapter | Prometheus textfile metrics sink — atomic .prom writes for node_exporter scraping |
| `lampy-telemetry-otel` | adapter | OTEL impl of the lampy telemetry port — tracing→OTLP span layer, W3C propagation, tracer/meter provider builders |
| `lampy-xtask` | tooling | Repo chores: check / test / coverage / package / site |

## Data flow

```
env (LAMPY_OTEL_* | <ALIAS>_OTEL_* | OTEL_*)
  └─ TelemetrySpec::resolve ──► TelemetryConfig { endpoint, service_name }
       └─ lampy-telemetry-otel::init ──► tracing subscriber + SdkTracerProvider
            └─ OTLP/HTTP ──► collector (:4318/v1/traces)

CLI flags (--tracing/--metrics provider:dest)
  └─ Observability::resolve ──► SignalSpec
       └─ consumer adapter ──► tracer_provider/meter_provider ──► OTLP
```

## Where it sits

Delivery → memory → process → interface: lampy is cross-cutting
*process* infrastructure — every daemon's observability — sitting
below orchestration (peregrin consumes it for `--tracing`/`--metrics`)
and beside distribution (tamias ships `libraries/lampy`). It owns
mechanism only; domain event semantics (what a "run finished" means)
stay with the consumer's own port.
