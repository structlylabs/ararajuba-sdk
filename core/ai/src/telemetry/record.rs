//! `record_span` — utility for recording OpenTelemetry-compatible spans
//! via the `tracing` crate.
//!
//! These helpers create `tracing` spans with attributes that follow the
//! GenAI semantic conventions. When a `tracing-opentelemetry` layer is
//! installed, these spans are exported as OpenTelemetry spans.

use super::config::TelemetrySettings;

/// Record a span for a top-level generation call (e.g., `generateText`,
/// `streamText`).
///
/// If telemetry is disabled, returns immediately without creating a span.
///
/// # Arguments
/// - `telemetry` — the telemetry settings
/// - `operation` — e.g., `"generateText"`, `"streamText"`
/// - `provider` — the provider name
/// - `model_id` — the model identifier
///
/// Returns an `Option<tracing::span::EnteredSpan>` that keeps the span
/// alive. Drop it when the operation is done.
pub fn record_generation_span(
    telemetry: &TelemetrySettings,
    operation: &str,
    provider: &str,
    model_id: &str,
) -> Option<tracing::span::EnteredSpan> {
    if !telemetry.is_enabled {
        return None;
    }

    let op_name = telemetry.operation_name(operation);
    let function_id = telemetry
        .function_id
        .as_deref()
        .unwrap_or("");

    let span = tracing::info_span!(
        "ai.generation",
        "operation.name" = %op_name,
        "ai.model.provider" = %provider,
        "ai.model.id" = %model_id,
        "ai.telemetry.functionId" = %function_id,
        "gen_ai.system" = %provider,
        "gen_ai.request.model" = %model_id,
    );

    // Attach metadata as span attributes.
    // Note: tracing doesn't support dynamic attributes on span creation,
    // so we record them as events within the span.
    let entered = span.entered();

    if !telemetry.metadata.is_empty() {
        for (key, value) in &telemetry.metadata {
            tracing::info!(
                parent: &entered,
                metadata_key = %key,
                metadata_value = %value,
                "ai.telemetry.metadata"
            );
        }
    }

    Some(entered)
}

/// Record response attributes on the current span after a generation completes.
///
/// Call this inside the span created by `record_generation_span`.
pub fn record_response_attributes(
    telemetry: &TelemetrySettings,
    finish_reason: &str,
    text: Option<&str>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
) {
    if !telemetry.is_enabled {
        return;
    }

    tracing::info!(
        "ai.response.finishReason" = %finish_reason,
        "gen_ai.response.finish_reasons" = %finish_reason,
    );

    if telemetry.record_outputs {
        if let Some(t) = text {
            // Truncate to avoid massive span attributes.
            let truncated = if t.len() > 1000 { &t[..1000] } else { t };
            tracing::info!("ai.response.text" = %truncated);
        }
    }

    if let Some(input) = input_tokens {
        tracing::info!(
            "ai.usage.promptTokens" = input,
            "gen_ai.usage.input_tokens" = input,
        );
    }

    if let Some(output) = output_tokens {
        tracing::info!(
            "ai.usage.completionTokens" = output,
            "gen_ai.usage.output_tokens" = output,
        );
    }
}

/// Record a span for a single model invocation step within a multi-step
/// generation.
pub fn record_step_span(
    telemetry: &TelemetrySettings,
    operation: &str,
    step: usize,
) -> Option<tracing::span::EnteredSpan> {
    if !telemetry.is_enabled {
        return None;
    }

    let span = tracing::debug_span!(
        "ai.generation.step",
        "ai.operation" = %operation,
        step = step,
    );

    Some(span.entered())
}

/// Record a span for a tool call execution.
pub fn record_tool_call_span(
    telemetry: &TelemetrySettings,
    tool_name: &str,
    tool_call_id: &str,
) -> Option<tracing::span::EnteredSpan> {
    if !telemetry.is_enabled {
        return None;
    }

    let span = tracing::debug_span!(
        "ai.toolCall",
        tool_name = %tool_name,
        tool_call_id = %tool_call_id,
    );

    Some(span.entered())
}
