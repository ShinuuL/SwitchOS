//! SwitchOS Hub 0.0.1 - biblioteca dos jogos recompilados, importação, mods, saves e "Jogar".
//!
//! O hub não roda jogo nenhum: ele organiza as pastas e chama o runtime unificado (lasrecomp.exe, engines/las-za) e o
//! importador (zaimport.exe). Configuração em %APPDATA%\SwitchOS\hub.json.

mod firstrun;
mod launch;
mod mods;
mod party;
mod saveimport;
mod sync;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ------------------------------------------------------------------ catálogo

#[derive(Serialize, Clone)]
struct GameDef {
    key: &'static str,
    name: &'static str,
    title_id: &'static str,
    /// roda no runtime unificado do hub (os outros ainda têm runtime próprio, fora do hub)
    supported: bool,
    color: &'static str,
}

const CATALOG: &[GameDef] = &[
    GameDef { key: "arceus", name: "Pokémon Legends: Arceus", title_id: "01001F5010DFA000", supported: true, color: "#c9a227" },
    GameDef { key: "za", name: "Pokémon Legends: Z-A", title_id: "0100F43008C44000", supported: true, color: "#2f7de1" },
    GameDef { key: "sword", name: "Pokémon Sword", title_id: "0100ABF008968000", supported: false, color: "#3aa0d8" },
    GameDef { key: "shield", name: "Pokémon Shield", title_id: "01008DB008C2C000", supported: false, color: "#d8425a" },
    GameDef { key: "bdsp", name: "Pokémon Brilliant Diamond", title_id: "0100000011D90000", supported: false, color: "#5b8def" },
    GameDef { key: "totk", name: "Zelda: Tears of the Kingdom", title_id: "0100F2C0115B6000", supported: false, color: "#3f8f5b" },
];

fn game_def(key: &str) -> Result<&'static GameDef, String> {
    CATALOG.iter().find(|g| g.key == key).ok_or_else(|| format!("jogo desconhecido: {key}"))
}

// ------------------------------------------------------------------ configuração

#[derive(Serialize, Deserialize, Clone, Default)]
struct Config {
    /// pasta do runtime (a que tem lasrecomp.exe; também é a pasta de dados: save/, mods/, logs/)
    runtime_dir: String,
    /// onde cada jogo importado fica: <games_dir>/<key>/ (exefs/, romfs.bin, lasgame.dll)
    games_dir: String,
    /// prod.keys do seu Switch (title.keys ao lado, se houver)
    keys_path: String,
}

fn config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("SwitchOS").join("hub.json")
}

/// Padrões: o repositório onde o hub está (dev) ou a pasta do executável
fn default_config() -> Config {
    let exe = std::env::current_exe().unwrap_or_default();
    let mut root = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    for _ in 0..6 {
        if root.join("engines").join("las-za").exists() {
            break;
        }
        if !root.pop() {
            break;
        }
    }
    let runtime = root.join("engines").join("las-za").join("build").join("dist-za").join("ZA Recomp");
    Config {
        runtime_dir: runtime.to_string_lossy().into(),
        games_dir: "D:\\Games\\SwitchOS".into(),
        keys_path: [PathBuf::from(r"D:\Games\Dumps\prod.keys"), dirs::home_dir().unwrap_or_default().join("Downloads").join("prod.keys")]
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
    }
}

#[tauri::command]
fn get_config() -> Config {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(default_config)
}

#[tauri::command]
fn set_config(config: Config) -> Result<(), String> {
    let p = config_path();
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&p, serde_json::to_string_pretty(&config).unwrap()).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ estado de cada jogo

#[derive(Serialize)]
struct GameState {
    #[serde(flatten)]
    def: GameDef,
    dir: String,
    imported: bool,
    built: bool,
    save_dir: String,
    save_modified: Option<u64>,
    mods: Vec<mods::ModInfo>,
    /// ícone do jogo (extraído do Control NCA pelo zaimport) como data URL
    icon: Option<String>,
    /// versão do jogo instalada (version.txt: DisplayVersion do Control NCA, gravado pelo zaimport)
    version: Option<String>,
    /// DLCs instaladas (<pasta do jogo>/aoc/<title id>.bin, lidas pelo serviço aoc:u do runtime)
    dlcs: Vec<DlcInfo>,
    /// o time do save (Z-A e Arceus: party.rs)
    party: Vec<party::PartyMon>,
    /// o runtime mudou desde a compilação: o código do jogo precisa ser compilado de novo (gamelib_stale)
    needs_rebuild: bool,
}

/// O código do jogo foi compilado por outro runtime? Compara o `gamelib.id` da pasta do programa
/// (gravado pelo tools/za/dist.sh: o GAMELIB_ID do lasrecomp.exe) com o `lasgame.dll.id` do jogo
/// (GAMELIB_ID[-v2]-<build id do jogo>[-p<patches>], runtime/gamelib.h). Sem um dos dois, não avisa.
fn gamelib_stale(runtime_dir: &Path, game_dir: &Path) -> bool {
    let read = |p: PathBuf| fs::read_to_string(p).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    let (Some(rt), Some(game)) = (read(runtime_dir.join("gamelib.id")), read(game_dir.join("lasgame.dll.id"))) else {
        return false;
    };
    // só o primeiro trecho é do runtime; o "-v2" (CPU sem AVX2) é decidido pelo runtime nesta máquina
    let first = |s: &str| s.split('-').next().unwrap_or("").to_ascii_lowercase();
    first(&rt) != first(&game)
}

#[derive(Serialize)]
struct DlcInfo {
    title_id: String,
    name: String,
}

