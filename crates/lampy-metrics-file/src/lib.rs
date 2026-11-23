//! lampy-metrics-file — a Prometheus textfile metrics sink.
//!
//! [`TextfileMetrics`] keeps an in-memory series set and rewrites the
//! target `.prom` file atomically (tmp + rename) on every [`set`] —
//! the shape node_exporter's textfile collector scrapes. Writes are
//! best-effort: telemetry must never break the pipeline it observes,
//! so I/O failures are swallowed (same contract as the bupha
//! `Telemetry` port this absorbs).
//!
//! ```no_run
//! use lampy_metrics_file::{MetricKind, TextfileMetrics};
//!
//! let metrics = TextfileMetrics::new("/var/lib/node_exporter/bupha.prom");
//! metrics.set("bupha_runs_total", "Completed CI runs.", MetricKind::Counter, 41.0);
//! metrics.set("bupha_run_wall_seconds", "Wall time of the last run.", MetricKind::Gauge, 12.0);
//! ```

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// The Prometheus metric type — rendered as the `# TYPE` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    /// `# TYPE <name> counter` — monotonically increasing.
    Counter,
    /// `# TYPE <name> gauge` — a point-in-time value.
    Gauge,
}

impl MetricKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
        }
    }
}

/// One label set → value. Labels render sorted for deterministic
/// output.
type Labels = Vec<(String, String)>;

#[derive(Debug)]
struct Series {
    help: String,
    kind: MetricKind,
    values: BTreeMap<Labels, f64>,
}

/// An atomic Prometheus textfile writer. Clone-free, `Send + Sync` —
/// share one behind an `Arc` across pipeline workers.
#[derive(Debug)]
pub struct TextfileMetrics {
    path: PathBuf,
    state: Mutex<BTreeMap<String, Series>>,
}

impl TextfileMetrics {
    /// A sink writing `path` (parent dirs are NOT created — the
    /// collector's textfile dir is the deploy's job).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            state: Mutex::new(BTreeMap::new()),
        }
    }

    /// Set `name`'s value (unlabeled series) and flush the file.
    /// `help` is recorded the first time the series is seen; a later
    /// `help` for the same name is ignored (Prometheus requires one
    /// HELP per metric name).
    pub fn set(&self, name: &str, help: &str, kind: MetricKind, value: f64) {
        self.set_labeled(name, help, kind, &[], value);
    }

    /// [`TextfileMetrics::set`] with a label set — `&[("repo", "x")]`.
    pub fn set_labeled(
        &self,
        name: &str,
        help: &str,
        kind: MetricKind,
        labels: &[(&str, &str)],
        value: f64,
    ) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let series = state.entry(name.to_string()).or_insert_with(|| Series {
            help: help.to_string(),
            kind,
            values: BTreeMap::new(),
        });
        let labels = labels
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        series.values.insert(labels, value);
        drop(state);
        self.flush();
    }

    /// Rewrite the `.prom` file from current state — tmp + rename so
    /// scrapers never read a partial file. Best-effort: errors are
    /// swallowed (fire-and-forget telemetry).
    pub fn flush(&self) {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out = String::new();
        for (name, series) in state.iter() {
            out.push_str("# HELP ");
            out.push_str(name);
            out.push(' ');
            out.push_str(&series.help);
            out.push('\n');
            out.push_str("# TYPE ");
            out.push_str(name);
            out.push(' ');
            out.push_str(series.kind.as_str());
            out.push('\n');
            for (labels, value) in &series.values {
                out.push_str(name);
                if !labels.is_empty() {
                    out.push('{');
                    for (i, (k, v)) in labels.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        out.push_str(k);
                        out.push_str("=\"");
                        out.push_str(&escape_label(v));
                        out.push('"');
                    }
                    out.push('}');
                }
                out.push(' ');
                out.push_str(&format_value(*value));
                out.push('\n');
            }
        }
        drop(state);
        let tmp = self.path.with_extension("tmp");
        if std::fs::File::create(&tmp)
            .and_then(|mut f| f.write_all(out.as_bytes()))
            .is_ok()
        {
            let _ = std::fs::rename(tmp, &self.path);
        }
    }
}

fn escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn format_value(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn set_writes_prom_textfile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("metrics.prom");
        let metrics = TextfileMetrics::new(&path);
        metrics.set(
            "bupha_runs_total",
            "Completed CI runs.",
            MetricKind::Counter,
            41.0,
        );
        metrics.set(
            "bupha_run_wall_seconds",
            "Wall time of the last run.",
            MetricKind::Gauge,
            12.5,
        );
        let body = std::fs::read_to_string(&path).expect("prom file written");
        assert!(body.contains("# HELP bupha_runs_total Completed CI runs."));
        assert!(body.contains("# TYPE bupha_runs_total counter"));
        assert!(body.contains("bupha_runs_total 41"));
        assert!(body.contains("# TYPE bupha_run_wall_seconds gauge"));
        assert!(body.contains("bupha_run_wall_seconds 12.5"));
    }

    #[test]
    fn labeled_series_render_sorted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("m.prom");
        let metrics = TextfileMetrics::new(&path);
        metrics.set_labeled(
            "runs",
            "Runs by repo.",
            MetricKind::Counter,
            &[("repo", "b")],
            2.0,
        );
        metrics.set_labeled(
            "runs",
            "Runs by repo.",
            MetricKind::Counter,
            &[("repo", "a")],
            1.0,
        );
        let body = std::fs::read_to_string(&path).expect("written");
        let a = body.find("runs{repo=\"a\"} 1").expect("series a");
        let b = body.find("runs{repo=\"b\"} 2").expect("series b");
        assert!(a < b, "labels sort: {body}");
    }

    #[test]
    fn label_values_are_escaped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("m.prom");
        let metrics = TextfileMetrics::new(&path);
        metrics.set_labeled(
            "runs",
            "Runs.",
            MetricKind::Counter,
            &[("repo", "a\"b\\c")],
            1.0,
        );
        let body = std::fs::read_to_string(&path).expect("written");
        assert!(body.contains("runs{repo=\"a\\\"b\\\\c\"} 1"), "{body}");
    }

    #[test]
    fn unwritable_path_is_swallowed() {
        // Fire-and-forget: a missing parent dir must not panic or
        // surface an error through `set`.
        let metrics = TextfileMetrics::new("/nonexistent-dir-9aia/x.prom");
        metrics.set("x", "x.", MetricKind::Counter, 1.0);
    }

    #[test]
    fn empty_state_writes_empty_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("m.prom");
        let metrics = TextfileMetrics::new(&path);
        metrics.flush();
        assert_eq!(std::fs::read_to_string(&path).expect("written"), "");
    }
}
