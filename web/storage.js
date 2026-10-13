// Saves the game's progress (each detective's arrests) and language in the
// browser's local storage, and tells it the browser's language. Registered as
// a miniquad plugin, so the wasm can import these (see src/storage.rs and
// src/lang.rs).
miniquad_add_plugin({
  name: "carmen_storage",
  version: 1,
  register_plugin: function (importObject) {
    const text = (ptr, len) => new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, ptr, len));
    importObject.env.carmen_storage_get = function (keyPtr, keyLen, out, cap) {
      let value = null;
      try {
        value = window.localStorage.getItem(text(keyPtr, keyLen));
      } catch (e) {
        // Storage blocked (private browsing, cookies off): play without saving.
      }
      if (value === null) return -1;
      const bytes = new TextEncoder().encode(value);
      if (bytes.length > cap) return -1;
      new Uint8Array(wasm_memory.buffer, out, bytes.length).set(bytes);
      return bytes.length;
    };
    // The browser's preferred language, so the game can start in Spanish.
    importObject.env.carmen_browser_language = function (out, cap) {
      const bytes = new TextEncoder().encode(navigator.language || "");
      if (bytes.length > cap) return -1;
      new Uint8Array(wasm_memory.buffer, out, bytes.length).set(bytes);
      return bytes.length;
    };
    importObject.env.carmen_storage_set = function (keyPtr, keyLen, valuePtr, valueLen) {
      try {
        window.localStorage.setItem(text(keyPtr, keyLen), text(valuePtr, valueLen));
      } catch (e) {
        // As above: progress just isn't kept.
      }
    };
  },
});
