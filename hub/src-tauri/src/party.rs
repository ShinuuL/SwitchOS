//! O time do save dos Pokémon Legends (Z-A e Arceus). Z-A - o mesmo leitor da tela inicial do ZA Recomp (engines/las-za,
//! runtime/ui/launcher.mm: party_block / party_decrypt / party_model), portado para o hub. Só lê o save.
//!
//! save/save_0/main: XOR com um pad de 0x7F bytes (os últimos 32 bytes são um SHA-256), depois blocos chave u32 + tipo
//! (+ tamanho) + dados, cada um XOR com um xorshift32 semeado pela chave. O time é o bloco 0x3AA1A9AD: 6 slots de 0x1E0,
//! cada um um Pokémon (0x148 bytes cifrados por LCG a partir da encryption constant e embaralhados em 4 blocos de 0x50,
//! depois os stats do time; nível em 0x148).

use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct PartyMon {
    pub name: String,
    pub species: u32,
    pub form: u32,
    pub level: u32,
    pub shiny: bool,
    pub egg: bool,
    /// ícone do próprio jogo (cache/party_icons, feito pelo zaprompts a partir do romfs.bin do jogador) como data URL
    pub icon: Option<String>,
}

const PAD: [u8; 0x7F] = [
    0xA0, 0x92, 0xD1, 0x06, 0x07, 0xDB, 0x32, 0xA1, 0xAE, 0x01, 0xF5, 0xC5, 0x1E, 0x84, 0x4F, 0xE3, 0x53, 0xCA, 0x37, 0xF4,
    0xA7, 0xB0, 0x4D, 0xA0, 0x18, 0xB7, 0xC2, 0x97, 0xDA, 0x5F, 0x53, 0x2B, 0x75, 0xFA, 0x48, 0x16, 0xF8, 0xD4, 0x8A, 0x6F,
    0x61, 0x05, 0xF4, 0xE2, 0xFD, 0x04, 0xB5, 0xA3, 0x0F, 0xFC, 0x44, 0x92, 0xCB, 0x32, 0xE6, 0x1B, 0xB9, 0xB1, 0x2E, 0x01,
    0xB0, 0x56, 0x53, 0x36, 0xD2, 0xD1, 0x50, 0x3D, 0xDE, 0x5B, 0x2E, 0x0E, 0x52, 0xFD, 0xDF, 0x2F, 0x7B, 0xCA, 0x63, 0x50,
    0xA4, 0x67, 0x5D, 0x23, 0x17, 0xC0, 0x52, 0xE1, 0xA6, 0x30, 0x7C, 0x2B, 0xB6, 0x70, 0x36, 0x5B, 0x2A, 0x27, 0x69, 0x33,
    0xF5, 0x63, 0x7B, 0x36, 0x3F, 0x26, 0x9B, 0xA3, 0xED, 0x7A, 0x53, 0x00, 0xA4, 0x48, 0xB3, 0x50, 0x9E, 0x14, 0xA0, 0x52,
    0xDE, 0x7E, 0x10, 0x2B, 0x1B, 0x77, 0x6E,
];

struct Xs {
    s: u32,
    c: u32,
}
impl Xs {
    fn adv(mut s: u32) -> u32 {
        s ^= s << 2;
        s ^= s >> 15;
        s ^= s << 13;
        s
    }
    fn new(key: u32) -> Self {
        let mut s = key;
        for _ in 0..key.count_ones() {
            s = Self::adv(s);
        }
        Xs { s, c: 0 }
    }
    fn next(&mut self) -> u8 {
        let r = (self.s >> (self.c * 8)) as u8;
        if self.c == 3 {
            self.s = Self::adv(self.s);
            self.c = 0;
        } else {
            self.c += 1;
        }
        r
    }
    fn next32(&mut self) -> u32 {
        let mut v = 0u32;
        for i in 0..4 {
            v |= (self.next() as u32) << (8 * i);
        }
        v
    }
}

fn rd32(p: &[u8]) -> u32 {
    u32::from_le_bytes([p[0], p[1], p[2], p[3]])
}

