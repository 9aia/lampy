//! lampy-telemetry — the telemetry **port**: the org-wide env
//! contract, the `--tracing`/`--metrics`-style signal specs, and the
//! provider vocabulary every consumer resolves against. Pure — no
//! OTEL, no I/O; impl crates (`lampy-telemetry-otel`,
//! `lampy-metrics-file`) interpret these specs.
//!
//! Env contract (canonical prefix `LAMPY_OTEL`, per-consumer
//! `<ALIAS>_OTEL_*` legacy aliases, then the standard `OTEL_*`
//! vars):
//!
//! - `LAMPY_OTEL_ENDPOINT` — OTLP/HTTP base URL
//!   (`http://collector:4318`); `/v1/traces` is appended when absent,
//!   mirroring `OTEL_EXPORTER_OTLP_ENDPOINT` semantics.
//! - `<ALIAS>_OTEL_ENDPOINT` / `<ALIAS>_OTEL_SERVICE_NAME` — the
//!   pre-lampy per-consumer vars (`HYPHA_OTEL_*`, `SEPIA_OTEL_*`,
//!   `TAMIAS_OTEL_*`, …), still honored as aliases.
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` / `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`
//!   — the standard vars also enable export (full `OTEL_EXPORTER_*`
//!   config: headers, timeout, compression, protocol).
//! - `LAMPY_OTEL_SERVICE_NAME` — `service.name` resource; falls back
//!   to the alias `*_OTEL_SERVICE_NAME`, then `OTEL_SERVICE_NAME`,
//!   then the spec's default service.
//! - `OTEL_SERVICE_NAME` / `OTEL_RESOURCE_ATTRIBUTES` — honored by
//!   the SDK's env resource detector.

mod spec;

pub use spec::{Observability, Provider, SignalSpec, TelemetryError};

/// Canonical env prefix — `LAMPY_OTEL_*` is the unified contract;
/// every consumer reads it first.
pub const ENV_PREFIX: &str = "LAMPY_OTEL";
/// `LAMPY_OTEL_ENDPOINT` — the canonical opt-in switch. OTLP/HTTP
/// base URL.
pub const ENV_OTEL_ENDPOINT: &str = "LAMPY_OTEL_ENDPOINT";
/// `LAMPY_OTEL_SERVICE_NAME` — canonical `service.name` override.
pub const ENV_OTEL_SERVICE_NAME: &str = "LAMPY_OTEL_SERVICE_NAME";
/// Standard endpoint vars the exporter honors — also enable export.
pub const OTEL_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";
pub const OTEL_TRACES_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT";
pub const OTEL_SERVICE_NAME: &str = "OTEL_SERVICE_NAME";

/// OTLP/HTTP signal paths appended to base-URL endpoints (mirrors
/// the exporter's own `OTEL_EXPORTER_OTLP_ENDPOINT` handling).
pub const TRACES_PATH: &str = "/v1/traces";
pub const METRICS_PATH: &str = "/v1/metrics";

/// `base` → the signal's full URL: verbatim when it already names
/// `path`, else the path appended to the trimmed base.
pub fn signal_url(base: &str, path: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if base.ends_with(path) {
        base.to_string()
    } else {
        format!("{base}{path}")
    }
}

/// What `init` needs from the environment for one consumer — the
/// service's own name plus any legacy per-consumer env prefixes it
/// still honors as aliases.
#[derive(Debug, Clone)]
pub struct TelemetrySpec {
    /// `service.name` fallback — the consumer's own name (e.g.
    /// `"hypha"`, `"sepia"`).
    pub service: String,
    /// Legacy env prefixes honored as aliases, in priority order —
    /// `"HYPHA"` reads `HYPHA_OTEL_ENDPOINT` /
    /// `HYPHA_OTEL_SERVICE_NAME`. Canonical `LAMPY_OTEL_*` always
    /// wins over aliases.
    pub aliases: Vec<String>,
}

impl TelemetrySpec {
    /// A spec for `service` with no legacy aliases.
    #[must_use]
    pub fn service(service: &str) -> Self {
        Self {
            service: service.to_string(),
            aliases: Vec::new(),
        }
    }

    /// Add a legacy alias prefix (uppercased, `_OTEL_*` suffix
    /// implied). Later aliases lose to earlier ones.
    #[must_use]
    pub fn with_alias(mut self, prefix: &str) -> Self {
        self.aliases.push(prefix.to_uppercase());
        self
    }

    /// `LAMPY_OTEL_<suffix>` canonical first, then each
    /// `<ALIAS>_OTEL_<suffix>` in order — first non-empty wins.
    fn lookup(&self, suffix: &str, get: &dyn Fn(&str) -> Option<String>) -> Option<String> {
        nonempty(get(&format!("{ENV_PREFIX}_{suffix}"))).or_else(|| {
            self.aliases
                .iter()
                .find_map(|alias| nonempty(get(&format!("{alias}_OTEL_{suffix}"))))
        })
    }

    /// Resolve just `service.name` — the env override chain, else
    /// the spec's default service. Unlike [`TelemetrySpec::resolve`]
    /// this does not require an endpoint, so flag-driven providers
    /// (explicit `--tracing`/`--metrics` destinations) still honor
    /// `*_SERVICE_NAME`.
    #[must_use]
    pub fn service_name(&self, get: impl Fn(&str) -> Option<String>) -> String {
        self.lookup("SERVICE_NAME", &get)
            .or_else(|| nonempty(get(OTEL_SERVICE_NAME)))
            .unwrap_or_else(|| self.service.clone())
    }

    /// [`TelemetrySpec::service_name`] against process env.
    #[must_use]
    pub fn service_name_env(&self) -> String {
        self.service_name(|k| std::env::var(k).ok())
    }

    /// Resolve the env contract; `None` means export is off
    /// entirely. `get` is the env read — pure, so tests never touch
    /// process env (edition 2024 makes `set_var` unsafe).
    pub fn resolve(&self, get: impl Fn(&str) -> Option<String>) -> Option<TelemetryConfig> {
        let get = &get;
        let endpoint = self.lookup("ENDPOINT", get).map_or_else(
            || {
                if nonempty(get(OTEL_TRACES_ENDPOINT)).is_some()
                    || nonempty(get(OTEL_ENDPOINT)).is_some()
                {
                    // Let the exporter's own env resolution pick the URL.
                    Some(None)
                } else {
                    None
                }
            },
            |base| Some(Some(signal_url(&base, TRACES_PATH))),
        )?;
        let service_name = self.service_name(|k| get(k));
        Some(TelemetryConfig {
            endpoint,
            service_name,
        })
    }

    /// [`TelemetrySpec::resolve`] against process env — `None` when
    /// no endpoint is configured.
    #[must_use]
    pub fn resolve_env(&self) -> Option<TelemetryConfig> {
        self.resolve(|k| std::env::var(k).ok())
    }

    /// `true` when an OTLP endpoint is configured — the flag callers
    /// use to log "telemetry on" at boot. Reads process env.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.resolve_env().is_some()
    }
}

/// What a [`TelemetrySpec`] resolved to — kept pure so tests never
/// touch process env.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Fully-resolved `/v1/traces` URL passed to `with_endpoint`, or
    /// `None` to let the exporter read `OTEL_EXPORTER_OTLP_*` itself.
    pub endpoint: Option<String>,
    /// `service.name` resource attribute.
    pub service_name: String,
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

/// The `<ALIAS>_OTEL_<SUFFIX>` var name for a legacy prefix.
#[must_use]
pub fn alias_env(prefix: &str, suffix: &str) -> String {
    let mut out = String::with_capacity(prefix.len() + suffix.len() + 7);
    out.push_str(&prefix.to_uppercase());
    out.push_str("_OTEL_");
    out.push_str(suffix);
    out
}

/// The canonical `<SUFFIX>` var name — `LAMPY_OTEL_<SUFFIX>`.
#[must_use]
pub fn canonical_env(suffix: &str) -> String {
    format!("{ENV_PREFIX}_{suffix}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::collections::HashMap;

    fn lookup<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn disabled_without_env() {
        let spec = TelemetrySpec::service("test").with_alias("HYPHA");
        assert!(spec.resolve(lookup(&[])).is_none());
        assert!(
            spec.resolve(lookup(&[("LAMPY_OTEL_ENDPOINT", "")]))
                .is_none()
        );
        assert!(
            spec.resolve(lookup(&[("HYPHA_OTEL_ENDPOINT", "   ")]))
                .is_none()
        );
    }

    #[test]
    fn canonical_endpoint_appends_traces_path() {
        let spec = TelemetrySpec::service("hypha");
        let config = spec
            .resolve(lookup(&[("LAMPY_OTEL_ENDPOINT", "http://localhost:4318/")]))
            .expect("endpoint should enable telemetry");
        assert_eq!(
            config.endpoint.as_deref(),
            Some("http://localhost:4318/v1/traces")
        );
        assert_eq!(config.service_name, "hypha");
    }

    #[test]
    fn alias_endpoint_enables_export() {
        let spec = TelemetrySpec::service("sepia").with_alias("SEPIA");
        let config = spec
            .resolve(lookup(&[("SEPIA_OTEL_ENDPOINT", "http://collector:4318")]))
            .expect("alias endpoint should enable telemetry");
        assert_eq!(
            config.endpoint.as_deref(),
            Some("http://collector:4318/v1/traces")
        );
    }

    #[test]
    fn canonical_wins_over_alias() {
        let spec = TelemetrySpec::service("hypha").with_alias("HYPHA");
        let config = spec
            .resolve(lookup(&[
                ("LAMPY_OTEL_ENDPOINT", "http://new:4318"),
                ("HYPHA_OTEL_ENDPOINT", "http://old:4318"),
            ]))
            .expect("enabled");
        assert_eq!(
            config.endpoint.as_deref(),
            Some("http://new:4318/v1/traces")
        );
    }

    #[test]
    fn earlier_alias_wins_over_later() {
        let spec = TelemetrySpec::service("peregrin")
            .with_alias("PEREGRIN")
            .with_alias("HYPHA");
        let config = spec
            .resolve(lookup(&[
                ("HYPHA_OTEL_ENDPOINT", "http://hypha:4318"),
                ("PEREGRIN_OTEL_ENDPOINT", "http://peregrin:4318"),
            ]))
            .expect("enabled");
        assert_eq!(
            config.endpoint.as_deref(),
            Some("http://peregrin:4318/v1/traces")
        );
    }

    #[test]
    fn endpoint_verbatim_when_path_present() {
        let spec = TelemetrySpec::service("test");
        let config = spec
            .resolve(lookup(&[(
                "LAMPY_OTEL_ENDPOINT",
                "http://collector:4318/v1/traces",
            )]))
            .expect("enabled");
        assert_eq!(
            config.endpoint.as_deref(),
            Some("http://collector:4318/v1/traces")
        );
    }

    #[test]
    fn standard_otlp_vars_enable_export() {
        let spec = TelemetrySpec::service("test");
        for key in [OTEL_ENDPOINT, OTEL_TRACES_ENDPOINT] {
            let config = spec
                .resolve(lookup(&[(key, "http://collector:4318")]))
                .unwrap_or_else(|| panic!("{key} should enable telemetry"));
            // Endpoint left to the exporter's own env resolution.
            assert!(config.endpoint.is_none());
        }
    }

    #[test]
    fn service_name_precedence() {
        let spec = TelemetrySpec::service("hypha").with_alias("HYPHA");
        let config = spec
            .resolve(lookup(&[
                ("LAMPY_OTEL_ENDPOINT", "http://localhost:4318"),
                ("LAMPY_OTEL_SERVICE_NAME", "lampy-svc"),
                ("HYPHA_OTEL_SERVICE_NAME", "hypha-node"),
                ("OTEL_SERVICE_NAME", "otel-name"),
            ]))
            .expect("enabled");
        assert_eq!(config.service_name, "lampy-svc");

        let config = spec
            .resolve(lookup(&[
                ("HYPHA_OTEL_ENDPOINT", "http://localhost:4318"),
                ("HYPHA_OTEL_SERVICE_NAME", "hypha-node"),
                ("OTEL_SERVICE_NAME", "otel-name"),
            ]))
            .expect("enabled");
        assert_eq!(config.service_name, "hypha-node");

        let config = spec
            .resolve(lookup(&[
                ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318"),
                ("OTEL_SERVICE_NAME", "otel-name"),
            ]))
            .expect("enabled");
        assert_eq!(config.service_name, "otel-name");

        let config = spec
            .resolve(lookup(&[("LAMPY_OTEL_ENDPOINT", "http://localhost:4318")]))
            .expect("enabled");
        assert_eq!(config.service_name, "hypha");
    }

    #[test]
    fn signal_url_appends_path_once() {
        assert_eq!(
            signal_url("http://c:4318/", TRACES_PATH),
            "http://c:4318/v1/traces"
        );
        assert_eq!(
            signal_url("http://c:4318/v1/traces", TRACES_PATH),
            "http://c:4318/v1/traces"
        );
        assert_eq!(
            signal_url("http://c:4318", METRICS_PATH),
            "http://c:4318/v1/metrics"
        );
    }

    #[test]
    fn alias_env_names() {
        assert_eq!(alias_env("hypha", "ENDPOINT"), "HYPHA_OTEL_ENDPOINT");
        assert_eq!(canonical_env("SERVICE_NAME"), "LAMPY_OTEL_SERVICE_NAME");
    }
}
