---
name: SwitchOS Hub
description: Um console pessoal para os jogos do Switch recompilados no PC.
colors:
  night: "oklch(17% 0.02 250)"
  ink: "oklch(97% 0.005 250)"
  ink-soft: "oklch(84% 0.015 250)"
  ink-quiet: "oklch(74% 0.02 250)"
  glass: "oklch(22% 0.02 250 / 0.55)"
  glass-strong: "oklch(19% 0.02 250 / 0.86)"
  edge: "oklch(100% 0 0 / 0.09)"
  edge-lit: "oklch(100% 0 0 / 0.16)"
  focus-cyan: "oklch(86% 0.13 195)"
  ready-green: "oklch(80% 0.15 155)"
  amber: "oklch(82% 0.14 75)"
  idle-gray: "oklch(68% 0.015 250)"
  alert-coral: "oklch(72% 0.17 25)"
typography:
  display:
    fontFamily: "Figtree, system-ui, sans-serif"
    fontSize: "40px"
    fontWeight: 750
    lineHeight: 1.08
    letterSpacing: "-0.02em"
  headline:
    fontFamily: "Figtree, system-ui, sans-serif"
    fontSize: "26px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "0.01em"
  title:
    fontFamily: "Figtree, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 650
    lineHeight: 1.3
  body:
    fontFamily: "Figtree, system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 500
    lineHeight: 1.45
  label:
    fontFamily: "Figtree, system-ui, sans-serif"
    fontSize: "17px"
    fontWeight: 750
    lineHeight: 1.2
    letterSpacing: "0.06em"
  data:
    fontFamily: "Cascadia Mono, Consolas, monospace"
    fontSize: "16px"
    fontWeight: 500
    lineHeight: 1.5
rounded:
  input: "12px"
  tile: "14px"
  card: "16px"
  sheet: "22px"
  pill: "99px"
spacing:
  xs: "6px"
  sm: "10px"
  md: "16px"
  lg: "26px"
  gutter: "56px"
components:
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.night}"
    rounded: "{rounded.pill}"
    padding: "9px 18px"
  button-secondary:
    backgroundColor: "{colors.edge}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
    padding: "9px 18px"
  tab-selected:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.night}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "8px 18px"
  tab:
    textColor: "{colors.ink-soft}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "8px 18px"
  game-tile:
    rounded: "{rounded.tile}"
    size: "150px"
  game-tile-selected:
    rounded: "{rounded.tile}"
    size: "240px"
  party-card:
    backgroundColor: "{colors.glass}"
    rounded: "{rounded.card}"
    padding: "8px 12px 8px 8px"
  input:
    backgroundColor: "oklch(8% 0.01 250 / 0.6)"
    textColor: "{colors.ink}"
    typography: "{typography.data}"
    rounded: "{rounded.input}"
    padding: "10px 14px"
  pill-ok:
    backgroundColor: "oklch(80% 0.15 155 / 0.16)"
    textColor: "{colors.ready-green}"
    rounded: "{rounded.pill}"
    padding: "3px 11px"
---

# Design System: SwitchOS Hub

## Overview

**Creative North Star: "O Console Pessoal"**

O hub é um console feito sob medida para uma pessoa. A forma é familiar: prateleira de jogos, dicas de botão no rodapé, navegação por controle. Mas não carrega a marca de ninguém. Ele se comporta como o menu inicial de um console, não como um app de desktop: o jogo selecionado manda na tela, a arte dele tinge o fundo, e a interface fica num segundo plano macio e escuro.

A densidade é baixa no topo e cresce para baixo. Na tela inicial vem o jogo e a ação de jogar, depois o time do save. Os detalhes (versão e DLC, mods, save, importação) ficam na página do jogo, em abas. Tudo precisa funcionar a três distâncias: monitor perto, TV longe e controle na mão.

