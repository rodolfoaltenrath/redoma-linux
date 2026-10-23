# Validação do aplicativo oficial Certisign

Início: 18/10/2026  
Última atualização: 18/10/2026  
Estado: e-token/acesso e persistência entre sessões confirmados pelo titular; reboot pendente.

Este registro acompanha a Fase 1 do [roadmap](../roadmap.MD). O objetivo é
comprovar cadastro e autenticação no aplicativo oficial antes de investir no
runtime incorporado e nos instaladores do produto.

## Ambiente de referência

| Item | Configuração verificada em 18/10/2026 |
| --- | --- |
| Distribuição | Fedora Linux 44 Workstation |
| Arquitetura | x86-64 |
| Kernel | `7.2.9-200.fc44.x86_64` |
| Sessão | Wayland, socket disponível |
| GPU | AMD Navi 10, família Radeon RX 5600/5700 |
| Binder | `CONFIG_ANDROID_BINDER_IPC=y` e `CONFIG_ANDROID_BINDERFS=y` |
| Memfd e PSI | `CONFIG_MEMFD_CREATE=y` e `CONFIG_PSI=y` |
| Espaço livre inicial | Aproximadamente 229 GiB no volume de trabalho/sistema |
| Aplicativo-alvo | `com.certisign.certisign`; confirmar dentro da loja oficial |
| Sistemas externos indicados pelo titular | GERID, Projudi, PJe e eproc |

Os requisitos do kernel foram consultados em `/boot/config-$(uname -r)`. Sua
presença não comprova que o contêiner ou o aplicativo funcionam.

## Sistemas de acesso a validar

O titular informou os sistemas abaixo em 18/10/2026. O primeiro endereço
identificado é o GERID INSS/Dataprev. As demais instâncias serão registradas
quando forem realizados os testes.

| Sistema | Instância/tribunal | E-token aceito | Login no navegador Linux | Integração de assinatura |
| --- | --- | --- | --- | --- |
| GERID | INSS/Dataprev, destino Atendimento INSS | Confirmado pelo titular no primeiro teste | Navegador, sistema do navegador e integração ainda não documentados | A verificar se aplicável |
| Projudi | A identificar | Pendente | Pendente | A verificar |
| PJe | A identificar | Pendente | Pendente | A verificar |
| eproc | A identificar | Pendente | Pendente | A verificar |

O primeiro objetivo é comprovar uma autenticação real com o e-token do app
executado no Linux. Depois, cada sistema deve ter seu próprio resultado,
incluindo navegador, versão e componente de assinatura quando necessário.
Uma aprovação em um sistema não deve ser generalizada para os demais.

O funcionamento do aplicativo Android não comprova, por si só, que o navegador
Linux consegue utilizar o certificado em nuvem. Se o acesso depender de uma
integração adicional, essa dependência e seu resultado devem ser registrados
separadamente.

### Primeiro alvo: GERID INSS/Dataprev

