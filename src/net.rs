//! Requests to the agency's server that never hold up a frame: started now,
//! asked each frame whether they're done. On desktops each runs on a thread
//! of its own; in browsers `fetch` does it, through the small plugin in
//! web/net.js, since macroquad has no web requests of its own.

/// How long to wait for the server before giving up on it.
const TIMEOUT_SECONDS: u64 = 8;

pub struct Request {
    inner: imp::Pending,
}

impl Request {
    pub fn get(url: &str) -> Request {
        Request { inner: imp::start("GET", url, None) }
    }

    /// PUT `json` to `url`.
    pub fn put(url: &str, json: String) -> Request {
        Request { inner: imp::start("PUT", url, Some(json)) }
    }

    /// None while it's under way; then the body of a successful answer, or
    /// why there wasn't one. Asking again after that gives an error.
    pub fn poll(&mut self) -> Option<Result<String, String>> {
        imp::poll(&mut self.inner)
    }
}

/// `text` made safe for a URL path.
pub fn escape(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::sync::mpsc::{Receiver, TryRecvError, channel};
    use std::time::Duration;

    pub struct Pending(Receiver<Result<String, String>>);

    pub fn start(method: &str, url: &str, body: Option<String>) -> Pending {
        let (tx, rx) = channel();
        let (method, url) = (method.to_string(), url.to_string());
        std::thread::spawn(move || {
            let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(super::TIMEOUT_SECONDS)).build();
            let request = agent.request(&method, &url);
            let answer = match body {
                Some(json) => request.set("Content-Type", "application/json").send_string(&json),
                None => request.call(),
            };
            let result = answer.map_err(|e| e.to_string()).and_then(|r| r.into_string().map_err(|e| e.to_string()));
            // Nobody may be listening any more, which is fine.
            let _ = tx.send(result);
        });
        Pending(rx)
    }

    pub fn poll(pending: &mut Pending) -> Option<Result<String, String>> {
        match pending.0.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("already answered".to_string())),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    unsafe extern "C" {
        /// Starts a fetch and returns its id. `get` is 1 for GET, 0 for a PUT
        /// of the JSON body.
        fn carmen_http_start(get: i32, url: *const u8, url_len: usize, body: *const u8, body_len: usize, timeout_ms: u32) -> u32;
        /// -1 while under way, -2 if it failed (or is unknown), else the
        /// length of the answer's body.
        fn carmen_http_state(id: u32) -> i32;
        /// Copies the finished answer into `out` and forgets the request.
        fn carmen_http_take(id: u32, out: *mut u8, cap: usize) -> i32;
        fn carmen_http_forget(id: u32);
    }

    /// Lets the JS loader check this plugin's version against the page's.
    #[unsafe(no_mangle)]
    pub extern "C" fn carmen_net_crate_version() -> u32 {
        1
    }

    pub struct Pending(u32);

    impl Drop for Pending {
        fn drop(&mut self) {
            // SAFETY: a plain number; unknown ids are ignored.
            unsafe { carmen_http_forget(self.0) }
        }
    }

    pub fn start(method: &str, url: &str, body: Option<String>) -> Pending {
        let body = body.unwrap_or_default();
        let timeout = (super::TIMEOUT_SECONDS * 1000) as u32;
        // SAFETY: the plugin only reads the two buffers, for the lengths given.
        let id = unsafe { carmen_http_start((method == "GET") as i32, url.as_ptr(), url.len(), body.as_ptr(), body.len(), timeout) };
        Pending(id)
    }

    pub fn poll(pending: &mut Pending) -> Option<Result<String, String>> {
        // SAFETY: a plain number.
        let state = unsafe { carmen_http_state(pending.0) };
        let len = match state {
            -1 => return None,
            n if n < 0 => return Some(Err("the request failed".to_string())),
            n => n as usize,
        };
        let mut buf = vec![0u8; len];
        // SAFETY: the plugin writes at most `cap` bytes into `buf`.
        let got = unsafe { carmen_http_take(pending.0, buf.as_mut_ptr(), buf.len()) };
        if got as usize != len {
            return Some(Err("the answer went missing".to_string()));
        }
        Some(String::from_utf8(buf).map_err(|e| e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_are_escaped_for_urls() {
        assert_eq!(super::escape("Jon O"), "Jon%20O");
        assert_eq!(super::escape("Ñ/?"), "%C3%91%2F%3F");
    }
}
