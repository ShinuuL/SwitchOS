# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

O hub é um app Tauri (HTML/CSS/JS em `hub/src`, backend Rust em `hub/src-tauri`). O app Android (`br.com.mastersys.switchos`) é outra superfície do mesmo produto; quando for desenhado, decidir se segue a linguagem do hub ou a do Android.

## Users

Por enquanto, um único usuário: o próprio desenvolvedor, jogando no PC dele. Não há distribuição pública planejada. Ele usa o hub para importar o próprio dump, compilar, abrir o jogo, cuidar de mods e acompanhar o save.

## Product Purpose

Um launcher único para jogos do Switch recompilados para rodar nativamente no PC (Arceus e Z-A hoje; Sword/Shield, BDSP e TotK depois), com saves no formato original do Switch sincronizados entre PC e Android. Dá certo quando importar, compilar e jogar acontece num lugar só, sem passar pelas telas próprias de cada runtime.

## Positioning

Recompilação estática do jogo, não emulação: cada título vira código nativo do PC a partir do dump do próprio usuário. Os saves continuam no formato do Switch e são identificados pelo Title ID, então passam entre PC e celular sem conversão.

## Operating Context

- Usado em três situações: PC de mesa com mouse e teclado, TV ou sofá só com controle, e celular (app Android com sync de saves).
- Fluxo: escolher o NSP/XCI do jogo base e da atualização, extrair com `prod.keys`, compilar neste PC, jogar. Mods em `mods/<title id>/romfs` (formato Atmosphère). Saves em `save/`.
- Sync de 3 vias pelo `switchos.py` (PC, nuvem e Android via ADB), sempre guardando cópia em `backups/` antes de sobrescrever.

## Capabilities and Constraints

- Hoje o hub lista os jogos, importa e compila (com progresso e log), abre o jogo, mostra versão, DLCs instaladas, mods, data do save e o time do save do Z-A.
- Pendente: verificar arquivos do jogo, avisar quando é preciso recompilar, gerar ícones e shaders na primeira vez, importar saves, ligar e desligar mods, time do save do Arceus (ver `pendencia.md`).
- Jogos marcados como não suportados ainda têm runtime próprio em `engines/` e aparecem como "Em breve".

## Brand Commitments

- Nome: SwitchOS. A interface é toda em português do Brasil.
- Sem marcas da Nintendo: nenhum logo, nome comercial ou arte oficial da Nintendo, além do ícone que sai do dump do próprio usuário.
- O hub nunca baixa nem distribui jogos ou chaves, e deixa isso claro: cada usuário faz o dump do próprio cartucho e das próprias chaves.
- O fork do Android é GPLv3: o código precisa ser publicado quando o APK for distribuído.

## Evidence on Hand

- Arte dos jogos: `icon.jpg` extraído de cada dump importado. Ícones do time vêm de `cache/party_icons` do runtime.
- Não há depoimentos, usuários externos nem números de desempenho. Não inventar nenhum.

## Product Principles

1. Um lugar só: o hub absorve o que hoje só a tela de cada runtime faz.
2. O save é sagrado: nada é sobrescrito sem uma cópia.
3. Cada jogo e cada chave é do próprio usuário. O hub só trabalha com o que ele fornece.
4. A mesma ação funciona com mouse, teclado e controle.

## Accessibility & Inclusion

Navegação completa por controle e por teclado. Texto legível a distância de TV.
