//! Telemetry configuration and helpers for OpenTelemetry integration.
//!
//! This module provides configuration types and span-recording utilities
//! that complement the `tracing` spans already emitted by the SDK. When
//! combined with `tracing-opentelemetry`, these spans are exported to any
//! OpenTelemetry-compatible backend.

pub mod config;
pub mod record;
