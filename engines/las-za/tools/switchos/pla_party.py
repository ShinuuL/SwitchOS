# Protótipo: o time do save do Pokémon Legends: Arceus (SwishCrypto como o Z-A; bloco e formato PA8 próprios).
import struct, sys

PAD = bytes([
    0xA0, 0x92, 0xD1, 0x06, 0x07, 0xDB, 0x32, 0xA1, 0xAE, 0x01, 0xF5, 0xC5, 0x1E, 0x84, 0x4F, 0xE3, 0x53, 0xCA, 0x37, 0xF4,
    0xA7, 0xB0, 0x4D, 0xA0, 0x18, 0xB7, 0xC2, 0x97, 0xDA, 0x5F, 0x53, 0x2B, 0x75, 0xFA, 0x48, 0x16, 0xF8, 0xD4, 0x8A, 0x6F,
    0x61, 0x05, 0xF4, 0xE2, 0xFD, 0x04, 0xB5, 0xA3, 0x0F, 0xFC, 0x44, 0x92, 0xCB, 0x32, 0xE6, 0x1B, 0xB9, 0xB1, 0x2E, 0x01,
    0xB0, 0x56, 0x53, 0x36, 0xD2, 0xD1, 0x50, 0x3D, 0xDE, 0x5B, 0x2E, 0x0E, 0x52, 0xFD, 0xDF, 0x2F, 0x7B, 0xCA, 0x63, 0x50,
    0xA4, 0x67, 0x5D, 0x23, 0x17, 0xC0, 0x52, 0xE1, 0xA6, 0x30, 0x7C, 0x2B, 0xB6, 0x70, 0x36, 0x5B, 0x2A, 0x27, 0x69, 0x33,
    0xF5, 0x63, 0x7B, 0x36, 0x3F, 0x26, 0x9B, 0xA3, 0xED, 0x7A, 0x53, 0x00, 0xA4, 0x48, 0xB3, 0x50, 0x9E, 0x14, 0xA0, 0x52,
    0xDE, 0x7E, 0x10, 0x2B, 0x1B, 0x77, 0x6E])
WIDTH = [0, 0, 0, 1, 0, 0, 0, 0, 1, 2, 4, 8, 1, 2, 4, 8, 4, 8]


class Xs:
    def __init__(self, key):
        self.s, self.c = key, 0
        for _ in range(bin(key).count('1')):
            self.s = self.adv(self.s)

    @staticmethod
    def adv(s):
        s ^= (s << 2) & 0xFFFFFFFF; s ^= s >> 15; s ^= (s << 13) & 0xFFFFFFFF
        return s & 0xFFFFFFFF

    def next(self):
        r = (self.s >> (self.c * 8)) & 0xFF
        if self.c == 3: self.s, self.c = self.adv(self.s), 0
        else: self.c += 1
        return r

    def next32(self):
        return self.next() | self.next() << 8 | self.next() << 16 | self.next() << 24


def blocks(p):
    p = bytearray(p); n = len(p) - 32
    for i in range(n): p[i] ^= PAD[i % 0x7F]
    o, out = 0, {}
    while o + 5 <= n:
        key = struct.unpack_from('<I', p, o)[0]; o += 4
        x = Xs(key); t = p[o] ^ x.next(); o += 1
        if t in (1, 2): continue
        if t == 4: size = struct.unpack_from('<I', p, o)[0] ^ x.next32(); o += 4
        elif t == 5:
            cnt = struct.unpack_from('<I', p, o)[0] ^ x.next32(); sub = p[o + 4] ^ x.next(); o += 5
            size = cnt * WIDTH[sub]
        elif t < 18 and WIDTH[t]: size = WIDTH[t]
        else: break
        out[key] = bytes(p[o + i] ^ x.next() for i in range(size)); o += size
    return out


POS = [0,1,2,3, 0,1,3,2, 0,2,1,3, 0,3,1,2, 0,2,3,1, 0,3,2,1, 1,0,2,3, 1,0,3,2, 2,0,1,3, 3,0,1,2, 2,0,3,1, 3,0,2,1,
       1,2,0,3, 1,3,0,2, 2,1,0,3, 3,1,0,2, 2,3,0,1, 3,2,0,1, 1,2,3,0, 1,3,2,0, 2,1,3,0, 3,1,2,0, 2,3,1,0, 3,2,1,0]


def decrypt(d, enc_end, block):  # PA8: 4 blocks of `block` bytes after the 8-byte header, then the party stats
    d = bytearray(d); ec = struct.unpack_from('<I', d, 0)[0]; seed = ec
    for i in range(8, len(d) - 1, 2):
        if i == enc_end: seed = ec
        seed = (seed * 0x41C64E6D + 0x6073) & 0xFFFFFFFF
        v = struct.unpack_from('<H', d, i)[0] ^ (seed >> 16); struct.pack_into('<H', d, i, v)
    sv = ((ec >> 13) & 31) % 24; src = bytes(d[8:8 + 4 * block])
    for b in range(4):
        d[8 + block * b: 8 + block * (b + 1)] = src[block * POS[sv * 4 + b]: block * (POS[sv * 4 + b] + 1)]
    return bytes(d)


bl = blocks(open(sys.argv[1], 'rb').read())
print(len(bl), 'blocks')
key = int(sys.argv[2], 16) if len(sys.argv) > 2 else 0x2985FE5D
party = bl.get(key)
print('party block', hex(key), len(party) if party else None)
if party:
    for slot in (0x178, 0x168):
        print('-- slot size', hex(slot))
        for i in range(6):
            raw = party[i * slot:(i + 1) * slot]
            if len(raw) < 0x178 or not struct.unpack_from('<I', raw, 0)[0]: continue
            d = decrypt(raw[:0x178], 0x168, 0x58)
            sp = struct.unpack_from('<H', d, 8)[0]
            name = d[0x60:0x60 + 26].decode('utf-16-le', 'ignore').split('\x00')[0]
            print(i, 'species', sp, 'form', d[0x24], 'lvl', d[0x168], 'name', repr(name))
