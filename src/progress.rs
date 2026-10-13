//! Each detective's arrests, kept between games so their rank carries on:
//! in a file on desktops, in the browser's local storage on the web (through
//! the small plugin in web/storage.js). A server may keep them one day.

use std::collections::BTreeMap;

/// Arrests by detective, keyed by name without regard to case or spacing.
pub struct Progress {
    arrests: BTreeMap<String, u32>,
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
        Progress::from_json(&store::read().unwrap_or_default())
    }

    fn from_json(text: &str) -> Progress {
        let arrests = serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .into_iter()
            .flatten()
            .filter_map(|(name, n)| Some((name, u32::try_from(n.as_u64()?).ok()?)))
            .collect();
        Progress { arrests }
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
        store::write(&self.to_json());
        n
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod store {
    use std::path::PathBuf;

    /// Where saved data belongs on this system.
    fn path() -> Option<PathBuf> {
        let home = || std::env::var_os("HOME").map(PathBuf::from);
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            home().map(|h| h.join("Library/Application Support"))
        } else {
            std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".local/share")))
        };
        Some(base?.join("where-in-the-world").join("progress.json"))
    }

    pub fn read() -> Option<String> {
        std::fs::read_to_string(path()?).ok()
    }

    pub fn write(text: &str) {
        let Some(path) = path() else { return };
        // Write beside it and rename, so a crash can't leave half a file.
        let temp = path.with_extension("json.tmp");
        let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&temp, text));
        if saved.and_then(|_| std::fs::rename(&temp, &path)).is_err() {
            eprintln!("Couldn't save progress to {}.", path.display());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod store {
    const KEY: &str = "where-in-the-world-progress";

    unsafe extern "C" {
        /// Copies the stored text into `out` and returns its length, or -1
        /// if there's none (or it doesn't fit).
        fn carmen_storage_get(key: *const u8, key_len: usize, out: *mut u8, cap: usize) -> i32;
        fn carmen_storage_set(key: *const u8, key_len: usize, value: *const u8, len: usize);
    }

    /// Lets the JS loader check this plugin's version against the page's.
    #[unsafe(no_mangle)]
    pub extern "C" fn carmen_storage_crate_version() -> u32 {
        1
    }

    pub fn read() -> Option<String> {
        let mut buf = vec![0u8; 64 * 1024];
        // SAFETY: the plugin writes at most `cap` bytes into `buf`, and reads `KEY` only.
        let len = unsafe { carmen_storage_get(KEY.as_ptr(), KEY.len(), buf.as_mut_ptr(), buf.len()) };
        buf.truncate(usize::try_from(len).ok()?);
        String::from_utf8(buf).ok()
    }

    pub fn write(text: &str) {
        // SAFETY: the plugin only reads the two buffers, for the lengths given.
        unsafe { carmen_storage_set(KEY.as_ptr(), KEY.len(), text.as_ptr(), text.len()) }
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
    fn survives_a_round_trip_and_ignores_junk() {
        let p = Progress::from_json(r#"{"a": 3, "b": "lots", "c": -1}"#);
        assert_eq!(Progress::from_json(&p.to_json()).arrests("a"), 3);
        assert_eq!(p.arrests("b"), 0);
        assert_eq!(Progress::from_json("not json").arrests("a"), 0);
    }
}