fn party_block(mut p: Vec<u8>, want: u32) -> Option<Vec<u8>> {
    const WIDTH: [u64; 18] = [0, 0, 0, 1, 0, 0, 0, 0, 1, 2, 4, 8, 1, 2, 4, 8, 4, 8];
    if p.len() < 64 {
        return None;
    }
    let n = p.len() - 32;
    for i in 0..n {
        p[i] ^= PAD[i % 0x7F];
    }
    let mut o = 0usize;
    while o + 5 <= n {
        let key = rd32(&p[o..]);
        o += 4;
        let mut x = Xs::new(key);
        let t = p[o] ^ x.next();
        o += 1;
        let size: u64 = match t {
            1 | 2 => continue,
            4 => {
                if o + 4 > n {
                    return None;
                }
                let s = (rd32(&p[o..]) ^ x.next32()) as u64;
                o += 4;
                s
            }
            5 => {
                if o + 5 > n {
                    return None;
                }
                let cnt = rd32(&p[o..]) ^ x.next32();
                let sub = p[o + 4] ^ x.next();
                o += 5;
                if sub >= 18 || WIDTH[sub as usize] == 0 {
                    return None;
                }
                cnt as u64 * WIDTH[sub as usize]
            }
            t if t < 18 && WIDTH[t as usize] != 0 => WIDTH[t as usize],
            _ => return None,
        };
        if o as u64 + size > n as u64 {
            return None;
        }
        let size = size as usize;
        if key == want {
            return Some((0..size).map(|i| p[o + i] ^ x.next()).collect());
        }
        o += size;
    }
    None
}

/// `enc_end`: fim da parte cifrada em blocos (depois vêm os stats do time); `blk`: tamanho de cada um dos 4 blocos
fn decrypt(d: &mut [u8], enc_end: usize, blk: usize) {
    const POS: [u8; 96] = [
        0, 1, 2, 3, 0, 1, 3, 2, 0, 2, 1, 3, 0, 3, 1, 2, 0, 2, 3, 1, 0, 3, 2, 1, 1, 0, 2, 3, 1, 0, 3, 2, 2, 0, 1, 3, 3, 0, 1, 2,
        2, 0, 3, 1, 3, 0, 2, 1, 1, 2, 0, 3, 1, 3, 0, 2, 2, 1, 0, 3, 3, 1, 0, 2, 2, 3, 0, 1, 3, 2, 0, 1, 1, 2, 3, 0, 1, 3, 2, 0,
        2, 1, 3, 0, 3, 1, 2, 0, 2, 3, 1, 0, 3, 2, 1, 0,
    ];
    let ec = rd32(&d[..]);
    let mut seed = ec;
    let mut i = 8;
    while i + 1 < d.len() {
        if i == enc_end {
            seed = ec;
        }
        seed = seed.wrapping_mul(0x41C64E6D).wrapping_add(0x6073);
        let v = u16::from_le_bytes([d[i], d[i + 1]]) ^ (seed >> 16) as u16;
        d[i..i + 2].copy_from_slice(&v.to_le_bytes());
        i += 2;
    }
    let src = d[8..8 + 4 * blk].to_vec();
    let sv = ((ec >> 13) & 31) as usize % 24;
    for b in 0..4 {
        let from = blk * POS[sv * 4 + b] as usize;
        d[8 + blk * b..8 + blk * (b + 1)].copy_from_slice(&src[from..from + blk]);
    }
}

/// número do modelo do ícone (Dex Nacional até a Gen 5; depois, o do próprio jogo)
fn model(species: u32) -> Option<u32> {
    const KALOS: [u16; 72] = [
        720, 721, 722, 717, 718, 719, 723, 724, 725, 711, 712, 753, 754, 755, 706, 707, 708, 704, 705, 713, 714, 715, 728, 729,
        752, 730, 701, 733, 734, 744, 745, 746, 758, 759, 742, 743, 726, 727, 747, 748, 709, 710, 756, 757, 731, 732, 737, 738,
        739, 740, 767, 761, 741, 766, 763, 764, 765, 760, 702, 703, 735, 736, 749, 751, 716, 762, 768, 769, 770, 772, 774, 773,
    ];
    const LATER: [(u32, u32); 59] = [
        (739, 859), (740, 860), (767, 866), (768, 867), (769, 822), (770, 823), (778, 819), (780, 856), (801, 882),
        (802, 883), (807, 888), (808, 891), (809, 892), (821, 918), (822, 919), (823, 920), (827, 901), (828, 902),
        (848, 921), (849, 922), (850, 923), (851, 923), (852, 964), (853, 965), (876, 942), (877, 941), (920, 1002),
        (926, 1074), (927, 1075), (931, 1064), (932, 1067), (933, 1068), (934, 1069), (935, 1107), (936, 1108),
        (937, 1109), (942, 1076), (943, 1077), (951, 1042), (952, 1043), (957, 1104), (958, 1105), (959, 1106),
        (965, 1072), (966, 1073), (967, 1057), (969, 1070), (970, 1071), (971, 1028), (972, 1029), (973, 1065),
        (977, 1035), (978, 1056), (979, 1114), (996, 1053), (997, 1054), (998, 1055), (999, 1080), (1000, 1081),
    ];
    match species {
        s if s < 650 => Some(s),
        s if s <= 721 => Some(KALOS[(s - 650) as usize] as u32),
        s => LATER.iter().find(|l| l.0 == s).map(|l| l.1),
    }
}

