# Pendências do SwitchOS

Atualizado em 18/11/2026.

## Hub 0.0.2: compilador integrado e entrada direta no jogo

Decisão: o hub (Tauri, `hub/`) continua sendo o aplicativo principal. A tela inicial do ZA Recomp (`engines/las-za/runtime/ui/launcher.mm`) não é renomeada nem vira o hub. Ela fica só para quem abrir o `lasrecomp.exe` direto.

Trazer para o hub o que hoje só a tela do ZA Recomp faz:

- [ ] Verificar os arquivos do jogo (hoje fica em `cache/verify.txt`).
- [ ] Avisar quando o runtime mudou e o jogo precisa ser recompilado (`lasgame.dll.id` diferente do `GAMELIB_ID`).
- [ ] Gerar os ícones dos Pokémon na primeira vez (`zaprompts --party-icons`). Hoje foram gerados à mão.
- [ ] Preparar os shaders na primeira vez (`zaprompts --shader-code`).
- [ ] Importar saves (do Switch, do Ryujinx e de outras fontes).
- [ ] "Jogar" abre direto no jogo, sem passar pela tela do ZA Recomp.
- [x] Interface nova (tela de console: prateleira, time, abas, controle).

## Hub: outros pontos

- [ ] `zaimport --probe`: listar o title id de cada arquivo sem extrair, para o hub avisar antes de importar um NSP/XCI de outro jogo.
- [ ] Restaurar save pelo hub (o `switchos.py` ainda não tem comando de restauração; hoje a aba Save só orienta).
- [ ] Time do save do Arceus (o formato do save é diferente do Z-A).
- [ ] Ligar e desligar mods pelo hub (prefixo `_` ou `mods_off` do runtime).
- [ ] Mostrar quando um mod traz `arc/data.trpfd` próprio e oferecer remover. A tradução parcial do Z-A derrubava o jogo por causa disso. O original está em `ZA Recomp/mods-originals/`.

## Runtime e jogos

- [ ] Arceus: piscar de texturas no chão.
- [ ] Arceus: atraso para carregar os Pokémon nas cutscenes.
- [ ] Arceus: verificar a história do começo ao fim jogando (na teoria, nada bloqueia).
- [ ] `tools/gen.py`: o rótulo `PLA_BUILDS` diz 1.0.1, mas a build `aee8f150` é a 1.1.1.
- [ ] Confirmar no jogo os textos da tradução PT-BR parcial do Z-A.
- [ ] Integrar ao runtime unificado os outros jogos: Sword/Shield, BDSP, TotK.

## Android

- [x] Testar num aparelho real: Galaxy M55 (SM-M556B), Android 16. O APK completo instala, o ambiente fica pronto e o Arceus 1.1.1 foi convertido no celular em 7 min 9 s.
- [ ] Desligar no fork o aviso de atualização do nx2androidX. Ele oferece a versão do upstream (0.3.0-alpha16), mas o fork está travado.
- [ ] Trocar o aviso "This app (nx2androidX)…" por SwitchOS, mantendo o link do código-fonte que a GPLv3 exige.
- [ ] Mover o dump do Brilliant Diamond (`Download/Pokemon Brilliant Diamond Switch NSP Base Game` e `Download/PokemonBrilliant Diamond…`) para `/sdcard/SwitchOS/`. Precisa do shell liberado no Screen Mirror ou ser feito à mão.
- [ ] Commitar as correções do build do APK no Windows: `build_nx_extract.py`, `build_assets.py`, `build_adrenotools.py`, `rootfs_trim.py` e `Env.kt`.
- [ ] Driver Vulkan para a Adreno 644. O Turnip embutido é da série 8xx.
- [ ] Sincronização de saves entre hub e celular (`switchos.py sync`) com o app `br.com.mastersys.switchos`.
- [ ] Otimizações para o celular nos perfis do Arceus e do Z-A (aquecimento no Snapdragon 7 Gen 1).
- [ ] **Escala de resolução do Arceus.** `DM_RENDER_SCALE` e `DM_DYN_SCALE` só funcionam em jogos cujo `patches.py` tenha os render-scale stubs (hoje só os Zelda e mais 3). Exige engenharia reversa: achar onde o motor da Game Freak define o tamanho do render alvo e criar os stubs no perfil `01001F5010DFA000`. É o maior ganho contra o calor (15-30% menos GPU).
  - Medição de base, Galaxy M55, sessão de ~25 min: 29-30 fps estáveis; bateria de 42 °C para 47 °C ainda subindo; 777 mA de consumo, que o carregador de 5 V/0,9 A não acompanha; estado térmico normal com corte leve na GPU; 640 MB do jogo em swap, com o lmkd matando apps.
- [x] Tela fixa em 30 Hz enquanto o jogo roda a 30 fps (`GameActivity.applyDisplayRate`). Falta verificar no aparelho.
- [ ] Controles na tela: estão feios e cobrem quase tudo. Refazer junto com a interface nova (skill de UI).
- [ ] GPLv3: publicar o código do fork quando o APK for distribuído.

Dumps no celular: `/sdcard/SwitchOS/`. Envios grandes (vários GB) derrubam o ADB nesse aparelho: copiar pelo Explorador do Windows.

## Ambiente

- [ ] Build do DX12 (Dozen): o meson falha porque o TEMP aponta para `C:\WINDOWS`. Hoje o DX12 é opcional.
