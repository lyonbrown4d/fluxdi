use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Builds an OpenTelemetry tracing layer that can be attached to a subscriber registry.
///
/// This helper is available when the `opentelemetry` feature is enabled and does not
/// install any global tracer provider by itself.
///
/// The tracer must have a thread-safe span type (for example
/// `opentelemetry_sdk::trace::SdkTracer` or `opentelemetry::trace::noop::NoopTracer`).
pub fn opentelemetry_layer<T>(
    tracer: T,
) -> tracing_opentelemetry::OpenTelemetryLayer<tracing_subscriber::Registry, T>
where
    T: opentelemetry::trace::Tracer + Send + Sync + 'static,
    T::Span: Send + Sync,
{
    tracing_opentelemetry::layer().with_tracer(tracer)
}

/// Installs a global tracing subscriber wired with an OpenTelemetry layer.
///
/// This function intentionally only wires `tracing` to a provided tracer. Exporter and
/// tracer provider setup should happen in application code.
pub fn try_init_opentelemetry<T>(tracer: T) -> Result<(), tracing_subscriber::util::TryInitError>
where
    T: opentelemetry::trace::Tracer + Send + Sync + 'static,
    T::Span: Send + Sync,
{
    tracing_subscriber::registry()
        .with(opentelemetry_layer(tracer))
        .try_init()
}
