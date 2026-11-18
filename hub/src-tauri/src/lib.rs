//! SwitchOS Hub 0.0.1 - biblioteca dos jogos recompilados, importação, mods, saves e "Jogar".
//!
//! O hub não roda jogo nenhum: ele organiza as pastas e chama o runtime unificado (lasrecomp.exe, engines/las-za) e o
//! importador (zaimport.exe). Configuração em %APPDATA%\SwitchOS\hub.json.

mod party;

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
struct ModInfo {
    name: String,
    files: usize,
}

#[derive(Serialize)]
struct GameState {
    #[serde(flatten)]
    def: GameDef,
    dir: String,
    imported: bool,
    built: bool,
    save_dir: String,
    save_modified: Option<u64>,
    mods: Vec<ModInfo>,
    /// ícone do jogo (extraído do Control NCA pelo zaimport) como data URL
    icon: Option<String>,
    /// versão do jogo instalada (version.txt: DisplayVersion do Control NCA, gravado pelo zaimport)
    version: Option<String>,
    /// DLCs instaladas (<pasta do jogo>/aoc/<title id>.bin, lidas pelo serviço aoc:u do runtime)
    dlcs: Vec<DlcInfo>,
    /// o time do save (por enquanto só o Z-A: o leitor é o da tela inicial do ZA Recomp)
    party: Vec<party::PartyMon>,
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

fn count_files(p: &Path) -> usize {
    fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| if e.path().is_dir() { count_files(&e.path()) } else { 1 }).sum())
        .unwrap_or(0)
}

/// O mesmo critério do runtime (services/mod_layout.c): outros jogos só pela pasta com o title id
fn mod_romfs(mod_dir: &Path, g: &GameDef) -> Option<PathBuf> {
    let tid = g.title_id;
    let mut roots = vec![
        mod_dir.join("atmosphere").join("contents").join(tid).join("romfs"),
        mod_dir.join("contents").join(tid).join("romfs"),
        mod_dir.join(tid).join("romfs"),
    ];
    if g.key == "za" {
        roots.push(mod_dir.join("romfs"));
    }
    roots.into_iter().find(|p| p.is_dir()).or_else(|| {
        // (sem diferenciar maiúsculas no title id, como o runtime)
        fs::read_dir(mod_dir).ok()?.flatten().find_map(|e| {
            (e.file_name().to_string_lossy().eq_ignore_ascii_case(tid) && e.path().join("romfs").is_dir())
                .then(|| e.path().join("romfs"))
        })
    })
}

fn mods_for(cfg: &Config, g: &GameDef) -> Vec<ModInfo> {
    let mut out = vec![];
    if let Ok(rd) = fs::read_dir(Path::new(&cfg.runtime_dir).join("mods")) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.starts_with('_') || !e.path().is_dir() {
                continue;
            }
            if let Some(r) = mod_romfs(&e.path(), g) {
                out.push(ModInfo { name, files: count_files(&r) });
            }
        }
    }
    out
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
                mods: mods_for(&cfg, g),
                version: fs::read_to_string(dir.join("version.txt")).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
                dlcs: dlcs_in(&dir),
                party: if g.key == "za" {
                    party::load(&sd.join("main"), &Path::new(&cfg.runtime_dir).join("cache").join("party_icons"))
                } else {
                    vec![]
                },
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
    let dir = Path::new(&cfg.games_dir).join(g.key);
    if !dir.join("lasgame.dll").is_file() {
        return Err("o código do jogo ainda não foi compilado: use Importar".into());
    }
    let exe = Path::new(&cfg.runtime_dir).join("lasrecomp.exe");
    let child = Command::new(&exe)
        .arg("--play")
        .arg(&dir)
        .current_dir(&cfg.runtime_dir)
        .spawn()
        .map_err(|e| format!("não abriu {}: {e}", exe.display()))?;
    Ok(child.id())
}

// ------------------------------------------------------------------ importar (zaimport + --build-game, em segundo plano)

#[derive(Serialize, Clone, Default)]
struct Job {
    running: bool,
    step: String,
    progress: f64,
    log: Vec<String>,
    error: Option<String>,
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
        .map_err(|e| format!("não iniciou: {e}"))?;
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    let (j2, k2) = (jobs.clone(), key.to_string());
    let t = std::thread::spawn(move || {
        for line in BufReader::new(err).lines().map_while(Result::ok) {
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
        job_update(jobs, key, |j| {
            j.log.push(line);
            if j.log.len() > 400 { j.log.remove(0); }
        });
    }
    let _ = t.join();
    let st = child.wait().map_err(|e| e.to_string())?;
    if st.success() { Ok(()) } else { Err(format!("terminou com código {:?}", st.code())) }
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
    let jobs: Jobs = state.inner().clone();
    if jobs.lock().unwrap().get(&key).map(|j| j.running).unwrap_or(false) {
        return Err("já existe uma importação em andamento".into());
    }
    job_update(&jobs, &key, |j| *j = Job { running: true, step: "Extraindo o jogo".into(), ..Default::default() });
    let dir = Path::new(&cfg.games_dir).join(g.key);
    let zaimport = Path::new(&cfg.runtime_dir).join("zaimport").join("zaimport.exe");
    let exe = Path::new(&cfg.runtime_dir).join("lasrecomp.exe");
    let (title, rt) = (g.title_id.to_string(), cfg.runtime_dir.clone());
    let keys = cfg.keys_path.clone();
    std::thread::spawn(move || {
        let r = (|| -> Result<(), String> {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let mut c = Command::new(&zaimport);
            c.env("SWITCHOS_TITLE", &title).arg(&keys).arg(&dir).args(&packages);
            run_logged(&jobs, &key, c, 0.0, 0.3)?;
            job_update(&jobs, &key, |j| { j.step = "Compilando o código do jogo (10 a 30 min)".into(); j.progress = 0.3; });
            let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
            let mut c = Command::new(&exe);
            c.env("LARECOMP_BUILD_JOBS", threads.to_string()).arg("--build-game").arg(&dir).current_dir(&rt);
            run_logged(&jobs, &key, c, 0.3, 0.0)?;
            Ok(())
        })();
        job_update(&jobs, &key, |j| {
            j.running = false;
            match r {
                Ok(()) => { j.step = "Pronto para jogar".into(); j.progress = 1.0; }
                Err(e) => { j.step = "Falhou".into(); j.error = Some(e); }
            }
        });
    });
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
            get_config, set_config, list_games, play_game, import_game, job_status, ensure_dir, mods_dir
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o SwitchOS Hub");
}
