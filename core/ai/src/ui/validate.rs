//! UI message validation — validate and safe-validate UI messages.

use super::types::{UIMessage, UIMessageRole, UIPart};
use crate::error::Error;

/// Validation error detail.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Path to the invalid field (e.g., "messages[0].parts[1]").
    pub path: String,
    /// Human-readable description of the issue.
    pub message: String,
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Validate a list of UI messages.
///
/// Returns `Ok(())` if all messages are valid, or an `Error` with details
/// about the first invalid message.
pub fn validate_ui_messages(messages: &[UIMessage]) -> Result<(), Error> {
    let issues = collect_validation_issues(messages);
    if issues.is_empty() {
        Ok(())
    } else {
        let detail = issues
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        Err(Error::InvalidArgument {
            message: format!("Invalid UI messages: {detail}"),
        })
    }
}

/// Safe-validate a list of UI messages.
///
/// Returns a list of validation issues (empty if all valid).
/// Never returns an error — issues are returned as data.
pub fn safe_validate_ui_messages(messages: &[UIMessage]) -> Vec<ValidationIssue> {
    collect_validation_issues(messages)
}

fn collect_validation_issues(messages: &[UIMessage]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        let msg_path = format!("messages[{i}]");

        // ID must be non-empty.
        if msg.id.is_empty() {
            issues.push(ValidationIssue {
                path: format!("{msg_path}.id"),
                message: "Message ID must not be empty".into(),
            });
        }

        // Parts must not be empty (except for system messages).
        if msg.parts.is_empty() && msg.role != UIMessageRole::System {
            issues.push(ValidationIssue {
                path: format!("{msg_path}.parts"),
                message: "Message must have at least one part".into(),
            });
        }

        // Validate individual parts.
        for (j, part) in msg.parts.iter().enumerate() {
            let part_path = format!("{msg_path}.parts[{j}]");
            validate_part(part, &part_path, &mut issues);
        }
    }

    issues
}

fn validate_part(part: &UIPart, path: &str, issues: &mut Vec<ValidationIssue>) {
    match part {
        UIPart::Text(t) => {
            if t.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Text part ID must not be empty".into(),
                });
            }
        }
        UIPart::Reasoning(r) => {
            if r.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Reasoning part ID must not be empty".into(),
                });
            }
        }
        UIPart::Tool(t) => {
            if t.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Tool part ID must not be empty".into(),
                });
            }
            if t.tool_name.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.tool_name"),
                    message: "Tool name must not be empty".into(),
                });
            }
        }
        UIPart::DynamicTool(t) => {
            if t.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Dynamic tool part ID must not be empty".into(),
                });
            }
        }
        UIPart::File(f) => {
            if f.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "File part ID must not be empty".into(),
                });
            }
            if f.media_type.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.media_type"),
                    message: "File media_type must not be empty".into(),
                });
            }
        }
        UIPart::Data(d) => {
            if d.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Data part ID must not be empty".into(),
                });
            }
        }
        UIPart::SourceUrl(s) => {
            if s.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Source URL part ID must not be empty".into(),
                });
            }
            if s.url.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.url"),
                    message: "Source URL must not be empty".into(),
                });
            }
        }
        UIPart::SourceDocument(s) => {
            if s.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Source document part ID must not be empty".into(),
                });
            }
        }
        UIPart::StepStart(s) => {
            if s.id.is_empty() {
                issues.push(ValidationIssue {
                    path: format!("{path}.id"),
                    message: "Step start part ID must not be empty".into(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::types::TextUIPart;

    #[test]
    fn test_valid_message_passes() {
        let messages = vec![UIMessage {
            id: "msg1".into(),
            role: UIMessageRole::Assistant,
            parts: vec![UIPart::Text(TextUIPart {
                id: "t1".into(),
                text: "Hello".into(),
            })],
            metadata: None,
            created_at: None,
        }];
        assert!(validate_ui_messages(&messages).is_ok());
    }

    #[test]
    fn test_empty_id_fails() {
        let messages = vec![UIMessage {
            id: "".into(),
            role: UIMessageRole::User,
            parts: vec![UIPart::Text(TextUIPart {
                id: "t1".into(),
                text: "Hi".into(),
            })],
            metadata: None,
            created_at: None,
        }];
        assert!(validate_ui_messages(&messages).is_err());
    }

    #[test]
    fn test_empty_parts_non_system_fails() {
        let messages = vec![UIMessage {
            id: "msg1".into(),
            role: UIMessageRole::User,
            parts: vec![],
            metadata: None,
            created_at: None,
        }];
        assert!(validate_ui_messages(&messages).is_err());
    }

    #[test]
    fn test_empty_parts_system_ok() {
        let messages = vec![UIMessage {
            id: "msg1".into(),
            role: UIMessageRole::System,
            parts: vec![],
            metadata: None,
            created_at: None,
        }];
        assert!(validate_ui_messages(&messages).is_ok());
    }

    #[test]
    fn test_safe_validate_returns_issues() {
        let messages = vec![UIMessage {
            id: "".into(),
            role: UIMessageRole::User,
            parts: vec![],
            metadata: None,
            created_at: None,
        }];
        let issues = safe_validate_ui_messages(&messages);
        assert!(issues.len() >= 2); // empty id + empty parts
    }
}
