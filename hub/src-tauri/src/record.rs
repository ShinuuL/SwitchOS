//! "Gravar sessão": o "Jogar" (launch.rs) gravando tudo o que um desenvolvedor precisa para ver e reproduzir um bug.
//!
//! Cada sessão vai para <pasta das gravações>/<jogo>-<AAAAMMDD-HHMMSS>/ (pasta nas Configurações, record_dir; vazia:
//! Documentos\SwitchOS\Gravações) com:
//! - video.mp4: só a janela do jogo, ~30 fps, H.264. Windows.Graphics.Capture (crate windows-capture) + o codificador
//!   do Media Foundation, dentro do hub: nada a instalar, nenhum exe auxiliar, e o jogo não sabe que está sendo gravado
//!   (a captura é do compositor do Windows; o custo é a cópia de uma textura por quadro e o codificador de hardware).
//!   A janela é achada pelo PID do lasrecomp e pelo título ("... — SwitchOS"); o tamanho do vídeo é o da janela no
//!   começo (se ela mudar depois, o quadro é cortado / completado com preto).
//! - input.rec: o que o jogador segurou, quadro a quadro (LARECOMP_RECORD_INPUT, runtime/hle/input.c), no formato do
//!   LARECOMP_INPUT=@<arquivo>;
//! - replay-<data>.zip: o bug report do runtime (LARECOMP_BUG_RECORD=1, runtime/bugrec.c: cópia do save de antes, a
//!   entrada com o ritmo, as marcas do F8, info e log), que o tools/za/bug-replay.sh e o ZA DevTools reproduzem.
//!   O runtime só o fecha quando o jogo fecha (ou no próximo início, depois de um crash);
//! - play.log, crash-*.log (os novos), larecomp.ini e info.txt (jogo, data, versões, fps60, duração);
//! - notas.txt: a anotação do jogador ("o que aconteceu e em que segundo"), pelo hub depois de parar.
//!
//! "Gravar vídeo" (video_only): só o video.mp4 e o info.txt, como um vídeo de gameplay comum.
//!
//! "Parar" para o vídeo e já copia logs e info; quando o jogo fecha, a cópia é refeita (log completo, zip do replay).

use crate::{game_def, gamelib_stale, get_config, launch, runtime_processes, Config, NEEDS_REBUILD};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

/// Pasta padrão das gravações: Documentos\SwitchOS\Gravações
pub(crate) fn default_base() -> PathBuf {
    dirs::document_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from(".")).join("SwitchOS").join("Gravações")
}

/// A pasta das gravações configurada (vazia: a padrão)
pub(crate) fn base_dir(record_dir: &str) -> PathBuf {
    let t = record_dir.trim();
    if t.is_empty() { default_base() } else { PathBuf::from(t) }
}

/// <base>/<jogo>-<AAAAMMDD-HHMMSS>; já existindo (duas no mesmo segundo), ganha -2, -3...
pub(crate) fn session_dir(base: &Path, key: &str, stamp: &str) -> PathBuf {
    let first = base.join(format!("{key}-{stamp}"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| base.join(format!("{key}-{stamp}-{n}"))).find(|p| !p.exists()).unwrap()
}

/// Valor de uma chave `nome = valor` do larecomp.ini (comentários com ; ou #)
pub(crate) fn ini_value<'a>(ini: &'a str, name: &str) -> Option<&'a str> {
    ini.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == name).map(|(_, v)| v.split([';', '#']).next().unwrap_or("").trim())
}

/// "1 min 05 s"
pub(crate) fn fmt_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 { format!("{s} s") } else if s < 3600 { format!("{} min {:02} s", s / 60, s % 60) } else { format!("{} h {:02} min {:02} s", s / 3600, s / 60 % 60, s % 60) }
}

