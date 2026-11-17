#!/usr/bin/env python3
"""SwitchOS - launcher unificado + sincronização de saves PC <-> Android.

Modelo: a "nuvem" (pasta sincronizada: Drive/OneDrive/Syncthing) guarda a versão
canônica de cada save, por Title ID. PC e Android (via ADB, app nx2androidX) são
réplicas. A sincronização é de 3 vias: compara o hash de cada lado com o último
hash sincronizado (state.json) para saber quem mudou; se os dois mudaram, vence o
mais recente e o perdedor vai para backups/ (nada é sobrescrito sem cópia).

Uso:
  python switchos.py list
  python switchos.py status
  python switchos.py sync [jogo] [--no-android] [--dry-run]
  python switchos.py play <jogo>          # sync -> joga -> sync
Requer Python 3.11+. ADB opcional (platform-tools no PATH).
"""
from __future__ import annotations

import argparse, hashlib, io, json, os, shutil, subprocess, sys, tarfile, time, tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def expand(p: str) -> Path | None:
    if not p:
        return None
    path = Path(os.path.expanduser(os.path.expandvars(p)))
    return path if path.is_absolute() else ROOT / path


@dataclass
class Game:
    key: str
    name: str
    title_id: str
    pc_save: Path | None
    pc_exe: Path | None
    pc_env: dict
    untrusted: bool
    pc_args: list


def load_config():
    with open(ROOT / "games.toml", "rb") as f:
        cfg = tomllib.load(f)
    games = {
        k: Game(k, g["name"], g["title_id"].upper(), expand(g.get("pc_save", "")),
                expand(g.get("pc_exe", "")), g.get("pc_env", {}), g.get("untrusted_binaries", False),
                g.get("pc_args", []))
        for k, g in cfg["games"].items()
    }
    return cfg["sync"], games


# ---------- snapshot helpers ----------

def tree_hash(d: Path | None) -> str | None:
    if not d or not d.is_dir() or not any(d.rglob("*")):
        return None
    h = hashlib.sha256()
    for f in sorted(p for p in d.rglob("*") if p.is_file()):
        h.update(f.relative_to(d).as_posix().encode())
        h.update(f.read_bytes())
    return h.hexdigest()


def tree_mtime(d: Path) -> float:
    return max((p.stat().st_mtime for p in d.rglob("*") if p.is_file()), default=0)


def replace_tree(src: Path, dst: Path) -> None:
    """Copia src para dst de forma atômica (escreve ao lado e troca)."""
    tmp = dst.with_name(dst.name + ".switchos-tmp")
    shutil.rmtree(tmp, ignore_errors=True)
    shutil.copytree(src, tmp)
    if dst.exists():
        old = dst.with_name(dst.name + ".switchos-old")
        shutil.rmtree(old, ignore_errors=True)
        dst.rename(old)
        tmp.rename(dst)
        shutil.rmtree(old, ignore_errors=True)
    else:
        dst.parent.mkdir(parents=True, exist_ok=True)
        tmp.rename(dst)


def backup(src: Path, cloud: Path, game: Game, origin: str, keep: int) -> None:
    bdir = cloud / "backups" / game.title_id
    bdir.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime('%Y%m%d-%H%M%S')
    dst, i = bdir / f"{stamp}-{origin}", 1
    while dst.exists():
        dst, i = bdir / f"{stamp}-{origin}-{i}", i + 1
    shutil.copytree(src, dst)
    olds = sorted(bdir.iterdir())
    for o in olds[:-keep]:
        shutil.rmtree(o, ignore_errors=True)


# ---------- Android (ADB) ----------

class Android:
    def __init__(self, package: str):
        self.pkg = package
        self.adb = shutil.which("adb")

    def available(self) -> bool:
        if not self.adb:
            return False
        out = subprocess.run([self.adb, "get-state"], capture_output=True, text=True)
        return out.stdout.strip() == "device"

    def _run_as(self, cmd: str, data: bytes | None = None) -> bytes:
        r = subprocess.run([self.adb, "exec-out", "run-as", self.pkg, "sh", "-c", cmd],
                           input=data, capture_output=True)
        if r.returncode != 0:
            raise RuntimeError(f"adb run-as falhou ({r.stderr.decode(errors='ignore').strip()}). "
                               "O APK precisa ser debuggable; senão use Exportar/Importar save no app.")
        return r.stdout

    def pull(self, title_id: str, dest: Path) -> bool:
        raw = self._run_as(f"cd files/save 2>/dev/null && [ -d {title_id} ] && tar -cf - -C {title_id} . || true")
        if not raw:
            return False
        shutil.rmtree(dest, ignore_errors=True)
        dest.mkdir(parents=True)
        with tarfile.open(fileobj=io.BytesIO(raw)) as t:
            t.extractall(dest, filter="data")
        return True

    def push(self, title_id: str, src: Path) -> None:
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode="w") as t:
            t.add(src, arcname=".")
        d = f"files/save/{title_id}"
        self._run_as(f"rm -rf {d}.new && mkdir -p {d}.new && tar -xf - -C {d}.new && rm -rf {d} && mv {d}.new {d}",
                     buf.getvalue())


