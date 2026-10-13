// A real text box for typing the detective's name on phones and tablets,
// whose on-screen keyboards only open for page inputs, not for the canvas.
// Registered as a miniquad plugin providing the imports in src/touch.rs.
// On devices without a touch screen it stays out of the way.
miniquad_add_plugin({
  name: "carmen_input",
  version: 1,
  register_plugin: function (importObject) {
    const touch = "ontouchstart" in window || navigator.maxTouchPoints > 0;
    let box = null;
    let entered = false;
    function make() {
      box = document.createElement("input");
      box.type = "text";
      box.maxLength = 20;
      box.autocomplete = "off";
      box.setAttribute("autocapitalize", "words");
      box.enterKeyHint = "done";
      // 16px stops iPhones zooming in on the box.
      box.style.cssText =
        "position:fixed;box-sizing:border-box;margin:0;padding:0 6px;border:2px solid #fff;" +
        "background:#0000aa;color:#fff;font:16px monospace;outline:none;border-radius:0;display:none;z-index:10";
      box.addEventListener("keydown", function (e) {
        if (e.key === "Enter") {
          entered = true;
          box.blur();
        }
      });
      document.body.appendChild(box);
    }
    importObject.env.carmen_name_box = function (show, x, y, w, h) {
      if (!touch) return;
      if (!show) {
        if (box && box.style.display !== "none") {
          box.blur();
          box.style.display = "none";
        }
        return;
      }
      if (!box) make();
      box.style.left = x + "px";
      box.style.top = y + "px";
      box.style.width = w + "px";
      box.style.height = Math.max(h, 28) + "px";
      box.style.display = "block";
    };
    importObject.env.carmen_name_take = function (out, cap) {
      if (!touch || !box || box.style.display === "none") return -1;
      const bytes = new TextEncoder().encode(box.value);
      if (bytes.length > cap) return -1;
      new Uint8Array(wasm_memory.buffer, out, bytes.length).set(bytes);
      return bytes.length;
    };
    importObject.env.carmen_name_entered = function () {
      const was = entered;
      entered = false;
      return was ? 1 : 0;
    };
  },
});
