// SwitchOS Hub 0.0.2 - frontend. Fala com o backend Rust (src-tauri/src/lib.rs) via invoke.
// Duas telas: a inicial (prateleira só com jogos suportados, time do save, próximos jogos)
// e a página do jogo (Jogar, Versão e DLC, Mods, Save, Importar).
// Mapeamento (igual em todo lugar): A/Enter/clique no jogo selecionado = Jogar (jogo ainda não
// compilado: abre a página do jogo na seção Importação). Na inicial X abre a página do jogo (opções).
// Na página A ativa o que está em foco (Jogar ao abrir), Y abre a pasta do jogo, LB/RB (Q/E) trocam
// a seção, B/Esc volta. Cima/baixo andam entre zonas de foco (zones()); esquerda/direita andam
// dentro da zona; na inicial, fora do time e das ações, esquerda/direita sempre trocam o jogo.
// Mouse, teclado e controle (Gamepad API) passam todos por command().
const { invoke } = window.__TAURI__.core;
const { open: pick } = window.__TAURI__.dialog;
const { openPath } = window.__TAURI__.opener;

const $ = (sel, el = document) => el.querySelector(sel);
const $$ = (sel, el = document) => [...el.querySelectorAll(sel)];
const store = {
  get: (k, d) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } },
  set: (k, v) => { try { localStorage.setItem(k, v); } catch {} },
};

let games = [];
let current = store.get("hub.current", "arceus");
let tab = store.get("hub.tab", "about");
const picked = {}; // key -> [arquivos escolhidos para importar]
let pollTimer = null;
let ambLayer = 0;
let ambKey = null;
let lastShown = null;
let view = "home"; // "home" | "game"
let jobKey = null; // jogo cuja importação está sendo acompanhada
const busy = new Set(); // jogos com importação rodando (ponto na miniatura)
let confirmCancel = false;
const lastJob = {}; // key -> arquivos da última importação (para "Tentar de novo")
const jobWasRunning = {}; // key -> último running visto (o erro vira toast só na transição)
const jobs = {}; // key -> último job_status visto (o status do rodapé e da miniatura depende dele)

const TABS = ["about", "mods", "save", "import"];