Rejeitados: tema claro, cara de painel administrativo (sidebar com lista, cards iguais em grade, tabelas) e as cores e a identidade da Nintendo.

**Key Characteristics:**
- Sempre escuro: o fundo é a arte do jogo desfocada sobre um azul-noite.
- Um jogo em foco por vez, maior, com contorno ciano que respira.
- Pílulas arredondadas e vidro fosco onde a interface flutua sobre a arte.
- Cada ação mostra o botão do controle que a dispara.

## Colors

Uma base azul-noite quase neutra, tinta clara e um único acento frio para o foco. A cor de verdade vem da arte de cada jogo.

### Primary
- **Ciano de Foco** (oklch(86% 0.13 195)): contorno do jogo selecionado, barra de progresso, cursor dos campos. Botões, abas e linhas em foco não usam ciano: ganham o inverso (ver Focus em Componentes), para não se confundirem com a capa selecionada. É a única cor da interface que diz "você está aqui".

### Neutral
- **Noite** (oklch(17% 0.02 250)): fundo base, por baixo da arte do jogo.
- **Tinta** (oklch(97% 0.005 250)): texto principal, fundo do botão primário e da aba selecionada.
- **Tinta Suave** (oklch(84% 0.015 250)): texto secundário, dicas do rodapé.
- **Tinta Discreta** (oklch(74% 0.02 250)): rótulos discretos e marca; abas e níveis do time passaram para a Tinta Secundária para dar 4.5:1 de longe.
- **Vidro** e **Vidro Denso**: superfícies que flutuam sobre a arte (cards do time, folha de configurações, avisos).
- **Aresta** e **Aresta Iluminada**: divisórias de 1px e o brilho no topo das superfícies de vidro.

### Status
- **Verde Pronto** (oklch(80% 0.15 155)): jogo pronto para jogar, versão lida, DLC instalada.
- **Âmbar** (oklch(82% 0.14 75)): falta compilar, estrela de shiny.
- **Cinza Parado** (oklch(68% 0.015 250)): não importado, importação cancelada, em breve.
- **Coral de Alerta** (oklch(72% 0.17 25)): erro na importação (ponto do rodapé e ponto no canto da miniatura).
- O ciano de foco também marca "Importando: <etapa>" (ponto do rodapé e da miniatura).
- O status sempre vem do build e do último job juntos, com texto além da cor: nunca "Pronto para jogar" sozinho ao lado de uma importação falhada (compilado com reimportação falhada: "Pronto para jogar · a última importação falhou"). Com o save deste PC mais novo que a última sincronização: "Pronto para jogar · save não sincronizado" com ponto Aviso. "Pronto para jogar" só aparece no rodapé (a aba Versão e DLC só mostra "Situação" quando o jogo não está compilado).

### Named Rules
**The Art Leads Rule.** A cor dominante da tela é sempre a da arte do jogo selecionado, nunca da interface. A interface só acrescenta o ciano de foco e as cores de status.

**The No Nintendo Red Rule.** Nada de vermelho Switch nem de qualquer cor ou forma que imite a identidade da Nintendo. O vermelho só aparece se vier da arte do próprio dump.

## Typography

**Display Font:** Figtree (com system-ui)
**Body Font:** Figtree
**Label/Mono Font:** Cascadia Mono (com Consolas), só para title ID, caminhos e log

**Character:** Uma sans geométrica e amigável, de console, com pesos fortes no título e médios no corpo. A monoespaçada aparece só em dado técnico de verdade.

### Escala para TV
O hub é lido de longe (TV a ~3 m). Todo tamanho é `calc(N * var(--s))`, com `--s: clamp(1px, min(100vw / 1180, 100vh / 720), 2px)`: vale 1px em 1180x720 e menores e cresce proporcional à janela (1920x1080 dá 1.5). Os valores abaixo são os de 1180x720. Nada que precise ser lido fica abaixo de 18px (rótulos em maiúsculas: 17px, compensados pela versal); o log da importação e o detalhe técnico do erro são área secundária, rolável e copiável, e usam o mono de 16px.

