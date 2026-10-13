//! Small things kept between games, each under a name: in a file in the
//! user's data folder on desktops (`<name>.json`), in the browser's local
//! storage on the web (`where-in-the-world-<name>`, through the small plugin
//! in web/storage.js).

pub use imp::{read, write};

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::path::PathBuf;

    /// Where saved data belongs on this system.
    fn path(name: &str) -> Option<PathBuf> {
        let home = || std::env::var_os("HOME").map(PathBuf::from);
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            home().map(|h| h.join("Library/Application Support"))
        } else {
            std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".local/share")))
        };
        Some(base?.join("where-in-the-world").join(format!("{name}.json")))
    }

    pub fn read(name: &str) -> Option<String> {
        std::fs::read_to_string(path(name)?).ok()
    }

    pub fn write(name: &str, text: &str) {
        let Some(path) = path(name) else { return };
        // Write beside it and rename, so a crash can't leave half a file.
        let temp = path.with_extension("json.tmp");
        let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&temp, text));
        if saved.and_then(|_| std::fs::rename(&temp, &path)).is_err() {
            eprintln!("Couldn't save {}.", path.display());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
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

    fn key(name: &str) -> String {
        format!("where-in-the-world-{name}")
    }

    pub fn read(name: &str) -> Option<String> {
        let key = key(name);
        let mut buf = vec![0u8; 64 * 1024];
        // SAFETY: the plugin writes at most `cap` bytes into `buf`, and only reads `key`.
        let len = unsafe { carmen_storage_get(key.as_ptr(), key.len(), buf.as_mut_ptr(), buf.len()) };
        buf.truncate(usize::try_from(len).ok()?);
        String::from_utf8(buf).ok()
    }

    pub fn write(name: &str, text: &str) {
        let key = key(name);
        // SAFETY: the plugin only reads the two buffers, for the lengths given.
        unsafe { carmen_storage_set(key.as_ptr(), key.len(), text.as_ptr(), text.len()) }
    }
}
