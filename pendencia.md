# Pendências do SwitchOS

Atualizado em 19/11/2026.

## Próximos passos, por prioridade

1. **Ligar no hub o backend que já existe** (a interface nova está pronta e aprovada):
   - botão de restaurar save (`switchos.py backups` / `restore`).
2. **Hub 0.0.2:** trazer para o hub o que hoje só a tela do ZA Recomp faz (lista abaixo).
3. **Arceus no PC:** piscar de texturas e atraso nas cutscenes. Precisa do usuário jogando.
4. **Projetos maiores:** escala de resolução do Arceus no Android; outros jogos no runtime unificado.

## Hub 0.0.2: compilador integrado e entrada direta no jogo

Decisão: o hub (Tauri, `hub/`) continua sendo o aplicativo principal. A tela inicial do ZA Recomp (`engines/las-za/runtime/ui/launcher.mm`) não é renomeada nem vira o hub. Ela fica só para quem abrir o `lasrecomp.exe` direto.

- [x] Interface nova (tela de console: prateleira, time, abas, controle). Aprovada em 19/11; o hub já abre o recomp.
- [ ] Verificar os arquivos do jogo (hoje fica em `cache/verify.txt`).
- [ ] Avisar quando o runtime mudou e o jogo precisa ser recompilado (`lasgame.dll.id` diferente do `GAMELIB_ID`).
- [ ] Gerar os ícones dos Pokémon na primeira vez (`zaprompts --party-icons`). Hoje foram gerados à mão.
- [ ] Preparar os shaders na primeira vez (`zaprompts --shader-code`).
- [ ] Importar saves (do Switch, do Ryujinx e de outras fontes).
- [ ] "Jogar" abre direto no jogo, sem passar pela tela do ZA Recomp.

## Hub: outros pontos

- [x] **Time do save do Arceus no hub.** `party.rs` lê Z-A e Arceus (`party::ZA` / `party::ARCEUS`); teste com cópias dos saves: `PARTY_TEST_PLA=... PARTY_TEST_ZA=... cargo test --lib party` (Cyndaquil Nv. 5, Tepig Nv. 7).
- [ ] Ícones do time do Arceus: o cache `party_icons` vem do romfs do Z-A, que não tem o Cyndaquil (o hub mostra a Poké Ball). Gerar ícones a partir do romfs do Arceus.
- [x] Hub usar o `zaimport --probe` (`probe<TAB>pacote<TAB>title id base<TAB>base|update|dlc`) para avisar antes de importar um NSP/XCI de outro jogo. Comando `probe_packages`; a aba Importação etiqueta cada arquivo (Jogo base, Atualização, DLC ou "É de ...") e confirma em linha antes de importar o que não é do jogo.
- [ ] Hub chamar `switchos.py backups <jogo>` / `restore <jogo> <backup|latest>` (o save atual vira backup `*-pre-restore` antes).
- [ ] Ligar e desligar mods pelo hub (prefixo `_` ou `mods_off` do runtime).

## Runtime e jogos

- [ ] Arceus: piscar de texturas no chão.
- [ ] Arceus: atraso para carregar os Pokémon nas cutscenes.
- [ ] Arceus: verificar a história do começo ao fim jogando (na teoria, nada bloqueia).
- [ ] `tools/gen.py`: o rótulo `PLA_BUILDS` diz 1.0.1, mas a build `aee8f150` é a 1.1.1. O `gen.py` entra no `GAMELIB_ID`: corrigir junto com a próxima mudança que já obrigue a recompilar.
- [ ] Confirmar no jogo os textos da tradução PT-BR parcial do Z-A.
- [ ] Crash raro do Z-A: ponteiro nulo (`0xd0`) uma vez, ao abrir com DLC + tradução. Não se repetiu em 8 rodadas. Log em `ZA Recomp/logs/crash-raros/`.
- [ ] Integrar ao runtime unificado os outros jogos: Sword/Shield, BDSP, TotK.
- [x] Crash do Z-A com DLC + tradução: não era o `data.trpfd` do mod (o runtime já ignora a cópia do mod). Era falta de memória no PC (commit negado, erro 1455); o diálogo agora explica "Your PC ran out of memory" (`rt_commit_failed`).
- [x] `boot_test.sh` encerra o jogo pelo PID do Windows (`taskkill`).

## Android

- [ ] **Escala de resolução do Arceus.** `DM_RENDER_SCALE` e `DM_DYN_SCALE` só funcionam em jogos cujo `patches.py` tenha os render-scale stubs (hoje só os Zelda e mais 3). Exige engenharia reversa: achar onde o motor da Game Freak define o tamanho do render alvo e criar os stubs no perfil `01001F5010DFA000`. É o maior ganho contra o calor (15-30% menos GPU).
  - Medição de base, Galaxy M55, ~25 min: 29-30 fps na introdução, 23 fps no título com o cenário; bateria de 42 °C para 47 °C ainda subindo; 777 mA, mais do que o carregador de 5 V/0,9 A entrega; estado térmico normal com corte leve na GPU; 640 MB do jogo em swap, com o lmkd matando apps.
- [ ] Controles na tela: estão feios e cobrem quase tudo. Refazer no estilo da interface nova.
- [ ] Mover o dump do Brilliant Diamond (`Download/Pokemon Brilliant Diamond Switch NSP Base Game` e `Download/PokemonBrilliant Diamond…`) para `/sdcard/SwitchOS/`. O shell do Screen Mirror só aceita comandos de leitura: fazer à mão.
- [ ] Sincronização de saves entre hub e celular (`switchos.py sync`) com o app `br.com.mastersys.switchos`.
- [ ] GPLv3: publicar o código do fork quando o APK for distribuído.
- [x] Testado num aparelho real: Galaxy M55 (SM-M556B, Snapdragon 7 Gen 1, Adreno 644), Android 16. O Arceus 1.1.1 foi convertido no celular em 7 min 9 s e roda com o Turnip Gen8 embutido.
- [x] Correções para montar o APK no Windows; o app refaz o ambiente quando o APK muda.
- [x] Aviso de atualização e licença com a marca SwitchOS (o fork não oferece mais o app do upstream).
- [x] Opção "Hold the screen at 30 Hz" no menu do jogo (desligada por padrão).

Dumps no celular: `/sdcard/SwitchOS/`. Envios grandes (vários GB) e o APK (288 MB) derrubam o ADB nesse aparelho: copiar pelo Explorador do Windows e instalar pelo Meus Arquivos.

## Ambiente

- [ ] Build do DX12 (Dozen), opcional: com TEMP/TMP/TMPDIR certos o `cc` funciona direto, mas chamado pelo meson ainda tenta `C:\WINDOWS\` e sai com `0xC0000409`. O meson passa outro ambiente ao compilador; investigar (arquivo nativo do meson com `[properties]` ou `[binaries]`, ou rodar fora do Git Bash, num shell UCRT64 puro).

## Ideias

- [ ] Login com OAuth (Google e GitHub).
- [ ] Saves na nuvem: Google Drive da conta do usuário ou bucket na Cloudflare (R2).
- [ ] Jogos na nuvem para baixar pelo hub, como numa loja online.
- [ ] Changelog automático nos releases, com as melhorias e os jogos novos do runtime.
- [ ] Criar comunidade no Scale para postar atualizações.
- [ ] Colocar o link da comunidade no GitHub e no README.