fn dlc_name(tid: &str) -> String {
    match tid {
        "0100f43008c45001" => "Nintendo Switch 2 Edition".into(),
        "0100f43008c45002" => "Mega Dimension".into(),
        _ => "Conteúdo adicional".into(),
    }
}

fn dlcs_in(dir: &Path) -> Vec<DlcInfo> {
    let mut v: Vec<DlcInfo> = fs::read_dir(dir.join("aoc"))
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().to_lowercase();
                    let tid = n.strip_suffix(".bin")?.to_string();
                    Some(DlcInfo { name: dlc_name(&tid), title_id: tid })
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    v
}

fn save_dir(cfg: &Config, g: &GameDef) -> PathBuf {
    // (runtime/services/fs.c: Z-A em save/save_0, os outros em save/<title id>/save_0)
    let base = Path::new(&cfg.runtime_dir).join("save");
    if g.key == "za" { base.join("save_0") } else { base.join(g.title_id.to_lowercase()).join("save_0") }
}

#[tauri::command]
fn list_games() -> Vec<GameState> {
    let cfg = get_config();
    CATALOG
        .iter()
        .map(|g| {
            let dir = Path::new(&cfg.games_dir).join(g.key);
            let sd = save_dir(&cfg, g);
            let save_modified = fs::read_dir(&sd).ok().and_then(|rd| {
                rd.flatten()
                    .filter_map(|e| e.metadata().ok()?.modified().ok())
                    .max()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
            });
            GameState {
                def: g.clone(),
                imported: dir.join("exefs").join("main").is_file() && dir.join("romfs.bin").is_file(),
                built: dir.join("lasgame.dll").is_file(),
                dir: dir.to_string_lossy().into(),
                save_dir: sd.to_string_lossy().into(),
                save_modified,
                mods: mods::list(Path::new(&cfg.runtime_dir), g),
                version: fs::read_to_string(dir.join("version.txt")).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
                dlcs: dlcs_in(&dir),
                party: match g.key {
                    "za" => party::load(&sd.join("main"), &Path::new(&cfg.runtime_dir).join("cache").join("party_icons"), &party::ZA),
                    "arceus" => party::load(&sd.join("main"), &Path::new(&cfg.runtime_dir).join("cache").join("party_icons"), &party::ARCEUS),
                    _ => vec![],
                },
                needs_rebuild: dir.join("lasgame.dll").is_file() && gamelib_stale(Path::new(&cfg.runtime_dir), &dir),
                icon: fs::read(dir.join("icon.jpg")).ok().map(|b| {
                    use base64::Engine;
                    format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(b))
                }),
            }
        })
        .collect()
}

// ------------------------------------------------------------------ jogar

#[tauri::command]
fn play_game(key: String) -> Result<u32, String> {
    let cfg = get_config();
    let g = game_def(&key)?;
    if !g.supported {
        return Err(format!("{} ainda não roda no runtime do hub", g.name));
    }
    launch::play(Path::new(&cfg.runtime_dir), &Path::new(&cfg.games_dir).join(g.key), runtime_processes())
}

/// Liga ou desliga um mod (pasta com/sem "_" em <runtime>/mods). Só com o jogo fechado.
/// Devolve true se o mod tem código (.pchtxt): o próximo "Jogar" recompila o código do jogo.
#[tauri::command]
fn set_mod_enabled(key: String, name: String, on: bool, state: tauri::State<Jobs>) -> Result<bool, String> {
    game_def(&key)?;
    if state.lock().unwrap().values().any(|j| j.running) {
        return Err("Espere a importação terminar para mexer nos mods.".into());
    }
    match runtime_processes() {
        Some(0) => {}
        Some(_) => return Err("Feche o jogo antes de ligar ou desligar mods: ele lê os mods ao abrir e pode estar usando os arquivos.".into()),
        None => return Err("Não consegui conferir se o jogo está aberto. Feche o jogo e tente de novo.".into()),
    }
    mods::set(Path::new(&get_config().runtime_dir), &name, on)
}

// ------------------------------------------------------------------ importar (zaimport + --build-game, em segundo plano)

#[derive(Serialize, Clone, Default)]
struct Job {
    running: bool,
    step: String,
    progress: f64,
    log: Vec<String>,
    error: Option<String>,
    /// processo filho da etapa atual (para cancelar)
    #[serde(skip)]
    pid: Option<u32>,
    #[serde(skip)]
    cancel: bool,
}

const CANCELLED: &str = "Cancelado pelo usuário";

fn cancelled(jobs: &Jobs, key: &str) -> bool {
    jobs.lock().unwrap().get(key).map(|j| j.cancel).unwrap_or(false)
}

/// Mata o processo e os filhos dele (o --build-game dispara compiladores)
fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let mut c = Command::new("taskkill");
        c.args(["/PID", &pid.to_string(), "/T", "/F"]).creation_flags(CREATE_NO_WINDOW);
        let _ = c.stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
    }
}

type Jobs = Arc<Mutex<HashMap<String, Job>>>;

fn job_update(jobs: &Jobs, key: &str, f: impl FnOnce(&mut Job)) {
    let mut m = jobs.lock().unwrap();
    f(m.entry(key.to_string()).or_default());
}

