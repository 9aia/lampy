---
layout: home

hero:
  name: lampy
  text: telemetry expertise
  tagline: one OTEL port, one OTLP impl, one env contract — the ecosystem's bioluminescence
  actions:
    - theme: brand
      text: Architecture
      link: /explanation/architecture
    - theme: alt
      text: Crates
      link: /reference/crates

features:
  - title: One env contract
    details: LAMPY_OTEL_* canonical prefix, per-consumer <ALIAS>_OTEL_* legacy aliases, and the standard OTEL_EXPORTER_OTLP_* vars — resolved once in the port
    link: /explanation/architecture
  - title: tracing → OTLP spans
    details: init() installs fmt + RUST_LOG + the OTLP span layer when an endpoint is configured; W3C traceparent propagation across HTTP hops
    link: /reference/crates
  - title: Flag-driven signals
    details: Observability/SignalSpec model --tracing/--metrics provider:dest destinations for engine bridges like peregrin's Observer
    link: /explanation/concepts
  - title: Metrics textfile sink
    details: TextfileMetrics writes atomic Prometheus .prom files for node_exporter scraping — fire-and-forget, never blocks the pipeline
    link: /reference/crates
---

## What it is

Lampy is the 9aia org's telemetry expertise repo (P-042): the port,
the OpenTelemetry impl, and the env contract that used to be
near-verbatim copies inside hypha, sepia, bupha, and peregrin.
Consumers depend on it as pinned git deps; the `libraries/lampy`
tamias pack ships the reference `otel-collector.yaml`.

## Where to start

- [Tutorial: getting started](tutorials/getting-started.md) —
  depend, init, emit.
- [How-to guides](how-to/) — task-shaped recipes.
- [Explanation](explanation/architecture.md) — the why behind the
  port/impl split.
- [Reference](reference/crates.md) — the generated crate map.