# ---------- sync engine ----------

def sync_game(game: Game, scfg: dict, android: Android | None, dry: bool) -> None:
    cloud_root = expand(scfg["cloud_dir"])
    keep = int(scfg.get("keep_backups", 10))
    cloud = cloud_root / "saves" / game.title_id
    state_f = cloud_root / "state.json"
    state = json.loads(state_f.read_text()) if state_f.exists() else {}
    last = state.get(game.title_id, {})

    replicas: dict[str, Path] = {}
    if game.pc_save:
        replicas["pc"] = game.pc_save
    tmp_android = cloud_root / ".android-pull" / game.title_id
    if android:
        try:
            if android.pull(game.title_id, tmp_android):
                replicas["android"] = tmp_android
            else:
                replicas["android"] = tmp_android
                shutil.rmtree(tmp_android, ignore_errors=True)
        except RuntimeError as e:
            print(f"  [android] {e}")

    # Quem mudou desde a última sync?
    cloud_h = tree_hash(cloud)
    changed = {n: p for n, p in replicas.items()
               if (h := tree_hash(p)) and h != last.get(n) and h != cloud_h}
    if cloud_h and cloud_h != last.get("cloud"):
        changed.setdefault("cloud", cloud)  # outro dispositivo atualizou a nuvem

    if len(changed) > 1:
        winner = max(changed, key=lambda n: tree_mtime(changed[n]))
        print(f"  CONFLITO entre {sorted(changed)} -> vence '{winner}' (mais recente); demais vão para backups/")
        for n, p in changed.items():
            if n != winner and not dry:
                backup(p, cloud_root, game, n, keep)
        source = changed[winner]
    elif changed:
        winner, source = next(iter(changed.items()))
    elif cloud_h:
        winner, source = "cloud", cloud
    else:
        print("  nenhum save encontrado em nenhum lado")
        return

    print(f"  origem: {winner}")
    if dry:
        return
    if winner != "cloud":
        if cloud.exists() and "cloud" not in changed:  # se mudou, já foi para backups/ no conflito
            backup(cloud, cloud_root, game, "cloud", keep)
        replace_tree(source, cloud)
    final = tree_hash(cloud)
    for n, p in replicas.items():
        if tree_hash(p) == final:
            continue
        print(f"  -> atualizando {n}")
        if n == "android":
            android.push(game.title_id, cloud)
        else:
            if p.exists():
                backup(p, cloud_root, game, n, keep)
            replace_tree(cloud, p)
    state[game.title_id] = {"cloud": final, **{n: final for n in replicas}, "at": time.time()}
    state_f.write_text(json.dumps(state, indent=2))
    shutil.rmtree(tmp_android, ignore_errors=True)


# ---------- CLI ----------

def main() -> int:
    ap = argparse.ArgumentParser(description="SwitchOS launcher + save sync")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("list")
    sub.add_parser("status")
    s = sub.add_parser("sync"); s.add_argument("game", nargs="?")
    s.add_argument("--no-android", action="store_true"); s.add_argument("--dry-run", action="store_true")
    p = sub.add_parser("play"); p.add_argument("game"); p.add_argument("--no-android", action="store_true")
    args = ap.parse_args()

    scfg, games = load_config()
    if args.cmd in ("list", "status"):
        for g in games.values():
            line = f"{g.key:8} {g.title_id}  {g.name}"
            if args.cmd == "status":
                pc = "save OK" if tree_hash(g.pc_save) else "sem save"
                exe = "exe OK" if g.pc_exe and g.pc_exe.is_file() else "não compilado"
                line += f"  | PC: {exe}, {pc}" + ("  [!] binários não verificados" if g.untrusted else "")
            print(line)
        return 0

    sel = [games[args.game]] if getattr(args, "game", None) else list(games.values())
    android = None if args.no_android else Android(scfg.get("android_package", "com.nx.android"))
    if android and not android.available():
        print("[android] nenhum dispositivo ADB conectado - sincronizando só PC <-> nuvem")
        android = None

    if args.cmd == "sync":
        for g in sel:
            print(f"[{g.key}] {g.name}")
            sync_game(g, scfg, android, args.dry_run)
        return 0

    g = sel[0]
    if not g.pc_exe or not g.pc_exe.is_file():
        print(f"Executável de {g.name} não encontrado. Compile o projeto e ajuste pc_exe em games.toml.")
        return 1
    if g.untrusted and input("Este jogo usa binários pré-compilados não verificados. Continuar? [s/N] ").lower() != "s":
        return 1
    sync_game(g, scfg, android, False)
    subprocess.run([str(g.pc_exe), *g.pc_args], cwd=g.pc_exe.parent, env={**os.environ, **g.pc_env})
    sync_game(g, scfg, android, False)
    return 0


if __name__ == "__main__":
    sys.exit(main())