/// Roda um processo sem janela e repassa cada linha ao job ("progress a b" do zaimport vira porcentagem)
fn run_logged(jobs: &Jobs, key: &str, mut cmd: Command, base: f64, span: f64) -> Result<(), String> {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("não iniciou {}: {e}", cmd.get_program().to_string_lossy()))?;
    let pid = child.id();
    job_update(jobs, key, |j| j.pid = Some(pid));
    if cancelled(jobs, key) { kill_tree(pid); } // cancelou entre uma etapa e outra
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    // a última linha "error: ..." (zaimport) diz o motivo real; sem ela, só o código de saída
    let last_err: Arc<Mutex<Option<String>>> = Arc::default();
    let note = |line: &str, le: &Arc<Mutex<Option<String>>>| {
        let t = line.trim_start();
        if let Some(m) = t.strip_prefix("error:").or_else(|| t.strip_prefix("Error:")) {
            *le.lock().unwrap() = Some(m.trim().to_string());
        }
    };
    let (j2, k2, le2) = (jobs.clone(), key.to_string(), last_err.clone());
    let t = std::thread::spawn(move || {
        for line in BufReader::new(err).lines().map_while(Result::ok) {
            note(&line, &le2);
            job_update(&j2, &k2, |j| {
                j.log.push(line);
                if j.log.len() > 400 { j.log.remove(0); }
            });
        }
    });
    for line in BufReader::new(out).lines().map_while(Result::ok) {
        let mut p = line.split_whitespace();
        if p.next() == Some("progress") {
            if let (Some(a), Some(b)) = (p.next().and_then(|x| x.parse::<f64>().ok()), p.next().and_then(|x| x.parse::<f64>().ok())) {
                if b > 0.0 {
                    job_update(jobs, key, |j| j.progress = base + span * a / b);
                }
            }
            continue;
        }
        note(&line, &last_err);
        job_update(jobs, key, |j| {
            j.log.push(line);
            if j.log.len() > 400 { j.log.remove(0); }
        });
    }
    let _ = t.join();
    let st = child.wait().map_err(|e| e.to_string())?;
    job_update(jobs, key, |j| j.pid = None);
    if cancelled(jobs, key) { return Err(CANCELLED.into()); }
    if st.success() { return Ok(()); }
    let code = st.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into());
    let why = last_err.lock().unwrap().take();
    Err(why.unwrap_or_else(|| format!("terminou com código {code}")))
}

/// O que tem num pacote, segundo o `zaimport --probe` (nada é extraído)
#[derive(Serialize)]
struct ProbeItem {
    file: String,
    title_id: String,
    kind: String,
    /// nome do jogo no catálogo (None se o title id não for de nenhum jogo conhecido)
    game: Option<String>,
    /// true quando o conteúdo é do jogo que está sendo importado
    matches: bool,
}

/// Lista o jogo de cada pacote escolhido, para avisar antes de importar o arquivo de outro jogo.
#[tauri::command]
async fn probe_packages(key: String, packages: Vec<String>) -> Result<Vec<ProbeItem>, String> {
    let cfg = get_config();
    let g = game_def(&key)?;
    if !Path::new(&cfg.keys_path).is_file() {
        return Err("prod.keys não encontrado: ajuste em Configurações".into());
    }
    let zaimport = Path::new(&cfg.runtime_dir).join("zaimport").join("zaimport.exe");
    if !zaimport.is_file() {
        return Err(format!("zaimport.exe não encontrado em {}", zaimport.display()));
    }
    let mut c = Command::new(&zaimport);
    // o "out dir" é ignorado pelo --probe; vai a pasta temporária só para preencher o argumento
    c.arg("--probe").arg(&cfg.keys_path).arg(std::env::temp_dir()).args(&packages);
    c.creation_flags(CREATE_NO_WINDOW);
    let out = tauri::async_runtime::spawn_blocking(move || c.stdin(Stdio::null()).output())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).lines().chain(text.lines()).filter(|l| l.starts_with("error:")).last().map(str::to_string);
        return Err(why.unwrap_or_else(|| format!("o zaimport --probe terminou com código {}", out.status.code().unwrap_or(-1))));
    }
    Ok(text
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.trim_end().split('\t').collect();
            if f.len() != 4 || f[0] != "probe" {
                return None;
            }
            let tid = f[2].to_uppercase();
            Some(ProbeItem {
                file: f[1].into(),
                game: CATALOG.iter().find(|c| c.title_id == tid).map(|c| c.name.to_string()),
                matches: tid == g.title_id,
                title_id: tid,
                kind: f[3].into(),
            })
        })
        .collect())
}

#[tauri::command]
fn import_game(key: String, packages: Vec<String>, state: tauri::State<Jobs>) -> Result<(), String> {
    let cfg = get_config();
    let g = game_def(&key)?;
    if !g.supported {
        return Err(format!("{} ainda não roda no runtime do hub", g.name));
    }
    if packages.is_empty() {
        return Err("escolha o jogo base (e a atualização, se tiver)".into());
    }
    if !Path::new(&cfg.keys_path).is_file() {
        return Err("prod.keys não encontrado: ajuste em Configurações".into());
    }
    let zaimport = Path::new(&cfg.runtime_dir).join("zaimport").join("zaimport.exe");
    let exe = Path::new(&cfg.runtime_dir).join("lasrecomp.exe");
    if !exe.is_file() {
        return Err(format!("lasrecomp.exe não encontrado em {}", cfg.runtime_dir));
    }
    if !zaimport.is_file() {
        return Err(format!("zaimport.exe não encontrado em {}", zaimport.display()));
    }
    let jobs: Jobs = state.inner().clone();
    if jobs.lock().unwrap().get(&key).map(|j| j.running).unwrap_or(false) {
        return Err("já existe uma importação em andamento".into());
    }
    job_update(&jobs, &key, |j| *j = Job { running: true, step: "Extraindo o jogo".into(), ..Default::default() });
    let dir = Path::new(&cfg.games_dir).join(g.key);
    let (title, rt) = (g.title_id.to_string(), cfg.runtime_dir.clone());
    let keys = cfg.keys_path.clone();
    std::thread::spawn(move || {
        let r = (|| -> Result<(), String> {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let mut c = Command::new(&zaimport);
            c.env("SWITCHOS_TITLE", &title).arg(&keys).arg(&dir).args(&packages);
            run_logged(&jobs, &key, c, 0.0, 0.3)?;
            if cancelled(&jobs, &key) { return Err(CANCELLED.into()); }
            job_update(&jobs, &key, |j| { j.step = "Compilando o código do jogo (10 a 30 min)".into(); j.progress = 0.3; });
            let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
            let mut c = Command::new(&exe);
            c.env("LARECOMP_BUILD_JOBS", threads.to_string()).arg("--build-game").arg(&dir).current_dir(&rt);
            run_logged(&jobs, &key, c, 0.3, 0.0)?;
            Ok(())
        })();
        job_update(&jobs, &key, |j| {
            j.running = false;
            j.pid = None;
            match r {
                Err(_) if j.cancel => { j.step = "Cancelado".into(); j.error = Some(CANCELLED.into()); }
                Ok(()) => { j.step = "Pronto para jogar".into(); j.progress = 1.0; }
                // "Falhou ao extrair" / "Falhou ao compilar": a etapa ajuda a explicar o erro
                Err(e) => {
                    j.step = if j.step.starts_with("Compilando") { "Falhou ao compilar" } else { "Falhou ao extrair" }.into();
                    j.error = Some(e);
                }
            }
        });
    });
    Ok(())
}

