use gt_common::commands::Command;

use super::validation::{categorize_command, CommandCategory};

/// Per-type sliding window rate limiter.
/// Different command categories have different rate limits.
pub(crate) struct RateLimiter {
    build_timestamps: Vec<std::time::Instant>,
    financial_timestamps: Vec<std::time::Instant>,
    research_timestamps: Vec<std::time::Instant>,
    espionage_timestamps: Vec<std::time::Instant>,
    general_timestamps: Vec<std::time::Instant>,
    chat_timestamps: Vec<std::time::Instant>,
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        Self {
            build_timestamps: Vec::new(),
            financial_timestamps: Vec::new(),
            research_timestamps: Vec::new(),
            espionage_timestamps: Vec::new(),
            general_timestamps: Vec::new(),
            chat_timestamps: Vec::new(),
        }
    }

    pub(crate) fn check_command(&mut self) -> bool {
        // Global fallback: 10 commands/sec across all types
        let now = std::time::Instant::now();
        let cutoff = now - std::time::Duration::from_secs(1);
        self.general_timestamps.retain(|t| *t > cutoff);
        if self.general_timestamps.len() >= 10 {
            return false;
        }
        self.general_timestamps.push(now);
        true
    }

    /// Per-type rate limit check. Returns true if allowed.
    pub(crate) fn check_typed_command(&mut self, command: &Command) -> bool {
        let now = std::time::Instant::now();
        let category = categorize_command(command);

        let (timestamps, window, max) = match category {
            CommandCategory::Build => (&mut self.build_timestamps, std::time::Duration::from_secs(1), 3usize),
            CommandCategory::Financial => (&mut self.financial_timestamps, std::time::Duration::from_secs(1), 2),
            CommandCategory::Research => (&mut self.research_timestamps, std::time::Duration::from_secs(5), 1),
            CommandCategory::Espionage => (&mut self.espionage_timestamps, std::time::Duration::from_secs(30), 1),
            CommandCategory::General => (&mut self.general_timestamps, std::time::Duration::from_secs(1), 10),
        };

        let cutoff = now - window;
        timestamps.retain(|t| *t > cutoff);
        if timestamps.len() >= max {
            return false;
        }
        timestamps.push(now);
        true
    }

    pub(crate) fn check_chat(&mut self) -> bool {
        let now = std::time::Instant::now();
        let cutoff = now - std::time::Duration::from_secs(10);
        self.chat_timestamps.retain(|t| *t > cutoff);
        if self.chat_timestamps.len() >= 5 {
            return false;
        }
        self.chat_timestamps.push(now);
        true
    }
}
