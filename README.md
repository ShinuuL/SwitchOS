# SwitchOS

Launcher unificado para recompilações nativas de jogos do Switch, com sincronização de saves entre PC e Android.

## Arquitetura

| Plataforma | Como o jogo roda | Projeto |
|---|---|---|
| PC (Windows/Linux) | Recompilação estática ARM64 → x86-64 | `vendor/PokeZ-ARecomp`, `Arceuszada`, `swsh-recomp`, `strahlender-xnu`, `totk-pc-recomp` |
| Android | Código arm64 original + host de serviços + NVN→Vulkan | `vendor/nx2androidX` (app `com.nx.android`) |

Os saves usam sempre o formato original do Switch, então são compatíveis entre as plataformas sem conversão. O
SwitchOS os identifica pelo **Title ID**.

```
 PC (save/save_0) ─┐                       ┌─ Android (files/save/<TitleID>, via ADB)
                   ├──► nuvem/saves/<TID> ◄─┤
 outro PC ─────────┘   (Drive/Syncthing)    └─ backups/<TID>/<data>-<origem>
```

Na sincronização de 3 vias, cada lado é comparado com o hash da última sync (`state.json`). Se só um lado mudou, ele
é propagado. Se dois mudaram, vence o mais recente e a outra versão é copiada para `backups/`. Nenhum save é
sobrescrito sem que exista uma cópia.

## Uso

```
python switchos.py status              # o que está compilado e o que tem save
python switchos.py sync                # sincroniza todos (PC + nuvem + Android se conectado)
python switchos.py sync za --dry-run   # mostra o que faria
python switchos.py play za             # sync → joga → sync
```

Requisitos: Python 3.11+ e, para o Android, `adb` no PATH com depuração USB. O `run-as` exige um APK debuggable. Se o
APK não for, use Exportar/Importar save no app nx2androidX e coloque o arquivo na pasta da nuvem.

## Requisitos legais

Nenhum repositório inclui jogos ou chaves. Cada usuário precisa fazer o dump do próprio cartucho e das próprias chaves.

## Segurança

O `vendor/Arceuszada` traz executáveis pré-compilados (`lasrecomp.exe`, `lasgen.exe`, `zaimport.exe` etc.) sem fonte
verificável e está marcado como `untrusted_binaries`. Antes de executar, analise os arquivos no VirusTotal ou compile
o equivalente a partir do `PokeZ-ARecomp`, que usa a mesma base LAS Recomp.
