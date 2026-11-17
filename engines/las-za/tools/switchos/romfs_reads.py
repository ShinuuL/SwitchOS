#!/usr/bin/env python3
"""SwitchOS: traduz o log de leituras (LARECOMP_LOG_READS: "? <offset hex> <tamanho hex> ...") para os arquivos do RomFS.

Uso: python -I tools/switchos/romfs_reads.py <romfs.bin> <log de leituras> [filtro]
Lê o índice do RomFS (cabeçalho nível 3: tabelas de diretórios e de arquivos) e lista, na ordem da primeira leitura,
cada arquivo lido, com quantas leituras teve. "filtro" mostra só caminhos que o contêm (ex.: message).
"""
import bisect
import struct
import sys


def index(path):
    with open(path, 'rb') as f:
        h = struct.unpack('<10Q', f.read(0x50))
        dir_meta_off, dir_meta_size = h[3], h[4]
        file_meta_off, file_meta_size = h[7], h[8]
        data_off = h[9]
        f.seek(dir_meta_off)
        dmeta = f.read(dir_meta_size)
        f.seek(file_meta_off)
        fmeta = f.read(file_meta_size)

    def dir_path(off):
        parts = []
        while True:
            parent, _sib, _child, _cfile, _hash, nlen = struct.unpack_from('<6I', dmeta, off)
            name = dmeta[off + 0x18:off + 0x18 + nlen].decode('utf-8', 'replace')
            if off == 0:
                break
            parts.append(name)
            off = parent
        return '/'.join(reversed(parts))

    files, o = [], 0
    while o + 0x20 <= len(fmeta):
        parent, _sib, foff, fsize, _hash, nlen = struct.unpack_from('<IIQQII', fmeta, o)
        name = fmeta[o + 0x20:o + 0x20 + nlen].decode('utf-8', 'replace')
        d = dir_path(parent)
        files.append((data_off + foff, fsize, f'{d}/{name}' if d else name))
        o += (0x20 + nlen + 3) & ~3
    files.sort()
    return files


def main():
    files = index(sys.argv[1])
    starts = [f[0] for f in files]
    filt = sys.argv[3] if len(sys.argv) > 3 else ''
    seen, order = {}, []
    for line in open(sys.argv[2], errors='ignore'):
        p = line.split()
        if len(p) < 3:
            continue
        try:
            off = int(p[1], 16)
        except ValueError:
            continue
        i = bisect.bisect_right(starts, off) - 1
        if i < 0 or off >= files[i][0] + max(files[i][1], 1):
            name = '(fora de arquivos: cabeçalho/índice)'
        else:
            name = files[i][2]
        if name not in seen:
            order.append(name)
            seen[name] = 0
        seen[name] += 1
    shown = [n for n in order if filt in n]
    for n in shown:
        print(f'{seen[n]:6}  {n}')
    print(f'-- {len(shown)} arquivos ({len(order)} no total, {len(files)} no RomFS)')


if __name__ == '__main__':
    main()