/// o ícone da forma, senão o da forma base; o shiny, senão o normal (como party_icon na tela inicial)
fn icon(icons: &Path, m: &PartyMon, dex: bool) -> Option<String> {
    let mdl = if m.egg { 0 } else if dex { m.species } else { model(m.species)? };
    let shinies: &[u32] = if m.shiny && !m.egg { &[1, 0] } else { &[0] };
    let forms = if m.egg { [0, 0, 0, 11] } else { [m.form, 11 + m.form, 0, 11] };
    for &sh in shinies {
        for f in forms {
            let p = icons.join(format!("pm{mdl:04}_{f:02}_00_00_{sh}.png"));
            if let Ok(b) = std::fs::read(&p) {
                use base64::Engine;
                return Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(b)));
            }
        }
    }
    None
}

/// formato do save de cada jogo
pub struct Format {
    block: u32,    // chave do bloco do time
    slot: usize,   // tamanho de cada slot no bloco
    len: usize,    // bytes do Pokémon (parte cifrada + stats do time)
    enc_end: usize, // fim dos 4 blocos embaralhados = offset do nível
    blk: usize,
    nick: usize,
    iv32: usize,   // bit 30 = ovo
    dex_icons: bool, // ícones pelo nº da Pokédex nacional (Arceus: prompt_textures.py pla_icons), não pelo modelo do Z-A
}

/// Pokémon Legends: Z-A (PA9): bloco 0x3AA1A9AD, 6 slots de 0x1E0, blocos de 0x50
pub const ZA: Format = Format { block: 0x3AA1A9AD, slot: 0x1E0, len: 0x158, enc_end: 0x148, blk: 0x50, nick: 0x58, iv32: 0x8C, dex_icons: false };
/// Pokémon Legends: Arceus (PA8): bloco 0x2985FE5D, 6 slots de 0x178, blocos de 0x58 (tools/switchos/pla_party.py)
pub const ARCEUS: Format = Format { block: 0x2985FE5D, slot: 0x178, len: 0x178, enc_end: 0x168, blk: 0x58, nick: 0x60, iv32: 0x94, dex_icons: true };

/// o save decifra e tem o bloco do time deste formato (o time pode estar vazio num save novo)
pub fn has_party_block(save_main: &Path, f: &Format) -> bool {
    match std::fs::read(save_main) {
        Ok(p) if p.len() < 64 << 20 => party_block(p, f.block).is_some(),
        _ => false,
    }
}

/// o time do save (vazio se não houver save ou se o formato não for reconhecido)
pub fn load(save_main: &Path, icons: &Path, f: &Format) -> Vec<PartyMon> {
    let Ok(p) = std::fs::read(save_main) else { return vec![] };
    if p.len() >= 64 << 20 {
        return vec![];
    }
    let Some(blk) = party_block(p, f.block) else { return vec![] };
    let mut out = vec![];
    let mut i = 0;
    while i + f.len <= blk.len() && i < 6 * f.slot {
        let mut d = blk[i..i + f.len].to_vec();
        i += f.slot;
        if rd32(&d) == 0 {
            continue;
        }
        decrypt(&mut d, f.enc_end, f.blk);
        let species = u16::from_le_bytes([d[8], d[9]]) as u32;
        let level = d[f.enc_end] as u32;
        if species == 0 || species > 1025 || level > 100 {
            continue;
        }
        let pid = rd32(&d[0x1C..]);
        let id = rd32(&d[0x0C..]);
        let name: Vec<u16> =
            (0..13).map(|k| u16::from_le_bytes([d[f.nick + 2 * k], d[f.nick + 1 + 2 * k]])).take_while(|&c| c != 0).collect();
        let mut m = PartyMon {
            name: String::from_utf16_lossy(&name),
            species,
            form: d[0x24] as u32,
            level,
            shiny: ((pid >> 16) ^ (pid & 0xFFFF) ^ (id >> 16) ^ (id & 0xFFFF)) < 16,
            egg: (rd32(&d[f.iv32..]) >> 30) & 1 == 1,
            icon: None,
        };
        m.icon = icon(icons, &m, f.dex_icons);
        out.push(m);
    }
    out
}

#[cfg(test)]
mod tests {
    /// PARTY_TEST_PLA / PARTY_TEST_ZA: cópias de saves (nunca versionadas); sem elas o teste não faz nada
    #[test]
    fn saves() {
        for (var, f) in [("PARTY_TEST_PLA", &super::ARCEUS), ("PARTY_TEST_ZA", &super::ZA)] {
            if let Ok(p) = std::env::var(var) {
                let t = super::load(std::path::Path::new(&p), std::path::Path::new("."), f);
                for m in &t {
                    println!("{var}: {} sp {} form {} lv {} shiny {} egg {}", m.name, m.species, m.form, m.level, m.shiny, m.egg);
                }
                assert!(!t.is_empty(), "{var}: time vazio");
            }
        }
    }
}
