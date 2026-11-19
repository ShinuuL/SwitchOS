//! Etapas de "primeira vez" que a tela do ZA Recomp (engines/las-za/runtime/ui/launcher.mm) faz sozinha e que o hub
//! agora mostra e dispara: verificar os arquivos do jogo, gerar os ícones dos Pokémon e preparar os shaders. Todas
//! rodam o zaprompts (<runtime_dir>/zaprompts/zaprompts.exe, tools/za/prompt_textures.py congelado) sobre o romfs.bin
//! do jogo, sem janela, e gravam no mesmo lugar e no mesmo formato da tela do ZA Recomp (<runtime_dir>/cache), para que
//! ela não refaça o que o hub já fez:
//!
//! - verificar: `zaprompts --romfs <romfs.bin> --verify cache/verify.txt` (log cache/verify.log). Primeira linha
//!   "ok <versão>", "damaged <versão> <n>" (+ os pacotes) ou "unknown". Feito quando cache/verify.src tem o tamanho e
//!   a data do romfs.bin (launcher_verify_kick).
//! - ícones: `zaprompts --romfs <romfs.bin> --party-icons cache/party_icons` (log cache/party_icons.log). Feito quando
//!   cache/party_icons/.made existe e cache/party_icons.src tem o tamanho e a data do romfs.bin (party_icons_kick).
//! - shaders: `zaprompts --romfs <romfs.bin> --shader-code cache/shader_code_game.bin --shader-need <...>.need` (log
//!   <...>.log). O .need é "# <tamanho hex> <data hex>" e um hash de código por linha, tirados da lista que o runtime
//!   embute (runtime/nvn/shader_variants_za.txt); o zaprompts copia o .need para .made no fim. Feito quando .made é
//!   igual ao .need (shader_code_kick).
//!
//! Só o Pokémon Z-A: o romfs do Arceus não tem arc/data.trpfs (os ícones dele são BNTX soltos,
//! pokeicon_s_<espécie>_<forma>_...), as cópias conhecidas do verify_files.py são só do Z-A e a lista de shaders
//! do Arceus vem com o código (shader_variants.txt). Ver pendencia.md.

use super::*;
use std::time::UNIX_EPOCH;

/// A lista de shaders do Z-A que o runtime embute (Makefile: shader_variants_za.inc)
const SHADER_LIST_ZA: &str = include_str!("../../../engines/las-za/runtime/nvn/shader_variants_za.txt");

#[derive(Serialize, Clone, Default)]
pub struct Step {
    /// "feito", "pendente", "danificado" (só verificar), "indisponivel"
    state: String,
    /// o que mostrar ao lado (versão verificada, motivo de indisponível, ...)
    detail: String,
}

#[derive(Serialize, Default)]
pub struct FirstRun {
    verify: Step,
    icons: Step,
    shaders: Step,
    /// zaprompts encontrado e jogo importado: dá para rodar as etapas pendentes
    can_run: bool,
}

fn step(state: &str, detail: impl Into<String>) -> Step {
    Step { state: state.into(), detail: detail.into() }
}

fn zaprompts(rt: &Path) -> PathBuf {
    rt.join("zaprompts").join(if cfg!(windows) { "zaprompts.exe" } else { "zaprompts" })
}

/// (tamanho, data em segundos desde 1970) do arquivo, como o stat() da tela do ZA Recomp
fn size_mtime(p: &Path) -> Option<(u64, u64)> {
    let m = fs::metadata(p).ok()?;
    Some((m.len(), m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs()))
}

/// O .src de verify / party_icons: "<tamanho> <data>" em decimal
fn src_stamp(romfs: &Path) -> Option<String> {
    size_mtime(romfs).map(|(s, t)| format!("{s} {t}"))
}

/// O .need do --shader-code, igual ao de shader_code_kick: identidade do romfs e os hashes vs= fs= cs= da lista
fn shader_need(romfs: &Path, list: &str) -> Option<String> {
    let (s, t) = size_mtime(romfs)?;
    let mut need = format!("# {s:x} {t:x}\n");
    let b = list.as_bytes();
    let mut i = 2;
    while let Some(off) = list[i..].find("s=") {
        let p = i + off;
        if (b[p - 1] == b'v' || b[p - 1] == b'f' || b[p - 1] == b'c') && b[p - 2] == b' ' {
            let hex: String = list[p + 2..].chars().take(16).take_while(|c| c.is_ascii_hexdigit()).collect();
            if let Ok(h) = u64::from_str_radix(&hex, 16) {
                if h != 0 {
                    need.push_str(&format!("{h:016x}\n"));
                }
            }
        }
        i = p + 2;
    }
    Some(need)
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_default()
}

