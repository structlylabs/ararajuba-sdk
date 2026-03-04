//! # ararajuba-core
//!
//! High-level AI SDK functions (Rust port of the Vercel AI SDK `ai` package).
//!
//! Provides `generate_text`, `stream_text`, `generate_object`, `stream_object`,
//! `embed`, `embed_many`, `generate_image`, `generate_speech`, `transcribe`,
//! `rerank`, `generate_video`, and the tool system.

pub mod agent;
pub mod chat;
pub mod embed;
pub mod error;
pub mod generate_image;
pub mod generate_object;
pub mod generate_speech;
pub mod generate_text;
pub mod generate_video;
pub mod middleware;
pub mod registry;
pub mod rerank;
pub mod telemetry;
pub mod tools;
pub mod transcribe;
pub mod types;
pub mod ui;
pub mod util;

// ── Core functions ──────────────────────────────────────────────────────────
pub use generate_text::generate_text::{generate_text, has_tool_call, step_count_is};
pub use generate_text::stream_text::stream_text;
pub use generate_object::generate_object::generate_object;
pub use generate_object::stream_object::stream_object;
pub use embed::embed::{embed, embed_many};
pub use generate_image::generate_image::generate_image;
pub use generate_speech::generate_speech::generate_speech;
pub use transcribe::transcribe::transcribe;
pub use rerank::rerank::rerank;
pub use generate_video::generate_video::generate_video;

// ── Options & Results ───────────────────────────────────────────────────────
pub use generate_text::options::GenerateTextOptions;
pub use generate_text::result::{GenerateTextResult, StepResult};
pub use generate_text::callbacks::{
    ChunkEvent, ErrorEvent, FinishEvent, StartEvent, StepStartEvent,
    ToolCallFinishEvent, ToolCallStartEvent,
};
pub use generate_text::prepare_step::{PrepareStepContext, PrepareStepFn, PrepareStepResult};
pub use generate_object::options::{GenerateObjectOptions, GenerateObjectFinishEvent, RepairTextFn};
pub use generate_object::output::{array_output, choice, json_output, object_output};
pub use embed::options::{EmbedManyOptions, EmbedOptions};
pub use embed::result::{EmbedManyResult, EmbedResult};
pub use generate_image::generate_image::{GenerateImageOptions, GenerateImageResult};
pub use generate_speech::generate_speech::{GenerateSpeechOptions, GenerateSpeechResult};
pub use transcribe::transcribe::{TranscribeOptions, TranscribeResult};
pub use rerank::rerank::{RerankHighLevelResult, RerankOptions};
pub use generate_video::generate_video::{GenerateVideoOptions, GenerateVideoResult};

// ── Tool system ─────────────────────────────────────────────────────────────
pub use tools::dynamic_tool::{dynamic_tool, DynamicToolCall, DynamicToolResult, TypedToolCall};
pub use tools::tool::{tool, ToolDef};
pub use tools::tool_approval::{OnToolApproval, ToolApprovalRequest, ToolApprovalResponse};
pub use tools::tool_call::ToolCall;
pub use tools::tool_result::ToolResult;
pub use tools::tool_set::ToolSet;

// ── Shared types ────────────────────────────────────────────────────────────
pub use types::call_settings::CallSettings;
pub use types::call_warning::CallWarning;
pub use types::finish_reason::FinishReason;

// ── Middleware ───────────────────────────────────────────────────────────────
pub use middleware::wrap_language_model::{
    wrap_language_model, wrap_language_model_chain, DoGenerate, DoStream, LanguageModelMiddleware,
    MiddlewareModelRef,
};pub use middleware::wrap_embedding_model::{
    wrap_embedding_model, DoEmbed, EmbeddingMiddlewareModelRef, EmbeddingModelMiddleware,
};
pub use middleware::wrap_image_model::{
    wrap_image_model, DoGenerateImage, ImageMiddlewareModelRef, ImageModelMiddleware,
};
pub use middleware::wrap_provider::{wrap_provider, WrapProviderOptions};
// ── Re-export provider traits ───────────────────────────────────────────────
pub use ararajuba_provider::{
    EmbeddingModelV4, ImageModelV4, LanguageModelV4, Provider, RerankingModelV4, SpeechModelV4,
    TranscriptionModelV4, VideoModelV4,
};

