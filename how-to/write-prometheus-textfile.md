# Write Prometheus metrics to a textfile

Goal: expose domain metrics to node_exporter's textfile
collector without a metrics server.

1. Depend on `lampy-metrics-file`.
2. Build a `TextfileMetrics` pointed at your `.prom` path.
3. `set` series as events happen — each write is an atomic
   tmp+rename, safe for concurrent scrapes.

I/O errors are swallowed by design: telemetry must never break the
pipeline it observes.