### Hierarchy
- **Display** (750, 40px, 1.08): nome do jogo selecionado. Fica numa linha só, com reticências (32px abaixo de 1100px, 26px abaixo de 760px).
- **Headline** (600, 26px, 1.2): título da folha de configurações.
- **Title** (650, 20px, 1.3): ações ao lado do nome do jogo (Jogar e Opções do jogo na inicial; Jogar e Pasta do jogo na página).
- **Body** (500, 18px, 1.45): texto corrido, nomes e níveis do time, dicas do rodapé, status, pílulas, com no máximo 68ch.
- **Label** (750, 17px, 0.06em, maiúsculas): abas e títulos de seção como "Seu time".
- **Data** (500, 16px, mono): title ID, caminhos. Log da importação também 16px.

### Contraste
Texto secundário usa `--ink-2` (84%); `--ink-3` (74%) só em textos que passam 4.5:1 sobre o vidro escuro. Fatos sobre a arte levam rótulo ("Versão 2.0.1", "DLC instalada", "1 mod"), cor `--ink-2` e sombra de texto; nenhum deles usa cor de status, para não parecer botão. `--ink-3` (74%) sobre o vidro da folha (19%) fica em torno de 8:1.

### Named Rules
**The Real Mono Rule.** Monoespaçada só para dado técnico de verdade (title ID, caminho, log). Nunca como enfeite de "tecnológico".

## Layout

Coluna única em tela cheia, sem sidebar, com respiro lateral de 56px (escalado por `--s`) (36px abaixo de 1100px, 18px abaixo de 760px). Duas telas no mesmo lugar. Tela inicial, de cima para baixo: barra superior (marca, relógio, configurações), prateleira (só jogos suportados), "Seu time" e uma linha discreta de "Próximos jogos" (os que ainda têm runtime próprio), centrados na altura disponível. Página do jogo: arte (200px), nome, ações, caixa da importação em andamento (fora das abas) e abas de detalhes. O rodapé fixo com as dicas de botão acompanha a tela. A troca entre as duas segue a Materialize Rule: a que sai desfoca e recua, a que entra chega do desfoque (280 a 380ms).

Na prateleira, o jogo selecionado (240px) fica sempre parado à esquerda, e a fileira de jogos (150px; 112px abaixo de 760px; espaço de 16px) desliza por baixo. O nome e as ações ficam ancorados à capa, nunca soltos no terço de cima: com miniaturas depois do selecionado, assentam na base da faixa acima delas (14px de folga, escalado); quando o selecionado é o último da fileira (não há miniaturas à direita), descem até a base da capa selecionada. Se a meta quebrar linha, o bloco cresce para cima sem passar do topo da prateleira (`safe flex-end`). O selecionado fica recuado por uma faixa de espia (40px; 30px abaixo de 760px): o jogo anterior aparece nela esmaecido (42%, dessaturado), clicável, mostrando que há jogos antes; os mais antigos saem pela esquerda. O time usa 6 colunas (3 abaixo de 1100px). Os detalhes têm no máximo 900px de largura. O espaço vertical da inicial (vão entre blocos 14px, topo e altura da prateleira) escala com `--s`; quando o jogo não tem time, a prateleira cresce (selecionado 290px, fileira 176px, a partir de 1101px) para não sobrar faixa vazia em 1920x1080. "Próximos jogos" é uma linha de rodapé (rótulo em versal 17px, nomes em Tinta 2, resto em Tinta Discreta, 18px), sem caixa nem foco.

## Elevation & Depth

Profundidade por material, não por bordas. A arte desfocada fica no fundo, coberta por gradientes escuros. Por cima, superfícies de vidro fosco com o topo levemente iluminado. As sombras são sempre suaves e deslocadas para baixo, sugerindo um objeto sobre a mesa, nunca um halo colorido.