/// O estado das três etapas a partir da pasta do runtime e do romfs.bin do jogo (sem rodar nada)
fn status_in(key: &str, rt: &Path, romfs: &Path) -> FirstRun {
    if key != "za" {
        let why = if key == "arceus" { "Ainda não feito para o Arceus" } else { "Jogo sem suporte" };
        return FirstRun {
            verify: step("indisponivel", why),
            icons: step("indisponivel", why),
            shaders: step("indisponivel", if key == "arceus" { "Vêm prontos com o runtime" } else { why }),
            can_run: false,
        };
    }
    if !romfs.is_file() {
        let s = step("pendente", "Importe o jogo primeiro");
        return FirstRun { verify: s.clone(), icons: s.clone(), shaders: s, can_run: false };
    }
    let cache = rt.join("cache");
    let stamp = src_stamp(romfs).unwrap_or_default();
    let same = |src: &str| read(&cache.join(src)).split_whitespace().take(2).collect::<Vec<_>>().join(" ") == stamp;

    let head = read(&cache.join("verify.txt")).lines().next().unwrap_or("").to_string();
    let verify = if head.is_empty() || !same("verify.src") {
        step("pendente", "Ainda não verificados")
    } else if let Some(r) = head.strip_prefix("damaged ") {
        let mut w = r.split_whitespace();
        let (v, n) = (w.next().unwrap_or("?"), w.next().unwrap_or("?"));
        step("danificado", format!("Versão {v}: {n} pacote(s) danificado(s). Importe o jogo de novo."))
    } else if let Some(v) = head.strip_prefix("ok ") {
        step("feito", format!("Versão {v} sem danos"))
    } else {
        step("feito", "Versão sem cópia de referência: não dá para verificar")
    };

    let icons = if cache.join("party_icons").join(".made").is_file() && same("party_icons.src") {
        step("feito", "Gerados")
    } else {
        step("pendente", "Ainda não gerados")
    };

    let made = read(&cache.join("shader_code_game.bin.made"));
    let shaders = if !made.is_empty() && Some(made) == shader_need(romfs, SHADER_LIST_ZA) {
        step("feito", "Preparados")
    } else {
        step("pendente", "Ainda não preparados")
    };
    FirstRun { verify, icons, shaders, can_run: zaprompts(rt).is_file() }
}

fn paths(key: &str) -> Result<(PathBuf, PathBuf), String> {
    let cfg = get_config();
    let g = game_def(key)?;
    Ok((PathBuf::from(&cfg.runtime_dir), Path::new(&cfg.games_dir).join(g.key).join("romfs.bin")))
}

#[tauri::command]
pub fn firstrun_status(key: String) -> Result<FirstRun, String> {
    let (rt, romfs) = paths(&key)?;
    Ok(status_in(&key, &rt, &romfs))
}

/// O job das etapas fica em "firstrun:<jogo>" (job_status com essa chave), separado do da importação
fn job_key(key: &str) -> String {
    format!("firstrun:{key}")
}

/// Roda o zaprompts com o log num arquivo (como a tela do ZA Recomp) e também no job
fn run_step(jobs: &Jobs, jk: &str, tool: &Path, rt: &Path, args: &[&std::ffi::OsStr], base: f64, span: f64) -> Result<(), String> {
    let mut c = Command::new(tool);
    c.args(args).current_dir(rt);
    run_logged(jobs, jk, c, base, span)
}

/// Roda as etapas pendentes (ou as pedidas em `steps`: "verify", "icons", "shaders") em segundo plano
#[tauri::command]
pub fn firstrun_run(key: String, steps: Vec<String>, state: tauri::State<Jobs>) -> Result<(), String> {
    let (rt, romfs) = paths(&key)?;
    let st = status_in(&key, &rt, &romfs);
    if !st.can_run {
        return Err(if key != "za" {
            "primeira vez: só o Pokémon Z-A por enquanto".into()
        } else if !romfs.is_file() {
            "primeira vez: importe o jogo primeiro".into()
        } else {
            format!("zaprompts não encontrado em {}", zaprompts(&rt).display())
        });
    }
    let want = |s: &str, step: &Step| if steps.is_empty() { step.state == "pendente" } else { steps.iter().any(|x| x == s) };
    let todo: Vec<&str> = [("verify", &st.verify), ("icons", &st.icons), ("shaders", &st.shaders)]
        .into_iter()
        .filter(|(s, step)| want(s, step))
        .map(|(s, _)| s)
        .collect();
    if todo.is_empty() {
        return Ok(());
    }
    let jobs: Jobs = state.inner().clone();
    let jk = job_key(&key);
    if jobs.lock().unwrap().get(&jk).map(|j| j.running).unwrap_or(false) {
        return Err("as etapas de primeira vez já estão rodando".into());
    }
    job_update(&jobs, &jk, |j| *j = Job { running: true, step: "Preparando".into(), ..Default::default() });
    let todo: Vec<String> = todo.into_iter().map(String::from).collect();
    std::thread::spawn(move || {
        let r = run_steps(&jobs, &jk, &rt, &romfs, &todo);
        job_update(&jobs, &jk, |j| {
            j.running = false;
            j.pid = None;
            match r {
                Ok(()) => { j.step = "Pronto".into(); j.progress = 1.0; }
                Err(e) => { j.step = format!("Falhou: {}", j.step); j.error = Some(e); }
            }
        });
    });
    Ok(())
}

