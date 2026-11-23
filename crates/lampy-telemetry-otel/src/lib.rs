//! lampy-telemetry-otel — the OTEL impl of the
//! [`lampy_telemetry`] port: the opt-in OpenTelemetry layer over a
//! consumer's `tracing` stack, W3C `traceparent`/`tracestate`
//! propagation, and the tracer/meter provider builders flag-driven
//! bridges (e.g. `--tracing`/`--metrics`) compose observers through.
//!
//! [`init`] builds the global subscriber every daemon main installs:
//! the existing `fmt` + `RUST_LOG` (`EnvFilter`) logging, plus — only
//! when an OTLP endpoint is configured — a `tracing-opentelemetry`
//! layer exporting spans to a collector. With no endpoint the
//! subscriber is exactly the tracing-only setup the daemons always
//! used; [`TelemetryGuard`] is inert.
//!
//! Env resolution lives in the port — see [`TelemetrySpec`]; the
//! canonical contract is `LAMPY_OTEL_*` with per-consumer
//! `<ALIAS>_OTEL_*` aliases and the standard `OTEL_EXPORTER_OTLP_*`
//! vars.
//!
//! Propagation: `init` installs the W3C `traceparent`/`tracestate`
//! propagator. [`trace_headers`] injects the current span's context
//! for outbound HTTP; [`server_span`] builds the inbound
//! `http_request` span with the remote parent attached. Both degrade
//! to no-ops when export is off — injection emits no headers on an
//! invalid context, and extraction yields a plain local-root span.

use opentelemetry::propagation::{Extractor, Injector};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer as _};

use lampy_telemetry::{
    METRICS_PATH, TRACES_PATH, TelemetryConfig, TelemetryError, TelemetrySpec, signal_url,
};

/// Keeps the SDK providers alive for the process's export lifetime;
/// `Drop` shuts each down (batch flush + exporter close). Inert when
/// no provider was built — the common case when export is off.
#[derive(Debug)]
pub struct TelemetryGuard {
    tracer: Option<SdkTracerProvider>,
    meter: Option<SdkMeterProvider>,
}

impl TelemetryGuard {
    /// A guard over already-built providers — flag-driven bridges
    /// (e.g. `--tracing`/`--metrics` observers) assemble their own
    /// and hand them here so one guard covers both signals.
    #[must_use]
    pub fn from_providers(
        tracer: Option<SdkTracerProvider>,
        meter: Option<SdkMeterProvider>,
    ) -> Self {
        Self { tracer, meter }
    }

    /// `true` when at least one provider is exporting.
    pub fn is_enabled(&self) -> bool {
        self.tracer.is_some() || self.meter.is_some()
    }

    /// Flush pending telemetry without consuming the guard (shutdown
    /// also flushes — this is for long-lived processes wanting a
    /// mid-run drain).
    pub fn force_flush(&self) {
        if let Some(provider) = &self.tracer {
            let _ = provider.force_flush();
        }
        if let Some(provider) = &self.meter {
            let _ = provider.force_flush();
        }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer.take()
            && let Err(e) = provider.shutdown()
        {
            tracing::warn!("lampy-telemetry: tracer shutdown failed: {e:?}");
        }
        if let Some(provider) = self.meter.take()
            && let Err(e) = provider.shutdown()
        {
            tracing::warn!("lampy-telemetry: meter shutdown failed: {e:?}");
        }
    }
}

fn build_tracer_provider(
    config: &TelemetryConfig,
    service_name: &str,
) -> Result<SdkTracerProvider, opentelemetry_otlp::ExporterBuildError> {
    use opentelemetry_otlp::WithExportConfig;
    let mut builder = opentelemetry_otlp::SpanExporter::builder().with_http();
    if let Some(endpoint) = &config.endpoint {
        builder = builder.with_endpoint(endpoint.clone());
    }
    let exporter = builder.build()?;
    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(service_name.to_string())
        .build();
    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build())
}

/// Build the tracing provider for an explicit flag destination —
/// `endpoint` is the flag's raw destination (`None` resolves the
/// spec's env contract: the configured `/v1/traces` URL, or `None`
/// to let the exporter read `OTEL_EXPORTER_OTLP_*` itself).
///
/// # Errors
/// [`TelemetryError::Exporter`] when the OTLP exporter fails to
/// build.
pub fn tracer_provider(
    spec: &TelemetrySpec,
    endpoint: Option<&str>,
) -> Result<SdkTracerProvider, TelemetryError> {
    let config = match endpoint {
        Some(dest) => TelemetryConfig {
            endpoint: Some(signal_url(dest, TRACES_PATH)),
            service_name: spec.service_name_env(),
        },
        None => spec.resolve_env().unwrap_or_else(|| TelemetryConfig {
            endpoint: None,
            service_name: spec.service_name_env(),
        }),
    };
    build_tracer_provider(&config, &config.service_name)
        .map_err(|e| TelemetryError::exporter("tracing", e))
}