/// Interrompe a importação: mata o processo da etapa atual e marca o job como cancelado.
/// A pasta do jogo fica com o que já foi extraído/compilado; um novo import roda as
/// duas etapas de novo por cima dela, e o jogo só conta como compilado quando o
/// --build-game termina, então o "Jogar" continua bloqueado.
/// Só compila de novo o código do jogo (o runtime mudou: needs_rebuild). Os arquivos do jogo ficam como estão.
#[tauri::command]
fn rebuild_game(key: String, state: tauri::State<Jobs>) -> Result<(), String> {
    let cfg = get_config();
    let g = game_def(&key)?;
    let dir = Path::new(&cfg.games_dir).join(g.key);
    if !(dir.join("exefs").join("main").is_file() && dir.join("romfs.bin").is_file()) {
        return Err(format!("{} ainda não foi importado", g.name));
    }
    let exe = Path::new(&cfg.runtime_dir).join("lasrecomp.exe");
    if !exe.is_file() {
        return Err(format!("lasrecomp.exe não encontrado em {}", cfg.runtime_dir));
    }
    let jobs: Jobs = state.inner().clone();
    if jobs.lock().unwrap().get(&key).map(|j| j.running).unwrap_or(false) {
        return Err("já existe uma importação em andamento".into());
    }
    job_update(&jobs, &key, |j| *j = Job { running: true, step: "Compilando o código do jogo (10 a 30 min)".into(), ..Default::default() });
    let rt = cfg.runtime_dir.clone();
    std::thread::spawn(move || {
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let mut c = Command::new(&exe);
        c.env("LARECOMP_BUILD_JOBS", threads.to_string()).arg("--build-game").arg(&dir).current_dir(&rt);
        let r = run_logged(&jobs, &key, c, 0.0, 0.0);
        job_update(&jobs, &key, |j| {
            j.running = false;
            j.pid = None;
            match r {
                Err(_) if j.cancel => { j.step = "Cancelado".into(); j.error = Some(CANCELLED.into()); }
                Ok(()) => { j.step = "Pronto para jogar".into(); j.progress = 1.0; }
                Err(e) => { j.step = "Falhou ao compilar".into(); j.error = Some(e); }
            }
        });
    });
    Ok(())
}

#[tauri::command]
fn cancel_import(key: String, state: tauri::State<Jobs>) -> Result<(), String> {
    let pid = {
        let mut m = state.lock().unwrap();
        let j = m.get_mut(&key).filter(|j| j.running).ok_or("não há importação em andamento")?;
        j.cancel = true;
        j.step = "Cancelando…".into();
        j.pid
    };
    if let Some(pid) = pid { kill_tree(pid); }
    Ok(())
}

#[tauri::command]
fn job_status(key: String, state: tauri::State<Jobs>) -> Option<Job> {
    state.lock().unwrap().get(&key).cloned()
}

// ------------------------------------------------------------------ utilidades

#[tauri::command]
fn ensure_dir(path: String) -> Result<String, String> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Resultado da checagem de um campo das configurações: ok + frase curta
#[derive(Serialize)]
struct PathCheck {
    ok: bool,
    msg: String,
}