### Shadow Vocabulary
- **Miniatura** (`0 10px 24px -12px oklch(0% 0 0 / 0.7)`): jogos não selecionados.
- **Miniatura em foco** (`0 22px 44px -18px oklch(0% 0 0 / 0.85)`): jogo selecionado, que parece mais perto.
- **Vidro** (`inset 0 1px 0 aresta iluminada, 0 8px 20px -14px oklch(0% 0 0 / 0.8)`): cards do time.
- **Folha** (`inset 0 1px 0 aresta iluminada, -20px 0 60px -20px oklch(0% 0 0 / 0.7)`): folha de configurações.

### Named Rules
**The Materialize Rule.** Superfícies de vidro entram com desfoque, deslocamento e opacidade juntos, como um material chegando, e saem pelo mesmo caminho. Uma tarefa que interrompe (configurações) escurece e desfoca o fundo. Um painel paralelo não escurece nada.

## Shapes

Formas macias e consistentes. Ações, abas, status e dicas de botão são pílulas (99px). As miniaturas de jogo têm cantos de 14px, os cards do time 16px, os campos 12px, e a folha lateral 22px, com 12px de margem até a borda da janela. Não há bordas laterais de destaque nem cantos retos.

## Components

### Buttons
- **Shape:** pílula (99px).
- **Primary:** fundo Tinta, texto Noite, peso 650, 9px 18px. Usado no máximo uma vez por grupo (Salvar, Importar e compilar).
- **Secondary:** branco a 10% com brilho interno de 1px. No hover sobe para 16%.
- **Press:** encolhe para 0.97 em 120ms com saída rápida. Desabilitado fica com 40% de opacidade.
- **Ação com botão do controle:** pílula transparente com o glifo (A, X, Y) à esquerda, 17px. Jogar é sempre A (Enter no teclado), na inicial e na página; Opções do jogo é X na inicial e Pasta do jogo é Y na página. Na inicial essas ações ficam ao lado do nome (onde o mouse clica); o rodapé só as repete no modo controle (A Jogar, X Opções, Menu Configurações), menores e em Tinta 2, porque é ali que quem está no sofá procura. No teclado o rodapé da inicial mostra só Configurações.

### Chips
- **Status:** pílula com a cor de status a 16% de fundo e o texto na mesma cor (versão, DLC instalada).
- **Glifos de botão:** dois desenhos, escolhidos pela última entrada física (`<html data-input="pad|kbd">`). Controle: círculo em Tinta (A), azulado claro (X, Y, B) e retângulo translúcido para LB, RB e Menu. Teclado: todo glifo vira keycap (retângulo de raio 7px, fundo translúcido, borda de 1px e base escura de 2px) com o nome da tecla (Enter, Esc, X, Y, Q, E, M); nunca círculo. O mouse não muda o modo: quem clica vê o glifo da última tecla ou botão usado (começa em controle). Vale para botões, rodapé, toast, primeiro uso e conteúdo criado depois (um MutationObserver preenche o texto).
- **Status (ponto):** verde pronto, ciano rodando, âmbar atenção, vermelho erro, cinza cheio "Não importado", anel vazio "Em breve".

### Cards / Containers
- **Cards do time:** vidro (blur 18px, saturação 160%), cantos de 16px, ícone em quadro de 52px e nome com nível. Os shiny ganham fundo âmbar no ícone e uma estrela.
- **Caixa da importação:** vidro denso, cantos de 16px, padding de 16px.
- **Caixa da importação com erro:** mesma caixa, com aresta Erro de 1px em volta, título em Erro (o que aconteceu, em PT-BR, nunca o texto cru), uma frase do que fazer e as ações: o conserto mais provável em pílula Tinta ("Tentar de novo", "Escolher outros arquivos" ou "Abrir configurações") e "Ver log", que abre o log inteiro. Cancelado pelo usuário é estado neutro: título em Tinta, sem aresta Erro, "Importar de novo".
- Nunca um card dentro de outro.

