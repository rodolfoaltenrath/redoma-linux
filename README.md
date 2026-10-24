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
- **Sem telemetria.** O Redoma não faz requisições próprias. Na instalação, os
  downloads são feitos pelo `dnf` (repositórios do Fedora) e pelo Waydroid
  (imagens oficiais em `ota.waydro.id`). Depois, quem acessa a internet é o
  Android do contêiner e o app oficial.
- **Não modifica o app.** O APK, sua assinatura e seu código permanecem os
  publicados na Play Store.
- **Root só com a sua autorização.** O uso normal (abrir o app, configurar
  janela e atalhos) roda como usuário comum. As etapas que exigem administrador
  abrem a janela de senha do sistema (`pkexec`) e executam apenas ações fixas
  de [src/privileged.rs](src/privileged.rs): instalar o Waydroid, ativar seu
  serviço, baixar a imagem Android, aplicar o ajuste de compatibilidade e ler
  o ID do dispositivo para o registro no Google. Nenhum pacote, caminho ou
  comando vem de argumento.
- **Abre um único app.** O pacote é fixo no código; não aceita outro pacote
  por argumento.
- **Poucas dependências.** GTK4 e libadwaita (interface), `async-channel`,
  `sha2` (verificação do backup) e `libc`. O código próprio está em `src/` e
  pode ser auditado por completo.

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
| Área de transferência | **Bloqueada por padrão.** O Waydroid só sincroniza com o pacote `python3-pyclip`, que o assistente instala apenas se você ativar a opção. Evite copiar senhas. |
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
histórico: [docs/emulator-block.md](docs/emulator-block.md) e
[docs/experiment-os-arch.md](docs/experiment-os-arch.md). O assistente aplica o
ajuste com backup protegido em `/var/lib/waydroid/certisign-os-arch-experiment`
e recusa sobrescrever o arquivo se ele tiver mudado. Para reverter:
`pkexec redoma privileged restore-os-arch`. O app pode mudar suas
verificações a qualquer atualização; use por sua conta e conforme os termos da
Certisign.

### O que o Redoma altera no seu sistema

Com autorização de administrador (uma vez, na instalação):

- Pacotes `waydroid` e, se a opção for ativada, `python3-pyclip` (via `dnf`).
- Serviço `waydroid-container` habilitado.
- Imagem Android com Google Play (`waydroid init -s GAPPS`) em `/var/lib/waydroid`.
- Ajuste `os.arch` em `/var/lib/waydroid/waydroid.cfg`, com backup.

Como usuário comum:

- Propriedades do Waydroid listadas em [Instalação e uso](#instalação-e-uso).
- Atalhos em `~/.local/share/applications`: cria `redoma.desktop`; marca com
  `NoDisplay=true` o atalho do Certisign gerado pelo Waydroid e, se a opção
  estiver ativa, os dos demais apps (lista em `~/.config/redoma/hidden-entries`).
- Favorito da dock do GNOME e `~/.config/autostart/redoma.desktop`, conforme as opções.
- Opções em `~/.config/redoma/options` e registro da sessão em
  `~/.local/state/redoma/session.log` (saída do Waydroid, sem credenciais).

Para desfazer: desligue as opções no assistente ("Configurar Redoma"), apague
`redoma.desktop`, `~/.config/redoma` e `~/.local/state/redoma`, reverta o
ajuste com `pkexec redoma privileged restore-os-arch` e, se quiser, remova o
Waydroid com `sudo dnf remove waydroid`. `redoma stop` encerra o Android; os
dados do app permanecem até você remover os dados do Waydroid.

### Relatar vulnerabilidades

Não abra issue pública para falhas de segurança. Use o
[relato privado de vulnerabilidades do GitHub](../../security/advisories/new)
deste repositório.

## Estado atual

Um único binário com **assistente gráfico** (GTK4/libadwaita) e comandos de
terminal. O projeto não é um port do Android para Rust: usa o Waydroid do
Fedora como dependência externa. Ao abrir o app, o Redoma inicia a sessão
Android em segundo plano, aguarda o fim do boot e é substituído pelo comando do
Waydroid com `exec`; não fica nenhum processo do Redoma residente.

Validado no ambiente de referência (Fedora 44, x86-64, GNOME/Wayland, Waydroid
1.6.3, Certisign 1.4.1): abertura da carteira, geração de e-token aceito no GERID
e persistência após encerrar e reiniciar o Android. Nesse ambiente, já
preparado, o assistente reconheceu e pulou todas as etapas. **A instalação do
zero pelo assistente ainda não foi validada numa máquina limpa.** Pendentes
também: outros sistemas (Projudi, PJe, eproc) e outras distribuições. Detalhes
em [docs/validation.md](docs/validation.md).

O pacote configurado corresponde ao app **Certisign**, não ao remoteID ou MobileID.

## Instalação e uso

Compilar (uma vez; só quem compila precisa destes pacotes):

```sh
sudo dnf install cargo gtk4-devel libadwaita-devel
cargo build --release
./target/release/redoma
```

Na primeira execução abre o **assistente**, que:

1. verifica Fedora, x86-64, Wayland, Binder e espaço livre;
2. oferece as opções: ocultar outros apps, fixar na dock, iniciar o Android com
   o computador e compartilhar a área de transferência;
3. pede a senha de administrador **uma vez** e instala o ambiente Android,
   pulando o que já estiver feito;
4. inicia o Android e configura janela, atalhos e opções;
5. abre a página do Certisign na Play Store e percebe sozinho quando ele foi
   instalado. Se a Play Store disser que o dispositivo não é certificado, obtém
   o ID do dispositivo e abre a página oficial de registro do Google;
6. abre o Certisign para você concluir o cadastro no app oficial.

Login no Google, registro do dispositivo e cadastro no Certisign são feitos por
você, pois envolvem sua conta e suas credenciais.

Depois disso, o ícone **Certisign** abre o app direto. Para voltar ao
assistente, clique com o botão direito no ícone e escolha "Configurar Redoma",
ou rode `redoma wizard`.

Comandos de terminal:

```sh
redoma           # abre o Certisign (ou o assistente, se faltar algo)
redoma wizard    # assistente gráfico
redoma setup     # reaplica a configuração sem interface
redoma open      # abre o Certisign
redoma background  # inicia o Android sem abrir janela
redoma status    # estado do Waydroid
redoma doctor    # diagnóstico básico
redoma stop      # encerra o Android
```

A configuração aplica as propriedades do Waydroid abaixo, reiniciando a sessão
só se alguma mudou:

| Propriedade | Valor | Motivo |
| --- | --- | --- |
| `persist.waydroid.multi_windows` | `false` | O modo multi-janela exibiu o app reduzido dentro de uma moldura transparente no GNOME. |
| `persist.waydroid.width` / `height` | `450` / `800` | Janela única em retrato 9:16. |
| `persist.waydroid.suspend` | `false` | Sem isso, fechar a janela congela o contêiner e as notificações param. |

Contatos, Mensagens, Play Store e similares fazem parte da imagem GAPPS e
continuam instalados no Android; com a opção ativa, apenas seus atalhos ficam
ocultos (`NoDisplay=true`, que o Waydroid preserva ao regenerar os atalhos).

A configuração também instala o atalho próprio `redoma.desktop`, que chama o
executável no caminho atual, e oculta o atalho do Certisign
gerado pelo Waydroid. Motivo: ao fechar a janela, o Waydroid mantém o app em
primeiro plano no Android, mas passa a ignorá-lo até ele sair da tela, então
`waydroid app launch` não traz a janela de volta. Com a sessão já ativa, `open`
traz por um instante o launcher do Android, que nunca vira janela e limpa
essa marca, e então abre o Certisign. O app não é reiniciado nesse processo.
Se o executável mudar de lugar, rode `redoma setup` novamente.

No GNOME a janela do Waydroid não recebe barra de título, pois o Mutter não
desenha decorações para clientes Wayland. Minimize com Super+H, feche com
Alt+F4 ou pelo X na Visão Geral; Alt+Tab funciona normalmente.

Execute na sessão gráfica Wayland do usuário, sem `sudo`.

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

## Histórico da preparação

Em 18/10/2026 o ambiente de referência foi preparado manualmente: Waydroid
1.6.3 com imagem GAPPS e Certisign 1.4.1, que inicialmente exibiu bloqueio de
emuladores. Após o ajuste reversível de `os.arch`, o titular confirmou a
abertura da carteira, o e-token aceito no GERID e a persistência após encerrar
e iniciar novamente o Android. Esse procedimento manual é o que o assistente
passou a automatizar. Registros: [docs/emulator-block.md](docs/emulator-block.md),
[docs/experiment-os-arch.md](docs/experiment-os-arch.md),
[docs/validation.md](docs/validation.md) e
[docs/runtime-inventory.md](docs/runtime-inventory.md).

O Redoma não baixa APK de terceiros nem modifica o app: o Certisign é instalado
por você pela Play Store.

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
lançamento, aplicação idempotente das propriedades, ocultação e reexibição de
atalhos, pré-requisito de Wayland, encerramento explícito, ausência de
inicialização no diagnóstico, propagação de falhas e a recusa do helper
administrativo sem root ou com ação desconhecida. Os testes unitários cobrem o
ajuste `os.arch` (incluindo backups criados pela antiga ferramenta Python), o
protocolo de progresso, a dock e a detecção da distribuição. Não executam
autenticação nem acessam certificados.

## Licença

Software livre sob a [GNU General Public License v3.0 ou posterior](LICENSE)
(`GPL-3.0-or-later`), a mesma licença do Waydroid. Você pode usar, estudar,
modificar e redistribuir; versões modificadas distribuídas devem manter o
código aberto sob a mesma licença. Sem garantia, conforme os termos da licença.

A licença cobre apenas o código deste repositório. O aplicativo Certisign, o
Android e os serviços Google têm termos próprios e não são distribuídos aqui.

## Referências

- [Certisign oficial na Play Store](https://play.google.com/store/apps/details?id=com.certisign.certisign)
- [Instalação do Waydroid](https://docs.waydro.id/usage/install-on-desktops)
- [Instalar e abrir aplicativos](https://docs.waydro.id/usage/install-and-run-android-applications)
- [Comandos do Waydroid](https://docs.waydro.id/usage/waydroid-command-line-options)
- [Implementação oficial de abertura e início da sessão](https://github.com/waydroid/waydroid/blob/main/tools/actions/app_manager.py)
