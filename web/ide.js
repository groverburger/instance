// Browser helpers of the Instance web IDE (window.instanceIde), called from Rust
// (crates/instance-ide/src/web.rs). Loaded by ide.html before the wasm module.
//
// Storage: IndexedDB database "instance", object store "files" (out-of-line
// string keys). "run/game.gm81" holds the game started with Run (read by
// play.html?game=idb:run/game.gm81); "autosave/project" holds {name, bytes, time}.
// Earlier builds used the database "gm81-clean": its autosave is offered when the
// new database has none.
(function () {
  "use strict";

  const DB_NAME = "instance";
  const LEGACY_DB_NAME = "gm81-clean";
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
  async function openDb(name = DB_NAME) {
    let db = await new Promise((resolve, reject) => {
      const r = indexedDB.open(name);
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
      const r = indexedDB.open(name, version);
      r.onupgradeneeded = () => {
        if (!r.result.objectStoreNames.contains(STORE)) r.result.createObjectStore(STORE);
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
  }

  async function withStore(mode, f, name = DB_NAME) {
    const db = await openDb(name);
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

  // The autosave of earlier builds (database "gm81-clean"), only if that database
  // exists (opening a missing database would create it).
  async function legacyAutosave() {
    if (!indexedDB.databases) return null;
    const dbs = await indexedDB.databases();
    if (!dbs.some((d) => d.name === LEGACY_DB_NAME)) return null;
    return withStore("readonly", (s) => s.get(AUTOSAVE_KEY), LEGACY_DB_NAME);
  }

  // Byte arrays from Rust are views into wasm memory: copy before keeping them.
  const copy = (bytes) => new Uint8Array(bytes);

  function closeGame() {
    const o = document.getElementById("instance-run-overlay");
    if (o) o.remove();
    const c = document.querySelector("canvas");
    if (c) c.focus();
  }

  function showGame() {
    closeGame();
    const overlay = document.createElement("div");
    overlay.id = "instance-run-overlay";
    overlay.style.cssText =
      "position:fixed;inset:0;z-index:1000;background:#000;display:flex;flex-direction:column";
    // a compact bar above the game; the game page (play.html) fits the game into
    // the area below it
    const bar = document.createElement("div");
    bar.style.cssText =
      "flex:none;display:flex;align-items:center;justify-content:space-between;gap:8px;" +
      "height:22px;box-sizing:border-box;padding:0 2px 0 8px;background:#f0f0f0;color:#222;" +
      "font:12px/1 system-ui,sans-serif;border-bottom:1px solid #999;user-select:none";
    const title = document.createElement("span");
    title.textContent = "Running game";
    const close = document.createElement("button");
    close.textContent = "✕";
    close.title = "Close the game (stops it)";
    close.style.cssText =
      "font:12px/1 system-ui,sans-serif;height:18px;padding:0 8px;margin:0;cursor:pointer;" +
      "border:1px solid #aaa;border-radius:2px;background:#fafafa";
    close.onclick = closeGame;
    bar.append(title, close);
    const frame = document.createElement("iframe");
    frame.src = "play.html?game=idb:" + encodeURIComponent(RUN_KEY);
    frame.allow = "autoplay; fullscreen; gamepad";
    frame.style.cssText = "flex:1 1 auto;min-height:0;display:block;border:0;width:100%;background:#000";
    frame.onload = () => frame.focus();
    overlay.append(bar, frame);
    document.body.append(overlay);
    frame.focus();
  }

  // play.html (in the iframe) may post "instance-close-game" when the game ends.
  window.addEventListener("message", (e) => {
    if (e.origin === location.origin && e.data === "instance-close-game") closeGame();
  });

  window.instanceIde = {
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
        console.warn("instance: autosave failed", e)
      );
    },

    async loadAutosave() {
      try {
        let v = await get(AUTOSAVE_KEY);
        if (!(v && v.bytes)) v = await legacyAutosave().catch(() => null);
        return v && v.bytes ? v : null;
      } catch (e) {
        console.warn("instance: cannot read the autosave", e);
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

    // Called by ide.html once the wasm module is initialised: its exports
    // open_project_url(url, open), open_project_bytes(name, bytes), new_project().
    setWasm(exports) {
      resolveWasm(exports);
    },

    // Shows the welcome screen (also Help > Examples).
    showWelcome() {
      welcome.show();
    },
  };

  let resolveWasm;
  const wasm = new Promise((r) => (resolveWasm = r));

  // The welcome screen (#welcome in ide.html): shown on a first visit, i.e. without
  // ?project= and without an autosave. Its links are real URLs (ide.html?project=...,
  // play.html?game=...), so they also work in a new tab; a plain click on "Edit"
  // opens the project in this page without reloading it.
  const welcome = (function () {
    const $ = (id) => document.getElementById(id);
    let wired = false;

    function hide() {
      const w = $("welcome");
      if (w) w.hidden = true;
      const c = document.querySelector("canvas");
      if (c) c.focus();
    }

    function loadingText(t) {
      const l = $("loading");
      if (l) l.textContent = t;
    }

    async function edit(a) {
      const params = new URL(a.href, location.href).searchParams;
      const url = params.get("project");
      const open = params.get("open");
      hide();
      loadingText("Loading " + url.split("/").pop() + "…");
      history.replaceState(null, "", a.getAttribute("href"));
      (await wasm).open_project_url(url, open || undefined);
    }

    async function openFile() {
      const files = await window.instanceIde.pickFiles(".gm81,.gmk,.gm6", false);
      if (!files || !files.length) return;
      hide();
      (await wasm).open_project_bytes(files[0].name, files[0].bytes);
    }

    function wire() {
      if (wired) return;
      wired = true;
      const w = $("welcome");
      w.addEventListener("click", (e) => {
        const a = e.target.closest("a[data-edit]");
        if (a && !(e.ctrlKey || e.metaKey || e.shiftKey || e.altKey || e.button)) {
          e.preventDefault();
          edit(a);
        } else if (e.target === w) {
          hide();
        }
      });
      $("welcome-close").onclick = hide;
      $("welcome-empty").onclick = async () => {
        hide();
        history.replaceState(null, "", location.pathname);
        (await wasm).new_project();
      };
      $("welcome-open").onclick = openFile;
      document.addEventListener("keydown", (e) => {
        if (e.key === "Escape" && !w.hidden) hide();
      });
      // examples that are supplied separately: shown when their file is on the server
      const sp = $("card-spelunky");
      if (sp) {
        fetch("examples/spelunky.gm81", { method: "HEAD" })
          .then((r) => { sp.hidden = !r.ok; })
          .catch(() => {});
      }
    }

    function show() {
      const w = $("welcome");
      if (!w) return;
      wire();
      w.hidden = false;
      const first = w.querySelector(".hero .btn.primary");
      if (first) first.focus();
    }

    return { show, hide };
  })();

  // First visit: no ?project= and nothing autosaved.
  async function firstVisit() {
    if (new URLSearchParams(location.search).get("project")) return false;
    return !(await window.instanceIde.loadAutosave());
  }
  const start = () => firstVisit().then((yes) => yes && welcome.show()).catch(() => {});
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start);
  else start();
})();