### Avisos (toast)
- **Posição:** ancorado 12px acima do rodapé (altura medida em `--hints-h`), centrado; nunca cobre título nem ações. Superfície opaca (oklch 21%), sem vidro: o conteúdo de trás não vaza.
- **Aviso comum:** pílula opaca acima do rodapé, `role=status`, some sozinho em 4s. Se cair sobre o item em foco (ex.: a última cópia da lista), a tela rola para cima até liberá-lo.
- **Toast de erro:** superfície opaca (oklch 20%), aresta Erro de 1px em volta, título (o que aconteceu) + frase (o que fazer). Fica até ser fechado (`role=alert`), recebe o foco e é a primeira camada do controle: esquerda/direita andam nos botões, A ativa, B/Esc fecha. Ações: "Ver log" (quando houve importação), "Ver detalhes" (só em erro desconhecido, mostra o texto técnico em mono), "Abrir configurações" (quando o conserto é lá) e "Fechar" com a dica B.
- O texto cru do backend passa sempre por `explainError()` (main.js) antes de chegar à tela.

### Validação das configurações
- Embaixo de cada campo, uma linha com o resultado de `check_paths`: a frase do próprio resultado, sem prefixo (ex.: "lasrecomp.exe encontrado." em Tinta 2, "Esta pasta não existe." em Erro; o texto diz, a cor só reforça). Se o Rust devolver mensagem vazia, o front usa um texto padrão por campo, então a linha nunca fica em branco, e o campo com problema ganha aresta Erro.
- Salvar com problema mostra um aviso em linha com "Salvar mesmo assim"; nunca salva calado. "Configurações salvas." fica 6s e é anunciado (`aria-live`).

### Inputs / Fields
- **Style:** fundo quase preto a 60%, aresta interna de 1px, cantos de 12px, texto em mono 13px, cursor ciano.
- **Focus:** a aresta vira um contorno ciano de 2px.
- **Foco de botão (teclado/controle):** todo botão, aba ou linha navegável em foco vira o inverso: fundo Tinta, texto escuro, contorno Tinta de 3px a 3px de distância e halo claro de 9px; o glifo dentro escurece. O botão primário (já Tinta) se distingue pelo contorno e halo. Aba selecionada = pílula Tinta sem contorno; aba em foco = contorno Tinta (a não selecionada ganha véu claro). O ciano que respira é só da capa selecionada.

### Navigation
- **Prateleira:** setas, analógico ou direcional trocam o jogo (com o foco na prateleira, no corpo ou fora das zonas; nas ações andam entre os itens). O time e "Próximos jogos" são informativos e não são paradas de foco.. Clique num jogo não selecionado (inclusive o anterior, que espia à esquerda) o seleciona. A, Enter ou clique no jogo selecionado jogam; num jogo não compilado levam à página do jogo na seção Importação. X (tecla X) abre a página do jogo (opções).
- **Página do jogo:** A joga (ou ativa o botão em foco), Y abre a pasta, LB/RB, Q/E ou esquerda/direita trocam a aba, B, Esc ou Backspace voltam à inicial com o mesmo jogo selecionado (com uma confirmação aberta, B primeiro desiste dela). A dica "No jogo, F1 abre as configurações" fica discreta logo abaixo do Jogar (só com o jogo compilado).
- **Abas:** trilho em pílula translúcida. A aba selecionada é uma pílula Tinta que desliza até ela (320ms). Troca com LB/RB ou Q/E.
- **Lista vazia / erro ao carregar:** no lugar da prateleira, título, explicação e ações ("Abrir Configurações"; no erro, "Tentar de novo" primeiro e o texto técnico em mono). Nunca tela em branco.
- **Rodapé:** status do jogo à esquerda, dicas de botão à direita, sobre vidro escuro. As dicas seguem o contexto (inicial, foco na página, folha, erro aberto, confirmação), esmaecem o que não se aplica e mostram glifos do controle ou do teclado (Enter, Esc, Q/E, M) conforme a última entrada. O title ID aparece só como linha discreta (`.fine`) "Código do jogo: …" no fim da aba Versão e DLC (e no detalhe técnico da aba Mods). As abas da página do jogo são substantivos: Versão e DLC, Mods, Save, Importação.
- **Aviso de sincronia na inicial:** o hub lê `save_info` de cada jogo da prateleira na carga e no foco da janela (e ao trocar de jogo, se não houver cache). Se o save deste PC mudou depois da última sincronização, a linha de meta do jogo selecionado (inicial e página) ganha o fato "Save não sincronizado" em Aviso, e o rodapé diz "Pronto para jogar · save não sincronizado". Sync não configurada ou ilegível não gera aviso na inicial.
- **Barra superior:** só a marca "SwitchOS" e um relógio discreto (Tinta 2, tamanho de corpo). A versão do hub fica no rodapé da folha de configurações.

