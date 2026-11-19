# Pendências do SwitchOS

Atualizado em 19/11/2026.

## Próximos passos, por prioridade

Foco (decidido em 19/11): os dois jogos que temos (Z-A e Arceus) no PC. Android e outros jogos ficam para depois.

1. **Arceus a 60 fps no PC, como o Z-A.** Em andamento. Pista do Android: `main+0x277d838` liga o present interval 1, mas o mundo anda 2x mais rápido; falta ligar o passo do mundo ao tempo real (como o `FRAME_DT` do Z-A em `framepace.c`).
2. **Arceus no PC:** piscar de texturas e atraso nas cutscenes (estes dois precisam do usuário jogando).
3. **Hub 0.0.2:** teste manual das telas novas (primeira vez, importar save, mods, recompilar).
4. **Depois:** escala de resolução do Arceus no Android; outros jogos no runtime unificado.

Dumps em `D:\Games\Dumps`, organizados por jogo (`Arceus/`, `Z-A/`: Base, Atualizacao, DLC, Traducao); `prod.keys` e `title.keys` na raiz.

## Hub 0.0.2: compilador integrado e entrada direta no jogo

Decisão: o hub (Tauri, `hub/`) continua sendo o aplicativo principal. A tela inicial do ZA Recomp (`engines/las-za/runtime/ui/launcher.mm`) não é renomeada nem vira o hub. Ela fica só para quem abrir o `lasrecomp.exe` direto.

- [x] Interface nova (tela de console: prateleira, time, abas, controle). Aprovada em 19/11; o hub já abre o recomp.
- [x] Verificar os arquivos do jogo, gerar os ícones dos Pokémon e preparar os shaders: `firstrun.rs` (`firstrun_status` / `firstrun_run`, job `firstrun:<jogo>`), na aba Sobre do jogo. Mesmos arquivos e marcadores da tela do ZA Recomp (`cache/verify.*`, `cache/party_icons*`, `cache/shader_code_game.bin.*`). Só Z-A.
- [x] Avisar quando o runtime mudou e o jogo precisa ser recompilado (`lasgame.dll.id` diferente do `GAMELIB_ID`).
- [x] Importar saves (do Switch, do Ryujinx e de outras fontes).
- [x] "Jogar" abre direto no jogo (`launch.rs`, `lasrecomp.exe --play <pasta>`; recusa se o jogo já está aberto). Runtime recompilado com a mudança de `runtime/main.c`: com o código do jogo desatualizado (runtime novo, `.pchtxt` ligado/desligado) o `--play` volta para a tela inicial em vez de fechar com erro.

## Hub: outros pontos

- [ ] **Pacote único hub + runtime para instalar em outro PC.** Hoje `npx tauri build` gera só o hub (`bundle/nsis/SwitchOS Hub_<versão>_x64-setup.exe` e `.msi`); o runtime (`lasrecomp.exe`, `lasgen/`, `zaprompts/`, `gamelib.id`, DLLs) fica em `build/dist-za/ZA Recomp`, achado pelas configurações. Fazer um empacotamento que não apague `build/dist-za` (o `dist.sh` faz `rm -rf` e leva os saves): montar numa pasta de staging própria, embutir o runtime como recurso do instalador do Tauri (ou instalar em pasta ao lado) e o hub já vir configurado com o `runtime_dir`. Sem arquivos da Nintendo, sem saves e sem `cache/` do usuário.

- [x] **Time do save do Arceus no hub.** `party.rs` lê Z-A e Arceus (`party::ZA` / `party::ARCEUS`); teste com cópias dos saves: `PARTY_TEST_PLA=... PARTY_TEST_ZA=... cargo test --lib party` (Cyndaquil Nv. 5, Tepig Nv. 7).
- [x] Ícones do time do Arceus: `prompt_textures.py` (`pla_icons`) lê os BNTX soltos do romfs do Arceus (`bin/appli/res_pokemon/icon_pokemon_s`, 683 ícones; ficam de fora só variantes extras como `_011`/`_041`) para `cache/party_icons_pla`, com o nº da Pokédex como modelo (`party::ARCEUS.dex_icons`). O hub gera pela aba Sobre (`firstrun.rs`: no Arceus só a etapa dos ícones). `zaprompts` recongelado e trocado na distribuição (anterior em `build/exe-anterior/`).
- [x] Hub usar o `zaimport --probe` (`probe<TAB>pacote<TAB>title id base<TAB>base|update|dlc`) para avisar antes de importar um NSP/XCI de outro jogo. Comando `probe_packages`; a aba Importação etiqueta cada arquivo (Jogo base, Atualização, DLC ou "É de ...") e confirma em linha antes de importar o que não é do jogo.
- [x] Restaurar save pelo hub (`restore_backup`, em Rust, compatível com o `switchos.py backups` / `restore`): o save atual vira `*-pre-restore` antes. Teste ponta a ponta: `restore_e2e` (variáveis `RESTORE_E2E_*`, só com cópias).
- [x] Ligar e desligar mods pelo hub (`mods.rs`, `set_mod_enabled`): renomeia com/sem `_`, tira de `mods_off` ao ligar, só com o jogo fechado. Falta teste manual na janela do hub.

