# Crates

<!-- crate-map -->
| crate | layer | purpose |
|---|---|---|
| `lampy-telemetry` | port | Telemetry port — the org env contract, signal specs, and provider vocabulary consumers resolve against |
| `lampy-metrics-file` | adapter | Prometheus textfile metrics sink — atomic .prom writes for node_exporter scraping |
| `lampy-telemetry-otel` | adapter | OTEL impl of the lampy telemetry port — tracing→OTLP span layer, W3C propagation, tracer/meter provider builders |
| `lampy-xtask` | tooling | Repo chores: check / test / coverage / package / site |

Regenerate with `bun run docs:gen` inside `docs/` (`cypera gen
crate-map --repo ..`).