fn run_steps(jobs: &Jobs, jk: &str, rt: &Path, romfs: &Path, todo: &[String]) -> Result<(), String> {
    let tool = zaprompts(rt);
    let cache = rt.join("cache");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let stamp = src_stamp(romfs).ok_or("romfs.bin não encontrado")?;
    let n = todo.len() as f64;
    for (i, s) in todo.iter().enumerate() {
        let (base, span) = (i as f64 / n, 1.0 / n);
        job_update(jobs, jk, |j| j.progress = base);
        match s.as_str() {
            "verify" => {
                job_update(jobs, jk, |j| j.step = "Verificando os arquivos do jogo".into());
                let _ = fs::remove_file(cache.join("verify.txt"));
                fs::write(cache.join("verify.src"), &stamp).map_err(|e| e.to_string())?;
                run_step(jobs, jk, &tool, rt, &["--romfs".as_ref(), romfs.as_os_str(), "--verify".as_ref(), "cache/verify.txt".as_ref()], base, span)?;
            }
            "icons" => {
                job_update(jobs, jk, |j| j.step = "Gerando os ícones dos Pokémon".into());
                let _ = fs::remove_file(cache.join("party_icons").join(".made"));
                fs::write(cache.join("party_icons.src"), &stamp).map_err(|e| e.to_string())?;
                run_step(jobs, jk, &tool, rt, &["--romfs".as_ref(), romfs.as_os_str(), "--party-icons".as_ref(), "cache/party_icons".as_ref()], base, span)?;
            }
            "shaders" => {
                job_update(jobs, jk, |j| j.step = "Preparando os shaders (até 2 min)".into());
                let need = shader_need(romfs, SHADER_LIST_ZA).ok_or("romfs.bin não encontrado")?;
                fs::write(cache.join("shader_code_game.bin.need"), need).map_err(|e| e.to_string())?;
                run_step(jobs, jk, &tool, rt, &[
                    "--romfs".as_ref(), romfs.as_os_str(),
                    "--shader-code".as_ref(), "cache/shader_code_game.bin".as_ref(),
                    "--shader-need".as_ref(), "cache/shader_code_game.bin.need".as_ref(),
                ], base, span)?;
            }
            other => return Err(format!("etapa desconhecida: {other}")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("switchos-firstrun-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("rt").join("cache").join("party_icons")).unwrap();
        fs::write(d.join("romfs.bin"), b"fake romfs").unwrap();
        d
    }

    #[test]
    fn need_has_the_list_hashes() {
        let d = tmp("need");
        let list = "# x\nC cs=0484e7076de81c5f l=32,1,1\nG vs=00000000000000aa fs=00000000000000bb x=1\nG ns=00000000000000cc\n";
        let need = shader_need(&d.join("romfs.bin"), list).unwrap();
        let lines: Vec<_> = need.lines().collect();
        assert!(lines[0].starts_with("# a "));
        assert_eq!(&lines[1..], ["0484e7076de81c5f", "00000000000000aa", "00000000000000bb"]);
        assert!(shader_need(&d.join("romfs.bin"), SHADER_LIST_ZA).unwrap().lines().count() > 10);
    }

    #[test]
    fn status_follows_the_cache() {
        let d = tmp("status");
        let (rt, romfs) = (d.join("rt"), d.join("romfs.bin"));
        let st = status_in("za", &rt, &romfs);
        assert_eq!((st.verify.state.as_str(), st.icons.state.as_str(), st.shaders.state.as_str()), ("pendente", "pendente", "pendente"));
        assert!(!st.can_run); // (sem zaprompts)
        let c = rt.join("cache");
        let stamp = src_stamp(&romfs).unwrap();
        fs::write(c.join("verify.src"), &stamp).unwrap();
        fs::write(c.join("verify.txt"), "damaged 2.0.2 1\nfoo\n").unwrap();
        fs::write(c.join("party_icons.src"), &stamp).unwrap();
        fs::write(c.join("party_icons").join(".made"), "10\n").unwrap();
        fs::write(c.join("shader_code_game.bin.made"), shader_need(&romfs, SHADER_LIST_ZA).unwrap()).unwrap();
        let st = status_in("za", &rt, &romfs);
        assert_eq!((st.verify.state.as_str(), st.icons.state.as_str(), st.shaders.state.as_str()), ("danificado", "feito", "feito"));
        fs::write(c.join("verify.txt"), "ok 2.0.2\n").unwrap();
        fs::write(c.join("party_icons.src"), "1 2").unwrap(); // (outro romfs)
        let st = status_in("za", &rt, &romfs);
        assert_eq!((st.verify.state.as_str(), st.icons.state.as_str()), ("feito", "pendente"));
        assert_eq!(status_in("arceus", &rt, &romfs).icons.state, "indisponivel");
        let _ = fs::remove_dir_all(&d);
    }
}