## Runtime e jogos

- [ ] Arceus: piscar de texturas no chão.
- [ ] Arceus: atraso para carregar os Pokémon nas cutscenes.
- [ ] Arceus: verificar a história do começo ao fim jogando (na teoria, nada bloqueia).
- [x] `tools/gen.py`: rótulo `PLA_BUILDS` corrigido para 1.1.1. GAMELIB_ID novo `965d7a15765fc205`; runtime, `lasgen` e os dois jogos recompilados (backups em `build/exe-anterior/`).
- [ ] Confirmar no jogo os textos da tradução PT-BR parcial do Z-A.
- [ ] Crash raro do Z-A: ponteiro nulo (`0xd0`) uma vez, ao abrir com DLC + tradução. Não se repetiu em 8 rodadas. Log em `ZA Recomp/logs/crash-raros/`.
- [ ] Integrar ao runtime unificado os outros jogos: Sword/Shield, BDSP, TotK.
  - Pesquisa (19/11): nenhum dump dos três em `D:\Games` (nada testável hoje).
  - Sword/Shield: o mais viável. `engines/las-swsh` é da mesma linhagem do las-za e roda (Sword 1.3.2). Molde do Arceus: `SWSH_BUILDS` + patches no `gen.py`, papéis dos subsdk (movie/glslc invertidos), portar os HLE do SwSh (60 fps, framepace, wildbattle...), perfil `party::SWSH` (SCBlock).
  - BDSP e TotK: runtime e gerador próprios (Unity/IL2CPP com exceções C++ e GC; TotK com HLE de SDK e Vulkan próprio). Fundir seria reescrita. Caminho barato: o hub lançar como engine externo (campo de exe e pasta de save por jogo no `GameDef`).
- [x] Crash do Z-A com DLC + tradução: não era o `data.trpfd` do mod (o runtime já ignora a cópia do mod). Era falta de memória no PC (commit negado, erro 1455); o diálogo agora explica "Your PC ran out of memory" (`rt_commit_failed`).
- [x] `boot_test.sh` encerra o jogo pelo PID do Windows (`taskkill`).

## Android

- [ ] **Escala de resolução do Arceus.** `DM_RENDER_SCALE` e `DM_DYN_SCALE` só funcionam em jogos cujo `patches.py` tenha os render-scale stubs (hoje só os Zelda e mais 3). Exige engenharia reversa: achar onde o motor da Game Freak define o tamanho do render alvo e criar os stubs no perfil `01001F5010DFA000`. É o maior ganho contra o calor (15-30% menos GPU).
  - Pesquisa (19/11, só leitura, nada aplicado): o Arceus 1.1.1 já tem resolução dinâmica própria (`main+0xcc35a4`: 4 níveis, até 0,8 x 0,7) e guarda a escala em `3DBufferWidthScale`/`HeightScale`. Três leitores: `0x29b2518`, `0x29fb338`, `0x2b9f940` (bytes originais `612245bd`, `e07340bd`, `c002221e`; x8 livre nos três). Stub proposto em `dm/games/01001F5010DFA000/patches.py`: trocar a escala lida pelo float do state +0x10 (`DM_RENDER_SCALE`; 1,0 = escala do jogo), como no BotW, com `FRAME_STEP` e `ctx.render_scale = 1` (conferir `fps_target` = 0). Os render targets continuam do tamanho cheio: o ganho é só de GPU.
  - Teste no M55: APK com o perfil, `DM_RENDER_SCALE=0.6` no `env.txt`, logcat `3D views rendered at 0.60`; comparar fps e temperatura na mesma cena. Borda preta ou lixo = algum leitor faltando (procurar os hashes `0x4aa9ba3dccea94ea` / `0x66e7a756352d0d9f`).
  - Correção: só BotW e TotK têm render-scale stub (não "os Zelda e mais 3"); Sword/Shield não tem, então não há molde da Game Freak.
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
