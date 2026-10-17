#![cfg(feature = "opentelemetry")]

use fluxdi::opentelemetry_layer;
use opentelemetry::trace::{TracerProvider, noop::NoopTracer};
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};
use tracing_subscriber::layer::SubscriberExt;

#[test]
fn builds_opentelemetry_layer_from_noop_tracer() {
    let _layer = opentelemetry_layer(NoopTracer::new());
}

#[test]
fn exports_sdk_spans_with_parent_context() {
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let subscriber =
        tracing_subscriber::registry().with(opentelemetry_layer(provider.tracer("fluxdi-test")));

    tracing::subscriber::with_default(subscriber, || {
        let parent = tracing::info_span!("parent");
        let _parent_guard = parent.enter();
        let child = tracing::info_span!("child");
        let _child_guard = child.enter();
    });
    provider.force_flush().expect("flush spans");

    let spans = exporter.get_finished_spans().expect("read exported spans");
    assert_eq!(spans.len(), 2);
    let parent = spans
        .iter()
        .find(|span| span.name == "parent")
        .expect("parent span");
    let child = spans
        .iter()
        .find(|span| span.name == "child")
        .expect("child span");
    assert!(parent.span_context.is_valid());
    assert_eq!(child.parent_span_id, parent.span_context.span_id());
    assert_eq!(
        child.span_context.trace_id(),
        parent.span_context.trace_id()
    );
    provider.shutdown().expect("shutdown provider");
}
