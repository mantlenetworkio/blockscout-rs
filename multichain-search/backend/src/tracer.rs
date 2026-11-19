// SPDX-License-Identifier: LicenseRef-Blockscout

use crate::settings::JaegerSettings;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    propagation::TraceContextPropagator, trace::SdkTracer, trace::SdkTracerProvider, Resource,
};
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, prelude::*};

pub fn init_logs(jaeger_settings: JaegerSettings) {
    let stdout = tracing_subscriber::fmt::layer().with_filter(
        tracing_subscriber::EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy(),
    );
    let registry = tracing_subscriber::registry()
        // output logs (tracing) to stdout with log level taken from env (default is INFO)
        .with(stdout);
    if jaeger_settings.enabled {
        let tracer =
            init_jaeger_tracer(&jaeger_settings.agent_endpoint).expect("failed to init tracer");
        registry
            // output traces to the collector with default log level (default is DEBUG)
            .with(
                tracing_opentelemetry::layer()
                    .with_tracer(tracer)
                    .with_filter(LevelFilter::DEBUG),
            )
            .try_init()
    } else {
        registry.try_init()
    }
    .expect("failed to register tracer with registry");
}

fn init_jaeger_tracer(agent_endpoint: &str) -> anyhow::Result<SdkTracer> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    // The endpoint is expected to be an OTLP gRPC endpoint (e.g. `http://127.0.0.1:4317`).
    // Jaeger (>= 1.35) and the OpenTelemetry Collector both accept OTLP natively.
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(agent_endpoint)
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name("multichain-search".to_string())
                .build(),
        )
        .build();
    Ok(provider.tracer("multichain-search"))
}