/// Confere as configurações antes de salvar, sem criar nada
#[tauri::command]
fn check_paths(config: Config) -> HashMap<String, PathCheck> {
    let mut m = HashMap::new();
    let ck = |ok: bool, msg: &str| PathCheck { ok, msg: msg.into() };
    let rt = Path::new(config.runtime_dir.trim());
    m.insert("runtime_dir".into(), if config.runtime_dir.trim().is_empty() {
        ck(false, "Escolha a pasta.")
    } else if !rt.is_dir() {
        ck(false, "Esta pasta não existe.")
    } else if !rt.join("lasrecomp.exe").is_file() {
        ck(false, "Não achei o lasrecomp.exe nesta pasta. Escolha a pasta onde ele está.")
    } else {
        ck(true, "lasrecomp.exe encontrado.")
    });
    let gd = Path::new(config.games_dir.trim());
    m.insert("games_dir".into(), if config.games_dir.trim().is_empty() {
        ck(false, "Escolha a pasta.")
    } else if gd.is_dir() {
        ck(true, "Pasta encontrada.")
    } else if gd.is_file() {
        ck(false, "Isto é um arquivo, não uma pasta.")
    } else if gd.ancestors().skip(1).find(|a| !a.as_os_str().is_empty() && a.exists()).map(|a| a.is_dir()).unwrap_or(false) {
        ck(true, "A pasta ainda não existe. Ela será criada na primeira importação.")
    } else {
        ck(false, "Este caminho não existe (confira a letra do disco).")
    });
    let kp = Path::new(config.keys_path.trim());
    m.insert("keys_path".into(), if config.keys_path.trim().is_empty() {
        ck(false, "Escolha o arquivo prod.keys.")
    } else if kp.is_dir() {
        ck(false, "Isto é uma pasta. Escolha o arquivo prod.keys dentro dela.")
    } else {
        match fs::read_to_string(kp) {
            Err(_) if !kp.exists() => ck(false, "Arquivo não encontrado."),
            Err(_) => ck(false, "Não consegui ler este arquivo."),
            Ok(t) if !t.lines().any(|l| l.trim_start().starts_with("header_key")) =>
                ck(false, "Este arquivo não parece um prod.keys completo (falta header_key)."),
            Ok(_) => {
                let tk = kp.parent().map(|d| d.join("title.keys").is_file() || d.join("title.keys.txt").is_file()).unwrap_or(false);
                ck(true, if tk { "prod.keys lido. title.keys encontrado ao lado." } else { "prod.keys lido." })
            }
        }
    });
    m
}

// ------------------------------------------------------------------ save: só leitura do que o switchos.py registra
//
// O switchos.py (raiz do SwitchOS) sincroniza o save em 3 vias: PC, nuvem (<cloud_dir>/saves/<TID>) e Android. Antes de
// sobrescrever qualquer lado ele copia para <cloud_dir>/backups/<TID>/<AAAAMMDD-HHMMSS>-<origem>, e grava em
// <cloud_dir>/state.json os hashes e a hora ("at") da última sync. O hub só LÊ isso: não sincroniza nem restaura nada.

#[derive(Serialize)]
struct BackupInfo {
    name: String,
    path: String,
    /// "AAAAMMDD-HHMMSS" em hora local, como o switchos.py grava
    stamp: String,
    /// pc, android ou cloud
    origin: String,
}

#[derive(Serialize)]
struct LastSync {
    /// segundos desde 1970 (time.time() do switchos.py)
    at: f64,
    /// lados que participaram além da nuvem (pc, android)
    sides: Vec<String>,
}

#[derive(Serialize, Default)]
struct SaveInfo {
    local_modified: Option<u64>,
    /// games.toml com [sync] cloud_dir encontrado
    sync_config: Option<String>,
    cloud_dir: Option<String>,
    cloud_save: Option<String>,
    cloud_exists: bool,
    cloud_modified: Option<u64>,
    backups_dir: Option<String>,
    /// até 10, mais recentes primeiro
    backups: Vec<BackupInfo>,
    backups_total: usize,
    keep_backups: Option<u32>,
    last_sync: Option<LastSync>,
}

fn newest_mtime(p: &Path) -> Option<u64> {
    let mut best: Option<u64> = None;
    for e in fs::read_dir(p).ok()?.flatten() {
        let t = if e.path().is_dir() {
            newest_mtime(&e.path())
        } else {
            e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
        };
        if let Some(t) = t {
            best = Some(best.map_or(t, |b| b.max(t)));
        }
    }
    best
}

/// Raiz do SwitchOS: a pasta com games.toml e switchos.py, subindo a partir do executável ou da pasta do programa
fn switchos_root(cfg: &Config) -> Option<PathBuf> {
    let starts = [std::env::current_exe().ok(), Some(PathBuf::from(&cfg.runtime_dir))];
    for s in starts.into_iter().flatten() {
        let mut d = s;
        for _ in 0..10 {
            if d.join("games.toml").is_file() && d.join("switchos.py").is_file() {
                return Some(d);
            }
            if !d.pop() {
                break;
            }
        }
    }
    None
}

/// Lê um valor da seção [sync] do games.toml (parser mínimo: `chave = valor` numa linha só)
fn toml_sync_value(text: &str, key: &str) -> Option<String> {
    let mut in_sync = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_sync = l == "[sync]";
            continue;
        }
        if !in_sync || l.starts_with('#') {
            continue;
        }
        let Some((k, v)) = l.split_once('=') else { continue };
        if k.trim() != key {
            continue;
        }
        let v = v.trim();
        return Some(if let Some(r) = v.strip_prefix('"') {
            // string básica: até a próxima aspas não escapada
            let mut out = String::new();
            let mut it = r.chars();
            while let Some(c) = it.next() {
                match c {
                    '"' => break,
                    '\\' => out.extend(it.next()),
                    c => out.push(c),
                }
            }
            out
        } else if let Some(r) = v.strip_prefix('\'') {
            r.split('\'').next().unwrap_or("").to_string()
        } else {
            v.split('#').next().unwrap_or("").trim().to_string()
        });
    }
    None
}

/// Como expand() do switchos.py: ~ e %VAR%; relativo à raiz do SwitchOS
fn expand_path(root: &Path, p: &str) -> PathBuf {
    let mut s = p.to_string();
    if let Some(rest) = s.strip_prefix('~') {
        s = format!("{}{}", dirs::home_dir().unwrap_or_default().to_string_lossy(), rest);
    }
    let mut out = String::new();
    let mut rest = s.as_str();
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) => {
                match std::env::var(&after[..j]) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => out.push_str(&rest[i..i + j + 2]),
                }
                rest = &after[j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    let path = PathBuf::from(out);
    if path.is_absolute() { path } else { root.join(path) }
}

