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

  // The running game: the game page (play.html) over the IDE, below a classic title
  // bar with a close button (styles in ide.html).
  function showGame(name) {
    closeGame();
    const overlay = document.createElement("div");
    overlay.id = "instance-run-overlay";
    overlay.setAttribute("role", "dialog");
    overlay.setAttribute("aria-label", `${name} - Instance`);
    const cap = document.createElement("div");
    cap.className = "cap";
    const icon = document.createElement("img");
    icon.src = "favicon.ico";
    icon.alt = "";
    const title = document.createElement("span");
    title.textContent = `${name} - Instance`;
    const close = document.createElement("button");
    close.className = "x";
    close.textContent = "✕";
    close.title = "Close the game (stops it)";
    close.setAttribute("aria-label", "Close");
    close.onclick = closeGame;
    cap.append(icon, title, close);
    const frame = document.createElement("iframe");
    frame.src = "play.html?game=idb:" + encodeURIComponent(RUN_KEY) + "&name=" + encodeURIComponent(name) + "&ide";
    frame.allow = "autoplay; fullscreen; gamepad";
    frame.title = name;
    frame.onload = () => frame.focus();
    overlay.append(cap, frame);
    document.body.append(overlay);
    frame.focus();
  }

  // play.html (in the iframe) may post "instance-close-game" when the game ends.
  window.addEventListener("message", (e) => {
    if (e.origin === location.origin && e.data === "instance-close-game") closeGame();
  });

  window.instanceIde = {
    async runGame(name, bytes) {
      try {
        await put(RUN_KEY, copy(bytes));
        showGame(name);
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

    // Called by the IDE once a project is open, or a question about it is shown: the
    // page's notices ("Loading Instance", "Opening ...") go.
    loaded() {
      const l = document.getElementById("loading");
      if (l) l.remove();
      const b = document.getElementById("busy");
      if (b) b.hidden = true;
    },

    // Shows "Opening NAME..." until the IDE calls loaded().
    busy(name) {
      const b = document.getElementById("busy");
      if (!b) return;
      document.getElementById("busy-text").textContent = "Opening " + name + "…";
      b.hidden = false;
    },

    // Called by ide.html once the wasm module is initialised: its exports
    // open_project_url(url, open), open_project_bytes(name, bytes), new_project().
    setWasm(exports) {
      resolveWasm(exports);
    },

    // Shows the welcome screen (also File > Examples).
    showWelcome() {
      welcome.show();
    },
  };

  let resolveWasm;
  const wasm = new Promise((r) => (resolveWasm = r));

  // The welcome screen (#welcome in ide.html): shown on a first visit, i.e. without
  // ?project= and without an autosave, and by File > Examples. Its links are real URLs (ide.html?project=...,
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

    async function edit(a) {
      const params = new URL(a.href, location.href).searchParams;
      const url = params.get("project");
      const open = params.get("open");
      hide();
      window.instanceIde.busy(url.split("/").pop());
      history.replaceState(null, "", a.getAttribute("href"));
      (await wasm).open_project_url(url, open || undefined);
    }

    async function openFile() {
      const files = await window.instanceIde.pickFiles(".gm81,.gmk,.gm6", false);
      if (!files || !files.length) return;
      hide();
      window.instanceIde.busy(files[0].name);
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
      // while the welcome screen is open its keys come first (the IDE keeps its
      // keyboard focus in a hidden text field): Esc closes it, arrows page through it
      window.addEventListener("keydown", (e) => {
        if (w.hidden) return;
        if (e.key === "Escape") hide();
        else if (e.key === "ArrowLeft" || e.key === "ArrowRight") go(current + (e.key === "ArrowLeft" ? -1 : 1));
        else return;
        e.preventDefault();
        e.stopPropagation();
      }, true);
      w.querySelectorAll(".tabs button").forEach((b, i) => (b.onclick = () => go(i)));
      // swipe on the screenshot
      const stage = w.querySelector(".stage");
      let x0 = null;
      stage.addEventListener("pointerdown", (e) => { x0 = e.clientX; });
      stage.addEventListener("pointerup", (e) => {
        if (x0 !== null && Math.abs(e.clientX - x0) > 40) go(current + (e.clientX < x0 ? 1 : -1));
        x0 = null;
      });
    }

    // The carousel: one screenshot (.stage img), slide text and tab per example, in order.
    let current = 0;
    function go(i) {
      const w = $("welcome");
      const imgs = w.querySelectorAll(".stage img");
      const slides = w.querySelectorAll(".slide");
      const tabs = w.querySelectorAll(".tabs button");
      current = (i + slides.length) % slides.length;
      slides.forEach((s, k) => (s.hidden = k !== current));
      imgs.forEach((im, k) => im.classList.toggle("on", k === current));
      tabs.forEach((d, k) => d.setAttribute("aria-current", String(k === current)));
    }

    function show() {
      const w = $("welcome");
      if (!w) return;
      wire();
      go(0);
      w.hidden = false;
      const first = w.querySelector(".slide:not([hidden]) .btn.primary");
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