/// O que vai no info.txt
pub(crate) struct Info<'a> {
    pub game: &'a str,
    pub key: &'a str,
    pub started: &'a str,
    pub ended: Option<&'a str>,
    pub duration: Duration,
    pub video: Option<Duration>,
    pub game_version: Option<&'a str>,
    pub runtime_gamelib: Option<&'a str>,
    pub game_gamelib: Option<&'a str>,
    pub fps60: Option<bool>,
    pub exit_code: Option<Option<i32>>,
    pub video_note: Option<&'a str>,
}

pub(crate) fn info_text(i: &Info) -> String {
    let opt = |v: Option<&str>| v.map(str::to_string).unwrap_or_else(|| "?".into());
    let mut s = String::new();
    s += &format!("jogo: {} ({})\n", i.game, i.key);
    s += &format!("versão do jogo: {}\n", opt(i.game_version));
    s += &format!("gamelib do runtime: {}\n", opt(i.runtime_gamelib));
    s += &format!("gamelib do código do jogo: {}\n", opt(i.game_gamelib));
    s += &format!("fps60: {}\n", match i.fps60 { Some(true) => "ligado", Some(false) => "desligado", None => "?" });
    s += &format!("início: {}\n", i.started);
    s += &format!("fim: {}\n", i.ended.unwrap_or("o jogo ainda estava aberto (vídeo parado pelo hub)"));
    s += &format!("duração: {}\n", fmt_duration(i.duration));
    s += &format!("vídeo: {}\n", match (i.video, i.video_note) {
        (Some(d), _) => format!("{} (video.mp4)", fmt_duration(d)),
        (None, Some(n)) => format!("não gravado: {n}"),
        (None, None) => "não gravado".into(),
    });
    if let Some(c) = i.exit_code {
        s += &format!("saída do jogo: {}\n", c.map(|c| c.to_string()).unwrap_or_else(|| "sem código (encerrado à força)".into()));
    }
    s += "\narquivos: video.mp4 (janela do jogo), input.rec (entrada, LARECOMP_INPUT=@input.rec), replay-*.zip (bug report do\n";
    s += "runtime: tools/za/bug-replay.sh), play.log, crash-*.log, larecomp.ini, notas.txt (anotação do jogador)\n";
    s
}

// ------------------------------------------------------------------ vídeo

