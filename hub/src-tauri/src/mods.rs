//! Mods de cada jogo: listar e ligar/desligar.
//!
//! O runtime (engines/las-za/runtime/services/fs.c e codemods.c) ignora uma pasta de mods/ quando o nome começa com
//! "_" ou "." ou quando ela está em `mods_off` do larecomp.ini (lista separada por "|", a página Mods da tela inicial
//! do ZA Recomp). O hub desliga renomeando para "_<nome>" e liga tirando o "_" (e o nome de `mods_off`, se estiver lá).
//! Só com o jogo fechado: o runtime lê os mods no boot e, aberto, pode estar com arquivos deles em uso.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::GameDef;

#[derive(Serialize, Debug)]
pub(crate) struct ModInfo {
    /// nome da pasta sem o "_" de desligado (o que o usuário vê e o que o hub manda de volta para ligar/desligar)
    pub name: String,
    pub files: usize,
    pub on: bool,
    /// tem patch de código (.pchtxt / subsdk em exefs/): ligar ou desligar muda o código do jogo, que é recompilado
    pub code: bool,
}

pub(crate) fn count_files(p: &Path) -> usize {
    fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| if e.path().is_dir() { count_files(&e.path()) } else { 1 }).sum())
        .unwrap_or(0)
}

/// Pasta com o title id dentro do mod (sem diferenciar maiúsculas, como o runtime), ou o próprio mod para o Z-A
fn mod_leaf(mod_dir: &Path, g: &GameDef, leaf: &str) -> Option<PathBuf> {
    let tid = g.title_id;
    let mut roots = vec![
        mod_dir.join("atmosphere").join("contents").join(tid).join(leaf),
        mod_dir.join("contents").join(tid).join(leaf),
        mod_dir.join(tid).join(leaf),
    ];
    if g.key == "za" {
        roots.push(mod_dir.join(leaf));
    }
    roots.into_iter().find(|p| p.is_dir()).or_else(|| {
        fs::read_dir(mod_dir).ok()?.flatten().find_map(|e| {
            (e.file_name().to_string_lossy().eq_ignore_ascii_case(tid) && e.path().join(leaf).is_dir())
                .then(|| e.path().join(leaf))
        })
    })
}

/// O mesmo critério do runtime (services/mod_layout.c): outros jogos só pela pasta com o title id
pub(crate) fn mod_romfs(mod_dir: &Path, g: &GameDef) -> Option<PathBuf> {
    mod_leaf(mod_dir, g, "romfs")
}

fn has_code(mod_dir: &Path, g: &GameDef) -> bool {
    mod_leaf(mod_dir, g, "exefs").map_or(false, |ex| {
        fs::read_dir(ex).map(|rd| rd.flatten().any(|e| e.path().is_file())).unwrap_or(false)
    })
}

/// mods_off do larecomp.ini (pasta de dados do runtime)
fn mods_off(runtime_dir: &Path) -> Vec<String> {
    fs::read_to_string(runtime_dir.join("larecomp.ini"))
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == "mods_off").then(|| v.trim().split('|').filter(|s| !s.is_empty()).map(String::from).collect())
            })
        })
        .unwrap_or_default()
}

/// Tira `name` de mods_off no larecomp.ini, mantendo o resto do arquivo como está
fn remove_from_mods_off(runtime_dir: &Path, name: &str) -> Result<(), String> {
    let ini = runtime_dir.join("larecomp.ini");
    let Ok(text) = fs::read_to_string(&ini) else { return Ok(()) };
    let mut changed = false;
    let out: Vec<String> = text
        .split_inclusive('\n')
        .map(|l| {
            if let Some((k, v)) = l.split_once('=') {
                if k.trim() == "mods_off" {
                    let eol = if l.ends_with("\r\n") { "\r\n" } else if l.ends_with('\n') { "\n" } else { "" };
                    let keep: Vec<&str> = v.trim().split('|').filter(|s| !s.is_empty() && *s != name).collect();
                    let new = format!("{}= {}{}", k, keep.join("|"), eol);
                    changed |= new != l;
                    return new;
                }
            }
            l.to_string()
        })
        .collect();
    if changed {
        fs::write(&ini, out.concat()).map_err(|e| format!("não consegui gravar {}: {e}", ini.display()))?;
    }
    Ok(())
}