// nome curto para a miniatura sem arte (jogos ainda não importados)
const shortName = (name) => name.replace(/^Pokémon( Legends:)?\s*/i, "").replace(/^Zelda:\s*/, "");
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
const fmtDate = (secs) => new Date(secs * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
const shelf = () => games.filter((g) => g.supported);
const game = () => shelf().find((g) => g.key === current) || shelf()[0];

// O status junta o estado do build com o do último job: nunca "Pronto" ao lado de uma importação falhada.
// tile: indicador da miniatura ("busy" ponto ciano, "failed" ponto coral); o texto vai no aria-label.
function status(g) {
  if (!g.supported) return { text: "Em breve no hub", cls: "soon" };
  const j = jobs[g.key];
  if (j?.running) return { text: `Importando: ${j.step}`, cls: "run", tile: "busy" };
  const ex = j?.error ? explainError(j.error, g, j.step) : null;
  const failed = ex && !ex.cancelled;
  if (g.built) return failed
    ? { text: "Pronto para jogar · a última importação falhou", cls: "ok", tile: "failed" }
    : unsynced(g) ? { text: "Pronto para jogar · save não sincronizado", cls: "warn" }
    : { text: "Pronto para jogar", cls: "ok" };
  if (failed) return { text: "Importação falhou", cls: "err", tile: "failed" };
  if (ex?.cancelled) return { text: "Importação cancelada", cls: "off" };
  if (g.imported) return { text: "Falta compilar", cls: "warn" };
  return { text: "Não importado", cls: "off" };
}
// pinta o status de um jogo no rodapé (se for o atual) e na miniatura
function paintStatus(key) {
  const g = games.find((x) => x.key === key);
  if (!g) return;
  const s = status(g);
  if (key === game()?.key && !(view === "home" && !$("#setup").hidden)) $("#g-status").innerHTML = `<i class="dot ${s.cls}"></i>${esc(s.text)}`;
  const t = $(`#row .tile[data-key="${key}"]`);
  if (t) {
    t.classList.toggle("busy", s.tile === "busy");
    t.classList.toggle("failed", s.tile === "failed");
    t.setAttribute("aria-label", `${g.name}: ${s.text}`);
  }
}
function setJob(key, j) {
  if (j) jobs[key] = j; else delete jobs[key];
  paintStatus(key);
  markBusy(key, !!j?.running);
}

/* ---------- dados ---------- */

async function refresh() {
  try { games = await invoke("list_games"); }
  catch (e) { return showLoadError(e); }
  $("#load-error").hidden = true;
  if (!shelf().some((g) => g.key === current)) current = shelf()[0]?.key;
  layoutHome();
  renderRow();
  renderUpcoming();
  renderGame();
  if (document.activeElement === document.body && !settingsOpen()) keepFocus(); // ao abrir: foco no jogo selecionado
  shelf().forEach((g) => invoke("job_status", { key: g.key }).then((j) => setJob(g.key, j)).catch(() => {}));
  shelf().forEach((g) => fetchSave(g, true)); // carga e foco da janela: relê o estado do save de cada jogo
}

// list_games rejeitado: tela de erro com "Tentar de novo" em vez de tela em branco
function showLoadError(e) {
  const box = $("#load-error");
  $("[data-detail]", box).textContent = String(e ?? "");
  box.hidden = false;
  $("#empty").hidden = true;
  $(".shelf").hidden = true;
  $("#team").hidden = true;
  $("#setup").hidden = $("#first-game").hidden = true;
  $("#g-status").innerHTML = `<i class="dot err"></i>A biblioteca não carregou`;
  if (!box.contains(document.activeElement)) $("[data-reload]", box).focus({ preventScroll: true });
}

/* ---------- primeiro uso ---------- */

// check_paths da config salva (null = ainda não conferido ou backend sem a checagem: não mostra nada)
let setupState = null;
let setupLeaving = false;
const SETUP_STEPS = [
  ["runtime_dir", "Pasta do programa", "A pasta com o lasrecomp.exe, o programa que roda os jogos."],
  ["games_dir", "Pasta dos jogos", "Onde cada jogo importado vai ficar."],
  ["keys_path", "prod.keys", "Chaves tiradas do seu próprio Switch. O hub não baixa jogos nem chaves."],
];
const setupPending = () => (setupState ? SETUP_STEPS.map(([k]) => k).filter((k) => !setupState[k]?.ok) : []);
const setupShown = () => !$("#setup").hidden && !setupLeaving;

async function checkSetup() {
  try { setupState = await invoke("check_paths", { config: await invoke("get_config") }); }
  catch { setupState = null; }
  if ($("#load-error").hidden) { layoutHome(); renderGame(); keepFocus(); }
  return setupPending();
}

// Decide o que a tela inicial mostra: primeiro uso, lista vazia, prateleira (+ orientação do primeiro jogo)
function layoutHome() {
  const box = $("#setup");
  const pending = setupPending();
  const none = !shelf().length;
  if (pending.length) {
    setupLeaving = false;
    box.classList.remove("leaving");
    $("#setup-steps").innerHTML = SETUP_STEPS.map(([k, name, why]) => {
      const ok = !pending.includes(k);
      return `<li class="${ok ? "done" : "todo"}"><span class="step-state">${ok ? "Feito" : "Falta"}</span><strong>${name}</strong><small>${why}</small></li>`;
    }).join("");
    box.hidden = false;
    applyGlyphs();
  } else if (!box.hidden && !setupLeaving) { // os 3 ficaram ok: o bloco sai e a prateleira entra no lugar
    setupLeaving = true;
    box.classList.add("leaving");
    setTimeout(() => {
      if (!setupLeaving) return;
      setupLeaving = false;
      box.hidden = true;
      box.classList.remove("leaving");
      layoutHome();
      lastShown = null; // o nome entra com a animação de sempre
      renderGame();
      keepFocus();
    }, 240);
  }
  const blocked = !box.hidden; // bloco na tela (ou saindo): a prateleira espera
  $("#empty").hidden = !none || blocked;
  $(".shelf").hidden = none || blocked;
  $("#first-game").hidden = none || blocked || shelf().some((g) => g.imported || g.built);
  if (none || blocked) $("#team").hidden = true;
  if (blocked) $("#g-status").innerHTML = `<i class="dot warn"></i>Falta configurar o hub`;
  else if (none) $("#g-status").innerHTML = `<i class="dot off"></i>Nenhum jogo na biblioteca`;
}

function markBusy(key, on) {
  if (on === busy.has(key)) return;
  if (on) busy.add(key); else busy.delete(key);
  if (!on && key !== jobKey) refresh(); // terminou em segundo plano: atualiza o status do jogo
}
// importações de outros jogos continuam sendo vigiadas enquanto você navega
setInterval(() => {
  for (const k of busy) if (!(k === jobKey && pollTimer))
    invoke("job_status", { key: k }).then((j) => setJob(k, j)).catch(() => {});
}, 2000);

/* ---------- prateleira ---------- */

function renderRow() {
  const row = $("#row");
  row.innerHTML = "";
  shelf().forEach((g) => {
    const b = document.createElement("button");
    b.type = "button";
    b.role = "option";
    b.className = "tile";
    b.dataset.key = g.key;
    b.style.setProperty("--c", g.color);
    const s = status(g);
    b.setAttribute("aria-label", `${g.name}: ${s.text}`);
    b.classList.toggle("busy", s.tile === "busy");
    b.classList.toggle("failed", s.tile === "failed");
    b.innerHTML = g.icon ? `<img src="${g.icon}" alt="" draggable="false">` : `<span class="mono-art">${esc(shortName(g.name))}</span>`;
    b.onclick = () => (g.key === current ? primary() : select(g.key, true)); // anteriores também: espiam à esquerda
    row.appendChild(b);
  });
  placeRow();
}

// O jogo selecionado fica sempre na mesma posição à esquerda; a fileira desliza por baixo.
function placeRow() {
  const row = $("#row");
  const idx = Math.max(0, shelf().findIndex((g) => g.key === current));
  row.querySelectorAll(".tile").forEach((t, i) => {
    const on = i === idx;
    t.classList.toggle("selected", on);
    t.classList.toggle("past", i < idx);
    t.setAttribute("aria-selected", on);
    t.tabIndex = on ? 0 : -1;
  });
  row.style.setProperty("--idx", idx);
}

function select(key, focus = false) {
  if (!key || key === current) return;
  current = key;
  store.set("hub.current", key);
  placeRow();
  renderGame();
  fetchSave(game()); // só lê do disco se ainda não houver cache deste jogo
  if (focus) $("#row .tile.selected")?.focus({ preventScroll: true });
}

function move(d) {
  const list = shelf();
  const i = list.findIndex((g) => g.key === current);
  const n = list[Math.min(list.length - 1, Math.max(0, i + d))];
  if (n) select(n.key, true);
}

// jogos com runtime próprio: fora da prateleira, só citados no fim da tela inicial
function renderUpcoming() {
  const soon = games.filter((g) => !g.supported);
  $("#upcoming").hidden = !soon.length;
  $("#up-names").textContent = soon.map((g) => g.name).join(", ");
}

/* ---------- página do jogo ---------- */

let viewTimer;
function setView(v) {
  if (v === view || !game()) return;
  view = v;
  const home = $("#view-home"), page = $("#view-game");
  const [show, hide] = v === "game" ? [page, home] : [home, page];
  clearTimeout(viewTimer);
  show.hidden = false;
  show.inert = false;
  hide.inert = true;
  requestAnimationFrame(() => { show.classList.add("on"); hide.classList.remove("on"); });
  viewTimer = setTimeout(() => { if (!hide.classList.contains("on")) hide.hidden = true; }, 420);
  renderGame();
  if (v === "game") {
    page.scrollTop = 0;
    renderTab(false); // a tinta das abas só mede com a página visível
    const playBtn = $('#p-meta [data-act="play"]');
    (playBtn && !playBtn.disabled ? playBtn : $("#tabs [aria-selected=true]"))?.focus({ preventScroll: true });
  } else {
    $("#row .tile.selected")?.focus({ preventScroll: true });
  }
}
const openGame = (t) => { if (t) setTab(t); setView("game"); };
// A / Enter / clique no selecionado: joga; sem compilação, leva para a seção Importação
function primary() {
  const g = game();
  if (!g) return;
  if (g.built) return play();
  if (view !== "game") openGame("import"); else setTab("import");
  focusEl($("[data-pick]"));
}
const closeGame = () => setView("home");

/* ---------- fundo ---------- */

function setAmbient(g) {
  if (ambKey === g.key + (g.icon ? "i" : "")) return;
  ambKey = g.key + (g.icon ? "i" : "");
  ambLayer ^= 1;
  const on = $(`[data-amb="${ambLayer}"]`);
  const off = $(`[data-amb="${ambLayer ^ 1}"]`);
  on.style.setProperty("--c", g.color);
  on.style.backgroundImage = g.icon ? `url("${g.icon}")` : ""; // sem arte: fica o brilho na cor do jogo (CSS)
  on.classList.add("on");
  off.classList.remove("on");
  document.documentElement.style.setProperty("--game", g.color);
}

/* ---------- jogo selecionado ---------- */

function renderGame() {
  const g = game();
  if (!g) return;
  setAmbient(g);

  const info = $("#info");
  if (lastShown !== g.key) { // só anima quando o jogo muda (não no refresh ao focar a janela)
    lastShown = g.key;
    info.classList.remove("swap");
    void info.offsetWidth;
    info.classList.add("swap");
    $("#team").classList.add("fresh");
  } else $("#team").classList.remove("fresh");
  $("#g-name").textContent = g.name;
  $("#g-name").title = g.name; // nome longo corta com reticências; o inteiro fica na dica

  const playLabel = playing ? "Abrindo…" : g.built ? "Jogar" : "Importar jogo";
  const playOff = playing ? "disabled" : "";
  const facts = `
    ${g.version ? `<span class="fact">Versão ${esc(g.version)}</span>` : ""}
    ${g.imported && g.dlcs.length ? `<span class="fact">DLC instalada</span>` : ""}
    ${g.mods.length ? `<span class="fact">${g.mods.length} mod${g.mods.length > 1 ? "s" : ""}</span>` : ""}
    <span class="fact warn" data-syncfact ${unsynced(g) ? "" : "hidden"}>Save não sincronizado</span>`;
  $("#g-meta").innerHTML = `
    <button class="act play" data-act="play" type="button" ${playOff}><kbd class="b-a" data-k="a"></kbd>${playLabel}</button>
    <button class="act" data-act="open" type="button"><kbd class="b-x" data-k="x"></kbd>Opções do jogo</button>${facts}`;
  $("#p-meta").innerHTML = `
    <button class="act play" data-act="play" type="button" ${playOff}><kbd class="b-a" data-k="a"></kbd>${playLabel}</button>
    <button class="act" data-act="folder" type="button"><kbd class="b-y" data-k="y"></kbd>Pasta do jogo</button>${facts}`;
  $("#p-tip").hidden = !g.built;
  document.querySelectorAll('[data-act="play"]').forEach((b) => (b.onclick = primary));
  $('[data-act="open"]').onclick = () => openGame();
  $('[data-act="folder"]').onclick = openFolder;
  $("#p-name").textContent = $("#p-name").title = g.name;
  $("#p-art").style.setProperty("--c", g.color);
  $("#p-art").innerHTML = g.icon ? `<img src="${g.icon}" alt="" draggable="false">` : `<span class="mono-art">${esc(shortName(g.name))}</span>`;

  paintStatus(g.key);
  renderKeys(g);

  // time do save
  const team = $("#team");
  team.hidden = !g.party.length || !$("#setup").hidden;
  $("#mons").innerHTML = g.party.map((m, i) => `
    <div class="mon${m.shiny ? " shiny" : ""}" style="--i:${i}">
      <div class="mon-art">${m.icon ? `<img src="${m.icon}" alt="">` : `<span class="ball"></span>`}</div>
      <div class="mon-text"><strong>${esc(m.egg ? "Ovo" : m.name)}</strong><small>${m.egg ? "&nbsp;" : `Nv. ${m.level}`}</small></div>
      ${m.shiny ? `<svg class="star" viewBox="0 0 24 24" role="img" aria-label="Shiny"><path d="M12 2.5l2.6 6.3 6.8.5-5.2 4.4 1.6 6.6L12 16.8l-5.8 3.5 1.6-6.6-5.2-4.4 6.8-.5z"/></svg>` : ""}
    </div>`).join("");

  renderTab();
  watchJob(g.key, true);
}

/* ---------- abas ---------- */

// Glifos: controle ou teclado, conforme a última tecla/botão usado. O mouse não muda o modo
// (quem clica vê o glifo da última entrada física; começa em controle).
// Todo <kbd data-k> segue o modo: o texto vem de GLYPHS e o desenho do atributo
// <html data-input> (círculo no controle, keycap no teclado). Conteúdo criado depois
// (toast, painéis, confirmações) é preenchido pelo MutationObserver abaixo.
const GLYPHS = {
  pad: { a: "A", b: "B", x: "X", y: "Y", lb: "LB", rb: "RB", menu: "Menu" },
  kbd: { a: "Enter", b: "Esc", x: "X", y: "Y", lb: "Q", rb: "E", menu: "M" },
};
let inputMode = "pad";
function setInputMode(m) {
  if (m === inputMode) return;
  inputMode = m;
  document.documentElement.dataset.input = m;
  syncKeys();
}
function applyGlyphs(root = document) { $$("kbd[data-k]", root).forEach((k) => (k.textContent = GLYPHS[inputMode][k.dataset.k])); }
document.documentElement.dataset.input = inputMode;
new MutationObserver((muts) => {
  for (const m of muts) for (const n of m.addedNodes) {
    if (n.nodeType !== 1) continue;
    if (n.matches("kbd[data-k]")) n.textContent = GLYPHS[inputMode][n.dataset.k];
    else applyGlyphs(n);
  }
}).observe(document.body, { childList: true, subtree: true });

// dicas do rodapé acompanham o contexto: camada aberta (erro, folha), confirmação, tela e foco
function renderKeys(g) {
  const k = (key, label, off = false) => `<span${off ? ' class="off"' : ""}><kbd class="b-${key === "menu" ? "lr" : key}" data-k="${key}"></kbd>${label}</span>`;
  const el = document.activeElement;
  let html;
  if (errorOpen()) html = k("a", "Selecionar") + k("b", "Fechar");
  else if (settingsOpen()) html = k("a", "Selecionar") + k("b", "Voltar");
  else if (view === "game" && confirmCancel) html = k("a", "Selecionar") + k("b", "Continuar importando");
  else if (view === "game" && $("[data-reimport-no]")) html = k("a", "Selecionar") + k("b", "Manter como está");
  else if (view === "game" && $("[data-restore-no]")) html = k("a", "Selecionar") + k("b", "Manter como está");
  else if (view === "game" && restoring) html = `<span>Restaurando o save…</span>`;
  // inicial: Jogar (A) e Opções (X) já aparecem ao lado do nome; o rodapé não repete
  // inicial: Jogar e Opções aparecem ao lado do nome (onde o mouse clica). No rodapé só voltam
  // no controle, menores e junto de Configurações, que é o lugar onde quem joga do sofá procura.
  else if (view === "home") html = (inputMode === "pad" ? k("a", g.built ? "Jogar" : "Importar jogo", playing) + k("x", "Opções") : "") + k("menu", "Configurações");
  else {
    const onBtn = el && $("#view-game").contains(el) && el.matches("button") && !el.matches('[data-act="play"]');
    const aTxt = el?.matches("#tabs [role=tab]") ? "Entrar na seção" : onBtn ? "Selecionar" : g.built ? "Jogar" : "Importar jogo";
    html = k("a", aTxt, aTxt === "Jogar" && playing) + k("y", "Pasta do jogo") +
      `<span class="lr"><kbd class="b-lr" data-k="lb"></kbd><kbd class="b-lr" data-k="rb"></kbd>Seções</span>` + k("b", "Voltar");
  }
  $("#keys").innerHTML = html;
  applyGlyphs();
}
function syncKeys() { const g = game(); if (g) renderKeys(g); else applyGlyphs(); }

function setTab(t) {
  if (!TABS.includes(t)) return;
  tab = t;
  store.set("hub.tab", t);
  renderTab();
}

function renderTab(animate = true) {
  const g = game();
  const tabs = $("#tabs");
  tabs.querySelectorAll("[data-tab]").forEach((b) => {
    const on = b.dataset.tab === tab;
    b.setAttribute("aria-selected", on);
    b.tabIndex = on ? 0 : -1;
    if (on) {
      tabs.style.setProperty("--ink-x", `${b.offsetLeft}px`);
      tabs.style.setProperty("--ink-w", `${b.offsetWidth}px`);
    }
  });
  const pane = $("#pane");
  if (!animate) return;
  pane.classList.remove("swap");
  void pane.offsetWidth;
  pane.classList.add("swap");
  pane.innerHTML = paneHTML[tab](g);
  paneWire[tab]?.(g, pane);
}

/* ---------- aba Save: o que o switchos.py registra (lib.rs: save_info) e a restauração (restore_backup) ---------- */
// O hub não sincroniza: mostra o save deste PC, a cópia na nuvem, a última sincronização e as cópias de
// segurança (backups/<TID>/<AAAAMMDD-HHMMSS>-<origem>). Cada cópia pode ser restaurada (com confirmação em
// linha; o save atual vira uma cópia "pc" antes) ou aberta na pasta.
const saveCache = {}; // key -> resultado de save_info (ou { error })
// Save deste PC mais novo que a última sincronização. Sem sync configurada (ou erro de leitura): false,
// porque isso não é um problema do jogo e a inicial não deve avisar nada.
function unsynced(g, info = saveCache[g.key]) {
  if (!info || info.error || !info.sync_config || !info.last_sync) return false;
  const local = info.local_modified ?? g.save_modified;
  return !!local && local > info.last_sync.at + 60;
}
// Lê save_info de um jogo (force: relê mesmo com cache) e atualiza o aviso da inicial e o status.
const saveReads = new Set();
async function fetchSave(g, force = false) {
  if (!g || (!force && saveCache[g.key]) || saveReads.has(g.key)) return;
  saveReads.add(g.key);
  let info;
  try { info = (await invoke("save_info", { key: g.key })) || { error: "sem resposta" }; } catch (e) { info = { error: String(e) }; }
  saveReads.delete(g.key);
  saveCache[g.key] = info;
  paintSync(g.key);
}
function paintSync(key) {
  const g = games.find((x) => x.key === key);
  if (!g) return;
  paintStatus(key);
  if (key === game()?.key) document.querySelectorAll("[data-syncfact]").forEach((f) => (f.hidden = !unsynced(g)));
}
const restored = {}; // key -> { when, from, created } da última restauração (aviso fixo na aba)
let restoring = false;
const ORIGIN = { pc: "PC", android: "Android", cloud: "nuvem" };
const originName = (o) => ORIGIN[o] || o;
// "20261118-153000" (hora local, como o switchos.py grava) -> Date
const stampDate = (s) => { const m = /^(\d{4})(\d\d)(\d\d)-(\d\d)(\d\d)(\d\d)$/.exec(s); return m ? new Date(m[1], m[2] - 1, m[3], m[4], m[5], m[6]) : null; };
const fmtWhen = (d) => d.toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
const joinPt = (xs) => (xs.length < 2 ? xs.join("") : `${xs.slice(0, -1).join(", ")} e ${xs.at(-1)}`);

function saveHTML(g, info) {
  const local = info?.local_modified ?? g.save_modified;
  const here = local ? `<strong>${fmtDate(local)}</strong>` : `<span class="muted">Ainda não há save</span>`;
  const rows = [`<div><dt>Neste PC</dt><dd>${here}</dd></div>`];
  let head, body = "", warn = "";
  if (!info) {
    head = `<p class="save-head">${local ? "Seu save está neste PC." : "Ainda não há save deste jogo."}</p>`;
    rows.push(`<div><dt>Sincronização</dt><dd class="muted">Lendo…</dd></div>`);
  } else if (info.error || !info.sync_config) {
    head = `<p class="save-head">${!local ? "Ainda não há save deste jogo." : info.error ? "Seu save está neste PC." : "Seu save está só neste PC."}</p>`;
    rows.push(`<div><dt>Sincronização</dt><dd><span class="pill warn">${info.error ? "Não deu para ler" : "Não configurada"}</span></dd></div>`);
    body = `<p class="lede">${info.error
      ? "O hub não conseguiu ler o estado da sincronização agora. O save deste PC não foi afetado."
      : "O hub não achou a configuração de sincronização do SwitchOS (o <code>games.toml</code>, ao lado do <code>switchos.py</code>). Sem ela, não há cópia na nuvem nem cópias de segurança: o save existe só nesta pasta."}</p>`;
  } else {
    const sync = info.last_sync;
    const syncAt = sync ? new Date(sync.at * 1000) : null;
    const sides = sync ? joinPt(["nuvem", ...sync.sides.map(originName)]) : "";
    rows.push(`<div><dt>Na nuvem</dt><dd>${info.cloud_exists
      ? `<strong>${info.cloud_modified ? fmtDate(info.cloud_modified) : "Cópia presente"}</strong>`
      : `<span class="muted">Nenhuma cópia ainda</span>`}</dd></div>`);
    rows.push(`<div><dt>Última sincronização</dt><dd>${sync
      ? `<strong>${fmtWhen(syncAt)}</strong><span class="muted">entre ${esc(sides)}</span>`
      : `<span class="muted">Nunca sincronizado</span>`}</dd></div>`);
    if (unsynced(g, info)) warn = `<p class="save-warnline"><span class="pill warn">O save deste PC mudou depois da última sincronização</span></p>
      <p class="fine sync-how">Para levar esta versão para a nuvem e para o Android, rode na pasta do SwitchOS:<br><code class="cmd">python switchos.py sync ${esc(g.key)}</code></p>`;
    const n = info.backups_total;
    head = `<p class="save-head">${n
      ? `Protegido por <strong>${n} cópia${n === 1 ? "" : "s"} de segurança</strong>.`
      : local ? "Seu save está aqui. Ainda não há cópias de segurança." : "Ainda não há save deste jogo."}</p>`;
    const list = info.backups.map((b) => {
      const d = stampDate(b.stamp);
      return `<li class="backup" data-zone data-name="${esc(b.name)}">
        <span class="b-when">${d ? fmtWhen(d) : esc(b.name)}</span>
        <span class="b-from">versão que estava ${b.origin === "cloud" ? "na" : "no"} ${esc(originName(b.origin))}</span>
        <span class="b-acts">
          <button class="btn" type="button" data-restore="${esc(b.name)}">Restaurar esta cópia</button>
          <button class="btn quiet" type="button" data-backup="${esc(b.path)}">Abrir pasta</button>
        </span></li>`;
    }).join("");
    const done = restored[g.key];
    body = `
      <h2 class="save-sub">Cópias de segurança${n > info.backups.length ? ` <span class="muted">(as ${info.backups.length} mais recentes de ${n})</span>` : ""}</h2>
      ${done ? `<p class="save-done" role="status"><strong>Save restaurado para a cópia de ${esc(done.when)}.</strong>
        ${done.created ? `O save que estava aqui ficou guardado como cópia de segurança (<code>${esc(done.created)}</code>).` : ""}
        A próxima sincronização leva esta versão para a nuvem e para o Android.</p>` : ""}
      ${n ? `<ul class="backups">${list}</ul>
      <p class="fine">Restaurar troca só o save deste PC. Feche o jogo antes.</p>`
      : `<p class="lede">Quando uma sincronização substituir um save, a versão anterior fica guardada aqui. O SwitchOS mantém as ${info.keep_backups ?? 10} mais recentes.</p>`}`;
  }
  return `${head}
    <dl class="facts save-facts">${rows.join("")}</dl>${warn}
    <div class="actions" data-zone>
      <button class="btn" data-save-folder type="button" ${local ? "" : "disabled"}>Abrir pasta do save</button>
      ${info?.cloud_exists ? `<button class="btn" data-cloud-folder type="button">Abrir cópia na nuvem</button>` : ""}
    </div>
    ${body}
    <p class="fine">O save fica na pasta do programa, em <code>save</code>, fora da pasta do jogo. Importar o jogo de novo não mexe nele.</p>`;
}

function wireSave(g, p) {
  const info = saveCache[g.key];
  const b = $("[data-save-folder]", p);
  if (b) b.onclick = () => openPath(g.save_dir).catch(() => toast("Ainda não há save deste jogo."));
  const c = $("[data-cloud-folder]", p);
  if (c) c.onclick = () => openPath(info.cloud_save).catch(() => toast("A cópia na nuvem não está mais lá."));
  $$("[data-backup]", p).forEach((el) => (el.onclick = () => openPath(el.dataset.backup).catch(() => toast("Essa cópia de segurança não está mais lá."))));
  $$("[data-restore]", p).forEach((el) => (el.onclick = () => askRestore(g, el.closest(".backup"))));
}

const backupLabel = (b) => { const d = stampDate(b.stamp); return { when: d ? fmtWhen(d) : b.name, from: originName(b.origin) }; };

// Confirmação em linha, na própria linha da cópia (como askReimport): foco em "Manter como está", B desiste
function askRestore(g, row) {
  if (restoring || !row) return;
  const b = saveCache[g.key]?.backups?.find((x) => x.name === row.dataset.name);
  if (!b) return;
  const { when, from } = backupLabel(b);
  const local = saveCache[g.key]?.local_modified ?? g.save_modified;
  row.classList.add("asking");
  $(".b-acts", row).innerHTML = `
    <span class="ask">Restaurar a cópia de ${esc(when)} (${esc(from)})? ${local
      ? "Seu save atual vira uma cópia de segurança antes."
      : "Ainda não há save neste PC: esta cópia passa a ser o save."}</span>
    <button class="btn primary" type="button" data-restore-yes>Restaurar</button>
    <button class="btn" type="button" data-restore-no>Manter como está</button>`;
  $("[data-restore-yes]", row).onclick = () => doRestore(g, b, row);
  $("[data-restore-no]", row).onclick = () => cancelRestore(g, b.name);
  $("[data-restore-no]", row).focus({ preventScroll: true }); // um A apressado não troca o save
  syncKeys();
}
// redesenha a aba e devolve o foco ao "Restaurar esta cópia" da mesma linha
function cancelRestore(g, name) {
  const body = $("#pane [data-save-body]");
  if (!body) return;
  body.innerHTML = saveHTML(g, saveCache[g.key]);
  wireSave(g, $("#pane"));
  focusEl($$("[data-restore]", body).find((x) => x.dataset.restore === name) || $("[data-save-folder]", body));
  syncKeys();
}

async function doRestore(g, b, row) {
  if (restoring) return;
  restoring = true;
  const { when, from } = backupLabel(b);
  row.classList.add("working");
  row.setAttribute("aria-busy", "true");
  $(".b-acts", row).innerHTML = `<span class="ask" role="status"><i class="spin" aria-hidden="true"></i>Restaurando a cópia de ${esc(when)}…</span>`;
  syncKeys();
  let res;
  try { res = await invoke("restore_backup", { key: g.key, backupName: b.name }); }
  catch (e) {
    restoring = false;
    cancelRestore(g, b.name);
    return errorToast(explainRestore(e));
  }
  restoring = false;
  restored[g.key] = { when, from, created: res?.created || null };
  try { saveCache[g.key] = (await invoke("save_info", { key: g.key })) || saveCache[g.key]; } catch {}
  const body = $("#pane [data-save-body]");
  if (body && tab === "save" && game()?.key === g.key) {
    body.innerHTML = saveHTML(g, saveCache[g.key]);
    wireSave(g, $("#pane"));
    focusEl($("[data-save-folder]", body));
  }
  toast("Save restaurado.");
  syncKeys();
}

// Erros do restore_backup (lib.rs, todos com "restaurar:") em linguagem clara; o resto cai no explainError comum
const RESTORE_ERRORS = [
  [/jogo está aberto|conferir se o jogo/, () => ({ title: "Feche o jogo antes de restaurar", action: "O save não foi trocado. Feche o jogo e tente de novo." })],
  [/importação em andamento/, () => ({ title: "Há uma importação em andamento", action: "O save não foi trocado. Espere a importação deste jogo terminar e tente de novo." })],
  [/não encontrada/, () => ({ title: "Essa cópia de segurança não está mais lá", action: "Ela pode ter sido apagada por uma sincronização. O save não foi trocado. Escolha outra cópia." })],
  [/sincronização não configurada/, () => ({ title: "A sincronização não está configurada", action: "Sem o games.toml do SwitchOS, o hub não sabe onde ficam as cópias. O save não foi trocado." })],
  [/access is denied|acesso negado|permission denied|os error 5\b|os error 32\b|em uso/, () => ({ title: "Sem permissão para trocar o save", action: "O save não foi trocado. Feche o jogo e programas que estejam usando a pasta do save (como a sincronização da nuvem) e tente de novo." })],
  [/restaurar:/, () => ({ title: "Não deu para restaurar", action: "O save atual continua no lugar. Se ele já tinha sido guardado, está entre as cópias de segurança.", unknown: true })],
];
function explainRestore(raw) {
  const text = String(raw ?? "").trim();
  const low = text.toLowerCase();
  for (const [re, f] of RESTORE_ERRORS) if (re.test(low)) return { ...f(), detail: text };
  return explainError(raw);
}

async function loadSave(g) {
  let info;
  try { info = (await invoke("save_info", { key: g.key })) || { error: "sem resposta" }; } catch (e) { info = { error: String(e) }; }
  const prev = JSON.stringify(saveCache[g.key]);
  saveCache[g.key] = info;
  paintSync(g.key);
  const body = $("#pane [data-save-body]");
  if (!body || tab !== "save" || game()?.key !== g.key || prev === JSON.stringify(info)) return;
  // troca o conteúdo sem perder o lugar do foco (mesmo índice entre os focáveis do painel)
  const before = focusables($("#pane"));
  const fi = before.indexOf(document.activeElement);
  body.innerHTML = saveHTML(g, info);
  wireSave(g, $("#pane"));
  if (fi >= 0) { const after = focusables($("#pane")); focusEl(after[Math.min(fi, after.length - 1)]); }
}

const paneHTML = {
  about: (g) => `
    <dl class="facts">
      <div><dt>Versão do jogo</dt><dd>${g.version
        ? `<span class="pill ok">${esc(g.version)}</span>`
        : `<span class="muted">${g.imported ? "Desconhecida. Importe de novo para ler." : "Jogo não importado"}</span>`}</dd></div>
      <div><dt>DLC</dt><dd>${g.imported && g.dlcs.length
        ? g.dlcs.map((d) => `<span class="pill ok" title="${esc(d.title_id)}">${esc(d.name)} instalada</span>`).join("")
        : `<span class="muted">Nenhuma instalada</span>`}</dd></div>
      ${g.built ? "" : `<div><dt>Situação</dt><dd>${hint(g)}</dd></div>`}
    </dl>
    <p class="fine">Código do jogo: <code>${esc(g.title_id)}</code></p>`,
  mods: (g) => `
    <p class="lede">Os mods ficam na pasta de mods, cada um na sua pasta, e valem só para este jogo.</p>
    <ul class="list">${g.mods.length
      ? g.mods.map((m) => `<li><strong>${esc(m.name)}</strong><small>${m.files} arquivo${m.files === 1 ? "" : "s"}</small></li>`).join("")
      : `<li class="empty">Nenhum mod para este jogo ainda.</li>`}</ul>
    <button class="btn" data-mods-folder type="button">Abrir pasta de mods</button>
    <p class="fine">Formato técnico: <code>${esc(g.title_id)}/romfs</code>, o mesmo do Atmosphère.</p>`,
  save: (g) => `<div class="save" data-save-body>${saveHTML(g, saveCache[g.key])}</div>`,
  import: (g) => `
    <p class="lede">Escolha o NSP/XCI do jogo base e da atualização. O hub extrai com o seu <code>prod.keys</code> e compila o código do jogo neste PC.</p>
    <ul class="list files" data-files></ul>
    <div class="actions">
      <button class="btn" data-pick type="button" ${g.supported ? "" : "disabled"}>Escolher arquivos</button>
      <button class="btn primary" data-import type="button" disabled>Importar e compilar</button>
    </div>`,
};

const paneWire = {
  mods: (g, p) => ($("[data-mods-folder]", p).onclick = async () => {
    try { await openPath(await invoke("ensure_dir", { path: await invoke("mods_dir") })); } catch (e) { fail(e); }
  }),
  save: (g, p) => {
    wireSave(g, p);
    loadSave(g);
  },
  import: (g, p) => {
    const files = $("[data-files]", p);
    const btn = $("[data-import]", p);
    const show = () => {
      const list = picked[g.key] || [];
      files.innerHTML = list.map((f) => `<li><strong>${esc(f.split(/[\\/]/).pop())}</strong></li>`).join("");
      btn.disabled = !list.length || !g.supported;
    };
    show();
    $("[data-pick]", p).onclick = async () => {
      const sel = await pick({ multiple: true, filters: [{ name: "Jogo do Switch", extensions: ["nsp", "xci", "nsz", "xcz", "nup"] }] });
      if (sel) { picked[g.key] = Array.isArray(sel) ? sel : [sel]; show(); }
    };
    btn.onclick = async () => {
      // já compilado: confirma em linha antes de substituir
      if (g.built && !p.querySelector("[data-reimport-yes]")) return askReimport(g, p);
      startImport(g.key);
    };
  },
};

// Reimportar um jogo já compilado: o save fica em <pasta do programa>/save, fora da pasta do
// jogo (<pasta dos jogos>/<jogo>), então a importação nunca toca nele (lib.rs: save_dir).
function askReimport(g, p) {
  const box = $(".actions", p);
  box.classList.add("ask-box");
  box.innerHTML = `
    <span class="ask">Este jogo já está pronto para jogar. Importar de novo substitui a compilação atual e leva de 10 a 30 min.
      Seu save não é tocado: ele fica guardado em outra pasta.</span>
    <button class="btn primary" data-reimport-yes type="button">Importar de novo</button>
    <button class="btn" data-reimport-no type="button">Manter como está</button>`;
  $("[data-reimport-yes]", box).onclick = () => startImport(g.key);
  $("[data-reimport-no]", box).onclick = () => { renderTab(); focusEl($("[data-pick]")); };
  $("[data-reimport-no]", box).focus({ preventScroll: true }); // um A apressado não substitui nada
  syncKeys();
}

async function startImport(key, files = picked[key]) {
  try { await invoke("import_game", { key, packages: files }); }
  catch (e) { return fail(e); }
  lastJob[key] = files;
  markBusy(key, true);
  if (tab === "import" && key === current) renderTab();
  watchJob(key);
}

function hint(g) {
  if (!g.supported) return `<span class="muted">Este jogo ainda roda fora do hub. Ele chega aqui nas próximas versões.</span>`;
  return `<span class="muted">Importe o jogo na seção Importação para liberar o Jogar.</span>`;
}

/* ---------- ações ---------- */

let playing = false;
async function play() {
  const g = game();
  if (!g?.built || playing || restoring) return; // não abre o jogo no meio de uma restauração do save
  playing = true;
  renderGame();
  try { await invoke("play_game", { key: g.key }); }
  catch (e) { playing = false; renderGame(); keepFocus(); return fail(e); } // falhou: libera na hora
  setTimeout(() => { playing = false; renderGame(); }, 3000);
}

function openFolder() {
  openPath(game().dir).catch(() => toast("A pasta do jogo ainda não existe. Importe o jogo primeiro."));
}

/* ---------- erros em linguagem clara ---------- */

// Traduz o texto cru que vem do Rust (ou do zaimport / lasrecomp, repassado pelo job) em
// { title, action, detail, settings }: o que aconteceu, o que fazer, o texto técnico original
// e se o conserto é nas Configurações. Padrões em minúsculas; o primeiro que casar vale.
const ERRORS = [
  [/cancelado pelo usu/, () => ({ title: "Importação cancelada", action: "O que já foi extraído ficou na pasta do jogo. Importe de novo quando quiser.", cancelled: true })],
  [/title key|title\.keys/, () => ({ title: "Falta o title.keys", action: "Coloque o arquivo title.keys do seu Switch na mesma pasta do prod.keys e tente de novo." })],
  [/prod\.keys não encontrado/, () => ({ title: "prod.keys não encontrado", action: "Abra as Configurações e escolha o arquivo prod.keys extraído do seu Switch.", settings: true })],
  [/has no header_key|prod\.keys has no|prod\.keys is needed/, () => ({ title: "O prod.keys está incompleto ou é antigo", action: "Extraia as chaves de novo com o Lockpick_RCM num Switch atualizado e escolha o arquivo novo nas Configurações.", settings: true })],
  [/isn't among these files|no game packages found/, (g) => ({ title: "Estes arquivos não são deste jogo", action: `Não achei o jogo base de ${g?.name || "este jogo"} nos arquivos escolhidos. Escolha o NSP ou XCI certo e tente de novo.`, pick: true })],
  [/not an nsp|wasn't downloaded|is damaged|ended early|cut short|no secure partition|\bnup\b|exefs|romfs/, () => ({ title: "Um dos arquivos está corrompido ou incompleto", action: "Copie ou extraia o arquivo do seu Switch de novo e escolha-o outra vez.", pick: true })],
  [/not enough space|no space left|os error 112|disk full|espaço insuficiente/, () => ({ title: "Falta espaço em disco", action: "Libere espaço no disco da pasta dos jogos e tente de novo." })],
  [/zaimport/, () => ({ title: "A ferramenta de importação não foi encontrada", action: "A pasta do programa escolhida não tem o zaimport. Confira nas Configurações ou reinstale o programa.", settings: true })],
  [/lasrecomp/, () => ({ title: "O programa do jogo não foi encontrado", action: "Nas Configurações, escolha a pasta do programa do hub (a que tem o lasrecomp.exe).", settings: true })],
  [/ainda não foi compilado/, () => ({ title: "Este jogo ainda não foi importado", action: "Abra a seção Importação, escolha os arquivos do jogo e importe para liberar o Jogar." })],
  [/ainda não roda no runtime/, (g) => ({ title: `${g?.name || "Este jogo"} ainda não roda no hub`, action: "Ele chega aqui nas próximas versões." })],
  [/não há importação em andamento/, () => ({ title: "A importação já tinha terminado", action: "Não havia nada para cancelar." })],
  [/importação em andamento/, () => ({ title: "Este jogo já está sendo importado", action: "Espere a importação atual terminar ou cancele antes de começar outra." })],
  [/escolha o jogo base/, () => ({ title: "Nenhum arquivo escolhido", action: "Escolha o NSP ou XCI do jogo base (e da atualização, se tiver)." })],
  [/access is denied|acesso negado|permission denied|os error 5\b/, () => ({ title: "Sem permissão para usar esta pasta", action: "Escolha outra pasta nas Configurações ou feche programas que estejam usando os arquivos.", settings: true })],
  [/os error 3\b|os error 2\b|cannot find the path|cannot find the file|não pode encontrar/, () => ({ title: "Uma pasta ou arquivo não foi encontrado", action: "Confira os caminhos nas Configurações.", settings: true })],
];
function explainError(raw, g = game(), step = "") {
  const text = String(raw ?? "").trim();
  const low = text.toLowerCase();
  for (const [re, f] of ERRORS) if (re.test(low)) return { ...f(g), detail: text };
  if (/compilar/i.test(step)) return { title: "A compilação falhou", action: "Tente de novo. Se falhar outra vez, veja o log: as últimas linhas dizem o motivo.", detail: text };
  if (/extrair/i.test(step)) return { title: "A extração do jogo falhou", action: "Confira se os arquivos são deste jogo e estão completos, e tente de novo. O log mostra o motivo.", detail: text };
  return { title: "Algo deu errado", action: "Tente de novo. Se continuar, veja os detalhes.", detail: text, unknown: true };
}

/* ---------- avisos (toast) ---------- */

// Aviso comum some sozinho (role=status). Erro fica até ser fechado (role=alert), recebe o foco
// para o controle alcançar os botões e fecha com B/Esc, clique em Fechar ou outra ação dele.
let toastTimer;
let toastBack = null; // foco antes do toast de erro
const toastEl = () => {
  let t = $("#toast");
  if (!t) { t = document.createElement("div"); t.id = "toast"; t.className = "toast"; document.body.appendChild(t); }
  return t;
};
const errorOpen = () => !!$("#toast.err.on");
// o aviso fica sempre logo acima do rodapé, cuja altura muda com a janela e com as dicas
new ResizeObserver(([e]) => document.documentElement.style.setProperty("--hints-h", `${Math.ceil(e.borderBoxSize?.[0]?.blockSize ?? e.target.offsetHeight)}px`)).observe($(".hints"));
function toast(msg) {
  if (errorOpen()) return; // não cobre um erro aberto
  const t = toastEl();
  t.className = "toast";
  t.role = "status";
  t.textContent = msg;
  requestAnimationFrame(() => {
    t.classList.add("on");
    // se o aviso cair em cima do que está em foco (ex.: última cópia da lista), rola a tela para cima
    const el = document.activeElement, v = el?.closest(".view");
    if (v) { const over = el.getBoundingClientRect().bottom + 12 - t.getBoundingClientRect().top; if (over > 0) v.scrollBy({ top: over + 12, behavior: "smooth" }); }
  });
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove("on"), 4000);
}
function errorToast(e, { logKey = null } = {}) {
  const t = toastEl();
  clearTimeout(toastTimer);
  if (!errorOpen()) toastBack = document.activeElement;
  t.className = "toast err";
  t.role = "alert";
  const more = e.unknown && e.detail;
  t.innerHTML = `
    <strong class="toast-title">${esc(e.title)}</strong>
    <p>${esc(e.action)}</p>
    ${more ? `<pre class="toast-detail" hidden>${esc(e.detail)}</pre>` : ""}
    <div class="toast-actions">
      ${logKey ? `<button class="btn" data-t="log" type="button">Ver log</button>` : ""}
      ${more ? `<button class="btn" data-t="detail" type="button" aria-expanded="false">Ver detalhes</button>` : ""}
      ${e.settings ? `<button class="btn" data-t="settings" type="button">Abrir configurações</button>` : ""}
      <button class="btn" data-t="close" type="button"><kbd class="b-b" data-k="b"></kbd>Fechar</button>
    </div>`;
  requestAnimationFrame(() => t.classList.add("on"));
  $('[data-t="close"]', t).onclick = () => closeError();
  if ($('[data-t="log"]', t)) $('[data-t="log"]', t).onclick = () => { closeError(false); showLog(logKey); };
  if ($('[data-t="settings"]', t)) $('[data-t="settings"]', t).onclick = () => { closeError(false); openSettings(); };
  if ($('[data-t="detail"]', t)) $('[data-t="detail"]', t).onclick = (ev) => {
    const d = $(".toast-detail", t);
    d.hidden = !d.hidden;
    ev.currentTarget.setAttribute("aria-expanded", !d.hidden);
    ev.currentTarget.textContent = d.hidden ? "Ver detalhes" : "Ocultar detalhes";
  };
  $(".toast-actions button", t).focus({ preventScroll: true });
  requestAnimationFrame(syncKeys); // errorOpen() só vale depois do .on
}
function closeError(restore = true) {
  const t = $("#toast");
  if (!t || !errorOpen()) return;
  t.classList.remove("on");
  if (restore && toastBack && visible(toastBack)) toastBack.focus({ preventScroll: true });
  toastBack = null;
  setTimeout(() => { if (!t.classList.contains("on")) t.innerHTML = ""; }, 300);
  keepFocus();
  syncKeys();
}
// erro vindo de um invoke: explica e mostra
const fail = (raw, opts) => errorToast(explainError(raw), opts);

/* ---------- importação em andamento ---------- */

// A caixa da importação fica na página do jogo, fora das seções: aparece em qualquer
// seção sem trocar a aba. Na tela inicial, o rodapé mostra a etapa.
function stopJob() { clearInterval(pollTimer); pollTimer = null; }
async function watchJob(key, quiet = false) {
  if (jobKey === key && pollTimer) return; // já acompanhando este jogo
  stopJob();
  jobKey = key;
  const el = $("#job");
  el.hidden = true;
  const tick = async () => {
    const j = await invoke("job_status", { key });
    if (key !== current) return stopJob();
    if (!j) { delete jobs[key]; paintStatus(key); return void (quiet && stopJob()); }
    jobs[key] = j;
    el.hidden = false;
    paintStatus(key);
    const ex = j.error ? explainError(j.error, games.find((x) => x.key === key), j.step) : null;
    el.dataset.state = j.running ? "run" : !ex ? "done" : ex.cancelled ? "cancel" : "err";
    $("[data-step]", el).textContent = ex ? ex.title : j.step;
    $("[data-step]", el).classList.toggle("err", !!ex && !ex.cancelled);
    $("[data-what]", el).textContent = ex ? ex.action : "";
    $("[data-what]", el).hidden = !ex;
    $(".bar", el).hidden = !!ex;
    markBusy(key, j.running);
    renderJobActions(j.running && j.step !== "Cancelando…", ex ? { key, log: j.log.length > 0, ex } : null);
    $("[data-bar]", el).style.width = `${Math.round(j.progress * 100)}%`;
    $(".bar", el).classList.toggle("indeterminate", j.running && j.progress >= 0.3);
    const log = $("[data-log]", el);
    log.textContent = (log.classList.contains("full") ? j.log : j.log.slice(-12)).join("\n");
    log.hidden = !!ex && !log.classList.contains("full"); // no erro, o log abre com "Ver log"
    if (jobWasRunning[key] && !j.running && ex && !ex.cancelled) errorToast(ex, { logKey: j.log.length ? key : null });
    jobWasRunning[key] = j.running;
    if (!j.running) { stopJob(); if (!j.error) refresh(); }
  };
  pollTimer = setInterval(tick, 1000);
  await tick();
}

// Cancelar: confirmação em linha (sem modal), dentro da própria caixa
function renderJobActions(on, ended = null) {
  const box = $("[data-job-actions]");
  const mode = on ? (confirmCancel ? "ask" : "idle") : ended ? `end:${ended.ex.title}:${ended.log}:${!!lastJob[ended.key]}` : "";
  if (box.dataset.mode === mode) return;
  const hadFocus = box.contains(document.activeElement);
  box.dataset.mode = mode;
  box.hidden = !mode;
  if (!on) confirmCancel = false;
  if (ended) { // terminou com erro ou cancelado: o conserto mais provável vem primeiro, depois o log
    const { ex } = ended;
    const again = !ex.pick && lastJob[ended.key]; // arquivo errado/corrompido: repetir não adianta
    box.innerHTML = `
      ${ex.settings ? `<button class="btn primary" data-settings type="button">Abrir configurações</button>` : ""}
      ${again ? `<button class="btn ${ex.settings ? "" : "primary"}" data-retry type="button">${ex.cancelled ? "Importar de novo" : "Tentar de novo"}</button>` : ""}
      ${!again ? `<button class="btn ${ex.settings ? "" : "primary"}" data-goimport type="button">${ex.pick ? "Escolher outros arquivos" : "Escolher arquivos"}</button>` : ""}
      ${ended.log ? `<button class="btn" data-showlog type="button" aria-expanded="${$("#job [data-log]").classList.contains("full")}">Ver log</button>` : ""}`;
    if ($("[data-retry]", box)) $("[data-retry]", box).onclick = () => startImport(ended.key, again);
    if ($("[data-settings]", box)) $("[data-settings]", box).onclick = () => openSettings();
    if ($("[data-goimport]", box)) $("[data-goimport]", box).onclick = () => { setTab("import"); focusEl($("[data-pick]")); };
    if ($("[data-showlog]", box)) $("[data-showlog]", box).onclick = () => toggleLog();
    if (hadFocus) $("button", box)?.focus({ preventScroll: true });
    return keepFocus();
  }
  box.innerHTML = !on ? "" : confirmCancel
    ? `<span class="ask">Cancelar a importação? O que já foi extraído fica na pasta do jogo.</span>
       <button class="btn danger" data-cancel-yes type="button">Sim, cancelar</button>
       <button class="btn" data-cancel-no type="button">Continuar importando</button>`
    : `<button class="btn" data-cancel type="button">Cancelar importação</button>`;
  const set = (v) => { confirmCancel = v; renderJobActions(true); };
  if ($("[data-cancel]", box)) $("[data-cancel]", box).onclick = () => set(true);
  if ($("[data-cancel-no]", box)) $("[data-cancel-no]", box).onclick = () => set(false);
  if ($("[data-cancel-yes]", box)) $("[data-cancel-yes]", box).onclick = async () => {
    try { await invoke("cancel_import", { key: jobKey }); } catch (e) { fail(e); }
    renderJobActions(false);
    $("#job [data-step]").textContent = "Cancelando…";
  };
  // na pergunta, o foco cai em "Continuar" para um A apressado não cancelar
  if (hadFocus) $(mode === "ask" ? "[data-cancel-no]" : "button", box)?.focus({ preventScroll: true });
  keepFocus();
  syncKeys();
}

// "Ver log": mostra o log inteiro da importação, na página do jogo
function toggleLog(open) {
  const log = $("#job [data-log]");
  const on = open ?? !log.classList.contains("full");
  log.classList.toggle("full", on);
  log.hidden = !on && $("#job").dataset.state === "err";
  $("#job [data-showlog]")?.setAttribute("aria-expanded", on);
  if ($("#job [data-showlog]")) $("#job [data-showlog]").textContent = on ? "Ocultar log" : "Ver log";
  if (on) { log.scrollTop = log.scrollHeight; log.scrollIntoView({ block: "nearest", behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" }); }
  return on;
}
async function showLog(key) {
  if (key !== current) select(key);
  if (view !== "game") openGame();
  await watchJob(key);
  toggleLog(true);
  focusEl($("#job [data-showlog]") || $("#job button"));
}

/* ---------- configurações (folha lateral) ---------- */

let lastFocus = null;
let savedTimer;

// texto padrão quando o Rust devolve msg vazia: a linha embaixo do campo nunca fica em branco
const CHECK_OK = { runtime_dir: "Achei o lasrecomp.exe nesta pasta.", games_dir: "Pasta dos jogos encontrada.", keys_path: "Arquivo de chaves encontrado." };
const CHECK_BAD = { runtime_dir: "O lasrecomp.exe não está nesta pasta.", games_dir: "Esta pasta não existe ou não pode ser usada.", keys_path: "O prod.keys não está neste caminho." };
// Confere os três caminhos no Rust (check_paths) e escreve o resultado embaixo de cada campo
async function checkPaths() {
  const form = $("#settings-form");
  let res = null;
  try { res = await invoke("check_paths", { config: Object.fromEntries(new FormData(form)) }); } catch {}
  if (!res) return true; // sem checagem (backend antigo): não bloqueia
  let ok = true;
  for (const [name, r] of Object.entries(res)) {
    const el = $(`[data-check="${name}"]`, form);
    if (!el) continue;
    ok &&= r.ok;
    el.hidden = false;
    el.className = `check ${r.ok ? "ok" : "bad"}`;
    el.textContent = r.msg?.trim() || (r.ok ? CHECK_OK : CHECK_BAD)[name] || (r.ok ? "Encontrado." : "Não encontrado.");
    form.elements[name]?.setAttribute("aria-invalid", !r.ok);
  }
  if (ok) $("#save-warn").hidden = true;
  return ok;
}
// field: nome do campo que recebe o foco (o primeiro pendente, no primeiro uso)
async function openSettings(field) {
  const sheet = $("#sheet"), scrim = $("#scrim"), form = $("#settings-form");
  const cfg = await invoke("get_config");
  for (const [k, v] of Object.entries(cfg)) if (form.elements[k]) form.elements[k].value = v;
  $("#save-warn").hidden = true;
  $("#saved").textContent = "";
  checkPaths();
  lastFocus = document.activeElement;
  sheet.hidden = scrim.hidden = false;
  $(".stage").inert = $(".topbar").inert = $(".hints").inert = true;
  requestAnimationFrame(() => { sheet.classList.add("on"); scrim.classList.add("on"); document.body.classList.add("sheet-open"); });
  form.elements[typeof field === "string" && form.elements[field] ? field : "runtime_dir"].focus({ preventScroll: true });
  requestAnimationFrame(syncKeys); // settingsOpen() só vale depois do .on
}
function closeSettings() {
  const sheet = $("#sheet"), scrim = $("#scrim");
  if (sheet.hidden) return;
  sheet.classList.remove("on"); scrim.classList.remove("on"); document.body.classList.remove("sheet-open");
  $(".stage").inert = $(".topbar").inert = $(".hints").inert = false;
  setTimeout(() => { if (!sheet.classList.contains("on")) sheet.hidden = scrim.hidden = true; }, 320);
  lastFocus?.focus?.({ preventScroll: true });
  keepFocus();
  syncKeys();
}
const settingsOpen = () => !$("#sheet").hidden && $("#sheet").classList.contains("on");

function wireSettings() {
  const form = $("#settings-form");
  form.querySelectorAll("[data-browse]").forEach((b) => (b.onclick = async () => {
    const d = await pick({ directory: true });
    if (d) { form.elements[b.dataset.browse].value = d; checkPaths(); }
  }));
  form.querySelectorAll("[data-browse-file]").forEach((b) => (b.onclick = async () => {
    const f = await pick({ filters: [{ name: "Chaves", extensions: ["keys", "txt"] }] });
    if (f) { form.elements[b.dataset.browseFile].value = f; checkPaths(); }
  }));
  // cada campo é conferido ao sair dele; o Salvar confere tudo de novo
  form.querySelectorAll("input").forEach((i) => (i.onchange = () => checkPaths())); // onchange: abrir de novo não empilha
  form.onsubmit = async (ev) => {
    ev.preventDefault();
    const force = ev.submitter?.hasAttribute("data-force");
    const ok = await checkPaths();
    if (!ok && !force) {
      $("#save-warn").hidden = false;
      return focusEl($("#save-warn [data-force]"));
    }
    try { await invoke("set_config", { config: Object.fromEntries(new FormData(form)) }); }
    catch (e) { return fail(e); }
    $("#save-warn").hidden = true;
    const s = $("#saved");
    clearTimeout(savedTimer);
    s.textContent = ok ? "Configurações salvas." : "Salvas, mas com caminhos a corrigir.";
    savedTimer = setTimeout(() => (s.textContent = ""), 6000);
    if (!visible(document.activeElement)) focusEl(form.querySelector('[type="submit"]'));
    refresh();
    checkSetup();
  };
  $("#open-settings").onclick = openSettings;
  $("#close-settings").onclick = closeSettings;
  $("#scrim").onclick = closeSettings;
}

/* ---------- entrada: teclado e controle ---------- */

const typing = () => /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName);

/* ---------- zonas de foco: cima/baixo trocam de zona, esquerda/direita andam dentro dela ---------- */

const visible = (e) => !!e && !e.disabled && !e.closest("[hidden], [inert]") && e.getClientRects().length > 0;
const focusables = (root) => (root ? $$("button, input, [tabindex]:not([tabindex='-1'])", root).filter(visible) : []);
// Painel da aba: uma zona só, ou uma por [data-zone] (a lista de backups anda linha a linha com cima/baixo)
function paneZones() {
  const p = $("#pane");
  const parts = $$("[data-zone]", p);
  if (!parts.length) return [{ id: "pane", els: focusables(p) }];
  return parts.map((z, i) => ({ id: `pane-${i}`, els: focusables(z) }));
}
function zones() {
  if (settingsOpen()) return [{ id: "sheet", els: focusables($("#sheet")) }]; // a folha é uma lista só
  const z = view === "home"
    ? [{ id: "setup", els: focusables($("#setup")) }, { id: "act", els: focusables($("#g-meta")) }, { id: "row", els: [$("#row .tile.selected")] },
       { id: "empty", els: [...focusables($("#empty")), ...focusables($("#load-error"))] }]
    // ordem visual da página: topo, caixa do job (fica acima das abas), abas, painel da aba
    : [{ id: "top", els: [$("#p-back"), ...focusables($("#p-meta"))] }, { id: "job", els: focusables($("#job")) },
       { id: "tabs", els: [$("#tabs [aria-selected=true]")] }, ...paneZones()];
  return z.map((x) => ({ ...x, els: x.els.filter(visible) })).filter((x) => x.els.length);
}
const focusEl = (el) => el?.focus({ preventScroll: false });
function defaultFocus() {
  if (settingsOpen()) return $("#settings-form").elements.runtime_dir;
  if (view === "home") return [$("#setup [data-setup-go]"), $("#row .tile.selected"), $("#load-error button"), $("#empty button")].find(visible);
  const p = $('#p-meta [data-act="play"]');
  return visible(p) ? p : $("#tabs [aria-selected=true]");
}
// o foco nunca fica no vazio nem num elemento escondido/inerte
function keepFocus() {
  const el = document.activeElement;
  if (!el || el === document.body || !visible(el)) focusEl(defaultFocus());
}
function nav(dir) {
  const zs = zones();
  const el = document.activeElement;
  const zi = zs.findIndex((z) => z.els.includes(el));
  // inicial: esquerda/direita trocam o jogo com o foco na prateleira, no corpo ou fora das zonas
  if (view === "home" && (dir === "left" || dir === "right") && !["act", "setup"].includes(zs[zi]?.id))
    return move(dir === "right" ? 1 : -1);
  if (zi < 0) return focusEl(defaultFocus());
  const z = zs[zi];
  if (z.id === "sheet") { // folha: cima/baixo percorrem campos e botões, em volta
    if (dir !== "up" && dir !== "down") return;
    const i = z.els.indexOf(el) + (dir === "down" ? 1 : -1);
    return focusEl(z.els[(i + z.els.length) % z.els.length]);
  }
  if (dir === "up" || dir === "down") {
    const n = zs[zi + (dir === "down" ? 1 : -1)];
    if (!n) return;
    // na zona nova, o item mais perto na horizontal
    const cx = (e) => { const r = e.getBoundingClientRect(); return r.left + r.width / 2; };
    const x = cx(el);
    return focusEl(n.els.reduce((a, b) => (Math.abs(cx(b) - x) < Math.abs(cx(a) - x) ? b : a)));
  }
  const d = dir === "right" ? 1 : -1;
  if (z.id === "row") return move(d);
  if (z.id === "tabs") return stepTab(d);
  const i = z.els.indexOf(el) + d;
  if (z.els[i]) focusEl(z.els[i]);
}
function stepTab(d) {
  setTab(TABS[(TABS.indexOf(tab) + TABS.length + d) % TABS.length]);
  $("#tabs [aria-selected=true]")?.focus({ preventScroll: true });
}

const DIRS = ["up", "down", "left", "right"];
function command(c) {
  document.body.classList.add("nav-key");
  if (!DIRS.includes(c) && !errorOpen()) keepFocus();
  const el = document.activeElement;
  if (errorOpen()) { // o toast de erro é a primeira camada: B fecha, esquerda/direita andam nos botões
    const bs = $$("#toast .toast-actions button");
    if (c === "back") return closeError();
    if (c === "a" && el?.closest("#toast")) return el.click();
    if (c === "left" || c === "right") return focusEl(bs[Math.max(0, Math.min(bs.length - 1, bs.indexOf(el) + (c === "right" ? 1 : -1)))] || bs[0]);
    if (!el?.closest("#toast")) closeError(false); // outro comando: o erro sai do caminho
    else return;
  }
  if (settingsOpen()) {
    if (c === "back" || c === "menu") return closeSettings();
    if (c === "up" || c === "down") return nav(c);
    if (c === "a" && el?.matches("button")) return el.click();
    return;
  }
  if (DIRS.includes(c)) return nav(c);
  if (view === "home") switch (c) {
    case "a":
      if (el?.matches("#info button, #setup button")) return el.click();
      return setupShown() ? openSetup() : primary(); // configuração pendente: A = Configurar
    case "x": return openGame();
    case "menu": return openSettings();
  } else switch (c) {
    case "back": // B desfaz uma confirmação aberta antes de sair da página
      if (confirmCancel) return $("[data-cancel-no]")?.click();
      if ($("[data-reimport-no]")) return $("[data-reimport-no]").click();
      if ($("[data-restore-no]")) return $("[data-restore-no]").click();
      if (restoring) return; // não sai da página no meio da restauração
      return closeGame();
    case "a":
      // numa aba, A desce para o painel; num botão, ativa; sem foco útil, joga
      if (el?.matches("#tabs [role=tab]")) return focusEl(zones().find((z) => z.id.startsWith("pane"))?.els[0]);
      if (el?.matches("button") && $("#view-game").contains(el)) return el.click();
      return primary();
    case "y": return openFolder();
    case "lb": return stepTab(-1);
    case "rb": return stepTab(1);
    case "menu": return openSettings();
  }
}

document.addEventListener("keydown", (e) => {
  if (!["Shift", "Control", "Alt", "Meta"].includes(e.key)) setInputMode("kbd");
  if (e.key === "Escape") return command("back");
  // Tab preso dentro da folha de configurações
  if (e.key === "Tab" && settingsOpen()) {
    const els = focusables($("#sheet"));
    const i = els.indexOf(document.activeElement);
    e.preventDefault();
    return els[e.shiftKey ? (i <= 0 ? els.length - 1 : i - 1) : (i + 1) % els.length]?.focus();
  }
  if (e.key === "Tab") return document.body.classList.add("nav-key");
  if (e.altKey || e.ctrlKey || e.metaKey) return;
  // num campo de texto só cima/baixo saem dele (numa linha só, não editam nada)
  if (typing() && e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
  if (e.key === "Backspace") return command("back");
  const k = { ArrowLeft: "left", ArrowRight: "right", ArrowUp: "up", ArrowDown: "down", q: "lb", e: "rb", x: "x", y: "y", m: "menu" }[e.key];
  if (k) { e.preventDefault(); return command(k); }
  if (e.key !== "Enter") return;
  // Enter = A: na prateleira (ou fora de um botão) joga; um botão focado age por conta própria
  const btn = document.activeElement?.closest?.("button");
  if (!btn || btn.closest("#row")) { e.preventDefault(); command("a"); }
});
document.addEventListener("pointerdown", () => document.body.classList.remove("nav-key"));
document.addEventListener("focusin", syncKeys);
$("#p-back").onclick = closeGame;

$("#tabs").addEventListener("click", (e) => { const b = e.target.closest("[data-tab]"); if (b) setTab(b.dataset.tab); });

// Gamepad API (layout padrão: 0=A 1=B 2=X 3=Y 4=LB 5=RB 9=Menu 12-15=direcional; analógico esquerdo também anda)
const padPrev = {};
let padRepeat = 0;
function pollPads(t) {
  for (const p of navigator.getGamepads?.() || []) {
    if (!p) continue;
    const ax = p.axes[0] || 0, ay = p.axes[1] || 0;
    const held = {
      left: p.buttons[14]?.pressed || ax < -0.6, right: p.buttons[15]?.pressed || ax > 0.6,
      up: p.buttons[12]?.pressed || ay < -0.6, down: p.buttons[13]?.pressed || ay > 0.6,
      a: p.buttons[0]?.pressed, back: p.buttons[1]?.pressed, x: p.buttons[2]?.pressed, y: p.buttons[3]?.pressed,
      lb: p.buttons[4]?.pressed, rb: p.buttons[5]?.pressed, menu: p.buttons[9]?.pressed,
    };
    for (const [c, on] of Object.entries(held)) {
      const id = p.index + c;
      const dirc = c === "left" || c === "right" || c === "up" || c === "down";
      if (on && !padPrev[id]) { setInputMode("pad"); command(c); if (dirc) padRepeat = t + 380; }
      else if (on && dirc && t > padRepeat) { command(c); padRepeat = t + (c === "up" || c === "down" ? 200 : 120); }
      padPrev[id] = on;
    }
  }
  requestAnimationFrame(pollPads);
}
requestAnimationFrame(pollPads);

/* ---------- relógio ---------- */

function tickClock() {
  $("#clock").textContent = new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}
tickClock();
setInterval(tickClock, 15000);

window.addEventListener("focus", () => { if (!settingsOpen()) { refresh(); checkSetup(); } });
window.addEventListener("resize", () => renderTab(false));
wireSettings();
$("#load-error [data-reload]").onclick = () => refresh();
$("#load-error [data-settings]").onclick = () => openSettings();
$("#empty [data-settings]").onclick = () => openSettings();
const openSetup = () => openSettings(setupPending()[0]);
$("#setup [data-setup-go]").onclick = openSetup;
// Primeira execução: o Rust não diz se já existe hub.json (get_config devolve os padrões), então um
// marcador local decide. Só abre a folha sozinha uma vez, e só se algum caminho estiver pendente.
(async () => {
  await refresh();
  const pending = await checkSetup();
  if (store.get("hub.setup-seen") !== "1") {
    store.set("hub.setup-seen", "1");
    if (pending.length && !settingsOpen()) openSettings(pending[0]);
  }
})();
