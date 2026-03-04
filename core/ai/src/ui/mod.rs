//! UI message stream — transforms model streams into structured SSE events
//! for consumption by frontend frameworks.

pub mod chunk;
pub mod convert;
pub mod reader;
pub mod response;
pub mod stream;
pub mod types;
pub mod validate;
