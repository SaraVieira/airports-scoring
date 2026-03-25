use std::collections::BTreeMap;
use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use chrono::Utc;
use serde::Serialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use super::AppState;

/// A single structured log entry emitted over SSE.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: BTreeMap<String, serde_json::Value>,
}

/// A `tracing_subscriber::Layer` that forwards log events into a
/// `tokio::sync::broadcast` channel for real-time SSE streaming.
pub struct BroadcastLogLayer {
    sender: tokio::sync::broadcast::Sender<LogEntry>,
}

impl BroadcastLogLayer {
    pub fn new(sender: tokio::sync::broadcast::Sender<LogEntry>) -> Self {
        Self { sender }
    }
}

/// Visitor that extracts field values from a tracing event.
struct FieldVisitor {
    message: Option<String>,
    fields: BTreeMap<String, serde_json::Value>,
}

impl FieldVisitor {
    fn new() -> Self {
        Self {
            message: None,
            fields: BTreeMap::new(),
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let val = format!("{:?}", value);
        if field.name() == "message" {
            self.message = Some(val);
        } else {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::String(val));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::Number(n));
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }
}

/// Visitor for extracting span fields.
struct SpanFieldVisitor {
    fields: BTreeMap<String, serde_json::Value>,
}

impl SpanFieldVisitor {
    fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }
}

impl Visit for SpanFieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{:?}", value)),
        );
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::Number(n));
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }
}

/// Storage for span fields so we can include them in log events.
#[derive(Default)]
struct SpanFields {
    fields: BTreeMap<String, serde_json::Value>,
}

impl<S> Layer<S> for BroadcastLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let mut visitor = SpanFieldVisitor::new();
        attrs.record(&mut visitor);

        if let Some(span) = ctx.span(id) {
            span.extensions_mut()
                .insert(SpanFields { fields: visitor.fields });
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        // Collect fields from all parent spans.
        let mut span_fields = BTreeMap::new();
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope {
                if let Some(stored) = span.extensions().get::<SpanFields>() {
                    // Inner spans override outer spans for the same key.
                    for (k, v) in &stored.fields {
                        span_fields.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        // Merge event fields on top of span fields.
        span_fields.extend(visitor.fields);

        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: event.metadata().level().to_string(),
            target: event.metadata().target().to_string(),
            message: visitor.message.unwrap_or_default(),
            fields: span_fields,
        };

        // Best-effort send; ignore errors when there are no active receivers.
        let _ = self.sender.send(entry);
    }
}

// ── SSE handler ─────────────────────────────────────────────────

/// `GET /api/admin/logs/stream` — streams log lines in real-time as SSE.
/// Auth via `?password=` query param since EventSource can't send headers.
pub async fn stream_logs(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, axum::http::StatusCode> {
    // Check admin password from query param
    let expected = std::env::var("ADMIN_PASSWORD").unwrap_or_default();
    if !expected.is_empty() {
        let provided = params.get("password").map(|s| s.as_str()).unwrap_or_default();
        if provided != expected {
            return Err(axum::http::StatusCode::FORBIDDEN);
        }
    }
    let rx = state.log_sender.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(entry) => {
            let json = serde_json::to_string(&entry).unwrap_or_default();
            Some(Ok(Event::default().data(json)))
        }
        // Lagged receiver — skip missed messages and continue.
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => None,
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}
