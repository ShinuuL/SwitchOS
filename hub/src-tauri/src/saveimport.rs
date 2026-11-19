//! Importar um save de fora para o save deste PC (aba Save do hub).
//!
//! Fontes: uma pasta escolhida pelo usuário (save exportado do Switch com JKSV ou Checkpoint, ou cópia de
//! outro lugar) e os saves do Ryujinx (%APPDATA%\Ryujinx\bis\user\save\<id>\0), achados pelo title id.
//! As fontes são só lidas. Antes de trocar, o save atual vira uma cópia "<stamp>-pre-restore":
//! em <cloud_dir>/backups/<TID> quando a sincronização está configurada (como a restauração), ou em
//! <runtime>/save-backups/<TID> quando não está (fica fora de save/, onde o runtime não olha, e nunca
//! é apagada pela sincronização).

use super::*;

/// Uma pasta que pode virar o save deste jogo
#[derive(Serialize, Clone, Debug)]
pub struct Candidate {
    /// pasta com o arquivo main (a que será copiada)
    pub path: String,
    pub modified: Option<u64>,
    pub files: usize,
    /// pronto para importar; senão `reason` diz por quê
    pub ok: bool,
    pub reason: Option<String>,
    /// Pokémon no time (Z-A e Arceus), quando o save foi lido
    pub party: Option<usize>,
}

#[derive(Serialize)]
pub struct ImportSources {
    /// pasta do Ryujinx (se existir) e os saves dele deste jogo
    pub ryujinx_dir: Option<String>,
    pub ryujinx: Vec<Candidate>,
    /// onde o save atual fica guardado antes de importar
    pub backups_dir: String,
    /// true: cópias da sincronização; false: pasta local do hub (sem sincronização configurada)
    pub synced: bool,
}

#[derive(Serialize)]
pub struct ImportResult {
    /// cópia do save que estava aqui (None se não havia save)
    pub created: Option<String>,
    pub backups_dir: String,
}

fn party_format(key: &str) -> Option<&'static party::Format> {
    match key {
        "za" => Some(&party::ZA),
        "arceus" => Some(&party::ARCEUS),
        _ => None,
    }
}

/// A pasta escolhida pode ser a do save (com main), a pasta do id do Ryujinx (com 0/main) ou uma pasta
/// com uma única subpasta de save (o JKSV cria uma por exportação).
pub fn resolve_dir(dir: &Path) -> Option<PathBuf> {
    if dir.join("main").is_file() {
        return Some(dir.to_path_buf());
    }
    if dir.join("0").join("main").is_file() {
        return Some(dir.join("0"));
    }
    let subs: Vec<PathBuf> = fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.join("main").is_file()).collect();
    if subs.len() == 1 { subs.into_iter().next() } else { None }
}

/// Confere se `dir` serve como save do jogo `key`. Só leitura.
pub fn check(key: &str, dir: &Path) -> Candidate {
    let mut c = Candidate { path: dir.to_string_lossy().into(), modified: None, files: 0, ok: false, reason: None, party: None };
    let Some(d) = resolve_dir(dir) else {
        c.reason = Some("importar: a pasta não tem o arquivo main de um save".into());
        return c;
    };
    c.path = d.to_string_lossy().into();
    c.modified = newest_mtime(&d);
    c.files = mods::count_files(&d);
    let main = d.join("main");
    if let Some(f) = party_format(key) {
        if !party::has_party_block(&main, f) {
            let other = ["za", "arceus"].into_iter().find(|k| *k != key && party::has_party_block(&main, party_format(k).unwrap()));
            c.reason = Some(match other.and_then(|k| game_def(k).ok()) {
                Some(g) => format!("importar: este save é de outro jogo ({})", g.name),
                None => "importar: o arquivo main não é um save deste jogo (não decifrou)".into(),
            });
            return c;
        }
        c.party = Some(party::load(&main, Path::new(""), f).len());
    }
    c.ok = true;
    c
}

// ------------------------------------------------------------------ Ryujinx

/// %APPDATA%\Ryujinx (a instalação portátil não é procurada)
fn ryujinx_root() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("Ryujinx")).filter(|d| d.is_dir())
}