/// Estado do save de um jogo: save local, nuvem, backups e última sync. Só leitura.
#[tauri::command]
fn save_info(key: String) -> Result<SaveInfo, String> {
    let g = game_def(&key)?;
    let cfg = get_config();
    let mut info = SaveInfo { local_modified: newest_mtime(&save_dir(&cfg, g)), ..Default::default() };
    let Some(root) = switchos_root(&cfg) else { return Ok(info) };
    let toml = root.join("games.toml");
    let Ok(text) = fs::read_to_string(&toml) else { return Ok(info) };
    let Some(cloud_dir) = toml_sync_value(&text, "cloud_dir").filter(|v| !v.is_empty()) else { return Ok(info) };
    let cloud_root = expand_path(&root, &cloud_dir);
    let tid = g.title_id.to_uppercase(); // switchos.py usa o title id em maiúsculas
    info.sync_config = Some(toml.to_string_lossy().into());
    info.keep_backups = toml_sync_value(&text, "keep_backups").and_then(|v| v.parse().ok()).or(Some(10));
    info.cloud_dir = Some(cloud_root.to_string_lossy().into());

    let cloud = cloud_root.join("saves").join(&tid);
    info.cloud_save = Some(cloud.to_string_lossy().into());
    info.cloud_exists = cloud.is_dir();
    info.cloud_modified = newest_mtime(&cloud);

    let bdir = cloud_root.join("backups").join(&tid);
    info.backups_dir = Some(bdir.to_string_lossy().into());
    let mut backups: Vec<BackupInfo> = fs::read_dir(&bdir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    // <AAAAMMDD>-<HHMMSS>-<origem>[-n]
                    let mut parts = name.splitn(3, '-');
                    let (d, t, rest) = (parts.next()?, parts.next()?, parts.next()?);
                    if d.len() != 8 || t.len() != 6 || !d.chars().chain(t.chars()).all(|c| c.is_ascii_digit()) {
                        return None;
                    }
                    // "pre-restore" (o save antes de uma restauração) é a única origem com hífen
                    let origin = if rest.starts_with("pre-restore") { "pre-restore".into() } else { rest.split('-').next().unwrap_or(rest).to_string() };
                    Some(BackupInfo { stamp: format!("{d}-{t}"), origin, path: e.path().to_string_lossy().into(), name })
                })
                .collect()
        })
        .unwrap_or_default();
    backups.sort_by(|a, b| b.name.cmp(&a.name));
    info.backups_total = backups.len();
    backups.truncate(10);
    info.backups = backups;

    if let Some(entry) = fs::read_to_string(cloud_root.join("state.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get(&tid).cloned())
    {
        if let Some(at) = entry.get("at").and_then(|a| a.as_f64()) {
            let sides = entry
                .as_object()
                .map(|o| o.keys().filter(|k| *k != "at" && *k != "cloud").cloned().collect())
                .unwrap_or_default();
            info.last_sync = Some(LastSync { at, sides });
        }
    }
    Ok(info)
}

// ------------------------------------------------------------------ restaurar uma cópia de segurança

/// Data e hora local "AAAAMMDD-HHMMSS", como o time.strftime('%Y%m%d-%H%M%S') do switchos.py
fn local_stamp() -> String {
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct SystemTime { y: u16, mo: u16, _dow: u16, d: u16, h: u16, mi: u16, s: u16, _ms: u16 }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetLocalTime(t: *mut SystemTime);
        }
        let mut t = SystemTime::default();
        unsafe { GetLocalTime(&mut t) };
        format!("{:04}{:02}{:02}-{:02}{:02}{:02}", t.y, t.mo, t.d, t.h, t.mi, t.s)
    }
    #[cfg(not(windows))]
    {
        // UTC (o hub só roda no Windows; isto só mantém o build em outros sistemas)
        let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let (days, rem) = ((s / 86400) as i64, s % 86400);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
        format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
    }
}

/// Nome de backup aceitável: um nome simples (só letras, dígitos, - e _; sem separadores nem "..")
/// no formato <AAAAMMDD>-<HHMMSS>-<origem>[-n] que o switchos.py grava
fn valid_backup_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return false;
    }
    let mut parts = name.splitn(3, '-');
    let (Some(d), Some(t), Some(rest)) = (parts.next(), parts.next(), parts.next()) else { return false };
    d.len() == 8 && t.len() == 6 && d.chars().chain(t.chars()).all(|c| c.is_ascii_digit()) && !rest.is_empty()
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let ty = e.file_type()?;
        let to = dst.join(e.file_name());
        if ty.is_dir() {
            copy_tree(&e.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(e.path(), &to)?;
        } // links ficam de fora (o save do runtime não tem)
    }
    Ok(())
}

/// backup() do switchos.py: copia src para <bdir>/<stamp>-<origem>[-n] e poda as mais antigas
/// além de keep (ordem por nome; keep <= 0 não poda, como o olds[:-0] do Python).
/// Apaga no máximo o que o switchos.py apagaria, e nunca a cópia recém-criada nem os nomes em `spare`.
fn backup_like_switchos(src: &Path, bdir: &Path, origin: &str, keep: i64, stamp: &str, spare: &[&str]) -> Result<String, String> {
    fs::create_dir_all(bdir).map_err(|e| format!("restaurar: não criou a pasta de cópias ({e})"))?;
    let (mut name, mut i) = (format!("{stamp}-{origin}"), 1);
    while bdir.join(&name).exists() {
        name = format!("{stamp}-{origin}-{i}");
        i += 1;
    }
    let dst = bdir.join(&name);
    if let Err(e) = copy_tree(src, &dst) {
        let _ = fs::remove_dir_all(&dst);
        return Err(format!("restaurar: não guardou a cópia do save atual ({e})"));
    }
    if keep > 0 {
        let mut olds: Vec<String> = fs::read_dir(bdir)
            .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        olds.sort();
        let cut = olds.len().saturating_sub(keep as usize);
        for o in &olds[..cut] {
            if *o != name && !spare.contains(&o.as_str()) && bdir.join(o).is_dir() {
                let _ = fs::remove_dir_all(bdir.join(o));
            }
        }
    }
    Ok(name)
}