### Primeiro uso (bloco de configuração)
- **Quando:** ao abrir, ao voltar à janela e depois de salvar as configurações, o hub roda `check_paths` com a config salva. Se algum dos três itens (Pasta do programa, Pasta dos jogos, prod.keys) falhar, o bloco ocupa o lugar da prateleira e do time (não é modal; "Próximos jogos" continua abaixo). O rodapé diz "Falta configurar o hub" com ponto Aviso.
- **Forma:** título no tamanho da lista vazia ("Vamos preparar o hub"), uma frase, e os três passos como linhas separadas por fio de 1px (nunca cards): pílula de estado em texto ("Feito" em OK, "Falta" em Aviso), nome do passo e uma linha em linguagem simples. O prod.keys repete o compromisso do produto: chaves do seu próprio Switch, o hub não baixa jogos nem chaves.
- **Ação:** pílula Tinta "Configurar" com a dica A, primeira zona do controle e foco padrão. A na inicial (com o bloco na tela) também configura. Abre a folha com o foco no primeiro campo pendente.
- **Primeira execução:** o Rust não diz se já existe `hub.json`, então um marcador local (`hub.setup-seen`) faz a folha abrir sozinha uma única vez, só se houver item pendente.
- **Saída:** quando os três ficam ok, o bloco sobe 8px e desfoca até sumir (240ms, `--ease-out`) e a prateleira entra com a animação do nome. Com movimento reduzido, só troca.
- **Primeiro jogo:** tudo ok e nenhum jogo importado: abaixo da prateleira, "Importe seu primeiro jogo" (rótulo de seção) e uma frase com a dica A, que já leva à seção Importação.
- **Save:** na seção Save, uma linha discreta diz que o save fica na pasta do programa (`save`), fora da pasta do jogo, e que importar de novo não mexe nele.

