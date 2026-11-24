# Add a legacy env alias

Goal: keep a pre-lampy `<MYREPO>_OTEL_*` prefix working after
migrating to the canonical `LAMPY_OTEL_*` contract.

1. In your `TelemetrySpec`, list the legacy prefix in `aliases`:

   ```rust
   TelemetrySpec::new("my-service").with_aliases(&["MYREPO_OTEL"])
   ```

2. Resolution order is canonical `LAMPY_OTEL_*` → each alias in
   order → standard `OTEL_*`. Nothing else to wire — `resolve()`
   returns the merged `TelemetryConfig`.

Alias prefixes are per-consumer; they never leak into other
services' resolution.
