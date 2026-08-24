# ZoomOSC Lite

Pequeno controlo OSC open source para o cliente Zoom no macOS. Permite controlar
partilha, microfone, vídeo e perfis de áudio a partir de um controlador OSC como
o Bitfocus Companion.

Não contém código nem SDK do Zoom. Controla o cliente oficial através da API de
Acessibilidade do macOS.

## Compilar

Requer macOS 13 ou posterior, Xcode Command Line Tools, Rust e
[mise](https://mise.jdx.dev/).

```sh
mise run app
```

A aplicação é criada em `dist/ZoomOSC Lite.app`.

## Janela de configuração

Ao abrir, a aplicação apresenta uma janela nativa onde é possível escolher:

- **Apenas este Mac** (`127.0.0.1`), o modo seguro predefinido;
- **Rede local** (`0.0.0.0`), para receber OSC de outro dispositivo;
- porta UDP, com `9000` como valor predefinido.

O botão **Aplicar e reiniciar** guarda a configuração nas preferências do macOS
e reinicia apenas o servidor OSC. A configuração é restaurada no próximo
arranque. A opção **Iniciar ao entrar no macOS** permite lançar automaticamente
a aplicação no início da sessão, através do mecanismo nativo do macOS.

## Primeira execução

1. Abre `ZoomOSC Lite.app`.
2. Em **Definições do Sistema → Privacidade e Segurança → Acessibilidade**,
   autoriza **ZoomOSC Lite**.
3. Volta a abrir a aplicação.

Ao abrir pela primeira vez, a aplicação escuta apenas em `127.0.0.1:9000`. Para
execução direta do helper Rust, também é possível usar:

```sh
zoomosc-lite serve 127.0.0.1:9000
```

Não encaminhes esta porta UDP na Internet.

> O protocolo UDP desta versão não tem autenticação. Usa o modo **Rede local**
> apenas numa rede de confiança.

## Comandos OSC

| Endereço | Ação |
| --- | --- |
| `/zoom/share/camera/start` | Abre Avançadas, seleciona a segunda câmara e partilha |
| `/zoom/me/startCameraShare` | Alias compatível com o estilo do ZoomOSC |
| `/zoom/share/stop` | Para a partilha atual |
| `/zoom/me/stopShare` | Alias compatível com o estilo do ZoomOSC |
| `/zoom/audio/mute` | Desativa o microfone se estiver ativo |
| `/zoom/audio/unmute` | Ativa o microfone se estiver desativado |
| `/zoom/video/on` | Liga o vídeo se estiver desligado |
| `/zoom/video/off` | Desliga o vídeo se estiver ligado |
| `/zoom/audio/profile/noise-removal` | Seleciona remoção de ruído |
| `/zoom/audio/profile/isolation` | Seleciona isolamento personalizado |
| `/zoom/audio/profile/original` | Seleciona som original para músicos |
| `/zoom/audio/profile/live-performance` | Seleciona áudio de performance ao vivo |

Aliases adicionais no estilo ZoomOSC: `/zoom/me/mute`, `/zoom/me/unmute`,
`/zoom/me/startVideo` e `/zoom/me/stopVideo`.

Os comandos de áudio e vídeo são absolutos e idempotentes: a aplicação lê
primeiro o estado atual e não envia um atalho quando o estado pedido já está
ativo.

O utilitário aceita mensagens OSC normais; os argumentos são ignorados nesta
versão porque estas ações não precisam deles.

## Diagnóstico

Com o Zoom visível, executa:

```sh
zoomosc-lite inspect
```

Isto lista os nomes que a versão instalada do Zoom expõe à Acessibilidade. É
útil caso uma atualização ou tradução altere os textos dos controlos.

## Licença

MIT

## Criar uma release

```sh
mise run release
```

Cria em `release/` um ZIP macOS ARM64 e o respetivo ficheiro SHA-256. A
assinatura atual é ad-hoc, adequada para testes e distribuição privada; uma
release pública sem avisos do Gatekeeper requer um certificado Developer ID e
notarização Apple.

## Limitações

O ZoomOSC Lite usa a interface de Acessibilidade do cliente Zoom. Alterações na
interface do Zoom podem exigir uma atualização dos seletores. Os nomes mais
comuns em português e inglês estão incluídos.

## Contribuir

Issues e pull requests são bem-vindos. Antes de enviar alterações, executa:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```
