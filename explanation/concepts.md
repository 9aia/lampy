# Concepts

## TelemetrySpec

The per-consumer descriptor: `service` (the `service.name`
fallback) plus `aliases` — legacy env prefixes the consumer still
honors (`HYPHA_OTEL_*`, `SEPIA_OTEL_*`, …). Resolution order is
canonical `LAMPY_OTEL_*` → aliases in order → standard `OTEL_*`.

## Env contract

`LAMPY_OTEL_ENDPOINT` is the opt-in switch — an OTLP/HTTP base URL;
`/v1/traces` is appended when absent (mirroring
`OTEL_EXPORTER_OTLP_ENDPOINT` semantics). Standard
`OTEL_EXPORTER_OTLP_*` vars also enable export with the exporter's
own resolution. `service.name` resolves `LAMPY_OTEL_SERVICE_NAME` →
alias `*_OTEL_SERVICE_NAME` → `OTEL_SERVICE_NAME` → spec default.

## Signal spec / provider

`Observability::resolve` turns `--tracing`/`--metrics` flag pairs
into `SignalSpec`s: `provider` (`otlp` | `noop`) + `dest`. The
`provider:dest` shorthand and bare provider names are accepted on
the flag itself; a bare destination defaults to `otlp`.

## TelemetryGuard

Owns the SDK providers for the process's export lifetime — `Drop`
shuts each down (batch flush + exporter close). Inert when export
is off. `from_providers` lets flag-driven bridges hand over
tracer+meter providers they built themselves.

## W3C propagation

`init` installs the `TraceContextPropagator`; `trace_headers()`
injects the current span's `traceparent`/`tracestate` into outbound
requests, `server_span()` extracts the remote parent onto an inbound
`http_request` span. Both degrade to no-ops when export is off.

## TextfileMetrics

The Prometheus textfile sink: series state in memory, atomic
tmp+rename `.prom` rewrite on every `set`. Fire-and-forget — I/O
errors are swallowed because telemetry must never break the
pipeline it observes.
