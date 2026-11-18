// SwitchOS Hub 0.0.1 - frontend. Fala com o backend Rust (src-tauri/src/lib.rs) via invoke.
const { invoke } = window.__TAURI__.core;
const { open: pick } = window.__TAURI__.dialog;
const { openPath } = window.__TAURI__.opener;

const $ = (sel, el = document) => el.querySelector(sel);
const content = $("#content");
const library = $("#library");

let games = [];
let current = localStorage.getItem("hub.current") || "arceus";
const picked = {}; // key -> [arquivos escolhidos para importar]
let pollTimer = null;

const initials = (name) =>
  name.replace(/^Pokémon( Legends:)?\s*/i, "").replace(/^Zelda:\s*/, "").split(/\s+/).map((w) => w[0]).join("").slice(0, 2).toUpperCase();

function fmtDate(secs) {
  return new Date(secs * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
}

function status(g) {
  if (!g.supported) return { text: "Em breve no hub", cls: "off" };
  if (g.built) return { text: "Pronto", cls: "ok" };
  if (g.imported) return { text: "Falta compilar", cls: "warn" };
  return { text: "Não importado", cls: "off" };
}

async function refresh() {
  games = await invoke("list_games");
  renderLibrary();
  if (current === "settings") renderSettings();
  else renderGame(games.find((g) => g.key === current) || games[0]);
}

function renderLibrary() {
  library.innerHTML = "";
  for (const g of games) {
    const s = status(g);
    const b = document.createElement("button");
    b.type = "button";
    b.className = "lib-item" + (g.key === current ? " active" : "");
    b.innerHTML = `<span class="tile" style="--c:${g.color}">${g.icon ? `<img src="${g.icon}" alt="">` : initials(g.name)}</span>
      <span class="lib-text"><strong>${g.name}</strong><small class="st ${s.cls}">${s.text}</small></span>`;
    b.onclick = () => select(g.key);
    library.appendChild(b);
  }
}

function select(key) {
  current = key;
  localStorage.setItem("hub.current", key);
  clearInterval(pollTimer);
  refresh();
}

function renderGame(g) {
  const node = $("#tpl-game").content.cloneNode(true);
  const s = status(g);
  $("[data-art]", node).style.setProperty("--c", g.color);
  if (g.icon) $("[data-art]", node).innerHTML = `<img src="${g.icon}" alt="">`;
  else $("[data-art]", node).textContent = initials(g.name);
  $("[data-tid]", node).textContent = g.title_id;
  $("[data-name]", node).textContent = g.name;
  $("[data-chips]", node).innerHTML =
    `<span class="chip ${s.cls}">${s.text}</span>` +
    (g.mods.length ? `<span class="chip">${g.mods.length} mod${g.mods.length > 1 ? "s" : ""}</span>` : "") +
    (g.save_modified ? `<span class="chip">Save: ${fmtDate(g.save_modified)}</span>` : "");

  const play = $("[data-play]", node);
  play.disabled = !g.built;
  play.onclick = async () => {
    try {
      play.disabled = true;
      play.textContent = "Abrindo…";
      await invoke("play_game", { key: g.key });
      setTimeout(() => { play.textContent = "Jogar"; play.disabled = false; }, 3000);
    } catch (e) { alert(e); play.textContent = "Jogar"; play.disabled = false; }
  };
  $("[data-hint]", node).textContent = !g.supported
    ? "Este jogo ainda tem runtime próprio (engines/). A integração ao hub vem nas próximas versões."
    : !g.built ? "Importe o jogo abaixo para liberar o Jogar." : "No jogo: F1 abre as configurações gráficas e de controles.";
  $("[data-folder]", node).onclick = () => openPath(g.dir).catch(() => alert("A pasta do jogo ainda não existe."));

  // importar
  const files = $("[data-files]", node);
  const btnImport = $("[data-import]", node);
  const showFiles = () => {
    const list = picked[g.key] || [];
    files.innerHTML = list.map((f) => `<li>${f.split(/[\\/]/).pop()}</li>`).join("");
    btnImport.disabled = !list.length || !g.supported;
  };
  showFiles();
  $("[data-pick]", node).onclick = async () => {
    const sel = await pick({ multiple: true, filters: [{ name: "Jogo do Switch", extensions: ["nsp", "xci", "nsz", "xcz", "nup"] }] });
    if (sel) { picked[g.key] = Array.isArray(sel) ? sel : [sel]; showFiles(); }
  };
  btnImport.onclick = async () => {
    try {
      await invoke("import_game", { key: g.key, packages: picked[g.key] });
      watchJob(g.key, jobEl);
    } catch (e) { alert(e); }
  };
  const jobEl = $("[data-job]", node);

  // mods
  $("[data-mods]", node).innerHTML = g.mods.length
    ? g.mods.map((m) => `<li><strong>${m.name}</strong><small>${m.files} arquivos</small></li>`).join("")
    : `<li class="muted">Nenhum mod para este jogo.</li>`;
  $("[data-mods-folder]", node).onclick = async () => openPath(await invoke("ensure_dir", { path: await invoke("mods_dir") }));

  // save
  $("[data-save]", node).textContent = g.save_modified
    ? `Último save em ${fmtDate(g.save_modified)}.`
    : "Ainda não há save deste jogo.";
  $("[data-save-folder]", node).onclick = () => openPath(g.save_dir).catch(() => alert("Ainda não há save deste jogo."));

  content.innerHTML = "";
  content.appendChild(node);
  watchJob(g.key, jobEl, true);
}

async function watchJob(key, el, quiet = false) {
  clearInterval(pollTimer);
  const tick = async () => {
    const j = await invoke("job_status", { key });
    if (!j) { if (quiet) el.hidden = true; return; }
    el.hidden = false;
    $("[data-step]", el).textContent = j.error ? `${j.step}: ${j.error}` : j.step;
    $("[data-bar]", el).style.width = `${Math.round(j.progress * 100)}%`;
    $("[data-bar]", el).parentElement.classList.toggle("indeterminate", j.running && j.progress >= 0.3);
    const log = $("[data-log]", el);
    log.textContent = j.log.slice(-12).join("\n");
    if (!j.running) { clearInterval(pollTimer); if (!j.error) refresh(); }
  };
  await tick();
  pollTimer = setInterval(tick, 1000);
}

async function renderSettings() {
  const node = $("#tpl-settings").content.cloneNode(true);
  const form = $("#settings-form", node);
  const cfg = await invoke("get_config");
  for (const [k, v] of Object.entries(cfg)) if (form.elements[k]) form.elements[k].value = v;
  form.querySelectorAll("[data-browse]").forEach((b) => (b.onclick = async () => {
    const d = await pick({ directory: true });
    if (d) form.elements[b.dataset.browse].value = d;
  }));
  form.querySelectorAll("[data-browse-file]").forEach((b) => (b.onclick = async () => {
    const f = await pick({ filters: [{ name: "Chaves", extensions: ["keys", "txt"] }] });
    if (f) form.elements[b.dataset.browseFile].value = f;
  }));
  form.onsubmit = async (ev) => {
    ev.preventDefault();
    await invoke("set_config", { config: Object.fromEntries(new FormData(form)) });
    const s = $("#saved");
    s.hidden = false;
    setTimeout(() => (s.hidden = true), 1500);
    games = await invoke("list_games");
    renderLibrary();
  };
  content.innerHTML = "";
  content.appendChild(node);
}

$("#open-settings").onclick = () => {
  current = "settings";
  clearInterval(pollTimer);
  library.querySelectorAll(".lib-item").forEach((b) => b.classList.remove("active"));
  renderSettings();
};

window.addEventListener("focus", () => { if (current !== "settings") refresh(); });
refresh();
