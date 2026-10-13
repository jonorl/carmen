//! The name box for phones and tablets: their on-screen keyboards only open
//! for real page inputs, so on the web a text box (web/input.js) sits over
//! the game's name field on touch screens. Everywhere else the game reads
//! the keyboard itself, and these do nothing.

use std::cell::Cell;

use macroquad::prelude::Rect;

thread_local! {
    /// Where this frame wants the box, if anywhere.
    static WANTED: Cell<Option<Rect>> = const { Cell::new(None) };
}

/// Ask for the box over `at` (window pixels) this frame.
pub fn want(at: Rect) {
    WANTED.set(Some(at));
}

/// Show the box where this frame asked for it, or hide it if it didn't.
pub fn end_frame() {
    name_box(WANTED.take());
}

#[cfg(target_arch = "wasm32")]
mod js {
    unsafe extern "C" {
        pub fn carmen_name_box(show: i32, x: f32, y: f32, w: f32, h: f32);
        pub fn carmen_name_take(out: *mut u8, cap: usize) -> i32;
        pub fn carmen_name_entered() -> i32;
    }

    /// Lets the JS loader check this plugin's version against the page's.
    #[unsafe(no_mangle)]
    pub extern "C" fn carmen_input_crate_version() -> u32 {
        1
    }
}

fn name_box(at: Option<Rect>) {
    #[cfg(target_arch = "wasm32")]
    {
        let r = at.unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        // SAFETY: plain numbers only.
        unsafe { js::carmen_name_box(at.is_some() as i32, r.x, r.y, r.w, r.h) }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = at;
}

/// What's in the box, while it's showing on a touch screen.
pub fn typed() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = vec![0u8; 256];
        // SAFETY: the plugin writes at most `cap` bytes into `buf`.
        let len = unsafe { js::carmen_name_take(buf.as_mut_ptr(), buf.len()) };
        buf.truncate(usize::try_from(len).ok()?);
        String::from_utf8(buf).ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// Whether the box's Done/Enter key was pressed since last asked.
pub fn entered() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: no arguments.
        unsafe { js::carmen_name_entered() == 1 }
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}