#[cfg(windows)]
mod video {
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
    use windows_capture::encoder::{AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder, VideoSettingsSubType};
    use windows_capture::frame::Frame;
    use windows_capture::graphics_capture_api::InternalCaptureControl;
    use windows_capture::settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    };
    use windows_capture::window::Window;

    type Error = Box<dyn std::error::Error + Send + Sync>;
    /// ~30 fps: quadros mais próximos que isto (unidades de 100 ns) são pulados - o jogo roda a 60
    const FRAME_GAP: i64 = 10_000_000 / 30 - 20_000;

    pub(crate) struct Rec {
        enc: Option<VideoEncoder>,
        last: Option<i64>,
        frames: Arc<AtomicU64>,
    }

    impl GraphicsCaptureApiHandler for Rec {
        type Flags = (u32, u32, String, Arc<AtomicU64>);
        type Error = Error;

        fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
            let (w, h, path, frames) = ctx.flags;
            let enc = VideoEncoder::new(
                VideoSettingsBuilder::new(w, h).sub_type(VideoSettingsSubType::H264).frame_rate(30).bitrate(8_000_000),
                AudioSettingsBuilder::default().disabled(true),
                ContainerSettingsBuilder::default(),
                path,
            )?;
            Ok(Self { enc: Some(enc), last: None, frames })
        }

        fn on_frame_arrived(&mut self, frame: &mut Frame, _: InternalCaptureControl) -> Result<(), Self::Error> {
            let t = frame.timestamp()?.Duration;
            if self.last.is_some_and(|l| t - l < FRAME_GAP) {
                return Ok(());
            }
            if let Some(e) = self.enc.as_mut() {
                e.send_frame(frame)?;
                self.last = Some(t);
                self.frames.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        }

        fn on_closed(&mut self) -> Result<(), Self::Error> {
            // a janela fechou (o jogo saiu): fecha o MP4 aqui mesmo
            if let Some(e) = self.enc.take() {
                e.finish()?;
            }
            Ok(())
        }
    }

    pub(crate) struct Capture {
        control: CaptureControl<Rec, Error>,
        pub frames: Arc<AtomicU64>,
    }

    impl Capture {
        /// Para a captura e fecha o MP4 (o que já veio fica gravado)
        pub fn stop(self) -> Result<(), String> {
            let cb = self.control.callback();
            let _ = self.control.stop();
            let enc = cb.lock().enc.take();
            match enc {
                Some(e) => e.finish().map_err(|e| e.to_string()),
                None => Ok(()),
            }
        }
        pub fn frames(&self) -> u64 {
            self.frames.load(Ordering::Relaxed)
        }
    }

    /// A janela visível do processo `pid` cujo título contém `title`
    pub(crate) fn find_window(pid: u32, title: &str) -> Option<Window> {
        Window::enumerate().ok()?.into_iter().find(|w| w.process_id().ok() == Some(pid) && w.title().is_ok_and(|t| t.contains(title)))
    }

    /// Começa a gravar `window` em `path` (MP4/H.264)
    pub(crate) fn start(window: Window, path: &Path) -> Result<Capture, String> {
        let r = window.rect().map_err(|e| e.to_string())?;
        // (o codificador H.264 quer lados pares; o tamanho da área cliente vem no 1º quadro, a borda some no corte)
        let w = ((window.width().unwrap_or(r.right - r.left)).max(64) as u32) & !1;
        let h = ((window.height().unwrap_or(r.bottom - r.top)).max(64) as u32) & !1;
        let frames = Arc::new(AtomicU64::new(0));
        let flags = (w, h, path.to_string_lossy().into_owned(), frames.clone());
        // sem a borda amarela nem o cursor; Windows antigo sem a opção da borda: tenta com o padrão
        let settings = |border| {
            Settings::new(
                window,
                CursorCaptureSettings::WithoutCursor,
                border,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Bgra8,
                flags.clone(),
            )
        };
        let control = Rec::start_free_threaded(settings(DrawBorderSettings::WithoutBorder))
            .or_else(|_| Rec::start_free_threaded(settings(DrawBorderSettings::Default)))
            .map_err(|e| format!("a captura da janela não começou: {e}"))?;
        Ok(Capture { control, frames })
    }
}

// ------------------------------------------------------------------ sessão

