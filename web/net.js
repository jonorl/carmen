// Web requests for the game (see src/net.rs), which macroquad lacks:
// registered as a miniquad plugin. Each fetch gets an id the game polls
// every frame until the answer is in.
miniquad_add_plugin({
  name: "carmen_net",
  version: 1,
  register_plugin: function (importObject) {
    const requests = new Map();
    let next = 1;
    function text(ptr, len) {
      return new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, ptr, len));
    }
    importObject.env.carmen_http_start = function (get, url, urlLen, body, bodyLen, timeoutMs) {
      const id = next++;
      const request = { state: "waiting", bytes: null };
      requests.set(id, request);
      const options = { method: get ? "GET" : "PUT", signal: AbortSignal.timeout(timeoutMs) };
      if (!get) {
        options.headers = { "Content-Type": "application/json" };
        options.body = text(body, bodyLen);
      }
      fetch(text(url, urlLen), options)
        .then(function (response) {
          if (!response.ok) throw new Error("HTTP " + response.status);
          return response.arrayBuffer();
        })
        .then(function (buffer) {
          request.bytes = new Uint8Array(buffer);
          request.state = "done";
        })
        .catch(function () {
          request.state = "failed";
        });
      return id;
    };
    importObject.env.carmen_http_state = function (id) {
      const request = requests.get(id);
      if (!request || request.state === "failed") return -2;
      if (request.state === "waiting") return -1;
      return request.bytes.length;
    };
    importObject.env.carmen_http_take = function (id, out, cap) {
      const request = requests.get(id);
      if (!request || request.state !== "done" || request.bytes.length > cap) return -1;
      new Uint8Array(wasm_memory.buffer, out, request.bytes.length).set(request.bytes);
      requests.delete(id);
      return request.bytes.length;
    };
    importObject.env.carmen_http_forget = function (id) {
      requests.delete(id);
    };
  },
});
