#!/usr/bin/env python3
"""Traz atualizações de um projeto upstream para o nosso fork, de forma controlada.

Fluxo: baixa o upstream, gera o diff entre o commit travado em upstream.lock e o novo
commit, e aplica esse diff sobre o nosso código com merge de 3 vias (git apply -3).
Conflitos ficam marcados nos arquivos para revisão. O lock só avança com --commit-lock.

  python tools/update_upstream.py status                 # mostra o que há de novo em cada upstream
  python tools/update_upstream.py pull las-za            # aplica até o HEAD do upstream
  python tools/update_upstream.py pull las-za --to <sha> # aplica até um commit específico
"""
from __future__ import annotations

import argparse, json, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCK = ROOT / "upstream.lock"


def git(*args, cwd=ROOT, check=True, input=None) -> str:
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, input=input)
    if check and r.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} falhou:\n{r.stderr.decode(errors='ignore')}")
    return r.stdout.decode(errors="ignore")


def fetch(url: str, tmp: Path) -> Path:
    repo = tmp / "up"
    git("clone", "--filter=blob:none", "--no-checkout", url, str(repo), cwd=tmp)
    return repo


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("status")
    p = sub.add_parser("pull"); p.add_argument("name"); p.add_argument("--to")
    p.add_argument("--commit-lock", action="store_true", help="avança o lock após revisar o merge")
    args = ap.parse_args()
    lock = json.loads(LOCK.read_text())

    if args.cmd == "status":
        for name, e in lock.items():
            head = git("ls-remote", e["url"], "HEAD").split()[0]
            mark = "em dia" if head == e["commit"] else f"NOVO -> {head[:10]}"
            print(f"{name:12} {e['commit'][:10]}  {mark}")
        return 0

    e = lock[args.name]
    with tempfile.TemporaryDirectory() as t:
        up = fetch(e["url"], Path(t))
        new = args.to or git("rev-parse", "HEAD", cwd=up).strip()
        if new == e["commit"]:
            print("Já está no commit mais recente.")
            return 0
        print(git("log", "--oneline", f"{e['commit']}..{new}", cwd=up))
        patch = git("diff", "--binary", e["commit"], new, cwd=up).encode()
        r = subprocess.run(["git", "apply", "-3", f"--directory={e['path']}", "-"], cwd=ROOT,
                           input=patch, capture_output=True)
        print(r.stderr.decode(errors="ignore"))
        if r.returncode != 0:
            print("Há conflitos: resolva os arquivos marcados e rode de novo com --commit-lock.")
        if args.commit_lock or r.returncode == 0:
            e["commit"] = new
            LOCK.write_text(json.dumps(lock, indent=2) + "\n")
            print(f"Lock de {args.name} avançado para {new[:10]}. Teste antes de commitar.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