/// Os mods deste jogo em <runtime>/mods, ligados e desligados, por nome
pub(crate) fn list(runtime_dir: &Path, g: &GameDef) -> Vec<ModInfo> {
    let off = mods_off(runtime_dir);
    let mut out = vec![];
    if let Ok(rd) = fs::read_dir(runtime_dir.join("mods")) {
        for e in rd.flatten() {
            let folder = e.file_name().to_string_lossy().to_string();
            if folder.starts_with('.') || !e.path().is_dir() {
                continue;
            }
            let (name, renamed_off) = match folder.strip_prefix('_') {
                Some(n) if !n.is_empty() => (n.to_string(), true),
                _ => (folder.clone(), false),
            };
            if let Some(r) = mod_romfs(&e.path(), g) {
                let on = !renamed_off && !off.contains(&folder);
                out.push(ModInfo { files: count_files(&r), code: has_code(&e.path(), g), name, on });
            } else if has_code(&e.path(), g) {
                let on = !renamed_off && !off.contains(&folder);
                out.push(ModInfo { files: 0, code: true, name, on });
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.starts_with('_') && !name.contains(['/', '\\', ':']) && name != ".."
}

fn rename_err(from: &Path, to: &Path, e: std::io::Error) -> String {
    // (Windows: 5 = acesso negado, 32 = arquivo em uso por outro programa)
    if matches!(e.raw_os_error(), Some(5) | Some(32)) || e.kind() == std::io::ErrorKind::PermissionDenied {
        format!(
            "A pasta do mod está em uso por outro programa (Explorer, editor, jogo). Feche o que estiver usando {} e tente de novo.",
            from.display()
        )
    } else {
        format!("não consegui renomear {} para {}: {e}", from.display(), to.display())
    }
}

/// Liga (`on`) ou desliga o mod `name` em <runtime_dir>/mods. Devolve se o mod tem código (precisa recompilar).
/// Quem chama garante que o jogo está fechado.
pub(crate) fn set(runtime_dir: &Path, name: &str, on: bool) -> Result<bool, String> {
    if !valid_name(name) {
        return Err(format!("nome de mod inválido: {name}"));
    }
    let mods = runtime_dir.join("mods");
    let lit = mods.join(name);
    let dark = mods.join(format!("_{name}"));
    let (from, to) = if on { (&dark, &lit) } else { (&lit, &dark) };
    if on && !dark.is_dir() && lit.is_dir() {
        // já tem o nome de ligado: só pode estar desligado pelo mods_off da tela inicial
        remove_from_mods_off(runtime_dir, name)?;
        return Ok(code_any(&lit));
    }
    if !on && !lit.is_dir() && dark.is_dir() {
        return Ok(code_any(&dark));  // já desligado
    }
    if !from.is_dir() {
        return Err(format!("o mod {name} não está mais na pasta de mods"));
    }
    if to.exists() {
        return Err(format!(
            "já existe {} e {}: apague ou renomeie uma das duas pastas",
            lit.file_name().unwrap().to_string_lossy(),
            dark.file_name().unwrap().to_string_lossy()
        ));
    }
    fs::rename(from, to).map_err(|e| rename_err(from, to, e))?;
    if on {
        remove_from_mods_off(runtime_dir, name)?;
    }
    Ok(code_any(to))
}

/// Código em qualquer exefs/ do mod (o jogo não importa aqui: só para avisar da recompilação)
fn code_any(dir: &Path) -> bool {
    fn walk(p: &Path, depth: u32) -> bool {
        if depth > 5 {
            return false;
        }
        fs::read_dir(p).map_or(false, |rd| {
            rd.flatten().any(|e| {
                let path = e.path();
                path.is_dir()
                    && if e.file_name().to_string_lossy().eq_ignore_ascii_case("exefs") {
                        fs::read_dir(&path).map_or(false, |r| r.flatten().any(|f| f.path().is_file()))
                    } else {
                        walk(&path, depth + 1)
                    }
            })
        })
    }
    walk(dir, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn za() -> &'static GameDef {
        crate::game_def("za").unwrap()
    }

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("switchos-hub-mods-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("mods")).unwrap();
        d
    }

    fn mk_mod(rt: &Path, folder: &str, code: bool) {
        let m = rt.join("mods").join(folder);
        fs::create_dir_all(m.join("romfs").join("a")).unwrap();
        fs::write(m.join("romfs").join("a").join("x.bin"), b"x").unwrap();
        if code {
            fs::create_dir_all(m.join("exefs")).unwrap();
            fs::write(m.join("exefs").join("p.pchtxt"), b"@nsobid-0").unwrap();
        }
    }

    #[test]
    fn lista_e_alterna() {
        let rt = scratch("toggle");
        mk_mod(&rt, "Cores", false);
        mk_mod(&rt, "_Rapido", true);
        let l = list(&rt, za());
        assert_eq!(l.len(), 2);
        assert!(l[0].name == "Cores" && l[0].on && !l[0].code && l[0].files == 1);
        assert!(l[1].name == "Rapido" && !l[1].on && l[1].code);

        assert_eq!(set(&rt, "Cores", false), Ok(false));
        assert!(rt.join("mods/_Cores").is_dir() && !rt.join("mods/Cores").exists());
        assert_eq!(set(&rt, "Rapido", true), Ok(true));
        assert!(rt.join("mods/Rapido").is_dir());
        // repetir não faz nada
        assert_eq!(set(&rt, "Rapido", true), Ok(true));
        let l = list(&rt, za());
        assert!(!l[0].on && l[1].on);
        // conflito: as duas pastas existem
        mk_mod(&rt, "Cores", false);
        assert!(set(&rt, "Cores", true).unwrap_err().contains("já existe"));
        assert!(set(&rt, "../x", true).is_err());
        assert!(set(&rt, "Sumiu", false).is_err());
        let _ = fs::remove_dir_all(&rt);
    }

    #[test]
    fn mods_off_do_ini() {
        let rt = scratch("ini");
        mk_mod(&rt, "Cores", false);
        mk_mod(&rt, "Outro", false);
        fs::write(rt.join("larecomp.ini"), "a = 1\r\nmods_off = Cores|Outro\r\nb = 2\r\n").unwrap();
        let l = list(&rt, za());
        assert!(!l[0].on && !l[1].on);
        set(&rt, "Cores", true).unwrap();
        assert_eq!(fs::read_to_string(rt.join("larecomp.ini")).unwrap(), "a = 1\r\nmods_off = Outro\r\nb = 2\r\n");
        assert!(list(&rt, za())[0].on);
        // desligar pelo hub renomeia, mesmo com o ini
        set(&rt, "Cores", false).unwrap();
        assert!(rt.join("mods/_Cores").is_dir());
        let _ = fs::remove_dir_all(&rt);
    }
}
