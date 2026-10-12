# EffectCraft Live Controls for OpenDeck

[Português do Brasil](#português-do-brasil) · [Contributing](CONTRIBUTING.md) · [EffectCraft](https://github.com/storytold/effectcraft) · [OpenDeck](https://github.com/nekename/OpenDeck)

An independent Linux-only [OpenDeck](https://github.com/nekename/OpenDeck) plugin that turns a Stream Deck, Stream Deck+, or D200X-style encoder deck into a practical live control surface for [EffectCraft](https://github.com/storytold/effectcraft). Use dedicated keys for transport, frame and keyframe navigation, layer work, effects, and the Render Queue; use encoders for relative opacity, rotation, and scale edits.

The dedicated controls connect to the running EffectCraft project's control bridge, so they operate on the composition and selection already on screen. The configurable `effectcraft-cli` action is still included for the occasional workflow that needs a command not covered by a ready-made control; it is no longer the primary way to use the deck.

Each action stores an executable path and a JSON array of arguments. The plugin launches the executable directly, without `sh -c` or another shell. That makes JSON parameters, spaces in paths, and untrusted filenames safe to pass as arguments.

## Features

- **EffectCraft Live Control**: playback, frame/keyframe navigation, split layer, add text/solid/shape/adjustment layers, add an effect, toggle transform keyframes, and show/start/stop the Render Queue.
- **EffectCraft Property Dial** for Stream Deck+ and Ulanzi D200X-style encoders: turn to adjust the selected layer's opacity, rotation, scale, or a custom numeric property; press to toggle its keyframe at the playhead.
- The live controls talk to EffectCraft's control bridge, so they operate on the project and selection already on screen.
- One configurable **Run EffectCraft CLI** escape hatch for keypad keys and encoders.
- Supports every EffectCraft CLI subcommand, option, and future addition: the action forwards the exact JSON argument array you configure.
- Loads the installed CLI's command or tool catalog in the Property Inspector and creates an app-specific, editable action template.
- Configurable executable path, working directory, timeout (1–300 seconds), label, and optional output summary on the key.
- Test a configuration in the Property Inspector before assigning it to a key.
- Linux x86_64 release ZIP created on every CI run; tagged versions are published as GitHub Releases.

## Real editing workflows

- **Cut a talking-head edit:** use Previous/Next Frame to find the cut, press **Split Layer**, then use the Opacity dial and its press action to keyframe a quick fade without reaching for the mouse.
- **Build a motion-graphics pass:** add a Text, Solid, Shape, or Adjustment layer, apply an effect, and navigate only the animated keyframes while the encoder makes small relative rotation or scale corrections.
- **Send a review render:** show the Render Queue, start the queued render, and keep a Stop Render key on the deck for a fast abort when the work changes.

## Requirements

- Linux x86_64
- OpenDeck with OpenAction-compatible plugin support
- EffectCraft and `effectcraft-cli` installed and available in `PATH`, or its absolute path entered in the Property Inspector
- For the live controls, launch the app with `effectcraft --control 9877` (or set the matching port in each action)

## Install

1. Download `getcraft-effectcraft-opendeck-linux-x86_64.zip` from the [latest release](../../releases/latest).
2. In OpenDeck, open **Plugins** → **Install from file** and select the ZIP. Do not extract or rename the `.sdPlugin` directory.
3. Start EffectCraft with `effectcraft --control 9877`.
4. Drag **EffectCraft Live Control** to keys for the timeline, layer, effects and Render Queue actions. Add **EffectCraft Property Dial** to an encoder for opacity, rotation or scale.
5. The generic **Run EffectCraft CLI** action remains available for any command not covered by a dedicated control.

## Configuration and examples

The **Arguments** field must contain a JSON array of strings. It is not a shell command.

| Goal | Arguments |
| --- | --- |
| List the installed command catalog | `["commands", "--json"]` |

Use the exact argument syntax shown by `effectcraft-cli --help` for project, file, export, and live-app control operations. The generic array accepts every documented subcommand and option.

The Property Inspector's **Load command catalog** button asks the installed CLI for the catalog supported by this application. Selecting an ID fills an app-specific, editable argument template; use **Test command** after adding the parameters and options required by your installed CLI version. The generic JSON array remains the canonical path to every CLI capability.

## Suggested deck layout

| Control | Suggested action |
| --- | --- |
| Key 1 | Play / Stop current preview |
| Key 2 / 3 | Previous / Next frame |
| Key 4 / 5 | Previous / Next keyframe |
| Key 6 | Split Layer |
| Key 7 | Add Text |
| Key 8 | Apply Effect (for example Gaussian Blur) |
| Key 9 | Show Render Queue |
| Encoder 1 | Property Dial: Opacity, step 1 |
| Encoder 2 | Property Dial: Rotation, step 1 |
| Encoder 3 | Property Dial: Scale, step 1 |

The dial reads the live selected layer before changing it, so it makes a relative change instead of overwriting a value. Its press action toggles a keyframe for that same property.

## CI/CD and release

Pull requests and pushes to `main` run formatting checks, unit tests, and `scripts/package.sh`. The workflow uploads the Linux installer ZIP as an artifact. Pushing a semantic tag such as `v0.1.0` publishes the same ZIP as a GitHub Release.

To package locally:

```bash
rustup target add x86_64-unknown-linux-gnu
bash scripts/package.sh
```

The output is `dist/getcraft-effectcraft-opendeck-linux-x86_64.zip`, containing exactly `wt.luizgustavo.getcraft.effectcraft.sdPlugin/` at its root.

## Local test installation

On Linux, build the local installation package with:

```bash
bash scripts/install-local.sh
```

By default, the installer only builds the ZIP and does not change OpenDeck's active plugin directory. This prevents development plugins from disrupting an active setup. To deliberately test one plugin in the active OpenDeck installation, opt in explicitly:

```bash
OPENDECK_ALLOW_LIVE_INSTALL=1 bash scripts/install-local.sh
```

Install one development plugin at a time, then use OpenDeck's plugin reload control or restart it manually. The installer itself never restarts or reloads OpenDeck. To target a separate configuration, set `OPENDECK_PLUGINS_DIR` before running the command.

## Platform roadmap

| Platform | Status |
| --- | --- |
| Linux x86_64 | Supported now |
| Windows | Planned; not implemented or packaged yet |
| macOS | Planned; not implemented or packaged yet |

## Security and governance

The plugin never evaluates a shell command. It executes the chosen binary with the strings in the JSON array, and it limits a process to five minutes.

This is an open-contribution repository. Direct write/merge access is reserved for `LuizGustavoWT`; outside contributors use forks and pull requests. GitHub does not have a separate permission that only controls closing issues: users without repository write/triage access cannot close other users' issues, so retaining those permissions only for the maintainer satisfies the governance model. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. EffectCraft and OpenDeck are independent upstream projects with their own licenses and release policies.

---

# Português do Brasil

Plugin independente, somente para Linux, do [OpenDeck](https://github.com/nekename/OpenDeck) que transforma Stream Deck, Stream Deck+ ou uma mesa com encoders no estilo D200X em uma superfície de controle ao vivo do [EffectCraft](https://github.com/storytold/effectcraft). Há teclas prontas para reprodução, frames, keyframes, camadas, efeitos e fila de renderização, além de encoders para ajustes relativos de opacidade, rotação e escala.

As ações prontas se conectam à bridge do projeto EffectCraft já aberto e trabalham sobre a composição e a seleção visíveis na tela. A ação configurável `effectcraft-cli` continua disponível para o fluxo pontual que não tiver uma tecla pronta, mas não é mais a forma principal de usar o deck.

Cada ação guarda o executável e uma lista JSON de argumentos. O plugin chama o executável diretamente, sem `sh -c` nem outro shell. Assim, JSON, caminhos com espaços e nomes de arquivos não confiáveis são passados com segurança.

## Recursos

- **EffectCraft Live Control**: reprodução, navegação por frames/keyframes, corte da layer, novas layers, efeitos e fila de renderização.
- **EffectCraft Property Dial**: para encoder do Stream Deck+ ou Ulanzi D200X, ajusta opacidade, rotação, escala ou uma propriedade numérica customizada; pressionar alterna o keyframe no playhead.
- Os controles usam a bridge do EffectCraft e alteram o projeto que já está aberto na tela.
- Ação configurável **Run EffectCraft CLI** continua disponível para comandos avançados.
- Todos os subcomandos, opções e futuras adições da CLI: a ação repassa exatamente o vetor JSON configurado.
- Catálogo dinâmico de comandos ou ferramentas da CLI no Property Inspector, com modelo de ação editável e específico do aplicativo.
- Caminho do executável, diretório de trabalho, timeout (1–300 s), rótulo e resumo de saída configuráveis.
- Botão de teste no Property Inspector antes de associar a ação a uma tecla.
- ZIP Linux x86_64 gerado em toda execução da CI; tags são publicadas como Releases.

## Fluxos de edição reais

- **Cortar uma entrevista:** use Frame anterior/próximo para encontrar o corte, pressione **Split Layer** e use o dial de Opacidade — incluindo o pressionar para criar keyframe — para montar uma transição rápida sem recorrer ao mouse.
- **Criar motion graphics:** adicione Texto, Sólido, Forma ou Adjustment Layer, aplique um efeito, navegue somente pelos keyframes animados e faça correções pequenas de rotação ou escala pelo encoder.
- **Enviar uma renderização para revisão:** mostre a Render Queue, inicie a fila e mantenha uma tecla Stop Render no deck para interromper rapidamente quando o trabalho mudar.

## Requisitos e instalação

Você precisa de Linux x86_64, OpenDeck compatível com OpenAction e EffectCraft com `effectcraft-cli` no `PATH` (ou o caminho absoluto no Property Inspector). Para as ações ao vivo, abra o EffectCraft com `effectcraft --control 9877`.

1. Baixe `getcraft-effectcraft-opendeck-linux-x86_64.zip` na [release mais recente](../../releases/latest).
2. No OpenDeck, abra **Plugins** → **Install from file** e escolha o ZIP. Não extraia nem renomeie a pasta `.sdPlugin`.
3. Inicie o EffectCraft com `effectcraft --control 9877`.
4. Arraste **EffectCraft Live Control** para as teclas e **EffectCraft Property Dial** para os encoders.
5. Use a ação genérica **Run EffectCraft CLI** apenas para um comando não coberto pelas ações prontas.

## Configuração

O campo **Arguments** precisa ser um vetor JSON de strings; ele não é um comando de shell.

| Objetivo | Arguments |
| --- | --- |
| Listar o catálogo de comandos instalado | `["commands", "--json"]` |

Use a sintaxe exata de `effectcraft-cli --help` para operações de projeto, arquivo, exportação e controle do aplicativo em execução. O vetor genérico aceita todos os subcomandos e opções documentados.

**Load command catalog** consulta a CLI instalada pelo catálogo compatível com este aplicativo. Ao escolher um ID, o inspetor preenche um modelo de argumentos específico e editável; use **Test command** depois de adicionar os parâmetros e opções exigidos pela versão instalada da CLI. O vetor JSON genérico continua sendo o caminho para toda a CLI.

## CI/CD, roadmap e contribuição

PRs e pushes em `main` executam formatação, testes e `scripts/package.sh`. O ZIP vira artefato; uma tag como `v0.1.0` cria uma GitHub Release.

Para gerar o pacote local no Linux, execute `bash scripts/install-local.sh`. Por padrão, o instalador gera o ZIP e não altera o diretório de plugins ativo do OpenDeck, protegendo a configuração em uso. Para testar deliberadamente um único plugin na instalação ativa, confirme explicitamente:

```bash
OPENDECK_ALLOW_LIVE_INSTALL=1 bash scripts/install-local.sh
```

Instale um plugin de desenvolvimento por vez e então recarregue-o pelo OpenDeck ou reinicie manualmente. O instalador jamais reinicia ou recarrega o OpenDeck. Para usar outra configuração, defina `OPENDECK_PLUGINS_DIR` antes do comando.

| Plataforma | Situação |
| --- | --- |
| Linux x86_64 | Suportado agora |
| Windows | Planejado; ainda não implementado nem empacotado |
| macOS | Planejado; ainda não implementado nem empacotado |

Contribuições são bem-vindas via fork e pull request. Apenas `LuizGustavoWT` possui acesso direto para merge e para fechar issues de terceiros. O GitHub não oferece uma permissão separada somente para fechar issues; sem acesso de escrita/triagem, colaboradores públicos não conseguem fechar issues de outras pessoas. Veja [CONTRIBUTING.md](CONTRIBUTING.md).

## Licença

MIT. EffectCraft e OpenDeck são projetos upstream independentes, cada um com sua própria licença.