// ── Errors ──────────────────────────────────────────────────────────────────
pub use error::Error;

// ── Registry ────────────────────────────────────────────────────────────────
pub use registry::{CustomProvider, ModelRegistry};

// ── Agent ────────────────────────────────────────────────────────────────────
pub use agent::agent::{Agent, AgentSettings, AGENT_VERSION};
pub use agent::agent_ui_stream::{
    create_agent_ui_stream, create_agent_ui_stream_response, pipe_agent_ui_stream_to_response,
    AgentUIStreamOptions,
};

// ── Chat Framework ──────────────────────────────────────────────────────────
pub use chat::chat::{Chat, ChatInit, IdGenerator, default_id_generator};
pub use chat::transport::ChatTransport;
pub use chat::direct_transport::DirectChatTransport;
pub use chat::types::{
    ChatFinishInfo, ChatFinishReason, ChatRequestOptions, ChatStatus, ChatTrigger,
    OnChatError, OnChatFinish, ReconnectOptions, SendMessagesOptions,
};

// ── Schema Utilities ────────────────────────────────────────────────────────
pub use types::schema::{as_schema, json_schema, Schema, ValidateFn, ValidationResult};

// ── UI Message Stream ──────────────────────────────────────────────────────────
pub use ui::chunk::UIMessageChunk;
pub use ui::stream::{chunks_to_sse, create_ui_message_stream};

// ── UI Types & Predicates ──────────────────────────────────────────────────────
pub use ui::types::{
    DataUIPart, DynamicToolUIPart, FileUIPart, ReasoningUIPart, SourceDocumentUIPart,
    SourceUrlUIPart, StepStartUIPart, TextUIPart, ToolInvocationState, ToolUIPart, UIMessage,
    UIMessageRole, UIPart,
};
pub use ui::types::{
    get_text_from_ui_message, get_tool_name, is_data_ui_part, is_dynamic_tool_ui_part,
    is_file_ui_part, is_reasoning_ui_part, is_source_document_ui_part, is_source_url_ui_part,
    is_step_start_ui_part, is_text_ui_part, is_tool_ui_part,
};
pub use ui::validate::{safe_validate_ui_messages, validate_ui_messages, ValidationIssue};
pub use ui::convert::{convert_file_list_to_file_ui_parts, convert_to_model_messages};
pub use ui::reader::{read_ui_message_stream, split_sse_and_parse};
pub use ui::response::{
    create_text_stream_response, create_ui_message_stream_response,
    pipe_text_stream_to_response, pipe_ui_message_stream_to_response, SseResponse,
};

// ── Telemetry ──────────────────────────────────────────────────────────────
pub use telemetry::config::TelemetrySettings;pub use telemetry::record::{
    record_generation_span, record_response_attributes, record_step_span, record_tool_call_span,
};

// ── Utilities ──────────────────────────────────────────────────────────
pub use util::consume_stream::{collect_stream, consume_stream};
pub use util::cosine_similarity::{cosine_similarity, cosine_similarity_f32};
pub use util::data_url::{get_mime_type_from_data_url, get_text_from_data_url};
pub use util::deep_equal::is_deep_equal_data;
pub use util::parse_partial_json::parse_partial_json;
pub use util::prune_messages::{
    estimate_token_count, prune_messages, PruneMessagesOptions, PruneStrategy,
};
pub use util::serial_executor::SerialJobExecutor;
pub use util::simulate_readable_stream::{simulate_readable_stream, SimulateReadableStreamOptions};
pub use util::smooth_stream::{smooth_stream, Chunking, SmoothStreamOptions};