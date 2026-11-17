#!/bin/bash
# SwitchOS: boot de teste invisível (LARECOMP_HIDDEN: sem janela nem caixas de diálogo; tudo vai para o log).
# Uso: tools/switchos/boot_test.sh <pasta do jogo> [segundos=60] [log]
# Encerra só o processo que ele mesmo abriu. Imprime como terminou e as linhas que importam para a depuração.
set -u
GAME="${1:?pasta do jogo (com exefs/ e lasgame.dll)}"
SECS="${2:-60}"
LOG="${3:-$(dirname "$GAME")/$(basename "$GAME")-boot.log}"
HUB="$(cd "$(dirname "$0")/../.." && pwd)/build/dist-za/ZA Recomp"

cd "$HUB" || exit 1
LARECOMP_HIDDEN=1 LARECOMP_NO_LAUNCHER=1 ./lasrecomp.exe --play "$GAME" > "$LOG" 2>&1 &
pid=$!
for ((i = 1; i <= SECS; i++)); do
    kill -0 $pid 2>/dev/null || break
    sleep 1
done
if kill -0 $pid 2>/dev/null; then
    echo "== ainda rodando após ${SECS}s (passou do ponto de parada anterior): encerrando"
    kill $pid
    wait $pid 2>/dev/null
else
    wait $pid
    echo "== saiu sozinho após ${i}s (exit $?)"
fi
echo "== log: $LOG"
grep -nE "FATAL|abort on result|\[crash\]|unhandled|no HLE implementation|unimplemented|unknown (svc|cmd|service)" "$LOG" | head -20
echo "== últimas linhas antes do fim"
n=$(grep -nE "abort on result|FATAL" "$LOG" | head -1 | cut -d: -f1)
if [ -n "$n" ]; then sed -n "$((n > 30 ? n - 30 : 1)),${n}p" "$LOG"; else grep -vE "^\[fps\]" "$LOG" | tail -30; fi
