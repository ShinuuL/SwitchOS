//! "Jogar": abre o lasrecomp.exe direto no jogo, sem a tela inicial do ZA Recomp.
//!
//! `lasrecomp.exe --play <pasta do jogo>` (engines/las-za/runtime/main.c) pula a tela inicial e nunca a mostra. O que
//! ela fazia antes de jogar continua garantido assim:
//! - verificação dos arquivos, ícones dos botões, ajustes de LOD/distância e conteúdo: launcher_prompts_kick, que o
//!   runtime chama no --play;
//! - código de mods (subsdk): compilado no boot quando falta (runtime/codemods.c codemods_load);
//! - código do jogo desatualizado (runtime novo, ou mod com .pchtxt ligado/desligado): o runtime sai sem janela com
//!   o código PLAY_EXIT_STALE ("--play: the game code isn't built" no log); o waiter abaixo marca o jogo e o hub
//!   oferece o "Recompilar" (lib.rs needs_rebuild). O caso do runtime novo o hub já pega antes de abrir (gamelib_stale);
//! - "Tela inicial" no menu F1 do jogo: no --play o runtime fecha e volta para o hub;
//! - ordem dos mods: aba Mods do hub (mods.rs move, mods_order do larecomp.ini);
//! - o que fica só na tela inicial: procurar atualização do ZA Recomp e escolher semente/Vanilla (só Zelda).
//!   DLCs não se escolhem: o serviço aoc:u lê <pasta do jogo>/aoc.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// Código de saída do `lasrecomp --play` quando o código do jogo não serve (runtime/main.c PLAY_EXIT_STALE)
pub(crate) const PLAY_EXIT_STALE: i32 = 4;

/// Jogos cujo último --play saiu com PLAY_EXIT_STALE (até recompilar: clear_stale)
static STALE: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
/// Último --play de cada jogo: None enquanto aberto, Some(código) ao fechar (play_exit, para o main.js)
static EXITS: Mutex<BTreeMap<String, Option<i32>>> = Mutex::new(BTreeMap::new());

pub(crate) fn is_stale(key: &str) -> bool {
    STALE.lock().unwrap().contains(key)
}
pub(crate) fn clear_stale(key: &str) {
    STALE.lock().unwrap().remove(key);
}
/// O último --play deste jogo: Some(None) ainda aberto, Some(Some(código)) já fechou, None nunca abriu
pub(crate) fn last_exit(key: &str) -> Option<Option<i32>> {
    EXITS.lock().unwrap().get(key).copied()
}

fn record_exit(key: &str, code: Option<i32>) {
    if code == Some(PLAY_EXIT_STALE) {
        STALE.lock().unwrap().insert(key.to_string());
    }
    EXITS.lock().unwrap().insert(key.to_string(), code);
}

pub(crate) fn play(key: &str, runtime_dir: &Path, game_dir: &Path, running: Option<usize>) -> Result<u32, String> {
    play_with(key, runtime_dir, game_dir, running, &[], |_| {})
}

/// O "Jogar" com variáveis de ambiente a mais (record.rs: a gravação da entrada e o replay do bug report) e `on_exit`,
/// chamado na thread que espera o processo, depois de o fim ficar registrado (play_exit)
pub(crate) fn play_with(
    key: &str,
    runtime_dir: &Path,
    game_dir: &Path,
    running: Option<usize>,
    envs: &[(&str, &std::ffi::OsStr)],
    on_exit: impl FnOnce(Option<i32>) + Send + 'static,
) -> Result<u32, String> {
    if !game_dir.join("lasgame.dll").is_file() {
        return Err("o código do jogo ainda não foi compilado: use Importar".into());
    }
    let exe = runtime_dir.join("lasrecomp.exe");
    if !exe.is_file() {
        return Err(format!("lasrecomp.exe não encontrado em {}", runtime_dir.display()));
    }
    if running.unwrap_or(0) > 0 {
        return Err("O jogo já está aberto (ou uma importação está compilando). Feche-o antes de abrir de novo.".into());
    }
    let mut child = Command::new(&exe)
        .arg("--play")
        .arg(game_dir)
        .current_dir(runtime_dir)
        .envs(envs.iter().copied())
        // (sem herdar a saída do hub: NUL conta como console para o runtime, que então escreve logs/play.log -
        // a gravação o copia; um pipe herdado mandaria o log para o pipe)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("não abriu {}: {e}", exe.display()))?;
    let pid = child.id();
    EXITS.lock().unwrap().insert(key.to_string(), None);
    let key = key.to_string();
    // espera o processo numa thread à parte: o hub segue livre e o main.js pergunta o fim por play_exit
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|s| s.code());
        record_exit(&key, code);
        on_exit(code);
    });
    Ok(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saida_desatualizada_marca_o_jogo() {
        record_exit("t-ok", Some(0));
        assert!(!is_stale("t-ok"));
        assert_eq!(last_exit("t-ok"), Some(Some(0)));
        record_exit("t-velho", Some(PLAY_EXIT_STALE));
        assert!(is_stale("t-velho"));
        clear_stale("t-velho");
        assert!(!is_stale("t-velho"));
        assert_eq!(last_exit("t-nunca"), None);
    }
}