/// imkvdb.arc do Ryujinx (o índice de saves do Switch): "IMKV", reservado, quantidade, e cada entrada
/// "IMEN", tamanho da chave, tamanho do valor, chave (SaveDataAttribute: program id u64 em 0, tipo u8 em 0x20)
/// e valor (SaveDataIndexerValue: save data id u64 em 0). Devolve (program id, tipo, save data id).
pub fn parse_imkvdb(b: &[u8]) -> Vec<(u64, u8, u64)> {
    let u32at = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()));
    let u64at = |o: usize| b.get(o..o + 8).map(|s| u64::from_le_bytes(s.try_into().unwrap()));
    let mut out = vec![];
    if b.get(0..4) != Some(b"IMKV") {
        return out;
    }
    let count = u32at(8).unwrap_or(0) as usize;
    let mut o = 12;
    for _ in 0..count.min(4096) {
        if b.get(o..o + 4) != Some(b"IMEN") {
            break;
        }
        let (Some(ks), Some(vs)) = (u32at(o + 4), u32at(o + 8)) else { break };
        let (ks, vs) = (ks as usize, vs as usize);
        let k = o + 12;
        let v = k + ks;
        if ks < 0x21 || vs < 8 || v + vs > b.len() {
            break;
        }
        out.push((u64at(k).unwrap(), b[k + 0x20], u64at(v).unwrap()));
        o = v + vs;
    }
    out
}

/// Pastas de save (<root>/bis/user/save/<id>/0) do title id: pelo imkvdb.arc e, se ele não ajudar, pelo
/// ExtraData0 que o Ryujinx grava em cada save (o SaveDataAttribute no começo). Só saves de conta (tipo 1).
pub fn ryujinx_saves(root: &Path, title_id: &str) -> Vec<PathBuf> {
    let Ok(want) = u64::from_str_radix(title_id, 16) else { return vec![] };
    let user = root.join("bis").join("user").join("save");
    let mut ids: Vec<String> = fs::read(root.join("bis").join("system").join("save").join("8000000000000000").join("0").join("imkvdb.arc"))
        .map(|b| parse_imkvdb(&b))
        .unwrap_or_default()
        .into_iter()
        .filter(|&(p, t, _)| p == want && t == 1)
        .map(|(_, _, id)| format!("{id:016x}"))
        .collect();
    if ids.is_empty() {
        ids = fs::read_dir(&user)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| {
                        fs::read(e.path().join("ExtraData0"))
                            .ok()
                            .filter(|b| b.len() > 0x20)
                            .map(|b| u64::from_le_bytes(b[0..8].try_into().unwrap()) == want && b[0x20] == 1)
                            .unwrap_or(false)
                    })
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
    }
    ids.sort();
    ids.dedup();
    ids.into_iter().map(|id| user.join(id).join("0")).filter(|p| p.join("main").is_file()).collect()
}

// ------------------------------------------------------------------ troca

/// Copia `src` (já conferida) para `save`: cópia temporária ao lado, save atual guardado em `bdir` como
/// "<stamp>-pre-restore" e troca com rename, como restore_into. Se a troca falhar, o save antigo volta.
pub fn import_into(src: &Path, save: &Path, bdir: &Path, keep: i64, stamp: &str) -> Result<Option<String>, String> {
    if !src.join("main").is_file() {
        return Err("importar: a pasta não tem o arquivo main de um save".into());
    }
    let parent = save.parent().ok_or("importar: pasta do save inválida")?;
    // a origem não pode ser o próprio save (nem estar dentro dele)
    if let (Ok(a), Ok(b)) = (src.canonicalize(), save.canonicalize()) {
        if a.starts_with(&b) {
            return Err("importar: essa pasta já é o save deste PC".into());
        }
    }
    fs::create_dir_all(parent).map_err(|e| format!("importar: não criou a pasta do save ({e})"))?;
    let base = save.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "save".into());
    let tmp = parent.join(format!("{base}.switchos-tmp"));
    let old = parent.join(format!("{base}.switchos-old"));
    let _ = fs::remove_dir_all(&tmp);
    if let Err(e) = copy_tree(src, &tmp) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("importar: não copiou o save escolhido ({e})"));
    }
    let created = if save.exists() {
        match backup_like_switchos(save, bdir, "pre-restore", if keep > 0 { keep + 1 } else { keep }, stamp, &[]) {
            Ok(n) => Some(n),
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp);
                return Err(e.replacen("restaurar:", "importar:", 1));
            }
        }
    } else {
        None
    };
    if save.exists() {
        let _ = fs::remove_dir_all(&old);
        if let Err(e) = fs::rename(save, &old) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(format!("importar: o save atual está em uso ({e})"));
        }
        if let Err(e) = fs::rename(&tmp, save) {
            let _ = fs::rename(&old, save);
            let _ = fs::remove_dir_all(&tmp);
            return Err(format!("importar: não trocou as pastas ({e})"));
        }
        let _ = fs::remove_dir_all(&old);
    } else if let Err(e) = fs::rename(&tmp, save) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("importar: não trocou as pastas ({e})"));
    }
    Ok(created)
}

