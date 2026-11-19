//! "Jogar": abre o lasrecomp.exe direto no jogo, sem a tela inicial do ZA Recomp.
//!
//! `lasrecomp.exe --play <pasta do jogo>` (engines/las-za/runtime/main.c) pula a tela inicial. O que ela fazia antes
//! de jogar continua garantido assim:
//! - verificação dos arquivos, ícones dos botões, ajustes de LOD/distância e conteúdo: launcher_prompts_kick, que o
//!   runtime chama no --play;
//! - código de mods (subsdk): compilado no boot quando falta (runtime/codemods.c codemods_load);
//! - código do jogo desatualizado (runtime novo, ou mod com .pchtxt ligado/desligado): o runtime volta para a tela
//!   inicial só nesse caso, para recompilar (main.c, "--play: the game code isn't built");
//! - o que fica só na tela inicial: procurar atualização do ZA Recomp, escolher semente/Vanilla e mudar a ordem de mods.
//!   DLCs não se escolhem: o serviço aoc:u lê <pasta do jogo>/aoc.

use std::path::Path;
use std::process::Command;

pub(crate) fn play(runtime_dir: &Path, game_dir: &Path, running: Option<usize>) -> Result<u32, String> {
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
    let child = Command::new(&exe)
        .arg("--play")
        .arg(game_dir)
        .current_dir(runtime_dir)
        .spawn()
        .map_err(|e| format!("não abriu {}: {e}", exe.display()))?;
    Ok(child.id())
}