- Portal: [GERID INSS](https://geridinss.dataprev.gov.br/cas/login).
- Destino informado no fluxo de acesso: `https://atendimento.inss.gov.br`.
- O titular forneceu o endereço de login com retorno OAuth para esse destino.
- A consulta pública da página em 18/10/2026 mostrou a opção
  **Entrar com Certificado Digital**, além dos campos de CPF e senha.
- Na consulta pública inicial não foi feita tentativa de login, seleção de
  certificado ou envio de credenciais pelo agente.
- Após a orientação para gerar o e-token e usá-lo nesse fluxo, o titular informou
  que funcionou perfeitamente. O primeiro teste fica registrado como confirmação
  do titular, sem captura de credenciais, códigos ou da sessão autenticada.
- A integração necessária para usar o certificado em nuvem no navegador Linux
  continua pendente de validação; a página inicial não comprova esse suporte.

## Componentes de referência instalados

Os pacotes vieram dos repositórios oficiais `fedora` e `updates`. Essa instalação
serve ao teste de viabilidade; o produto continua planejado com runtime próprio.

| Pacote | Versão instalada |
| --- | --- |
| waydroid | `1.6.3-1.fc44.noarch` |
| waydroid-selinux | `1.6.3-1.fc44.noarch` |
| lxc | `6.0.6-1.fc44.x86_64` |
| lxc-libs | `6.0.6-1.fc44.x86_64` |
| lxc-templates | `6.0.6-1.fc44.x86_64` |
| lxcfs | `6.0.6-1.fc44.x86_64` |
| libgbinder | `1.1.47-1.fc44.x86_64` |
| libglibutil | `1.0.82-1.fc44.x86_64` |
| python3-gbinder | `1.3.0-2.fc44.x86_64` |
| Python do sistema, já disponível | `3.14.8-1.fc44.x86_64` |

Comando executado com autorização administrativa do sistema:

```sh
pkexec /usr/bin/dnf --repo=fedora --repo=updates -y install waydroid
```

Resultado: instalação concluída. O pacote habilitou o serviço
`waydroid-container.service`, posteriormente observado como ativo.

**Ocorrência no hospedeiro:** o DNF informou que uma transação de atualização
offline anteriormente agendada foi invalidada pela instalação. O usuário foi
informado; reagendar as atualizações pelo Atualizador de Software permanece
uma ação separada. Não foram aplicadas atualizações gerais do sistema.

## Preparação da imagem Android

Comando executado com sucesso:

```sh
pkexec /usr/bin/waydroid init -s GAPPS \
  -c https://ota.waydro.id/system \
  -v https://ota.waydro.id/vendor
```

- Imagem solicitada: Android com serviços Google, arquitetura x86-64.
- Canal de sistema: `https://ota.waydro.id/system`.
- Canal de vendor: `https://ota.waydro.id/vendor`.
- Download de sistema observado: `lineage-20.0-20260927-GAPPS-waydroid_x86_64-system.zip`.
- Download de vendor observado: `lineage-20.0-20260927-MAINLINE-waydroid_x86_64-vendor.zip`.
- Os dois arquivos passaram pela verificação de integridade do inicializador e foram extraídos.
- Imagens instaladas: `system.img`, 2.582.528.000 bytes; `vendor.img`, 562.388.992 bytes.
- A sessão informou `Android with user 0 is ready`; sessão e contêiner foram observados em execução.
- `waydroid show-full-ui` e a abertura da página do Certisign na Play Store retornaram sucesso.
- A Play Store foi identificada na lista de aplicativos antes da pausa do titular.
- O Certisign ainda não estava instalado na consulta anterior à pausa.
- Na retomada, o contêiner estava `FROZEN`; não houve nova confirmação da instalação do app.
- A tentativa de reabrir encontrou `Already tracking a session`; a recuperação
  com `waydroid session stop` e `waydroid session start` voltou a informar Android pronto.
- Após a recuperação, a lista de pacotes confirmou Play Store presente e Certisign ausente;
  a abertura da página do app na loja foi solicitada novamente com sucesso.

O uso da imagem GAPPS neste teste não estabelece permissão de redistribuição
dos componentes Google dentro de um pacote próprio.

## Execução e critérios de aceitação

| Etapa | Estado | Evidência ou condição |
| --- | --- | --- |
| Inspecionar a máquina | Concluída | Configuração registrada acima. |
| Instalar Waydroid de referência | Concluída | RPMs consultados e serviço ativo. |
| Inicializar a imagem Android | Concluída | Sistema e vendor verificados e extraídos; inicializador terminou com sucesso. |
| Iniciar sessão e exibir Android | Concluída | Android pronto e visualização do app confirmados pelo titular. |
| Preparar acesso à Play Store | Concluída | Titular confirmou acesso à loja e instalação do aplicativo. |
| Instalar o Certisign oficial | Concluída | Versão 1.4.1, versionCode 127, ABI x86-64, consultados no dispositivo. |
| Abrir o app pelo launcher Rust | Confirmada após ajuste experimental | Titular confirmou abertura e apresentou a carteira do app com opção e-Token. |
| Disponibilizar o e-token no app oficial | Funcionamento confirmado pelo titular | Procedimento de cadastro não inspecionado nem reproduzido pelo agente. |
| Obter e-token aceito em acesso real | Primeiro teste confirmado pelo titular | Resposta positiva à orientação de gerar o código e usá-lo no GERID; ambiente do navegador ainda a documentar. |
| Repetir após encerrar/reabrir sessão | Confirmada pelo titular | Certificados disponíveis e e-token novamente aceito após encerrar e iniciar a sessão Android. |
| Repetir após reiniciar computador | Pendente | Nova autenticação após boot. |
| Verificar dependência de notificações | Pendente | Registrar apenas o comportamento, sem conteúdo pessoal. |

Instalar Android, conseguir abrir o app ou visualizar um código são resultados
intermediários. A Fase 1 só termina após autenticação aceita e testes de persistência.

## Procedimento para o titular

Quando o Android estiver disponível:

1. Entrar na Play Store pela interface do Android.
2. Se houver aviso de dispositivo não certificado, seguir o procedimento oficial
   de [registro do dispositivo](https://docs.waydro.id/faq/google-play-certification).
   Esse registro de acesso ao Google Play não garante aprovação nas verificações
   de integridade de outros aplicativos.
3. Instalar o [Certisign oficial](https://play.google.com/store/apps/details?id=com.certisign.certisign).
4. Concluir a ativação/cadastro pelo próprio app.
5. Usar um e-token no sistema de acesso habitual e informar somente se o acesso
   foi aceito ou qual foi a mensagem de falha, sem enviar o código.
6. Repetir o teste após encerrar e reabrir a sessão.
7. Em momento combinado, repetir após reiniciar o computador.

Não informar neste arquivo ou no chat senha Google, senha/PIN Certisign,
e-token, QR code de ativação ou identificadores pessoais. Não coletar telas ou
logs brutos de autenticação como evidência.

## Comandos de acompanhamento

Depois de concluir a inicialização:

```sh
waydroid status
waydroid show-full-ui
```

Depois de instalar o Certisign:

```sh
cargo run --release -- doctor
cargo run --release -- open
```

Em outro terminal, para encerrar a sessão inteira:

```sh
cargo run --release -- stop
```

Esses comandos não alteram cadastro nem removem dados. Encerrar a sessão
interrompe também outros aplicativos Android e o recebimento de notificações.

## Histórico desta validação

### 2026-10-18 — Início da preparação

- Inspecionados sistema, kernel, sessão gráfica, GPU e espaço livre.
- Instalados Waydroid 1.6.3 e suas dependências de referência.
- Observado serviço de contêiner ativo após instalação.
- Iniciada a preparação da imagem GAPPS pelo inicializador oficial.
- Cadastro e autenticação reais ainda não executados.

### 2026-10-18 — Android inicializado e loja aberta

- Concluídos os downloads, verificações de integridade e extração de sistema e vendor.
- Iniciada a sessão sem privilégios administrativos; observado Android pronto.
- Executado o `doctor` do launcher Rust contra o Waydroid real, com sucesso.
- Identificada a Play Store instalada e solicitada a abertura da página oficial
  do Certisign por `waydroid app intent`.
- Solicitada ao titular a instalação pela loja e a informação de eventual aviso
  de dispositivo não certificado; resposta ainda pendente na retomada.
- O titular precisou se ausentar. Na retomada, a sessão continuava registrada
  e o contêiner estava `FROZEN`. Não se presume que o app foi instalado nesse intervalo.
- Próximo passo: confirmar o resultado da instalação, resolver eventual aviso
  da loja e então testar cadastro e autenticação oficiais.

### 2026-10-18 — Retomada após ausência do titular

- Encontrado contêiner `FROZEN` com registro antigo de sessão; a primeira
  tentativa de abertura retornou `Already tracking a session`.
- Encerrada e iniciada novamente a sessão pelos comandos normais do Waydroid.
- Observada nova mensagem de Android pronto; Play Store presente e Certisign ausente.
- Solicitada novamente a abertura da loja na página do Certisign com sucesso.
- Pendente: instalação pelo titular e relato de eventual mensagem de erro ou
  aviso de dispositivo não certificado.

## Impedimento identificado em 18/10/2026

O titular concluiu a instalação e apresentou tela do Certisign com recusa de
uso em emuladores. Cadastro e autenticação real continuam pendentes. Metadados,
análise limitada e limites da conclusão estão em [emulator-block.md](emulator-block.md).

O diagnóstico encontrou referências a checagens locais de arquitetura; não
comprovou ausência ou presença de validação adicional pelo servidor. Na análise
inicial não foi aplicada alteração de APK ou tentativa de contorno.

### Primeiro experimento autorizado

Posteriormente, o titular autorizou um experimento reversível de configuração.
Foi aplicada a opção `-Dos.arch=aarch64` na inicialização Java, seguida de
reaplicação offline da configuração e nova sessão Android. O Android iniciou,
o Certisign foi reaberto e o resultado visual foi solicitado ao titular.
Os APKs base e x86-64 permanecem idênticos às cópias anteriores à alteração.
Detalhes e restauração: [experiment-os-arch.md](experiment-os-arch.md).

O titular confirmou que o app abriu corretamente após o ajuste e enviou uma
captura da carteira com a opção e-Token disponível. Foi ultrapassada a tela
inicial de restrição; a captura não foi armazenada no repositório e seus detalhes
de certificados não foram transcritos.

Após a orientação para gerar o código e usá-lo no GERID, o titular respondeu
que funcionou perfeitamente. O teste inicial de geração/aceitação fica registrado
com base nesse relato, sem acesso do agente aos códigos ou à sessão autenticada.
O sistema operacional do navegador, sua versão e eventuais componentes de
integração não foram informados; não se declara validada a integração completa
do navegador Linux a partir desse relato.

O titular também confirmou que os certificados continuam disponíveis e o e-token
é aceito após encerrar e iniciar novamente a sessão Android. O teste após
reiniciar o computador permanece pendente. A Fase 1 continua em andamento,
assim como os testes dos outros sistemas e o detalhamento do navegador.

### 2026-10-18 — Primeiro teste funcional confirmado pelo titular

- O titular informou que funcionou perfeitamente após o teste proposto de
  geração do e-token no app e uso no GERID INSS/Dataprev.
- Evidência: confirmação textual no chat; nenhum código ou credencial coletado.
- Escopo: app oficial 1.4.1 no Waydroid de referência com o ajuste documentado.
- Pendente: repetição após nova sessão Android e reboot do hospedeiro, detalhes
  do navegador e validação individual de Projudi, PJe e eproc.

### 2026-10-18 — Início do teste de persistência entre sessões

- Encerrada a sessão pelo comando `waydroid session stop` e iniciada uma nova
  sessão gerenciada pelo systemd do usuário.
- A nova sessão informou Android pronto e manteve `-Dos.arch=aarch64`.
- Reaberto o Certisign pelo launcher Rust, com retorno de sucesso do comando.
- Solicitada ao titular a confirmação de que os certificados continuam
  disponíveis e o e-token é aceito no mesmo acesso.
- Resultado funcional pendente; não houve reboot do computador.

### 2026-10-18 — Persistência entre sessões confirmada

- Titular respondeu positivamente à verificação de certificados disponíveis
  e aceitação do e-token no mesmo acesso após a nova sessão Android.
- Resultado: funcionamento preservado no ciclo de encerramento e reabertura.
- Evidência: confirmação explícita do titular; códigos e credenciais não coletados.
- Pendente: reboot do computador, identificação do navegador e integrações,
  validação individual de Projudi, PJe e eproc.

## Próximo teste: reinício do computador

Realizar em momento conveniente para o titular, após salvar o trabalho aberto.
O computador não foi reiniciado pelo agente.

Após o boot, abrir um terminal na pasta do projeto e executar:

```sh
./target/release/certisign-linux open
```

Verificar se a carteira continua disponível e repetir a geração/aceitação do
e-token no mesmo acesso já testado. Informar somente o resultado ou a mensagem
de erro, sem enviar o código. Registrar o resultado antes de concluir M1.

## Referências de preparação

- [Instalação oficial do Waydroid no Fedora](https://docs.waydro.id/usage/install-on-desktops)
- [Pacote Waydroid no Fedora](https://packages.fedoraproject.org/pkgs/waydroid/waydroid/)
- [Registro de dispositivo para Google Play](https://docs.waydro.id/faq/google-play-certification)
- A ajuda e os arquivos do Waydroid 1.6.3 instalado foram usados para conferir
  os argumentos disponíveis nesta versão.