/// Pasta das cópias e quantas manter: a da sincronização ou, sem ela, <runtime>/save-backups/<TID> (10)
fn backup_target(cfg: &Config, g: &GameDef) -> (PathBuf, i64, bool) {
    let tid = g.title_id.to_uppercase();
    if let Some(root) = switchos_root(cfg) {
        if let Ok(text) = fs::read_to_string(root.join("games.toml")) {
            if let Some(cloud) = toml_sync_value(&text, "cloud_dir").filter(|v| !v.is_empty()) {
                let keep = toml_sync_value(&text, "keep_backups").and_then(|v| v.parse::<i64>().ok()).unwrap_or(10);
                return (expand_path(&root, &cloud).join("backups").join(&tid), keep, true);
            }
        }
    }
    (Path::new(&cfg.runtime_dir).join("save-backups").join(&tid), 10, false)
}

// ------------------------------------------------------------------ comandos

#[tauri::command]
pub fn import_sources(key: String) -> Result<ImportSources, String> {
    let g = game_def(&key)?;
    let cfg = get_config();
    let (bdir, _, synced) = backup_target(&cfg, g);
    let root = ryujinx_root();
    let ryujinx = root.as_deref().map(|r| ryujinx_saves(r, g.title_id).iter().map(|p| check(&key, p)).collect()).unwrap_or_default();
    Ok(ImportSources { ryujinx_dir: root.map(|r| r.to_string_lossy().into()), ryujinx, backups_dir: bdir.to_string_lossy().into(), synced })
}

#[tauri::command]
pub fn check_import(key: String, path: String) -> Result<Candidate, String> {
    game_def(&key)?;
    Ok(check(&key, Path::new(&path)))
}