/// Build the metrics provider for an explicit flag destination.
///
/// # Errors
/// [`TelemetryError::Exporter`] when the OTLP exporter fails to
/// build.
pub fn meter_provider(
    spec: &TelemetrySpec,
    endpoint: Option<&str>,
) -> Result<SdkMeterProvider, TelemetryError> {
    use opentelemetry_otlp::WithExportConfig;
    let mut builder = opentelemetry_otlp::MetricExporter::builder().with_http();
    if let Some(dest) = endpoint {
        builder = builder.with_endpoint(signal_url(dest, METRICS_PATH));
    }
    let exporter = builder
        .build()
        .map_err(|e| TelemetryError::exporter("metrics", e))?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter).build();
    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(spec.service_name_env())
        .build();
    Ok(SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build())
}

/// Install the global subscriber: `fmt` + `EnvFilter` (`RUST_LOG`,
/// default `info`) — the daemons' long-standing behavior — plus the
/// OTLP span layer when an endpoint is configured. Also installs the
/// W3C trace-context propagator so [`trace_headers`]/[`server_span`]
/// work regardless.
///
/// `scope` is the instrumentation scope name (pass
/// `env!("CARGO_PKG_NAME")` at the call site so spans attribute to
/// the consumer's crate, not lampy).
///
/// Keep the returned [`TelemetryGuard`] alive for the process's span
/// export; dropping it flushes and shuts the provider down.
///
/// A subscriber already installed simply wins — `init` never panics
/// on a second call (each caller still gets its own guard).
pub fn init(
    spec: &TelemetrySpec,
    scope: impl Into<std::borrow::Cow<'static, str>>,
) -> TelemetryGuard {
    let config = spec.resolve_env();
    init_with(config, scope)
}

/// [`init`]'s body with the env read lifted out — tests inject the
/// config (process env can't be set under `forbid(unsafe_code)`).
fn init_with(
    config: Option<TelemetryConfig>,
    scope: impl Into<std::borrow::Cow<'static, str>>,
) -> TelemetryGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    global::set_text_map_propagator(TraceContextPropagator::new());

    let configured = config.is_some();
    let (otel_layer, provider, build_err) = match config {
        Some(config) => match build_tracer_provider(&config, &config.service_name) {
            Ok(provider) => {
                let tracer = provider.tracer(scope.into());
                (
                    Some(tracing_opentelemetry::layer().with_tracer(tracer)),
                    Some(provider),
                    None,
                )
            }
            Err(e) => (None, None, Some(e)),
        },
        None => (None, None, None),
    };

    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter.clone()))
        .with(otel_layer.map(|l| l.with_filter(filter)))
        .try_init();

    if let Some(e) = build_err {
        tracing::warn!("OTEL endpoint configured but exporter build failed; tracing-only: {e}");
    } else if configured {
        tracing::info!("OTEL tracing enabled (OTLP/HTTP)");
    }
    TelemetryGuard {
        tracer: provider,
        meter: None,
    }
}

/// [`Injector`] over a header list — ureq (and any client accepting
/// name/value pairs) consumes the returned `Vec` one `.header()` at a
/// time.
struct VecInjector<'a>(&'a mut Vec<(String, String)>);

impl Injector for VecInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        self.0.push((key.to_string(), value));
    }
}

/// [`Extractor`] over an `http::HeaderMap` — case-insensitive header
/// lookup per the `Extractor` contract.
struct HeaderMapExtractor<'a>(&'a http::HeaderMap);

impl Extractor for HeaderMapExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(http::HeaderName::as_str).collect()
    }
}

/// W3C `traceparent`/`tracestate` headers carrying the *current* span's
/// context — inject into outbound requests. Empty when the current
/// span has no valid OTEL context (export off or no span entered), so
/// callers can apply unconditionally.
pub fn trace_headers() -> Vec<(String, String)> {
    let cx = tracing::Span::current().context();
    let mut headers = Vec::new();
    global::get_text_map_propagator(|p| p.inject_context(&cx, &mut VecInjector(&mut headers)));
    headers
}

/// The inbound `http_request` server span: `method`/`target` fields plus
/// the remote parent extracted from `traceparent`/`tracestate` (no
/// remote → a local-root span, which is the normal browser path).
/// Record `http.status_code` (and `otel.status_code = "error"` on 5xx)
/// onto it once the response is known.
pub fn server_span(
    method: &http::Method,
    target: &str,
    headers: &http::HeaderMap,
) -> tracing::Span {
    let parent = global::get_text_map_propagator(|p| p.extract(&HeaderMapExtractor(headers)));
    let span = tracing::info_span!(
        "http_request",
        http.request.method = %method,
        url.path = %target,
        otel.kind = "server",
        otel.status_code = tracing::field::Empty,
        http.status_code = tracing::field::Empty,
    );
    if parent.span().span_context().is_valid() {
        let _ = span.set_parent(parent);
    }
    span
}

