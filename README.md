# Redoma

Executa o **aplicativo oficial Certisign** (`com.certisign.certisign`) no
Linux, isolado dentro de um contêiner Android ([Waydroid](https://waydro.id)),
numa janela própria em formato de celular. Cadastro, e-token e autorização
acontecem exclusivamente dentro do aplicativo oficial.

> **Projeto independente.** Não é afiliado, patrocinado nem homologado pela
> Certisign. "Certisign" é marca de seu titular e aparece aqui apenas para
> identificar o aplicativo compatível. O app é instalado pelo canal oficial
> (Play Store) e não é modificado, redistribuído nem descompilado para uso.

> **Somente Linux, somente em contêiner.** O Redoma não roda no Windows nem no
> macOS e não executa o app Android "solto" no sistema: o Android inteiro vive
> num contêiner LXC gerenciado pelo Waydroid, com sistema de arquivos, processos
> e rede próprios. Requer sessão gráfica **Wayland** e kernel com suporte a
> Binder. Leia a seção [Segurança](#segurança) antes de usar.

O planejamento e o histórico estão em [roadmap.MD](roadmap.MD).

## Para quem é

Nasceu num escritório de advocacia que usa Linux e precisa do **certificado
digital ICP-Brasil em nuvem** no dia a dia forense: peticionar, consultar
processos e acessar sistemas públicos sem depender do celular para gerar o
e-token a cada login.

| Sistema | Uso típico | Situação com o Redoma |
| --- | --- | --- |
| GERID (INSS/Dataprev) | Atendimento INSS, previdenciário | E-token aceito no ambiente de referência |
| PJe | Processo Judicial Eletrônico (Justiça Federal, do Trabalho e estaduais) | A validar |
| eproc | Processo eletrônico (TRF4 e tribunais que o adotam) | A validar |
| Projudi | Processo eletrônico em tribunais estaduais | A validar |

O Redoma resolve **a geração e a autorização no app oficial**. A integração do
certificado com o navegador ou com o assinador do tribunal é uma etapa separada
e pode exigir os componentes próprios de cada sistema. Quem usa certificado A3
em token USB ou cartão não precisa do Redoma: esses dispositivos funcionam
nativamente no Linux via PKCS#11.

Cada escritório deve avaliar o uso conforme suas políticas de segurança, os
termos da Certisign e as normas de uso do certificado digital.

## Segurança

### O que o Redoma faz e não faz

- **Não recebe credenciais.** Não lê, solicita, armazena nem transmite PIN,
  senha, e-token, certificados ou códigos de autenticação. Tudo isso fica dentro
  do app oficial, no contêiner.
- **Não acessa a rede.** O launcher não faz requisições, não tem telemetria e
  não baixa nada. Quem acessa a internet é o Android do contêiner e o app oficial.
- **Não modifica o app.** O APK, sua assinatura e seu código permanecem os
  publicados na Play Store.
- **Não usa root.** Roda como usuário comum, sem `sudo`. Só aciona o Waydroid
  pelos seus comandos públicos (`session`, `app`, `prop`, `status`).
- **Abre um único app.** O pacote é fixo no código; não aceita outro pacote
  por argumento.
- **Não tem dependências Cargo externas.** O código próprio é um único arquivo
  ([src/main.rs](src/main.rs)) e pode ser auditado por completo.

### Modelo de isolamento: contêiner, não máquina virtual

O Android roda num contêiner LXC iniciado pelo serviço de sistema do Waydroid
(`waydroid-container`, executado como root, instalado pelo pacote da
distribuição). Esse modelo isola bem o uso normal, mas **compartilha o kernel
do Linux**; não oferece a mesma barreira de uma máquina virtual.

| Recurso | Situação |
| --- | --- |
| Arquivos pessoais (`/home`, documentos, downloads) | **Não montados** no contêiner. |
| Dados do Android (cadastro do app, contas) | Pasta própria em `~/.local/share/waydroid/data`, montada como `/data`. |
| Sistema Android | Imagens somente leitura em `/var/lib/waydroid`. |
| Rede | Interface própria (`waydroid0`) com NAT; o Android acessa a internet. |
| Tela e áudio | Sockets Wayland e PulseAudio da sessão do usuário. |
| Área de transferência | **Pode ser sincronizada** entre Linux e Android pelo Waydroid (quando o suporte de clipboard está instalado). Evite copiar senhas. |
| Notificações | Encaminhadas pelo Waydroid para a área de trabalho. |
| GPU e dispositivos | Nó de renderização da GPU, Binder, FUSE, TUN e UHID. |
| Kernel | **Compartilhado**, com capacidades restritas e perfil seccomp do Waydroid. |

Consequências práticas: qualquer pessoa com acesso à sua conta Linux pode abrir
o app, assim como num celular desbloqueado. Proteja a conta com senha e bloqueio
de tela, e use a senha do próprio app Certisign. Quem tem root na máquina tem
acesso aos dados do contêiner.

### Ajuste de ambiente necessário

O app oficial recusa ambientes que identifica como emulador. No ambiente de
referência, foi necessário um ajuste reversível **no Android do contêiner, não
no app**: a propriedade Java `os.arch` informada às apps (`dalvik.vm.extra-opts
= -Dos.arch=aarch64` em `/var/lib/waydroid/waydroid.cfg`). Não são alterados
fabricante, modelo, fingerprint, root, bootloader nem atestação. Diagnóstico e
procedimento de reversão: [docs/emulator-block.md](docs/emulator-block.md) e
[docs/experiment-os-arch.md](docs/experiment-os-arch.md). O app pode mudar suas
verificações a qualquer atualização; use por sua conta e conforme os termos da
Certisign.

### O que o `setup` altera no seu sistema

- Propriedades do Waydroid listadas em [Compilar e usar](#compilar-e-usar).
- Atalhos em `~/.local/share/applications`: cria `redoma.desktop` e marca os
  atalhos `waydroid.*.desktop` com `NoDisplay=true` (apenas ocultos).
- Favorito da dock do GNOME, se apontava para o atalho do Waydroid.
- Registro da sessão em `~/.local/state/redoma/session.log` (saída do Waydroid,
  sem credenciais).

Para desfazer: apague `redoma.desktop` e `~/.local/state/redoma`, remova as
linhas `NoDisplay=true` dos atalhos `waydroid.*` e redefina as propriedades com
`waydroid prop set`. `redoma stop` encerra o Android; os dados do app
permanecem até você remover os dados do Waydroid.

### Relatar vulnerabilidades

Não abra issue pública para falhas de segurança. Use o
[relato privado de vulnerabilidades do GitHub](../../security/advisories/new)
deste repositório.

## Estado atual

A interface é por terminal e por um atalho no menu. O projeto não é um port do
Android para Rust: usa o Waydroid instalado como dependência externa. Ao abrir
o app, o launcher inicia a sessão Android em segundo plano (fora do terminal),
aguarda o fim do boot e então é substituído pelo comando do Waydroid com `exec`;
não fica nenhum processo do Redoma residente.

Validado no ambiente de referência (Fedora 44, x86-64, GNOME/Wayland, Waydroid
1.6.3, Certisign 1.4.1): abertura da carteira, geração de e-token aceito no GERID
e persistência após encerrar e reiniciar o Android. Pendentes: teste após
reinício do computador, outros sistemas (Projudi, PJe, eproc) e outras
distribuições. Detalhes em [docs/validation.md](docs/validation.md).

O pacote configurado corresponde ao app **Certisign**, não ao remoteID ou MobileID.

## Compilar e usar

```sh
cargo build --release
./target/release/redoma doctor
./target/release/redoma status
./target/release/redoma setup   # uma vez; repetir é seguro
./target/release/redoma open
```

`setup` aplica as propriedades do Waydroid abaixo (reiniciando a sessão só se
alguma mudou) e oculta do menu do Linux os atalhos dos outros apps Android:

| Propriedade | Valor | Motivo |
| --- | --- | --- |
| `persist.waydroid.multi_windows` | `false` | O modo multi-janela exibiu o app reduzido dentro de uma moldura transparente no GNOME. |
| `persist.waydroid.width` / `height` | `450` / `800` | Janela única em retrato 9:16. |
| `persist.waydroid.suspend` | `false` | Sem isso, fechar a janela congela o contêiner e as notificações param. |

Contatos, Mensagens, Play Store e similares fazem parte da imagem GAPPS e
continuam instalados no Android; apenas seus atalhos ficam ocultos
(`NoDisplay=true`, que o Waydroid preserva ao regenerar os atalhos).

O `setup` também instala o atalho próprio `redoma.desktop`, que chama
`open` com o caminho atual do executável, e oculta o atalho do Certisign
gerado pelo Waydroid. Motivo: ao fechar a janela, o Waydroid mantém o app em
primeiro plano no Android, mas passa a ignorá-lo até ele sair da tela, então
`waydroid app launch` não traz a janela de volta. Com a sessão já ativa, `open`
traz por um instante o launcher do Android, que nunca vira janela e limpa
essa marca, e então abre o Certisign. O app não é reiniciado nesse processo.
Se o executável mudar de lugar, rode `setup` novamente.

No GNOME a janela do Waydroid não recebe barra de título, pois o Mutter não
desenha decorações para clientes Wayland. Minimize com Super+H, feche com
Alt+F4 ou pelo X na Visão Geral; Alt+Tab funciona normalmente.

Execute na sessão gráfica Wayland do usuário, sem `sudo`. O serviço de
contêiner deve estar disponível e o Waydroid inicializado previamente.

Para encerrar o Android:

```sh
./target/release/redoma stop
```

Esse comando para a **sessão Waydroid inteira**, inclusive outros aplicativos.
Não apaga o cadastro ou os dados persistentes. Fechar a janela do Certisign ou
o terminal não encerra o Android: ele continua em segundo plano, recebendo
notificações. Com o Android parado, notificações de autorização ficam
indisponíveis. A saída da sessão fica em `~/.local/state/redoma/session.log`.

`doctor` verifica somente se há um executável Waydroid no PATH e um socket
Wayland disponível. Não confirma inicialização do contêiner, instalação do
Certisign, conectividade ou funcionamento do e-token. `status` exibe o resultado
do Waydroid. Os comandos encaminhados preservam sua saída e código de saída;
algumas falhas do próprio Waydroid podem ser reportadas apenas como mensagens.

## Preparação no Fedora

O ambiente encontrado é Fedora 44, x86-64, com Wayland. O Waydroid ainda não
estava instalado durante a implementação inicial do launcher. Em 18/10/2026,
foi instalado o Waydroid 1.6.3 de referência e concluída a inicialização do Android
com Play Store. O Certisign 1.4.1 inicialmente exibiu bloqueio de emuladores.
Após um ajuste reversível no ambiente, o titular confirmou a abertura da carteira.
O diagnóstico está em [docs/emulator-block.md](docs/emulator-block.md), e o
experimento, com backup e restauração, está em
[docs/experiment-os-arch.md](docs/experiment-os-arch.md).
O titular confirmou funcionamento no primeiro teste proposto de e-token/acesso
ao GERID e após encerrar e iniciar novamente o Android. O teste após reboot e
a integração do navegador Linux ainda precisam de validação específica.
O acompanhamento está em [docs/validation.md](docs/validation.md).
O levantamento inicial das dependências está em
[docs/runtime-inventory.md](docs/runtime-inventory.md).

Procedimento usado para preparar o ambiente de referência:

1. Instale o Waydroid pelo repositório do Fedora: `sudo dnf install waydroid`.
2. Abra o Waydroid e conclua sua inicialização seguindo a documentação oficial.
3. Instale o Certisign pela Play Store em uma imagem GAPPS. A disponibilidade
   do app e a certificação da instalação precisam ser verificadas nessa etapa.
4. Cadastre o dispositivo pelo fluxo do app oficial e teste uma autenticação.

O Redoma não instala pacotes do sistema, não baixa APK de terceiros e não
altera a imagem Android. Essa preparação é feita separadamente nesta fase de
validação e não representa ainda o instalador integrado do produto.

## Validação necessária antes de otimizar

- Conseguir instalar e abrir o pacote oficial na arquitetura da máquina.
- Concluir o cadastro e obter um e-token aceito em um acesso real.
- Confirmar persistência após `stop`, nova abertura e reinício do computador.
- Verificar se o fluxo usado depende de autorização por notificação.
- Medir tempo de abertura, memória e CPU do conjunto Android + app, em repouso
  e durante o uso. Não há metas de consumo demonstradas nesta versão.

A integração do certificado com o navegador ou sistema de assinatura é uma
etapa separada. Executar o app não instala o DesktopID no Linux.

## Verificação do código

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Os testes de integração usam um executável Waydroid simulado e sockets locais.
Verificam a seleção exclusiva do pacote oficial, início da sessão antes do
lançamento, aplicação idempotente das propriedades, ocultação de atalhos,
pré-requisito de Wayland,
encerramento explícito, ausência de inicialização no diagnóstico e propagação
de falhas do processo. Não executam autenticação nem acessam certificados.

## Licença

A definir. Até lá, todos os direitos reservados ao autor; abra uma issue se
quiser usar ou contribuir.

## Referências

- [Certisign oficial na Play Store](https://play.google.com/store/apps/details?id=com.certisign.certisign)
- [Instalação do Waydroid](https://docs.waydro.id/usage/install-on-desktops)
- [Instalar e abrir aplicativos](https://docs.waydro.id/usage/install-and-run-android-applications)
- [Comandos do Waydroid](https://docs.waydro.id/usage/waydroid-command-line-options)
- [Implementação oficial de abertura e início da sessão](https://github.com/waydroid/waydroid/blob/main/tools/actions/app_manager.py)