struct Session {
    key: String,
    /// o lasrecomp.exe aberto pela gravação
    #[cfg_attr(not(test), allow(dead_code))]
    pid: u32,
    game: &'static str,
    dir: PathBuf,
    runtime_dir: PathBuf,
    game_dir: PathBuf,
    started: Instant,
    started_wall: SystemTime,
    started_text: String,
    /// o jogo fechou (Some(código))
    exit: Option<Option<i32>>,
    /// "Parar" pedido (ou o jogo fechou): o vídeo não começa mais
    stopped: bool,
    #[cfg(windows)]
    capture: Option<video::Capture>,
    video_started: Option<Instant>,
    video_len: Option<Duration>,
    video_error: Option<String>,
    /// arquivos já copiados para a pasta da sessão
    files: Vec<String>,
    /// "Gravar vídeo": só o vídeo da janela (sem entrada, replay nem logs)
    video_only: bool,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

#[derive(Serialize)]
pub(crate) struct RecordStatus {
    key: String,
    dir: String,
    /// o vídeo está gravando agora
    recording: bool,
    /// esperando a janela do jogo aparecer
    waiting: bool,
    /// a sessão terminou para o hub (Parar ou o jogo fechou)
    done: bool,
    /// o jogo ainda está aberto
    game_open: bool,
    seconds: u64,
    video_only: bool,
    video_error: Option<String>,
    files: Vec<String>,
}

fn now_text() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn read_trim(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn modified(p: &Path) -> Option<SystemTime> {
    fs::metadata(p).ok()?.modified().ok()
}

/// Copia os logs, o larecomp.ini e o zip do replay para a pasta da sessão e (re)escreve o info.txt
fn collect(s: &mut Session) {
    let rt = &s.runtime_dir;
    let since = s.started_wall - Duration::from_secs(2);
    // só vídeo: nada de logs nem replay (um vídeo de gameplay comum, com o info.txt ao lado)
    let video_only = s.video_only;
    let mut copy = |from: &Path, name: &str| {
        if video_only {
            return;
        }
        if from.is_file() && fs::copy(from, s.dir.join(name)).is_ok() && !s.files.iter().any(|f| f == name) {
            s.files.push(name.to_string());
        }
    };
    copy(&rt.join("logs").join("play.log"), "play.log");
    copy(&rt.join("larecomp.ini"), "larecomp.ini");
    // crash-<data>.log: os desta sessão (o runtime separa o log de um crash no próximo início)
    if let Ok(rd) = fs::read_dir(rt.join("logs")) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with("crash-") && n.ends_with(".log") && modified(&e.path()).is_some_and(|t| t >= since) {
                copy(&e.path(), &n);
            }
        }
    }
    // o bug report do runtime: bug-reports/new.txt diz qual zip acabou de ser fechado (runtime/bugrec.c pack)
    if let Some(zip) = read_trim(&rt.join("bug-reports").join("new.txt")) {
        let zp = rt.join(&zip);
        if modified(&zp).is_some_and(|t| t >= since) {
            let name = format!("replay-{}", zp.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "sessao.zip".into()));
            copy(&zp, &name);
        }
    }
    if !video_only && s.dir.join("input.rec").is_file() && !s.files.iter().any(|f| f == "input.rec") {
        s.files.push("input.rec".into());
    }
    let ini = fs::read_to_string(rt.join("larecomp.ini")).unwrap_or_default();
    let ended = s.exit.map(|_| now_text());
    let text = info_text(&Info {
        game: s.game,
        key: &s.key,
        started: &s.started_text,
        ended: ended.as_deref(),
        duration: s.started.elapsed(),
        video: s.video_len,
        game_version: read_trim(&s.game_dir.join("version.txt")).as_deref(),
        runtime_gamelib: read_trim(&rt.join("gamelib.id")).as_deref(),
        game_gamelib: read_trim(&s.game_dir.join("lasgame.dll.id")).as_deref(),
        fps60: ini_value(&ini, "fps60").map(|v| v != "0"),
        exit_code: s.exit,
        video_note: s.video_error.as_deref(),
    });
    let _ = fs::write(s.dir.join("info.txt"), text);
}

/// Para o vídeo (se gravando) e marca a duração
fn stop_video(s: &mut Session) {
    s.stopped = true;
    #[cfg(windows)]
    if let Some(c) = s.capture.take() {
        let frames = c.frames();
        if let Err(e) = c.stop() {
            s.video_error = Some(e);
        }
        s.video_len = s.video_started.map(|t| t.elapsed());
        if frames == 0 {
            s.video_error.get_or_insert_with(|| "nenhum quadro da janela chegou".into());
            s.video_len = None;
        } else {
            s.files.insert(0, "video.mp4".into());
        }
    }
    if s.video_len.is_none() && s.video_error.is_none() {
        s.video_error = Some("a janela do jogo não apareceu".into());
    }
}