/// Restaura <bdir>/<name> em `save`: copia para uma pasta irmã temporária, guarda o save atual
/// como cópia "pre-restore" (keep_backups + 1, como o switchos.py restore) e só então troca as pastas com rename (o replace_tree do switchos.py).
/// Se a troca falhar, o save antigo volta para o lugar; ele também fica na cópia recém-criada.
/// Devolve o nome da cópia criada (None quando não havia save atual).
fn restore_into(bdir: &Path, name: &str, save: &Path, keep: i64, stamp: &str) -> Result<Option<String>, String> {
    if !valid_backup_name(name) {
        return Err("restaurar: cópia de segurança não encontrada (nome inválido)".into());
    }
    let src = bdir.join(name);
    let meta = fs::symlink_metadata(&src).map_err(|_| "restaurar: cópia de segurança não encontrada".to_string())?;
    if !meta.is_dir() {
        return Err("restaurar: cópia de segurança não encontrada".into());
    }
    let parent = save.parent().ok_or("restaurar: pasta do save inválida")?;
    fs::create_dir_all(parent).map_err(|e| format!("restaurar: não criou a pasta do save ({e})"))?;
    let base = save.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "save".into());
    let tmp = parent.join(format!("{base}.switchos-tmp"));
    let old = parent.join(format!("{base}.switchos-old"));
    let _ = fs::remove_dir_all(&tmp);
    if let Err(e) = copy_tree(&src, &tmp) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("restaurar: não copiou a cópia escolhida ({e})"));
    }
    let created = if save.exists() {
        match backup_like_switchos(save, bdir, "pre-restore", if keep > 0 { keep + 1 } else { keep }, stamp, &[name]) {
            Ok(n) => Some(n),
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp);
                return Err(e);
            }
        }
    } else {
        None
    };
    if save.exists() {
        let _ = fs::remove_dir_all(&old);
        if let Err(e) = fs::rename(save, &old) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(format!("restaurar: o save atual está em uso ({e})"));
        }
        if let Err(e) = fs::rename(&tmp, save) {
            let _ = fs::rename(&old, save); // devolve o save original
            let _ = fs::remove_dir_all(&tmp);
            return Err(format!("restaurar: não trocou as pastas ({e})"));
        }
        let _ = fs::remove_dir_all(&old);
    } else if let Err(e) = fs::rename(&tmp, save) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("restaurar: não trocou as pastas ({e})"));
    }
    Ok(created)
}

/// Quantos lasrecomp.exe estão rodando (None se não deu para perguntar ao Windows)
fn runtime_processes() -> Option<usize> {
    #[cfg(windows)]
    {
        let mut c = Command::new("tasklist");
        c.args(["/FI", "IMAGENAME eq lasrecomp.exe", "/FO", "CSV", "/NH"]).creation_flags(CREATE_NO_WINDOW);
        let out = c.stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.to_ascii_lowercase().contains("\"lasrecomp.exe\"")).count())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[derive(Serialize)]
struct RestoreResult {
    /// cópia de segurança feita do save que estava aqui (None se não havia save)
    created: Option<String>,
}

/// Volta o save deste PC para uma cópia de segurança. Não toca na nuvem nem no Android:
/// a próxima sincronização do switchos.py leva esta versão para os outros lados.
#[tauri::command]
fn restore_backup(key: String, backup_name: String, state: tauri::State<Jobs>) -> Result<RestoreResult, String> {
    let g = game_def(&key)?;
    let cfg = get_config();
    let (importing, building) = {
        let m = state.lock().unwrap();
        let importing = m.get(&key).map(|j| j.running).unwrap_or(false);
        // o --build-game de qualquer importação também é um lasrecomp.exe
        let building = m.values().filter(|j| j.running && j.step.starts_with("Compilando")).count();
        (importing, building)
    };
    if importing {
        return Err("restaurar: há uma importação em andamento deste jogo".into());
    }
    match runtime_processes() {
        Some(n) if n > building => return Err("restaurar: o jogo está aberto".into()),
        Some(_) => {}
        None => return Err("restaurar: não deu para conferir se o jogo está aberto".into()),
    }
    let root = switchos_root(&cfg).ok_or("restaurar: sincronização não configurada")?;
    let text = fs::read_to_string(root.join("games.toml")).map_err(|_| "restaurar: sincronização não configurada".to_string())?;
    let cloud_dir = toml_sync_value(&text, "cloud_dir").filter(|v| !v.is_empty()).ok_or("restaurar: sincronização não configurada")?;
    let keep = toml_sync_value(&text, "keep_backups").and_then(|v| v.parse::<i64>().ok()).unwrap_or(10);
    let bdir = expand_path(&root, &cloud_dir).join("backups").join(g.title_id.to_uppercase());
    let created = restore_into(&bdir, &backup_name, &save_dir(&cfg, g), keep, &local_stamp())?;
    Ok(RestoreResult { created })
}

#[cfg(test)]
mod gamelib_tests {
    use super::*;

