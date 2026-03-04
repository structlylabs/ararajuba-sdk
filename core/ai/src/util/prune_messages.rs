//! Message pruning — trim message history to fit within a token budget.
//!
//! This is a simplified implementation that prunes messages by estimated
//! character count. In production, you'd use a tokenizer for accurate counts.

use ararajuba_provider::language_model::v4::prompt::Message;

/// Options for pruning messages.
#[derive(Debug, Clone)]
pub struct PruneMessagesOptions {
    /// Maximum total "token" budget. Uses character-based estimation
    /// unless a custom tokenizer is provided.
    pub max_tokens: usize,
    /// Average characters per token (default: 4). Used for estimation.
    pub chars_per_token: usize,
    /// Strategy for pruning.
    pub strategy: PruneStrategy,
}

/// Strategy for deciding which messages to prune.
#[derive(Debug, Clone, Default)]
pub enum PruneStrategy {
    /// Remove oldest messages first (keeping system messages).
    #[default]
    RemoveOldest,
    /// Remove from the middle, keeping the first and last N messages.
    KeepEnds {
        /// Number of messages to keep from the start (after system).
        keep_start: usize,
        /// Number of messages to keep from the end.
        keep_end: usize,
    },
    /// Summarize: keep system + last N messages. Caller is responsible
    /// for inserting a summary message in place of the pruned ones.
    KeepLast { count: usize },
}

impl Default for PruneMessagesOptions {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            chars_per_token: 4,
            strategy: PruneStrategy::default(),
        }
    }
}

/// Estimate the token count of a message based on character length.
fn estimate_message_tokens(msg: &Message, chars_per_token: usize) -> usize {
    let char_count = format!("{:?}", msg).len();
    (char_count + chars_per_token - 1) / chars_per_token
}

/// Prune a list of messages to fit within the token budget.
///
/// System messages are always kept. Other messages are pruned according
/// to the chosen strategy.
///
/// # Example
/// ```ignore
/// use ararajuba_core::util::prune_messages::{prune_messages, PruneMessagesOptions};
///
/// let pruned = prune_messages(&messages, &PruneMessagesOptions::default());
/// ```
pub fn prune_messages(
    messages: &[Message],
    options: &PruneMessagesOptions,
) -> Vec<Message> {
    let max_chars = options.max_tokens * options.chars_per_token;

    // Separate system messages (always kept) from the rest.
    let (system_msgs, other_msgs): (Vec<_>, Vec<_>) = messages
        .iter()
        .enumerate()
        .partition(|(_, m)| matches!(m, Message::System { .. }));

    let system_chars: usize = system_msgs
        .iter()
        .map(|(_, m)| format!("{:?}", m).len())
        .sum();

    if system_chars >= max_chars {
        // System messages alone exceed budget — return just system messages.
        return system_msgs.into_iter().map(|(_, m)| m.clone()).collect();
    }

    let remaining_chars = max_chars - system_chars;

    let kept_others = match &options.strategy {
        PruneStrategy::RemoveOldest => {
            prune_oldest(&other_msgs, remaining_chars, options.chars_per_token)
        }
        PruneStrategy::KeepEnds {
            keep_start,
            keep_end,
        } => prune_keep_ends(&other_msgs, *keep_start, *keep_end, remaining_chars, options.chars_per_token),
        PruneStrategy::KeepLast { count } => {
            prune_keep_last(&other_msgs, *count, remaining_chars, options.chars_per_token)
        }
    };

    // Reconstruct in original order.
    let mut result: Vec<(usize, Message)> = Vec::new();
    for (idx, msg) in &system_msgs {
        result.push((*idx, (*msg).clone()));
    }
    for (idx, msg) in kept_others {
        result.push((idx, msg));
    }
    result.sort_by_key(|(idx, _)| *idx);
    result.into_iter().map(|(_, m)| m).collect()
}

/// Remove oldest messages first (from the beginning), keeping the most recent.
fn prune_oldest(
    messages: &[(usize, &Message)],
    max_chars: usize,
    _chars_per_token: usize,
) -> Vec<(usize, Message)> {
    let mut total: usize = 0;
    let mut kept = Vec::new();

    // Iterate from newest to oldest.
    for (idx, msg) in messages.iter().rev() {
        let chars = format!("{:?}", msg).len();
        if total + chars > max_chars {
            break;
        }
        total += chars;
        kept.push((*idx, (*msg).clone()));
    }

    kept.reverse();
    kept
}

/// Keep messages from the start and end, removing the middle.
fn prune_keep_ends(
    messages: &[(usize, &Message)],
    keep_start: usize,
    keep_end: usize,
    max_chars: usize,
    _chars_per_token: usize,
) -> Vec<(usize, Message)> {
    if messages.len() <= keep_start + keep_end {
        return messages
            .iter()
            .map(|(idx, msg)| (*idx, (*msg).clone()))
            .collect();
    }

    let start_msgs = &messages[..keep_start];
    let end_msgs = &messages[messages.len() - keep_end..];

    let mut kept = Vec::new();
    let mut total: usize = 0;

    for (idx, msg) in start_msgs.iter().chain(end_msgs.iter()) {
        let chars = format!("{:?}", msg).len();
        if total + chars > max_chars {
            break;
        }
        total += chars;
        kept.push((*idx, (*msg).clone()));
    }

    kept
}

/// Keep only the last N messages.
fn prune_keep_last(
    messages: &[(usize, &Message)],
    count: usize,
    max_chars: usize,
    _chars_per_token: usize,
) -> Vec<(usize, Message)> {
    let start = messages.len().saturating_sub(count);
    let last_msgs = &messages[start..];

    let mut total: usize = 0;
    let mut kept = Vec::new();

    for (idx, msg) in last_msgs.iter().rev() {
        let chars = format!("{:?}", msg).len();
        if total + chars > max_chars {
            break;
        }
        total += chars;
        kept.push((*idx, (*msg).clone()));
    }

    kept.reverse();
    kept
}

/// Estimate the total token count of a message list.
pub fn estimate_token_count(
    messages: &[Message],
    chars_per_token: usize,
) -> usize {
    messages
        .iter()
        .map(|m| estimate_message_tokens(m, chars_per_token))
        .sum()
}
