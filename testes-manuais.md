# Testes que precisam do usuário

Testes que só dão para fazer jogando ou olhando a janela. Marque com [x] e anote o que viu.

## Arceus no PC

- [ ] **60 fps (F1 > 60 fps ligado):** segurar o direcional 3 s andando e correndo, a 30 e a 60 fps, do mesmo ponto. A distância tem que ser igual.
- [ ] **60 fps:** animações, água, céu, hora do dia e câmera na velocidade normal. Anotar qualquer coisa isolada correndo 2x (efeito, objeto do cenário, interface).
- [ ] **60 fps** (gravação de 19/11: caiu para 30 aos 0:58 numa cutscene e não voltou; refazer sem gravar, religando no F1): batalha, menus e a volta automática para 30 fps quando cai abaixo de 50.
- [x] Piscar de texturas no chão: onde acontece e se some com o 60 fps desligado (F1). — listras diagonais no chão com capim, câmera baixa (5:49 da gravação de 19/11); falta saber se some a 30 fps — naquela hora já estava a 30.
- [ ] Atraso para carregar os Pokémon nas cutscenes: em qual cena.
- [ ] História do começo ao fim (nada deve travar).
- [x] Fotos do time na tela do hub (depois dos ícones do Arceus). — Cyndaquil com foto no print do hub de 19/11.

- [x] Jogando pelo hub: título da janela "Pokémon Legends: Arceus — SwitchOS" (acentos e travessão certos, também na barra de tarefas). — ok na gravação de 19/11 (log e vídeo); barra de tarefas não conferida.

## Gravação no hub

- [ ] **Nova Gravar sessão** (exe de 19/11 noite, que grava o save certo): passar pelo chão com capim onde apareceram as listras diagonais (5:49 da gravação de 19/11, câmera baixa no chão perto da Akari) e apertar **F8** logo depois.

- [x] **Gravar sessão** no Arceus até o chão piscar e até uma cutscene com atraso (F8 em cada vez); mandar a pasta (Configurações > Pasta das gravações). — gravado em 19/11, mas o replay daquela versão trazia o save errado: ver "Nova Gravar sessão".
- [x] **Gravar vídeo**: o `video.mp4` mostra o jogo e o jogo não perde fps gravando. — vídeo 1296x758 30 fps 12:24, jogo inteiro. Perda de fps gravando: não confirmada.
- [x] Jogar normal: `logs/play.log` é criado (o jogo não herda mais a saída do hub). — criado pelo --play do hub (gravação de 19/11).

## Z-A no PC

- [ ] Aberto pela tela inicial do ZA Recomp: título continua "ZA Recomp" e a verificação dos arquivos funciona.

- [ ] Textos da tradução PT-BR parcial.
- [ ] Crash raro ao abrir com DLC + tradução: avisar se repetir (log em `ZA Recomp/logs/`).

## Hub 0.0.2

- [ ] Primeira vez (aba Sobre): verificar arquivos, ícones, shaders.
- [ ] Importar save (Switch/Ryujinx).
- [ ] Ligar e desligar mods.
- [ ] Aviso "Recompilar" e a recompilação pelo hub.
- [ ] Restaurar save (o atual vira `pre-restore`).
- [x] Jogar pelo hub: F1 > Sair mostra só "Voltar ao hub"; confirma e volta ao hub. — volta ao hub ok (log da gravação de 19/11: "exiting to the hub"); rótulo não visto.
- [ ] Ligar um mod com `.pchtxt` e clicar em Jogar: o jogo não abre e aparece o Recompilar. Desligar de novo: volta a abrir.
- [ ] Aba Mods: mudar a ordem com ▲/▼ (teclado e controle); o foco acompanha o mod e a ordem vale no jogo.
- [ ] `lasrecomp.exe` aberto sem o hub: a tela inicial do ZA Recomp continua aparecendo.