### Aba Save (o save é sagrado)
- **Fonte:** comando só-leitura `save_info` (lib.rs). Acha a raiz do SwitchOS (pasta com `games.toml` e `switchos.py`, subindo do executável ou da pasta do programa), lê `[sync] cloud_dir` e `keep_backups`, e então `<cloud_dir>/saves/<TID>`, `<cloud_dir>/backups/<TID>/<AAAAMMDD-HHMMSS>-<origem>` e `<cloud_dir>/state.json`. O hub nunca sincroniza; a única escrita é `restore_backup` (abaixo).
- **Forma:** frase de estado no tamanho de título ("Protegido por N cópias de segurança", número em OK; ou "Seu save está só neste PC"), depois uma faixa de três colunas separadas por filete (Neste PC, Na nuvem, Última sincronização com os lados), uma pílula Aviso se o save do PC é mais novo que a última sync, seguida de uma nota com o comando real para sincronizar (`python switchos.py sync <jogo>`, selecionável; o hub não tem botão de sync), as ações (Abrir pasta do save, Abrir cópia na nuvem) e a lista "Cópias de segurança" (até 10, mais recentes primeiro): cada linha tem data, "versão que estava no PC/Android/na nuvem" e dois botões: "Restaurar esta cópia" (pílula) e "Abrir pasta" (`.btn.quiet`, sem fundo). Abaixo, só notas curtas (`.fine`, máx. 68ch).
- **Controle:** cada linha da lista e a fileira de ações são zonas próprias (`data-zone`): cima/baixo anda linha a linha, esquerda/direita entre Restaurar e Abrir pasta.
- **Restaurar (`restore_backup(key, backup_name)`, lib.rs):** aceita só um nome simples no formato `<AAAAMMDD>-<HHMMSS>-<origem>[-n]` que exista como pasta em `backups/<TID>`; recusa com importação deste jogo em andamento ou com `lasrecomp.exe` aberto (via `tasklist`, descontando os `--build-game` das importações; se o Windows não responder, recusa). Ordem: copia a cópia para `save_0.switchos-tmp`, guarda o save atual como cópia `pc` igual ao `backup()` do `switchos.py` (mesmo nome, sufixo `-n`, poda por `keep_backups` sem apagar a cópia nova nem a escolhida), e troca as pastas por rename (`save_0.switchos-old`, como o `replace_tree`). Se a troca falha, o save antigo volta. Nuvem e Android não são tocados: a próxima sync propaga.
- **Fluxo:** confirmação em linha na própria linha (realce de vidro com filete ciano): "Restaurar a cópia de <data> (<origem>)? Seu save atual vira uma cópia de segurança antes.", foco em "Manter como está", B desiste. Durante: a linha mostra um giro pequeno e "Restaurando…", o rodapé diz "Restaurando o save…" e Jogar/sair da página ficam bloqueados. Sucesso: aviso fixo verde acima da lista (data restaurada, nome da cópia criada, que a próxima sync propaga) e toast curto. Erro: toast de erro com título próprio (jogo aberto, importação em andamento, cópia não encontrada, sem permissão/pasta em uso, sync não configurada), sempre dizendo que o save não foi trocado.
- **Sem configuração / erro:** pílula Aviso "Não configurada" (ou "Não deu para ler") e uma frase simples; o resto da aba continua funcionando.

### Prateleira de Jogos (assinatura)
A peça que define o hub. O jogo selecionado cresce para 240px com contorno ciano de 4px que respira em 2.4s. A fileira desliza em 420ms com a curva de gaveta. Ao mesmo tempo, o fundo cruza para a arte do novo jogo (700ms) e o nome entra subindo 6px, saindo do desfoque. Jogos sem arte mostram o nome curto escrito no próprio quadro, na cor do jogo.

## Do's and Don'ts

### Do:
- **Do** deixar a arte do jogo selecionado tingir o fundo, desfocada (blur 64px) sob gradientes escuros.
- **Do** mostrar, ao lado de cada ação importante, o botão do controle que a dispara.
- **Do** usar a curva de saída forte (`cubic-bezier(0.23, 1, 0.32, 1)`) para entradas e a de gaveta (`cubic-bezier(0.32, 0.72, 0, 1)`) para o que se move na tela, sempre abaixo de 450ms.
- **Do** trocar movimento por esmaecimento simples quando o sistema pede menos movimento, e vidro por superfície sólida quando pede menos transparência.

### Don't:
- **Don't** criar tema claro. O hub é só escuro.
- **Don't** voltar para cara de painel administrativo: sidebar com lista de jogos, grade de cards iguais ou tabelas.
- **Don't** usar o vermelho Switch, logos ou qualquer elemento que imite a identidade da Nintendo.
- **Don't** usar glifos Unicode ou emoji no lugar de ícones. Ícones são SVG de traço de 1.8px.
