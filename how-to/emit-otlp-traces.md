# Emit OTLP traces from a service

Goal: spans from your service reach an OTLP/HTTP collector.

1. Depend on `lampy-telemetry-otel` (see the
   [tutorial](../tutorials/getting-started.md)).
2. Call `lampy_telemetry_otel::init(service, aliases)` once during
   startup and keep the returned `TelemetryGuard` alive for the
   process lifetime — dropping it flushes and shuts down the SDK
   providers.
3. Set `LAMPY_OTEL_ENDPOINT` to the collector base URL
   (`http://host:4318`); `/v1/traces` is appended automatically.
4. Instrument with `tracing` spans as usual — the OTLP layer picks
   them up. Use `trace_headers()`/`server_span()` to propagate W3C
   context across HTTP hops.

Verify: hit your service, then check the collector's trace UI for
`service.name` = your service.