/// Espera a janela do jogo (até 3 min: o primeiro boot compila shaders) e começa o vídeo
#[cfg(windows)]
fn video_thread(pid: u32, dir: PathBuf) {
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        {
            let g = SESSION.lock().unwrap();
            match g.as_ref() {
                Some(s) if s.dir == dir && !s.stopped && s.exit.is_none() => {}
                _ => return,
            }
        }
        if let Some(w) = video::find_window(pid, "SwitchOS") {
            // (um instante para a janela ganhar o tamanho final)
            std::thread::sleep(Duration::from_millis(500));
            let r = video::start(w, &dir.join("video.mp4"));
            let mut g = SESSION.lock().unwrap();
            let Some(s) = g.as_mut().filter(|s| s.dir == dir) else {
                if let Ok(c) = r { let _ = c.stop(); }
                return;
            };
            match r {
                Ok(c) if !s.stopped && s.exit.is_none() => {
                    s.capture = Some(c);
                    s.video_started = Some(Instant::now());
                }
                Ok(c) => { let _ = c.stop(); }
                Err(e) => s.video_error = Some(e),
            }
            return;
        }
        if Instant::now() > deadline {
            if let Some(s) = SESSION.lock().unwrap().as_mut().filter(|s| s.dir == dir) {
                s.video_error = Some("a janela do jogo não apareceu em 3 minutos".into());
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn game_closed(dir: &Path, code: Option<i32>) {
    let mut g = SESSION.lock().unwrap();
    let Some(s) = g.as_mut().filter(|s| s.dir == dir) else { return };
    s.exit = Some(code);
    if !s.stopped {
        stop_video(s);
    }
    // (o runtime fecha o zip do replay e o log antes de sair; um instante para o disco)
    collect(s);
}

fn status_of(s: &Session) -> RecordStatus {
    #[cfg(windows)]
    let recording = s.capture.is_some();
    #[cfg(not(windows))]
    let recording = false;
    RecordStatus {
        key: s.key.clone(),
        dir: s.dir.to_string_lossy().into(),
        recording,
        waiting: !recording && !s.stopped && s.exit.is_none() && s.video_error.is_none(),
        done: s.stopped || s.exit.is_some(),
        game_open: s.exit.is_none(),
        seconds: s.started.elapsed().as_secs(),
        video_only: s.video_only,
        video_error: s.video_error.clone(),
        files: s.files.clone(),
    }
}

/// "Gravar sessão": abre o jogo como o Jogar, gravando entrada, vídeo da janela e o bug report do runtime
#[tauri::command]
/// `video_only` ("Gravar vídeo"): só o vídeo da janela, como um vídeo de gameplay comum - sem LARECOMP_RECORD_INPUT,
/// sem o bug report do runtime e sem copiar logs (fica só o info.txt ao lado do video.mp4)
pub(crate) fn record_start(key: String, video_only: Option<bool>) -> Result<RecordStatus, String> {
    let cfg: Config = get_config();
    let g = game_def(&key)?;
    if !g.supported {
        return Err(format!("{} ainda não roda no runtime do hub", g.name));
    }
    if SESSION.lock().unwrap().as_ref().is_some_and(|s| s.exit.is_none()) {
        return Err("Já há uma gravação com o jogo aberto. Feche o jogo antes de gravar outra.".into());
    }
    let rt = PathBuf::from(&cfg.runtime_dir);
    let dir = Path::new(&cfg.games_dir).join(g.key);
    if dir.join("lasgame.dll").is_file() && (gamelib_stale(&rt, &dir) || launch::is_stale(g.key)) {
        return Err(format!("{NEEDS_REBUILD}o código do jogo precisa ser recompilado"));
    }
    start_session(g, &rt, &rt, &dir, &base_dir(&cfg.record_dir), video_only.unwrap_or(false), &[])
}

/// A sessão: `exe_dir` tem o lasrecomp.exe, `data_dir` é a pasta de dados do runtime (logs/, larecomp.ini,
/// bug-reports/; a mesma no hub, outra num teste com LARECOMP_DATA_ROOT em `extra`)
fn start_session(
    g: &'static crate::GameDef,
    exe_dir: &Path,
    data_dir: &Path,
    dir: &Path,
    base: &Path,
    video_only: bool,
    extra: &[(&str, &std::ffi::OsStr)],
) -> Result<RecordStatus, String> {
    let (rt, dir) = (data_dir.to_path_buf(), dir.to_path_buf());
    let now = chrono::Local::now();
    let sdir = session_dir(base, g.key, &now.format("%Y%m%d-%H%M%S").to_string());
    fs::create_dir_all(&sdir).map_err(|e| format!("não criou a pasta da gravação {}: {e}", sdir.display()))?;
    let input = sdir.join("input.rec");
    let mut envs: Vec<(&str, &std::ffi::OsStr)> = if video_only {
        vec![]
    } else {
        vec![("LARECOMP_RECORD_INPUT", input.as_os_str()), ("LARECOMP_BUG_RECORD", "1".as_ref())]
    };
    envs.extend_from_slice(extra);
    let close_dir = sdir.clone();
    let pid = match launch::play_with(g.key, exe_dir, &dir, runtime_processes(), &envs, move |code| game_closed(&close_dir, code)) {
        Ok(pid) => pid,
        Err(e) => {
            let _ = fs::remove_dir(&sdir); // (vazia: nada foi gravado)
            return Err(e);
        }
    };
    let s = Session {
        key: g.key.into(),
        pid,
        game: g.name,
        dir: sdir.clone(),
        runtime_dir: rt,
        game_dir: dir,
        started: Instant::now(),
        started_wall: SystemTime::now(),
        started_text: now.format("%Y-%m-%d %H:%M:%S").to_string(),
        exit: None,
        stopped: false,
        #[cfg(windows)]
        capture: None,
        video_started: None,
        video_len: None,
        video_error: None,
        files: vec![],
        video_only,
    };
    let st = status_of(&s);
    *SESSION.lock().unwrap() = Some(s);
    #[cfg(windows)]
    std::thread::spawn(move || video_thread(pid, sdir));
    #[cfg(not(windows))]
    let _ = pid;
    Ok(st)
}

/// "Parar": para o vídeo e já junta logs e info (o jogo segue aberto; ao fechar, a cópia é refeita)
#[tauri::command]
pub(crate) fn record_stop() -> Option<RecordStatus> {
    let mut g = SESSION.lock().unwrap();
    let s = g.as_mut()?;
    if !s.stopped {
        stop_video(s);
        collect(s);
    }
    Some(status_of(s))
}

/// A gravação atual ou a última (null: nenhuma desde que o hub abriu)
#[tauri::command]
pub(crate) fn record_status() -> Option<RecordStatus> {
    SESSION.lock().unwrap().as_ref().map(status_of)
}

/// A anotação do jogador, em notas.txt da última gravação
#[tauri::command]
pub(crate) fn record_note(text: String) -> Result<String, String> {
    let g = SESSION.lock().unwrap();
    let s = g.as_ref().ok_or("Nenhuma gravação para anotar.")?;
    let p = s.dir.join("notas.txt");
    fs::write(&p, text.replace("\r\n", "\n").replace('\n', "\r\n")).map_err(|e| format!("não salvou {}: {e}", p.display()))?;
    Ok(p.to_string_lossy().into())
}

/// A pasta das gravações (criada se faltar), para o "Abrir pasta" sem sessão
#[tauri::command]
pub(crate) fn record_base() -> String {
    let b = base_dir(&get_config().record_dir);
    let _ = fs::create_dir_all(&b);
    b.to_string_lossy().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminhos_da_sessao() {
        assert_eq!(base_dir("  "), default_base());
        assert!(default_base().ends_with(Path::new("SwitchOS").join("Gravações")));
        assert_eq!(base_dir(r" D:\Rec "), PathBuf::from(r"D:\Rec"));
        let b = std::env::temp_dir().join(format!("switchos-hub-rec-{}", std::process::id()));
        let _ = fs::remove_dir_all(&b);
        let p = session_dir(&b, "arceus", "20261119-164012");
        assert_eq!(p, b.join("arceus-20261119-164012"));
        fs::create_dir_all(&p).unwrap();
        assert_eq!(session_dir(&b, "arceus", "20261119-164012"), b.join("arceus-20261119-164012-2"));
        fs::remove_dir_all(&b).unwrap();
    }

    #[test]
    fn ini_e_duracao() {
        let ini = "[x]\nfps60 = 1 ; 60 quadros\nfps60_lock = 0\nscale=2\n";
        assert_eq!(ini_value(ini, "fps60"), Some("1"));
        assert_eq!(ini_value(ini, "fps60_lock"), Some("0"));
        assert_eq!(ini_value(ini, "scale"), Some("2"));
        assert_eq!(ini_value(ini, "nada"), None);
        assert_eq!(fmt_duration(Duration::from_secs(42)), "42 s");
        assert_eq!(fmt_duration(Duration::from_secs(65)), "1 min 05 s");
        assert_eq!(fmt_duration(Duration::from_secs(3725)), "1 h 02 min 05 s");
    }

    #[test]
    fn info_txt() {
        let t = info_text(&Info {
            game: "Pokémon Legends: Arceus",
            key: "arceus",
            started: "2026-11-19 16:40:12",
            ended: Some("2026-11-19 16:45:00"),
            duration: Duration::from_secs(288),
            video: Some(Duration::from_secs(280)),
            game_version: Some("1.1.1"),
            runtime_gamelib: Some("965d7a15765fc205"),
            game_gamelib: None,
            fps60: Some(true),
            exit_code: Some(Some(0)),
            video_note: None,
        });
        assert!(t.contains("jogo: Pokémon Legends: Arceus (arceus)\n"));
        assert!(t.contains("versão do jogo: 1.1.1\n"));
        assert!(t.contains("gamelib do runtime: 965d7a15765fc205\n"));
        assert!(t.contains("gamelib do código do jogo: ?\n"));
        assert!(t.contains("fps60: ligado\n"));
        assert!(t.contains("duração: 4 min 48 s\n"));
        assert!(t.contains("vídeo: 4 min 40 s (video.mp4)\n"));
        assert!(t.contains("saída do jogo: 0\n"));
        let t = info_text(&Info { ended: None, video: None, fps60: Some(false), exit_code: None, video_note: Some("a janela do jogo não apareceu"), ..Info {
            game: "g", key: "za", started: "s", ended: None, duration: Duration::ZERO, video: None, game_version: None,
            runtime_gamelib: None, game_gamelib: None, fps60: None, exit_code: None, video_note: None } });
        assert!(t.contains("fps60: desligado\n"));
        assert!(t.contains("fim: o jogo ainda estava aberto"));
        assert!(t.contains("vídeo: não gravado: a janela do jogo não apareceu\n"));
        assert!(!t.contains("saída do jogo"));
    }

    /// O vídeo de verdade, numa janela de teste própria fora da tela (PowerShell/WinForms em -4000,-4000): grava 4 s
    /// e confere o MP4 (ffprobe, se houver). `cargo test --lib -- --ignored captura_de_janela`
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn captura_de_janela() {
        use std::process::Command;
        let out = std::env::temp_dir().join(format!("switchos-hub-cap-{}.mp4", std::process::id()));
        let _ = fs::remove_file(&out);
        let ps = "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object Windows.Forms.Form; $f.Text = 'Teste de captura — SwitchOS'; \
                  $f.StartPosition = 'Manual'; $f.Location = New-Object Drawing.Point(-4000,-4000); $f.Size = New-Object Drawing.Size(640,360); \
                  $f.ShowInTaskbar = $false; $t = New-Object Windows.Forms.Timer; $t.Interval = 16; $n = 0; \
                  $t.Add_Tick({ $script:n++; $f.BackColor = [Drawing.Color]::FromArgb(($script:n*7)%256, ($script:n*3)%256, 128); $f.Text = \"Teste de captura — SwitchOS $script:n\" }); \
                  $t.Start(); $f.Add_Shown({ $f.Location = New-Object Drawing.Point(-4000,-4000) }); [Windows.Forms.Application]::Run($f)";
        let mut child = Command::new("powershell").args(["-NoProfile", "-Command", ps]).spawn().unwrap();
        let pid = child.id();
        let w = (0..40).find_map(|_| {
            std::thread::sleep(Duration::from_millis(250));
            video::find_window(pid, "SwitchOS")
        });
        let r = (|| {
            let c = video::start(w.ok_or("a janela de teste não apareceu")?, &out)?;
            std::thread::sleep(Duration::from_secs(4));
            let n = c.frames();
            c.stop()?;
            Ok::<u64, String>(n)
        })();
        let _ = child.kill();
        let _ = child.wait();
        let frames = r.unwrap();
        eprintln!("quadros enviados: {frames}; arquivo: {} ({} bytes)", out.display(), fs::metadata(&out).map(|m| m.len()).unwrap_or(0));
        assert!(frames >= 60, "poucos quadros: {frames}");
        assert!(fs::metadata(&out).unwrap().len() > 1000);
        if let Ok(p) = Command::new("ffprobe").args(["-v", "error", "-show_entries", "stream=codec_name,width,height,nb_frames,avg_frame_rate", "-of", "default=nw=1"]).arg(&out).output() {
            let s = String::from_utf8_lossy(&p.stdout);
            eprintln!("ffprobe:\n{s}");
            assert!(s.contains("codec_name=h264"));
        }
        let _ = fs::remove_file(&out);
    }

    /// Uma sessão de verdade, escondida (LARECOMP_HIDDEN=1: sem janela, logo sem vídeo), numa pasta de dados de
    /// trabalho (LARECOMP_DATA_ROOT) que nunca é a do jogador: entrada gravada (LARECOMP_INPUT roteirizado), Parar com
    /// o jogo aberto, e a cópia de logs/info de novo quando o jogo fecha.
    /// SWITCHOS_REC_EXE=<pasta do lasrecomp.exe> SWITCHOS_REC_GAME=<pasta do jogo> SWITCHOS_REC_DATA=<pasta de trabalho>
    /// SWITCHOS_REC_OUT=<saída> cargo test --lib -- --ignored sessao_escondida --nocapture
    #[test]
    #[ignore]
    fn sessao_escondida() {
        let var = |k: &str| PathBuf::from(std::env::var(k).unwrap_or_else(|_| panic!("falta {k}")));
        let (exe, game, data, out) = (var("SWITCHOS_REC_EXE"), var("SWITCHOS_REC_GAME"), var("SWITCHOS_REC_DATA"), var("SWITCHOS_REC_OUT"));
        let g = game_def("arceus").unwrap();
        let input = "120:A:20,400:A:20,600:STICK_UP:120,900:B:10";
        let extra: [(&str, &std::ffi::OsStr); 4] = [
            ("LARECOMP_HIDDEN", "1".as_ref()),
            ("LARECOMP_NO_LAUNCHER", "1".as_ref()),
            ("LARECOMP_DATA_ROOT", data.as_os_str()),
            ("LARECOMP_INPUT", input.as_ref()),
        ];
        let st = start_session(g, &exe, &data, &game, &out, false, &extra).unwrap();
        let dir = PathBuf::from(&st.dir);
        let pid = SESSION.lock().unwrap().as_ref().unwrap().pid;
        let secs: u64 = std::env::var("SWITCHOS_REC_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
        std::thread::sleep(Duration::from_secs(secs));
        let st = record_stop().unwrap();
        eprintln!("Parar: {:?} vídeo: {:?}", st.files, st.video_error);
        assert!(st.done && st.game_open);
        assert!(dir.join("info.txt").is_file() && dir.join("play.log").is_file() && dir.join("larecomp.ini").is_file());
        // fecha o jogo de teste (só ele, pelo PID)
        let _ = std::process::Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output();
        for _ in 0..40 {
            if record_status().is_some_and(|s| !s.game_open) {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let st = record_status().unwrap();
        eprintln!("fechou: {:?}\n{}", st.files, fs::read_to_string(dir.join("info.txt")).unwrap());
        assert!(!st.game_open);
        let rec = fs::read_to_string(dir.join("input.rec")).unwrap_or_default();
        eprintln!("input.rec: {rec}");
        assert!(rec.contains(":A:"), "a entrada não foi gravada");
    }
}
