// Browser helpers of the gm81-clean web IDE (window.gm81ide), called from Rust
// (crates/instance-ide/src/web.rs). Loaded by ide.html before the wasm module.
//
// Storage: IndexedDB database "gm81-clean", object store "files" (out-of-line
// string keys). "run/game.gm81" holds the game started with Run (read by
// play.html?game=idb:run/game.gm81); "autosave/project" holds {name, bytes, time}.
(function () {
  "use strict";

  const DB_NAME = "gm81-clean";
  const STORE = "files";
  const RUN_KEY = "run/game.gm81";
  const AUTOSAVE_KEY = "autosave/project";

  function req(r) {
    return new Promise((resolve, reject) => {
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
  }

  // Opens the database, adding the object store if it is missing (another page
  // may have created the database with other stores or another version).
  async function openDb() {
    let db = await new Promise((resolve, reject) => {
      const r = indexedDB.open(DB_NAME);
      r.onupgradeneeded = () => {
        if (!r.result.objectStoreNames.contains(STORE)) r.result.createObjectStore(STORE);
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    if (db.objectStoreNames.contains(STORE)) return db;
    const version = db.version + 1;
    db.close();
    return new Promise((resolve, reject) => {
      const r = indexedDB.open(DB_NAME, version);
      r.onupgradeneeded = () => {
        if (!r.result.objectStoreNames.contains(STORE)) r.result.createObjectStore(STORE);
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
  }

  async function withStore(mode, f) {
    const db = await openDb();
    try {
      const tx = db.transaction(STORE, mode);
      const result = await req(f(tx.objectStore(STORE)));
      await new Promise((resolve, reject) => {
        tx.oncomplete = resolve;
        tx.onerror = () => reject(tx.error);
        tx.onabort = () => reject(tx.error);
      });
      return result;
    } finally {
      db.close();
    }
  }

  const put = (key, value) => withStore("readwrite", (s) => s.put(value, key));
  const get = (key) => withStore("readonly", (s) => s.get(key));
  const del = (key) => withStore("readwrite", (s) => s.delete(key));

  // Byte arrays from Rust are views into wasm memory: copy before keeping them.
  const copy = (bytes) => new Uint8Array(bytes);

  function closeGame() {
    const o = document.getElementById("gm81-run-overlay");
    if (o) o.remove();
    const c = document.querySelector("canvas");
    if (c) c.focus();
  }

  function showGame() {
    closeGame();
    const overlay = document.createElement("div");
    overlay.id = "gm81-run-overlay";
    overlay.style.cssText =
      "position:fixed;inset:0;z-index:1000;background:#000;display:flex;flex-direction:column";
    const bar = document.createElement("div");
    bar.style.cssText =
      "display:flex;align-items:center;justify-content:space-between;padding:2px 6px;" +
      "background:#f0f0f0;font:13px system-ui,sans-serif;border-bottom:1px solid #999";
    const title = document.createElement("span");
    title.textContent = "Running game";
    const close = document.createElement("button");
    close.textContent = "✕";
    close.title = "Close the game";
    close.style.cssText = "font-size:14px;padding:0 8px;cursor:pointer";
    close.onclick = closeGame;
    bar.append(title, close);
    const frame = document.createElement("iframe");
    frame.src = "play.html?game=idb:" + encodeURIComponent(RUN_KEY);
    frame.allow = "autoplay; fullscreen; gamepad";
    frame.style.cssText = "flex:1;border:0;width:100%;background:#000";
    frame.onload = () => frame.focus();
    overlay.append(bar, frame);
    document.body.append(overlay);
    frame.focus();
  }

  // play.html (in the iframe) may post "gm81-close-game" when the game ends.
  window.addEventListener("message", (e) => {
    if (e.origin === location.origin && e.data === "gm81-close-game") closeGame();
  });

  window.gm81ide = {
    async runGame(bytes) {
      try {
        await put(RUN_KEY, copy(bytes));
        showGame();
      } catch (e) {
        alert("Cannot start the game: " + e);
      }
    },

    saveAutosave(name, bytes) {
      return put(AUTOSAVE_KEY, { name, bytes: copy(bytes), time: Date.now() }).catch((e) =>
        console.warn("gm81-clean: autosave failed", e)
      );
    },

    async loadAutosave() {
      try {
        const v = await get(AUTOSAVE_KEY);
        return v && v.bytes ? v : null;
      } catch (e) {
        console.warn("gm81-clean: cannot read the autosave", e);
        return null;
      }
    },

    clearAutosave() {
      return del(AUTOSAVE_KEY).catch(() => {});
    },

    pickFiles(accept, multiple) {
      return new Promise((resolve) => {
        const input = document.createElement("input");
        input.type = "file";
        if (accept) input.accept = accept;
        input.multiple = !!multiple;
        input.style.display = "none";
        let done = false;
        const finish = (v) => {
          if (done) return;
          done = true;
          input.remove();
          resolve(v);
        };
        input.addEventListener("cancel", () => finish(null));
        input.addEventListener("change", async () => {
          const files = Array.from(input.files || []);
          if (!files.length) return finish(null);
          const out = [];
          for (const f of files) {
            out.push({ name: f.name, bytes: new Uint8Array(await f.arrayBuffer()) });
          }
          finish(out);
        });
        document.body.append(input);
        input.click();
      });
    },

    download(name, bytes) {
      const url = URL.createObjectURL(new Blob([copy(bytes)], { type: "application/octet-stream" }));
      const a = document.createElement("a");
      a.href = url;
      a.download = name || "download";
      a.style.display = "none";
      document.body.append(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 30000);
    },

    async fetchBytes(url) {
      const r = await fetch(url);
      if (!r.ok) throw new Error(r.status + " " + r.statusText);
      return new Uint8Array(await r.arrayBuffer());
    },

    queryParam(name) {
      return new URLSearchParams(location.search).get(name);
    },

    loaded() {
      const l = document.getElementById("loading");
      if (l) l.remove();
    },
  };
})();
