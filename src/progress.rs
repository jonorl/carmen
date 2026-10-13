//! Each detective's arrests, kept between games so their rank carries on:
//! in a file on desktops, in the browser's local storage on the web (through
//! the small plugin in web/storage.js), and on the agency's server
//! (server/), so the rank follows them to other devices. The game never
//! waits for the server for long: without it, the copy on this device
//! carries on alone, and the two catch up next time. A tally only ever goes
//! up, so catching up just means taking the higher.

use std::collections::BTreeMap;

use carmen_shared::{Detective, Tally};

use crate::net::{self, Request};
use crate::storage;

/// The server's address, from CARMEN_API when the game was built (default:
/// the live one). Built with CARMEN_API empty, it keeps to this device.
fn server() -> Option<&'static str> {
    match option_env!("CARMEN_API") {
        Some("") => None,
        Some(url) => Some(url.trim_end_matches('/')),
        None => Some("https://carmen-api.jonathan-orlowski.dev"),
    }
}

/// Ask the server for every detective, for the roster. None without one.
pub fn roster_request() -> Option<Request> {
    server().map(|server| Request::get(&format!("{server}/detectives")))
}

/// What it's saved under (see `storage`).
const STORE: &str = "progress";

/// Arrests by detective, keyed by name without regard to case or spacing.
pub struct Progress {
    arrests: BTreeMap<String, u32>,
    pending: Vec<Pending>,
}

/// A request to the server still under way.
struct Pending {
    name: String,
    /// A look-up when they sign in, rather than recording a tally.
    lookup: bool,
    request: Request,
}

fn key(name: &str) -> String {
    match name.trim().to_lowercase() {
        n if n.is_empty() => "detective".to_string(),
        n => n,
    }
}

impl Progress {
    /// What was saved, or nobody's arrests if nothing (or nothing readable) was.
    pub fn load() -> Progress {
        Progress::from_json(&storage::read(STORE).unwrap_or_default())
    }

    fn from_json(text: &str) -> Progress {
        let arrests = serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .into_iter()
            .flatten()
            .filter_map(|(name, n)| Some((name, u32::try_from(n.as_u64()?).ok()?)))
            .collect();
        Progress { arrests, pending: Vec::new() }
    }

    fn to_json(&self) -> String {
        serde_json::to_string(&self.arrests).unwrap_or_default()
    }

    pub fn arrests(&self, name: &str) -> u32 {
        self.arrests.get(&key(name)).copied().unwrap_or(0)
    }

    /// Count an arrest for `name` and save at once, so it isn't lost if the
    /// game closes. Returns their new tally.
    pub fn add_arrest(&mut self, name: &str) -> u32 {
        let n = self.arrests.entry(key(name)).or_insert(0);
        *n += 1;
        let n = *n;
        storage::write(STORE, &self.to_json());
        self.send(name, false);
        n
    }

    /// Everyone on this device, most arrests first: the roster when the
    /// server can't be reached. Names are as filed, in lower case.
    pub fn local_roster(&self) -> Vec<Detective> {
        let mut list: Vec<Detective> = self.arrests.iter().map(|(name, &arrests)| Detective { name: name.clone(), arrests }).collect();
        list.sort_by(|a, b| b.arrests.cmp(&a.arrests).then_with(|| a.name.cmp(&b.name)));
        list
    }

    /// Ask the server for `name`'s arrests, as they sign in.
    pub fn look_up(&mut self, name: &str) {
        self.send(name, true);
    }

    /// Whether the server's answer about `name` is still to come.
    pub fn looking_up(&self, name: &str) -> bool {
        self.pending.iter().any(|p| p.lookup && key(&p.name) == key(name))
    }

    /// A look-up (`lookup`), or the tally on this device sent to be kept.
    fn send(&mut self, name: &str, lookup: bool) {
        let (Some(server), Some(_)) = (server(), carmen_shared::key(name)) else {
            return;
        };
        let url = format!("{server}/detectives/{}", net::escape(carmen_shared::tidy(name).as_str()));
        let request = if lookup {
            Request::get(&url)
        } else {
            let tally = Tally { arrests: self.arrests(name) };
            Request::put(&url, serde_json::to_string(&tally).unwrap_or_default())
        };
        self.pending.push(Pending { name: name.to_string(), lookup, request });
    }

    /// Take in whatever the server has answered. Call every frame.
    pub fn poll(&mut self) {
        let mut answers = Vec::new();
        self.pending.retain_mut(|p| match p.request.poll() {
            None => true,
            Some(answer) => {
                answers.push((p.name.clone(), p.lookup, answer));
                false
            }
        });
        for (name, lookup, answer) in answers {
            // No answer just means this device's copy carries on alone for now.
            let Some(detective) = answer.ok().and_then(|body| serde_json::from_str::<Detective>(&body).ok()) else {
                continue;
            };
            if self.raise(&name, detective.arrests) {
                storage::write(STORE, &self.to_json());
            }
            // Signing in puts them on file (and on the roster) even before
            // their first arrest, and brings in any arrests made here while
            // the server was out of reach. The server answers 0 for a name it
            // doesn't know, so it's sent every time; it never lowers a tally.
            if lookup {
                self.send(&name, false);
            }
        }
    }

    /// Take the server's tally if it's higher. Whether it was.
    fn raise(&mut self, name: &str, arrests: u32) -> bool {
        let n = self.arrests.entry(key(name)).or_insert(0);
        let higher = arrests > *n;
        *n = (*n).max(arrests);
        higher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_however_they_are_typed() {
        let p = Progress::from_json(r#"{"jon orlo": 4, "detective": 1}"#);
        assert_eq!(p.arrests("  Jon Orlo "), 4);
        assert_eq!(p.arrests(""), 1);
        assert_eq!(p.arrests("Someone New"), 0);
    }

    #[test]
    fn the_higher_tally_wins() {
        let mut p = Progress::from_json(r#"{"jon": 4}"#);
        assert!(!p.raise("Jon", 2));
        assert_eq!(p.arrests("jon"), 4);
        assert!(p.raise(" JON ", 6));
        assert_eq!(p.arrests("Jon"), 6);
        assert!(p.raise("Ana", 1));
    }

    #[test]
    fn survives_a_round_trip_and_ignores_junk() {
        let p = Progress::from_json(r#"{"a": 3, "b": "lots", "c": -1}"#);
        assert_eq!(Progress::from_json(&p.to_json()).arrests("a"), 3);
        assert_eq!(p.arrests("b"), 0);
        assert_eq!(Progress::from_json("not json").arrests("a"), 0);
    }
}