/// Record the terminal response status onto a [`server_span`] — marks
/// 5xx `ERROR` per the OTLP server-span convention.
pub fn record_response(span: &tracing::Span, status: u16) {
    span.record("http.status_code", status);
    if status >= 500 {
        span.record("otel.status_code", "error");
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::items_after_statements
    )]
    use super::*;

    fn spec() -> TelemetrySpec {
        TelemetrySpec::service("test")
    }

    #[test]
    fn disabled_guard_is_inert() {
        let guard = TelemetryGuard {
            tracer: None,
            meter: None,
        };
        assert!(!guard.is_enabled());
        guard.force_flush();
    }

    #[test]
    fn provider_builds_and_guard_flushes() {
        let provider = build_tracer_provider(
            &TelemetryConfig {
                endpoint: Some("http://127.0.0.1:4318/v1/traces".into()),
                service_name: "test".into(),
            },
            "test",
        )
        .expect("exporter builds without connecting");
        let guard = TelemetryGuard::from_providers(Some(provider), None);
        assert!(guard.is_enabled());
        guard.force_flush();
        // Drop flushes + shuts the provider down.
    }

    #[test]
    fn init_installs_tracing_only_when_unconfigured() {
        // One-shot global — one init() call per test process (nextest
        // isolates tests into their own processes).
        let guard = init(&spec(), "test-scope");
        assert_eq!(guard.tracer.is_some(), spec().resolve_env().is_some());
    }

    #[test]
    fn init_with_builds_the_otlp_layer() {
        // The `Some(config)` → provider arm: real exporter build, no
        // connection needed.
        let guard = init_with(
            Some(TelemetryConfig {
                endpoint: Some("http://127.0.0.1:4318/v1/traces".into()),
                service_name: "test".into(),
            }),
            "test-scope",
        );
        assert!(guard.is_enabled());
    }

    #[test]
    fn init_with_degrades_on_a_bad_endpoint() {
        // `build_tracer_provider` fails → tracing-only + the warn arm.
        let guard = init_with(
            Some(TelemetryConfig {
                endpoint: Some(":// definitely not a url".into()),
                service_name: "test".into(),
            }),
            "test-scope",
        );
        assert!(!guard.is_enabled());
    }

    #[test]
    fn drop_logs_shutdown_errors() {
        // A second shutdown errs — Drop takes the warn arm.
        let provider = build_tracer_provider(
            &TelemetryConfig {
                endpoint: Some("http://127.0.0.1:4318/v1/traces".into()),
                service_name: "test".into(),
            },
            "test",
        )
        .expect("exporter builds without connecting");
        let _ = provider.shutdown();
        let guard = TelemetryGuard::from_providers(Some(provider), None);
        drop(guard);
    }

    #[test]
    fn tracer_provider_builds_from_flag_dest() {
        let provider = tracer_provider(&spec(), Some("http://127.0.0.1:4318"))
            .expect("exporter builds without connecting");
        let guard = TelemetryGuard::from_providers(Some(provider), None);
        assert!(guard.is_enabled());
    }

    #[test]
    fn meter_provider_builds_from_flag_dest() {
        let provider = meter_provider(&spec(), Some("http://127.0.0.1:4318"))
            .expect("exporter builds without connecting");
        let guard = TelemetryGuard::from_providers(None, Some(provider));
        assert!(guard.is_enabled());
    }

    #[test]
    fn propagation_round_trip() {
        global::set_text_map_propagator(TraceContextPropagator::new());
        // Inject a fabricated valid remote context — covers VecInjector
        // deterministically (a bare tracing span carries no OTEL
        // context without the exporter layer).
        use opentelemetry::trace::{SpanContext, SpanId, TraceFlags, TraceId, TraceState};
        let sc = SpanContext::new(
            TraceId::from(0xabc_u128),
            SpanId::from(0xdef_u64),
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        );
        let cx = opentelemetry::Context::new().with_remote_span_context(sc);
        let mut headers: Vec<(String, String)> = Vec::new();
        global::get_text_map_propagator(|p| {
            p.inject_context(&cx, &mut VecInjector(&mut headers));
        });
        assert!(headers.iter().any(|(k, _)| k == "traceparent"));
        let mut map = http::HeaderMap::new();
        for (k, v) in &headers {
            map.insert(
                http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        // Extraction attaches the remote parent (valid context arm).
        let server = server_span(&http::Method::GET, "/x", &map);
        record_response(&server, 503);
        record_response(&server, 200);
        // `trace_headers` on a bare span context emits nothing — still
        // exercises the injector path.
        let _ = trace_headers();
        // A garbage traceparent degrades to a local-root span.
        let mut bad = http::HeaderMap::new();
        bad.insert("traceparent", "nope".parse().unwrap());
        let _ = server_span(&http::Method::POST, "/y", &bad);
    }

    #[test]
    fn extractor_enumerates_keys() {
        let mut map = http::HeaderMap::new();
        map.insert("x-a", "1".parse().unwrap());
        let ex = HeaderMapExtractor(&map);
        assert_eq!(ex.get("x-a"), Some("1"));
        assert!(ex.get("missing").is_none());
        assert_eq!(ex.keys(), vec!["x-a"]);
    }
}
