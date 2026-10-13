use crate::config;
use opentelemetry::trace::{Link, SpanKind, TraceId};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::{Config, Sampler, SamplingResult, SdkTracerProvider, ShouldSample};
use tracing::warn;

/// Builds the configured trace provider, returning `None` when disabled or initialization fails.
///
/// The caller owns flushing and shutdown of the returned provider.
pub fn tracer_provider(config: &config::SharedConfig) -> Option<SdkTracerProvider> {
    if !config.otel.tracing_enabled {
        return None;
    }

    match otlp_tracer_provider(config) {
        Ok(provider) => Some(provider),
        Err(error) => {
            warn!("Failed to initialize OpenTelemetry tracing: {error:#}");
            None
        }
    }
}

/// Builds a provider with a batched OTLP/HTTP exporter, configured through the
/// SDK's standard environment variables.
fn otlp_tracer_provider(config: &config::SharedConfig) -> anyhow::Result<SdkTracerProvider> {
    let resource = crate::metrics::resource(config)?;
    let exporter = SpanExporter::builder().with_http().build()?;

    // Child spans follow their root, so a sampled job keeps all of its spans.
    let baseline = Config::default().sampler;
    let sampler = Sampler::ParentBased(Box::new(CratesSampler { baseline }));

    Ok(SdkTracerProvider::builder()
        .with_resource(resource)
        .with_sampler(sampler)
        .with_batch_exporter(exporter)
        .build())
}

/// Samples all background job traces and defers other root spans to the
/// sampler configured through the SDK's environment variables.
#[derive(Clone, Debug)]
struct CratesSampler {
    baseline: Box<dyn ShouldSample>,
}

impl ShouldSample for CratesSampler {
    fn should_sample(
        &self,
        parent: Option<&Context>,
        trace_id: TraceId,
        name: &str,
        kind: &SpanKind,
        attributes: &[KeyValue],
        links: &[Link],
    ) -> SamplingResult {
        let is_job = attributes.iter().any(|kv| kv.key.as_str() == "job.typ");
        let sampler: &dyn ShouldSample = match is_job {
            true => &Sampler::AlwaysOn,
            false => self.baseline.as_ref(),
        };
        sampler.should_sample(parent, trace_id, name, kind, attributes, links)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_sdk::trace::SamplingDecision;

    fn decision(attributes: &[KeyValue]) -> SamplingDecision {
        let sampler = CratesSampler {
            baseline: Box::new(Sampler::AlwaysOff),
        };
        let trace_id = TraceId::from(1);
        let kind = SpanKind::Internal;
        let result = sampler.should_sample(None, trace_id, "test", &kind, attributes, &[]);
        result.decision
    }

    #[test]
    fn test_jobs_are_always_sampled() {
        let attributes = [KeyValue::new("job.typ", "sync_admins")];
        assert_eq!(decision(&attributes), SamplingDecision::RecordAndSample);
    }

    #[test]
    fn test_other_spans_use_baseline() {
        assert_eq!(decision(&[]), SamplingDecision::Drop);
    }
}
