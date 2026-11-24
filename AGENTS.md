# lampy

Telemetry expertise for the 9aia ecosystem — the OTEL port, the
OTLP impl, and the env contract every service shares. Named for
*Lampyridae* — the firefly family; telemetry as the ecosystem's
bioluminescence. Rust workspace, ports-and-adapters.
`lampy-telemetry` is the port (env contract + signal specs, pure);
`lampy-telemetry-otel` and `lampy-metrics-file` are the impls.

## Layout

| Crate | Role |
| --- | --- |
| `crates/lampy-telemetry` | telemetry port — `TelemetrySpec` env contract (`LAMPY_OTEL_*` canonical + `<ALIAS>_OTEL_*` legacy aliases + standard `OTEL_*`), `TelemetryConfig` resolution, `Observability`/`SignalSpec`/`Provider` flag model, `TelemetryError` — pure, no OTEL deps |
| `crates/lampy-telemetry-otel` | OTEL impl — `init` (fmt + `EnvFilter` + OTLP span layer), `TelemetryGuard` (tracer+meter providers, flush/shutdown), `trace_headers`/`server_span`/`record_response` W3C propagation, `tracer_provider`/`meter_provider` builders for flag-driven bridges |
| `crates/lampy-metrics-file` | Prometheus textfile sink — `TextfileMetrics` atomic `.prom` writes (absorbs `bupha-telemetry-otel`) |
| `libraries/lampy` | the tamias pack — `type: files`, ships the reference `otel-collector.yaml`; no driver (lampy exposes no ops) |
| `xtask` (`lampy-xtask`) | check / test / coverage / package / site |

## Boundary

Lampy owns **telemetry mechanism** — env contract, OTLP export,
W3C propagation, metrics textfile exposition. It never names a
consumer's domain: no campaign/node/session/run types. Neighbors:

- **hypha** — pure distributed substrate; its daemons consume
  `lampy-telemetry-otel` for the tracing layer.
- **peregrin** — orchestration; `peregrin-telemetry` stays as the
  thin `Observer`-bridge adapter over lampy's provider builders.
- **bupha** — CI; keeps its own `Telemetry`/`RunEvent` domain port
  (in `bupha-watch`), sinks through `lampy-metrics-file`.
- **tamias** — distribution; consumers take lampy as pinned git
  deps, `libraries/lampy` rides `tamias publish`.

## Commands

```bash
cargo xtask check          # fmt + clippy -D warnings
cargo xtask test           # whole suite (nextest if present)
cargo xtask coverage       # llvm-cov report; extra args pass through
cargo xtask package        # cargo package --workspace (verify)
cargo xtask site           # build the docs site (needs docs/ worktree)
```

Git hooks live in `.hooks/` (`core.hooksPath` = `.hooks`):
pre-commit runs `cargo fmt --check`, pre-push `cargo xtask check`.

## The env contract

One canonical prefix, legacy aliases, standard OTEL vars —
resolution lives in `lampy_telemetry::TelemetrySpec`:

- `LAMPY_OTEL_ENDPOINT` — OTLP/HTTP base URL; `/v1/traces` appended
  when absent. `<ALIAS>_OTEL_ENDPOINT` (e.g. `HYPHA_OTEL_ENDPOINT`)
  still honored per consumer.
- `OTEL_EXPORTER_OTLP_ENDPOINT` / `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`
  — standard vars also enable export.
- `LAMPY_OTEL_SERVICE_NAME` → `<ALIAS>_OTEL_SERVICE_NAME` →
  `OTEL_SERVICE_NAME` → the spec's default service.

## Rules

- Workspace lints are strict (pedantic + `unwrap_used`/`expect_used`
  warn, `unsafe_code` forbid); `-D warnings` in `xtask check`.
- No `unsafe`. Edition 2024, rust-version 1.85.
- The port stays pure — no OTEL/OTLP/fs deps in `lampy-telemetry`.
- Telemetry is fire-and-forget: impls swallow their own I/O errors;
  export must never break the pipeline it observes.
- Tests `#![allow]` liberally; process env is never mutated in tests
  (edition 2024 `set_var` is unsafe) — inject through the pure seams.

## Docs

`docs/` worktree — VitePress + Diátaxis → GitHub Pages at
`https://9aia.com/lampy/`. `cargo xtask site` runs `bun install` +
`vitepress build` in the worktree.

<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
`issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep
statuses honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->

<!-- sturn:agent-awareness -->

## Sturn awareness

- **No PRs.** The pushed branch/commit is the deliverable;
review streams continuously (sturn WAL), merge is a
human/sturn call.
- sturn-wired repo (`sturn.toml` present): claim before edits —
`sturn claim "<glob>" --ws <ws>` surfaces conflicts in ms.
- Ops via `sturn write|rename|delete`; `sturn snap` folds the ws
into a real commit; `sturn push` publishes heads +
refs/sturn/*.
- `sturn leave` on exit — drops the actor's claims.

<!-- /sturn:agent-awareness -->

<!-- kuda:agent-awareness -->

## Kuda awareness

- `.kuda/` cards are committed artifacts — read them before
re-exploring (`kuda map`, `kuda card <mod>`); `kuda status`
reports per-card drift vs HEAD.
- Code changes that invalidate a card → regenerate it
(`kuda sync`) in the same change; cards ride the diff they
describe.
- `.kuda/index.sqlite` is gitignored + rebuildable; the `*.yaml`
cards are the checked-in source of truth.

<!-- /kuda:agent-awareness -->
