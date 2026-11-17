#!/usr/bin/env python3
"""SwitchOS: traduz endereços do guest (linhas "guest #n <addr>" do log, ou argumentos) para módulo + símbolo.

Usa o mesmo layout do runtime (main em 0x7000000000, sdk em 0x7008000000, subsdks logo após o main: gamesub.c).
Rode com o python do gen-venv (precisa de lz4):
  build/gen-venv/Scripts/python.exe tools/switchos/symbolize.py <exefs> <log ou endereços hex...>
"""
import bisect
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from nso import Nso  # noqa: E402

MAIN_BASE, SDK_BASE, MOD_ALIGN = 0x7000000000, 0x7008000000, 0x200000


def align(v, a):
    return (v + a - 1) & ~(a - 1)


def image_size(path):
    h = open(path, 'rb').read(0x40)
    data_mem, data_size = int.from_bytes(h[0x34:0x38], 'little'), int.from_bytes(h[0x38:0x3C], 'little')
    bss = int.from_bytes(h[0x3C:0x40], 'little')
    return (data_mem + data_size + bss + 0x3FFF) & ~0x3FFF


def layout(exefs):
    mods = [('main', MAIN_BASE), ('sdk', SDK_BASE)]
    at = align(MAIN_BASE + image_size(os.path.join(exefs, 'main')), MOD_ALIGN)
    for k in range(4):
        p = os.path.join(exefs, f'subsdk{k}')
        if os.path.exists(p):
            mods.append((f'subsdk{k}', at))
            at = align(at + image_size(p), MOD_ALIGN)
    out = []
    for name, base in mods:
        n = Nso(os.path.join(exefs, name))
        syms = sorted((v, s) for s, info, sh, v, sz in n.syms if sh and v)
        out.append((name, base, image_size(os.path.join(exefs, name)), [v for v, _ in syms], [s for _, s in syms]))
    return out


def where(mods, a):
    for name, base, size, addrs, names in mods:
        if base <= a < base + size:
            off = a - base
            i = bisect.bisect_right(addrs, off) - 1
            sym = f'{names[i]}+{off - addrs[i]:#x}' if i >= 0 else '?'
            return f'{name}+{off:#x}  {sym}'
    return 'fora dos módulos (HLE/heap/stack)'


def main():
    exefs, rest = sys.argv[1], sys.argv[2:]
    mods = layout(exefs)
    addrs = []
    for r in rest:
        if os.path.exists(r):
            addrs += [int(m, 16) for m in re.findall(r'guest #\d+ ([0-9a-f]+)', open(r, errors='ignore').read())]
        else:
            addrs.append(int(r, 16))
    for a in addrs:
        print(f'{a:x}  {where(mods, a)}')


if __name__ == '__main__':
    main()
