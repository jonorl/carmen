//! What the game and its server agree on: how long a detective's name may
//! be, how names are matched, and what the server says about a detective.

use serde::{Deserialize, Serialize};

/// The longest name a detective can sign in with, in characters.
pub const MAX_NAME: usize = 8;

/// More arrests than anyone will make, to turn away nonsense.
pub const MAX_ARRESTS: u32 = 10_000;

/// A detective's arrests, as the server reports them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detective {
    pub name: String,
    pub arrests: u32,
}

/// What the game sends to record its tally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tally {
    pub arrests: u32,
}

/// The name as it should be kept: trimmed, with no control characters.
pub fn tidy(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string()
}

/// The name records are filed under, so "Jon" and " jon " are one
/// detective. None if it's empty or too long.
pub fn key(name: &str) -> Option<String> {
    let name = tidy(name);
    let len = name.chars().count();
    (1..=MAX_NAME).contains(&len).then(|| name.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_however_they_are_typed() {
        assert_eq!(key("  Jon "), Some("jon".to_string()));
        assert_eq!(key("JON"), key("jon"));
        assert_eq!(key("Ñandú"), Some("ñandú".to_string()));
    }

    #[test]
    fn empty_and_long_names_are_refused() {
        assert_eq!(key("   "), None);
        assert_eq!(key("\n"), None);
        assert_eq!(key("12345678"), Some("12345678".to_string()));
        assert_eq!(key("123456789"), None);
    }
}
