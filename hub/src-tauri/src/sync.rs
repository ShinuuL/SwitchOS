//! Botão "Sincronizar" da aba Save: roda `python switchos.py sync <jogo>` na raiz do SwitchOS, em segundo plano.
//! O switchos.py guarda o save anterior em backups/ antes de substituir qualquer réplica (PC, nuvem).
use super::*;

#[derive(Serialize)]
pub struct SyncResult {
    ok: bool,
    /// últimas linhas da saída (stdout + stderr) do switchos.py
    resumo: String,
}

/// Só chaves do catálogo e só [a-z0-9]: nada que o argparse leia como opção nem que mude o comando
fn valid_key(key: &str) -> Result<&'static GameDef, String> {
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) {
        return Err(format!("sincronizar: jogo inválido: {key}"));
    }
    game_def(key).map_err(|e| format!("sincronizar: {e}"))
}

fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

fn run_sync(root: &Path, key: &str) -> Result<SyncResult, String> {
    // "python" e, se não existir no PATH, o lançador "py -3" do Windows
    let mut last_err = String::new();
    for (exe, pre) in [("python", &[][..]), ("py", &["-3"][..])] {
        let mut c = Command::new(exe);
        c.args(pre).arg("switchos.py").arg("sync").arg(key).current_dir(root);
        c.env("PYTHONIOENCODING", "utf-8").stdin(Stdio::null());
        #[cfg(windows)]
        c.creation_flags(CREATE_NO_WINDOW);
        match c.output() {
            Ok(out) => {
                let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
                let resumo = tail(&text, 12);
                return Ok(SyncResult { ok: out.status.success(), resumo: if out.status.success() || !resumo.is_empty() { resumo } else { format!("sincronizar: switchos.py saiu com {}", out.status) } });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => last_err = e.to_string(),
            Err(e) => return Err(format!("sincronizar: não deu para rodar o Python: {e}")),
        }
    }
    Err(format!("sincronizar: Python não encontrado ({last_err})"))
}

#[tauri::command]
pub async fn sync_save(key: String, state: tauri::State<'_, Jobs>) -> Result<SyncResult, String> {
    let g = valid_key(&key)?;
    if !g.supported {
        return Err(format!("sincronizar: {} ainda não roda no hub", g.name));
    }
    // mesma checagem do restore_backup: nada de trocar o save com o jogo aberto
    let (importing, building) = {
        let m = state.lock().unwrap();
        let importing = m.get(&key).map(|j| j.running).unwrap_or(false);
        let building = m.values().filter(|j| j.running && j.step.starts_with("Compilando")).count();
        (importing, building)
    };
    if importing {
        return Err("sincronizar: há uma importação em andamento deste jogo".into());
    }
    match runtime_processes() {
        Some(n) if n > building => return Err("sincronizar: o jogo está aberto".into()),
        Some(_) => {}
        None => return Err("sincronizar: não deu para conferir se o jogo está aberto".into()),
    }
    let cfg = get_config();
    let root = switchos_root(&cfg).ok_or("sincronizar: sincronização não configurada")?;
    let key = g.key;
    tauri::async_runtime::spawn_blocking(move || run_sync(&root, key))
        .await
        .map_err(|e| format!("sincronizar: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert!(valid_key("za").is_ok());
        assert!(valid_key("arceus").is_ok());
        for bad in ["", "--dry-run", "za --no-android", "ZA", "za;calc", "../za", "nope"] {
            assert!(valid_key(bad).is_err(), "{bad:?} deveria ser recusada");
        }
    }

    #[test]
    fn tail_lines() {
        assert_eq!(tail("a\n\nb\nc\n", 2), "b\nc");
        assert_eq!(tail("", 3), "");
    }
}