    #[test]
    fn stale() {
        let d = std::env::temp_dir().join(format!("switchos-hub-gamelib-{}", std::process::id()));
        let (rt, game) = (d.join("rt"), d.join("game"));
        fs::create_dir_all(&rt).unwrap();
        fs::create_dir_all(&game).unwrap();
        assert!(!gamelib_stale(&rt, &game)); // sem os arquivos: não avisa
        fs::write(rt.join("gamelib.id"), "cc28575f6333d0f6
").unwrap();
        assert!(!gamelib_stale(&rt, &game));
        fs::write(game.join("lasgame.dll.id"), "cc28575f6333d0f6-AEE8F150DDA1B5A8
").unwrap();
        assert!(!gamelib_stale(&rt, &game));
        fs::write(game.join("lasgame.dll.id"), "cc28575f6333d0f6-v2-AEE8F150DDA1B5A8-p0123").unwrap();
        assert!(!gamelib_stale(&rt, &game));
        fs::write(game.join("lasgame.dll.id"), "0123456789abcdef-AEE8F150DDA1B5A8").unwrap();
        assert!(gamelib_stale(&rt, &game));
        let _ = fs::remove_dir_all(&d);
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("switchos-hub-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Restauração de verdade, para conferir junto com o switchos.py num ambiente isolado:
    /// RESTORE_E2E_BDIR (pasta backups/<TID>), RESTORE_E2E_NAME (cópia) e RESTORE_E2E_SAVE (pasta save_0).
    /// Sem as variáveis, não faz nada. Nunca apontar para o save real.
    #[test]
    fn restore_e2e() {
        let (Ok(b), Ok(n), Ok(s)) = (std::env::var("RESTORE_E2E_BDIR"), std::env::var("RESTORE_E2E_NAME"), std::env::var("RESTORE_E2E_SAVE")) else {
            return;
        };
        let created = restore_into(Path::new(&b), &n, Path::new(&s), 10, &local_stamp()).unwrap();
        println!("restaurado {n}; save anterior em {created:?}");
    }

    #[test]
    fn names() {
        assert!(valid_backup_name("20261118-153000-pc"));
        assert!(valid_backup_name("20261118-153000-android-2"));
        for bad in ["", ".", "..", "../20261118-153000-pc", "20261118-153000-pc/..", "a\\b", "C:x", "20261118-153000-", "2026-153000-pc", "x-y-z"] {
            assert!(!valid_backup_name(bad), "{bad}");
        }
    }

    #[test]
    fn restore_flow() {
        let d = scratch("restore");
        let bdir = d.join("backups").join("TID");
        let save = d.join("save").join("save_0");
        fs::create_dir_all(bdir.join("20260101-000000-cloud").join("sub")).unwrap();
        fs::write(bdir.join("20260101-000000-cloud").join("main"), b"old").unwrap();
        fs::write(bdir.join("20260101-000000-cloud").join("sub").join("x"), b"x").unwrap();
        fs::create_dir_all(bdir.join("20260102-000000-pc")).unwrap();
        fs::create_dir_all(&save).unwrap();
        fs::write(save.join("main"), b"current").unwrap();

        // nome fora do formato ou inexistente: recusa e não mexe no save
        assert!(restore_into(&bdir, "..", &save, 10, "20261118-120000").is_err());
        assert!(restore_into(&bdir, "20991231-000000-pc", &save, 10, "20261118-120000").is_err());
        assert_eq!(fs::read(save.join("main")).unwrap(), b"current");

        // keep=2 vira 3 na restauração (como o switchos.py restore): com 3 cópias nada sai, e a escolhida é sempre poupada
        let created = restore_into(&bdir, "20260101-000000-cloud", &save, 2, "20261118-120000").unwrap();
        assert_eq!(created.as_deref(), Some("20261118-120000-pre-restore"));
        assert_eq!(fs::read(save.join("main")).unwrap(), b"old");
        assert_eq!(fs::read(save.join("sub").join("x")).unwrap(), b"x");
        assert_eq!(fs::read(bdir.join("20261118-120000-pre-restore").join("main")).unwrap(), b"current");
        assert!(bdir.join("20260101-000000-cloud").is_dir());
        assert!(bdir.join("20260102-000000-pc").is_dir());
        assert!(!d.join("save").join("save_0.switchos-tmp").exists());
        assert!(!d.join("save").join("save_0.switchos-old").exists());

        // mesmo segundo: ganha o sufixo -1, como no switchos.py
        let again = restore_into(&bdir, "20260101-000000-cloud", &save, 0, "20261118-120000").unwrap();
        assert_eq!(again.as_deref(), Some("20261118-120000-pre-restore-1"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn restore_without_current_save() {
        let d = scratch("nosave");
        let bdir = d.join("b");
        fs::create_dir_all(bdir.join("20260101-000000-android")).unwrap();
        fs::write(bdir.join("20260101-000000-android").join("main"), b"a").unwrap();
        let save = d.join("save").join("save_0");
        assert_eq!(restore_into(&bdir, "20260101-000000-android", &save, 10, "20261118-120000").unwrap(), None);
        assert_eq!(fs::read(save.join("main")).unwrap(), b"a");
        let _ = fs::remove_dir_all(&d);
    }
}

#[tauri::command]
fn mods_dir() -> String {
    Path::new(&get_config().runtime_dir).join("mods").to_string_lossy().into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Jobs::default())
        .invoke_handler(tauri::generate_handler![
            get_config, set_config, list_games, play_game, set_mod_enabled, import_game, rebuild_game, probe_packages, cancel_import, job_status, ensure_dir, mods_dir, check_paths, save_info,
            restore_backup, saveimport::import_sources, saveimport::check_import, saveimport::import_save, firstrun::firstrun_status, firstrun::firstrun_run, sync::sync_save
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o SwitchOS Hub");
}