/// Troca o save deste PC pela pasta `path` (conferida de novo aqui). Bloqueia com o jogo aberto.
#[tauri::command]
pub fn import_save(key: String, path: String, state: tauri::State<Jobs>) -> Result<ImportResult, String> {
    let g = game_def(&key)?;
    let cfg = get_config();
    let (importing, building) = {
        let m = state.lock().unwrap();
        let importing = m.get(&key).map(|j| j.running).unwrap_or(false);
        let building = m.values().filter(|j| j.running && j.step.starts_with("Compilando")).count();
        (importing, building)
    };
    if importing {
        return Err("importar: há uma importação em andamento deste jogo".into());
    }
    match runtime_processes() {
        Some(n) if n > building => return Err("importar: o jogo está aberto".into()),
        Some(_) => {}
        None => return Err("importar: não deu para conferir se o jogo está aberto".into()),
    }
    let c = check(&key, Path::new(&path));
    if !c.ok {
        return Err(c.reason.unwrap_or_else(|| "importar: save inválido".into()));
    }
    let (bdir, keep, _) = backup_target(&cfg, g);
    let created = import_into(Path::new(&c.path), &save_dir(&cfg, g), &bdir, keep, &local_stamp())?;
    Ok(ImportResult { created, backups_dir: bdir.to_string_lossy().into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("switchos-hub-import-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn imen(program: u64, ty: u8, id: u64) -> Vec<u8> {
        let mut k = vec![0u8; 0x40];
        k[0..8].copy_from_slice(&program.to_le_bytes());
        k[0x20] = ty;
        let mut v = vec![0u8; 0x40];
        v[0..8].copy_from_slice(&id.to_le_bytes());
        let mut e = b"IMEN".to_vec();
        e.extend(0x40u32.to_le_bytes());
        e.extend(0x40u32.to_le_bytes());
        e.extend(k);
        e.extend(v);
        e
    }

    #[test]
    fn ryujinx_mapping() {
        let d = scratch("ryu");
        let tid = 0x01001F5010DFA000u64;
        let mut db = b"IMKV".to_vec();
        db.extend(0u32.to_le_bytes());
        db.extend(3u32.to_le_bytes());
        db.extend(imen(tid, 1, 0x10));
        db.extend(imen(tid, 3, 0x11)); // device save: fica de fora
        db.extend(imen(0x0100F43008C44000, 1, 0x12));
        let sys = d.join("bis/system/save/8000000000000000/0");
        fs::create_dir_all(&sys).unwrap();
        fs::write(sys.join("imkvdb.arc"), &db).unwrap();
        for id in ["0000000000000010", "0000000000000011", "0000000000000012"] {
            fs::create_dir_all(d.join("bis/user/save").join(id).join("0")).unwrap();
            fs::write(d.join("bis/user/save").join(id).join("0/main"), b"x").unwrap();
        }
        assert_eq!(parse_imkvdb(&db).len(), 3);
        assert_eq!(ryujinx_saves(&d, "01001F5010DFA000"), vec![d.join("bis/user/save/0000000000000010/0")]);

        // sem imkvdb: cai no ExtraData0
        fs::remove_file(sys.join("imkvdb.arc")).unwrap();
        let mut extra = vec![0u8; 0x40];
        extra[0..8].copy_from_slice(&0x0100F43008C44000u64.to_le_bytes());
        extra[0x20] = 1;
        fs::write(d.join("bis/user/save/0000000000000012/ExtraData0"), &extra).unwrap();
        assert_eq!(ryujinx_saves(&d, "0100F43008C44000"), vec![d.join("bis/user/save/0000000000000012/0")]);
        assert!(ryujinx_saves(&d, "01001F5010DFA000").is_empty());
        assert!(parse_imkvdb(b"IMKV\0\0\0\0\xff\xff\xff\xffIMEN").is_empty()); // truncado: sem pânico
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn check_sources() {
        let d = scratch("check");
        let empty = d.join("vazia");
        fs::create_dir_all(&empty).unwrap();
        assert!(!check("totk", &empty).ok);
        // JKSV: pasta do jogo com uma subpasta da exportação
        let jksv = d.join("jksv").join("2026-11-19");
        fs::create_dir_all(&jksv).unwrap();
        fs::write(jksv.join("main"), b"qualquer coisa").unwrap();
        let c = check("totk", &d.join("jksv"));
        assert!(c.ok, "{c:?}");
        assert_eq!(Path::new(&c.path), jksv.as_path());
        // Z-A/Arceus: o main tem de decifrar
        let c = check("za", &jksv);
        assert!(!c.ok && c.reason.unwrap().contains("não decifrou"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn import_flow() {
        let d = scratch("flow");
        let src = d.join("src");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("main"), b"new").unwrap();
        fs::write(src.join("sub/x"), b"x").unwrap();
        let save = d.join("save").join("save_0");
        let bdir = d.join("backups").join("TID");

        // sem save atual: nada guardado
        assert_eq!(import_into(&src, &save, &bdir, 10, "20261119-100000").unwrap(), None);
        assert_eq!(fs::read(save.join("main")).unwrap(), b"new");

        fs::write(save.join("main"), b"current").unwrap();
        let created = import_into(&src, &save, &bdir, 10, "20261119-100000").unwrap();
        assert_eq!(created.as_deref(), Some("20261119-100000-pre-restore"));
        assert_eq!(fs::read(bdir.join("20261119-100000-pre-restore/main")).unwrap(), b"current");
        assert_eq!(fs::read(save.join("main")).unwrap(), b"new");
        assert_eq!(fs::read(save.join("sub/x")).unwrap(), b"x");
        assert_eq!(fs::read(src.join("main")).unwrap(), b"new"); // a origem não muda
        assert!(!d.join("save/save_0.switchos-tmp").exists() && !d.join("save/save_0.switchos-old").exists());

        // origem sem main ou o próprio save: recusa sem mexer
        assert!(import_into(&d.join("nada"), &save, &bdir, 10, "20261119-100001").is_err());
        assert!(import_into(&save, &save, &bdir, 10, "20261119-100001").is_err());
        assert_eq!(fs::read(save.join("main")).unwrap(), b"new");
        let _ = fs::remove_dir_all(&d);
    }
}
