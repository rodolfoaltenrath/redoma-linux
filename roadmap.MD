# Roadmap — Redoma (Certisign no Linux)

Criado em: 16/10/2026  
Última atualização: 24/10/2026  
Estado: Fase 1 em andamento; e-token/acesso e persistência entre sessões confirmados pelo titular; reboot pendente.

Este é o documento de acompanhamento do projeto. Registra a arquitetura
pretendida, a ordem de execução, os critérios de conclusão e o histórico de
entregas. Uma etapa planejada não deve ser apresentada como implementada.

## 1. Objetivo

Criar um aplicativo Linux em Rust que permita usar o **aplicativo oficial
Certisign** para obter o e-token e autorizar acessos ao certificado digital em
nuvem, reduzindo a dependência do celular nessa etapa.

A experiência pretendida é instalar um pacote, abrir o aplicativo, concluir a
configuração guiada e usar o Certisign. O usuário não deverá procurar e instalar
manualmente as dependências específicas do ambiente Android.

O funcionamento depende de comprovar que o aplicativo e o serviço Certisign
aceitam o ambiente escolhido. Essa compatibilidade ainda não foi demonstrada.

Sistemas de interesse informados em 18/10/2026: **GERID, Projudi, PJe e eproc**.
Os testes devem distinguir a aceitação do e-token e a integração do navegador
Linux com o certificado. As instâncias/tribunais serão identificados durante
a validação de cada acesso.

## 2. Decisões acordadas

| Tema | Direção definida |
| --- | --- |
| Código próprio | Rust para a aplicação, diagnóstico e gerenciamento do ambiente. |
| Autenticação | Exclusivamente dentro do aplicativo oficial Certisign. |
| Pacote Android inicial | `com.certisign.certisign`, conforme configurado no protótipo. |
| Base Android | Reutilizar o Waydroid e acompanhar o projeto original. |
| Dependências específicas | Incorporar um runtime próprio, com versões controladas, reutilizável entre distribuições. |
| Distribuição inicial | RPM, com Fedora como primeiro ambiente de validação. |
| Expansão | DEB e outras distribuições usando a mesma base, por arquitetura e compatibilidade de bibliotecas. |
| Configuração | Procedimentos internos, repetíveis, com progresso e diagnóstico. |
| Otimização | Medir Android + Certisign + aplicação Linux antes de definir metas. |
| Manutenção | Atualizar componentes de terceiros com o mínimo necessário de modificações locais. |

O runtime conterá componentes de outras linguagens, como o gerenciamento do
Waydroid em Python. Usar Rust no projeto não significa reescrever o Waydroid ou
o Android integralmente em Rust.

### Limites do escopo

- Não criar autenticador alternativo, gerar e-token no Rust ou extrair seus segredos.
- Não modificar o aplicativo Certisign para contornar verificações de dispositivo.
- Não apresentar o produto como aplicativo oficial ou homologado pela Certisign.
- Não prometer suporte a todas as distribuições, kernels ou GPUs.
- A integração do certificado com navegador, DesktopID ou sistema de assinatura
  é uma frente separada; executar o app não resolve automaticamente essa etapa.
- Uma instalação offline dos componentes não significa autenticação offline.

## 3. Situação atual comprovada

| Entrega ou verificação | Estado | Evidência |
| --- | --- | --- |
| Projeto Cargo e perfil de release | Concluído | `Cargo.toml` e `Cargo.lock`. |
| Launcher Rust por terminal | Concluído | `src/main.rs`. |
| Comandos `doctor`, `status`, `open` e `stop` | Implementados | Código e testes com backend simulado. |
| Seleção fixa do pacote Certisign | Implementada | `open` encaminha `com.certisign.certisign`. |
| Verificação de executável e socket Wayland | Implementada | `doctor` e pré-requisitos de `open`. |
| Ausência de dependências Cargo externas | Verificada no protótipo | Manifesto e lockfile atuais. |
| Entrega da execução ao Waydroid por `exec` | Implementada | Launcher não mantém supervisor Rust após abrir. |
| Testes de integração | 6 aprovados na implementação inicial | `tests/launcher.rs`; Waydroid simulado. |
| Formatação, Clippy e compilação de release | Aprovados na implementação inicial | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo build --release`. |
| Diagnóstico na máquina de desenvolvimento | Executado | Fedora 44, x86-64, socket Wayland disponível; Waydroid ausente naquele momento. |
| Preparação do ambiente de referência | Concluída para o teste inicial | Waydroid 1.6.3 e imagens verificadas; sessão Android e Play Store disponíveis. Ver `docs/validation.md`. |
| Instalação e abertura do Certisign oficial | Carteira aberta após ajuste | App 1.4.1, versionCode 127, x86-64; abertura confirmada pelo titular com captura da carteira. Ver `docs/validation.md`. |
| Experimento reversível de configuração | Primeiro teste funcional confirmado | Ajuste da propriedade Java `os.arch`, backup protegido e quatro testes aprovados; abertura e teste inicial de e-token/acesso confirmados pelo titular. Ver `docs/experiment-os-arch.md`. |
| Controle de versão local | Inicializado | Repositório Git criado na branch `main`; primeiro commit pendente. |
| Inventário do ambiente de referência | Levantamento inicial | Dependências diretas, integrações e metadados das imagens em `docs/runtime-inventory.md`; inventário transitivo ainda pendente. |
| Documentação inicial | Concluída | `README.md`. |
| Runtime incorporado, imagem Android e scripts de configuração | Pendente | Ainda não implementados. |
| Interface gráfica e instaladores RPM/DEB | Pendente | Ainda não implementados. |
| Geração de e-token e acesso | Confirmados pelo titular, inclusive após nova sessão Android | Certificados disponíveis e e-token novamente aceito; reboot, detalhes do navegador e outros sistemas pendentes. Ver `docs/validation.md`. |
| Otimização do consumo do conjunto | Pendente | Não há medições ou metas demonstradas. |

Os seis testes atuais verificam o pacote lançado, recusa de abertura sem socket
Wayland, encerramento sem depender de display, propagação do código de falha,
diagnóstico sem iniciar Android e rejeição de argumentos para outro pacote.
Eles não comprovam autenticidade de um APK instalado nem compatibilidade com
os servidores da Certisign.

O protótipo atual ainda procura `waydroid` no `PATH`. A adoção do runtime próprio
é a arquitetura planejada e exige mudanças posteriores.

## 4. Arquitetura pretendida

```text
Pacote de instalação: RPM / DEB
│
├── Aplicação Rust
│   ├── Interface gráfica e CLI
│   ├── Diagnóstico de compatibilidade
│   ├── Assistente de configuração
│   ├── Gerenciamento do runtime
│   └── Diagnóstico de falhas e atualizações
│
├── Runtime próprio, versionado por arquitetura
│   ├── Waydroid em versão conhecida
│   ├── Python e módulos necessários
│   ├── LXC e bibliotecas/ferramentas incorporáveis
│   └── Manifesto de versões, origens, hashes e licenças
│
├── Imagens Android testadas
│   ├── system.img
│   └── vendor.img
│
├── Integração com o Linux
│   ├── Serviço do contêiner
│   ├── D-Bus e autorização administrativa
│   ├── Regras de segurança e rede necessárias
│   └── Atalhos e associação com a sessão gráfica
│
└── Dados persistentes
    ├── Dados do aplicativo Android, gerenciados pelo Android
    ├── Configuração local do produto
    └── Diagnósticos sem credenciais ou códigos de autenticação
```

### Fronteira entre o runtime e o sistema

A meta é compartilhar o mesmo runtime entre as distribuições suportadas,
evitando resolver separadamente suas dependências específicas. O inventário
final dos componentes incorporados será definido após uma prova de empacotamento.

Kernel, suporte Binder/binderfs, GPU, compositor gráfico e serviços fundamentais
continuam exigindo compatibilidade com o sistema hospedeiro. Bibliotecas
básicas, como a libc, também precisam de uma estratégia explícita de compatibilidade.
Um único arquivo não elimina essas diferenças.

Os instaladores devem concentrar as adaptações necessárias. A aplicação Rust
deve usar interfaces comuns, evitando espalhar condicionais por distribuição
no fluxo principal.

O aplicativo oficial Certisign e eventuais componentes Google têm condições
próprias de distribuição. Sua inclusão no pacote depende de verificar essas
condições. Se necessário, o assistente conduzirá a instalação pelo canal oficial.
Nunca serão distribuídos tokens ou dados de uma conta previamente cadastrada.

## 5. Estrutura prevista do repositório

Os arquivos atuais incluem `src/main.rs`, `tests/launcher.rs`, os arquivos Cargo,
`README.md`, `.gitignore`, este roadmap e os registros em `docs/validation.md`,
`docs/runtime-inventory.md`, `docs/emulator-block.md` e `docs/experiment-os-arch.md`.
O laboratório também possui `scripts/experiment-os-arch.py` e
`tests/test_os_arch_experiment.py`, separados da aplicação Rust.
A árvore abaixo é uma proposta; os demais módulos serão criados conforme as etapas.

```text
certisign-android/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── roadmap.MD
├── src/
│   ├── main.rs                 # Entrada da aplicação
│   ├── cli.rs                  # Comandos e argumentos
│   ├── diagnostics.rs          # Pré-requisitos e estado do ambiente
│   ├── runtime.rs              # Localização, versão e ciclo de execução
│   ├── setup.rs                # Configuração inicial e recuperação
│   └── ui/                     # Interface gráfica; tecnologia a definir
├── runtime/
│   ├── manifest.toml           # Componentes, versões, origens e hashes
│   ├── patches/                # Alterações mínimas no Waydroid
│   └── licenses/               # Licenças e atribuições dos componentes
├── scripts/
│   ├── build-runtime.sh        # Construção da base compartilhada
│   ├── build-packages.sh       # Geração dos instaladores
│   └── verify-artifacts.sh     # Validação dos artefatos produzidos
├── packaging/
│   ├── rpm/                    # Spec e integração RPM
│   ├── deb/                    # Metadados e integração DEB
│   └── common/                 # Atalhos, serviços e ações compartilhadas
├── tests/
│   ├── launcher.rs             # Testes já existentes
│   └── integration/            # Configuração, falhas e ciclo de execução
├── docs/
│   ├── architecture.md         # Decisões técnicas e seus motivos
│   ├── compatibility.md        # Sistemas, hardware e versões testados
│   ├── validation.md           # Evidências de testes reais
│   ├── runtime-inventory.md    # Levantamento do ambiente de referência
│   ├── performance.md          # Método, medições e metas
│   ├── maintenance.md          # Atualizações e acompanhamento de terceiros
│   └── releases.md             # Histórico das versões entregues
└── dist/                       # Artefatos gerados; fora do controle de versão
```

Os fontes de terceiros poderão entrar por arquivo de origem verificado,
submódulo ou cópia versionada. O mecanismo será decidido na etapa do runtime;
nenhum checkout móvel de `main` deve determinar uma versão de release.

## 6. Sequência de execução

As fases indicam dependências e ordem de trabalho, não datas prometidas.
Não avançar para distribuição pública sem comprovar autenticação real.

### Fase 0 — Registrar a base e o plano

- [x] Criar o projeto Rust e o launcher inicial.
- [x] Documentar uso, limitações e verificações do protótipo.
- [x] Criar este roadmap com histórico e critérios de conclusão.
- [x] Inicializar o controle de versão local do projeto.
- [x] Registrar a base inicial em um commit.
- [x] Definir a licença do código próprio (GPL-3.0-or-later) e a identificação do produto (Redoma).

**Conclusão:** base rastreável, documentos acessíveis e histórico inicial registrado.

### Fase 1 — Comprovar a viabilidade do app oficial

Em execução desde 18/10/2026. Ambiente, versões, comandos, ocorrências e resultados
estão em [docs/validation.md](docs/validation.md).

1. Registrar a configuração da máquina de teste e o sistema-alvo de assinatura.
2. Confirmar que o usuário utiliza o app `com.certisign.certisign`.
3. Preparar um Waydroid de referência, antes de montar o runtime incorporado.
4. Instalar o app pelo canal oficial e registrar as versões utilizadas.
5. Verificar arquitetura, serviços necessários e eventuais restrições do app.
6. Concluir o cadastro pelo fluxo oficial, com participação do titular da conta.
7. Gerar o e-token e comprovar sua aceitação em uma autenticação real.
8. Repetir o acesso após encerrar a sessão, reabri-la e reiniciar a máquina.
9. Verificar se o fluxo depende de notificações, câmera, biometria ou rede.

- [x] Registrar o resultado em `docs/validation.md`, sem dados de autenticação.
- [x] Registrar o bloqueio explícito de emuladores observado no app oficial.
- [x] Confirmar com o titular a abertura da carteira após o primeiro experimento reversível.
- [x] Registrar a confirmação do titular para o primeiro teste de geração e aceitação do e-token.
- [x] Repetir o teste após encerrar e iniciar uma nova sessão Android.
- [ ] Repetir o teste após reinício do computador, em momento apropriado para o titular.
- [ ] Definir continuidade após investigar o impedimento de compatibilidade.

**Conclusão:** app oficial cadastrado e autenticação aceita, com versões e
procedimento documentados. Uma janela aberta ou um código exibido não bastam.
Se houver impedimento do provedor, registrar e avaliar suporte oficial antes de
continuar o investimento em empacotamento.

### Fase 2 — Definir o contrato de compatibilidade

Há um levantamento preliminar em [docs/runtime-inventory.md](docs/runtime-inventory.md),
produzido durante a preparação do ambiente. Isso não encerra o inventário nem
estabelece a portabilidade do runtime.

1. Usar Fedora 44 x86-64 com Wayland como ambiente inicial de desenvolvimento.
2. Escolher uma segunda distribuição de família DEB para validar a portabilidade.
3. Definir versões mínimas de kernel, libc e serviços necessários.
4. Inventariar todas as dependências transitivas do Waydroid.
5. Separar componentes incorporados e requisitos inevitáveis do hospedeiro.
6. Escolher a base de compilação e a arquitetura inicial, começando por x86-64.
7. Definir como identificar uma instalação externa do Waydroid e evitar conflitos.
8. Registrar os critérios para declarar uma combinação suportada.

- [ ] Criar a matriz em `docs/compatibility.md`.
- [ ] Registrar as decisões em `docs/architecture.md`.

**Conclusão:** requisitos explícitos e inventário suficiente para construir uma
base comum. ARM64 e outros ambientes permanecem futuros até serem testados.

### Fase 3 — Construir o runtime incorporado

1. Fixar a versão/commit do Waydroid e registrar sua origem.
2. Incorporar Python, módulos, LXC e demais componentes definidos no inventário.
3. Construir em ambiente reproduzível, respeitando a compatibilidade mínima.
4. Ajustar caminhos, carregamento de bibliotecas e localização de recursos.
5. Manter arquivos e executáveis próprios separados dos equivalentes do sistema.
6. Adaptar o Rust para localizar o runtime instalado pelo produto.
7. Manter qualquer uso do Waydroid externo como modo explícito de desenvolvimento.
8. Inventariar licenças, atribuições, fontes correspondentes e modificações locais.
9. Executar a mesma base nas duas distribuições selecionadas.

- [ ] Criar o manifesto e os scripts de construção/verificação.
- [ ] Testar em máquinas sem Waydroid, Python adicional ou LXC pré-instalados.

**Conclusão:** runtime construído e rastreável, sem exigir instalação manual de
suas dependências específicas nas distribuições declaradas. Diferenças ainda
necessárias devem ficar documentadas, não escondidas no instalador.

### Fase 4 — Preparar e versionar a imagem Android

1. Escolher uma imagem compatível com o resultado da Fase 1.
2. Começar com o mínimo de alterações e preservar serviços necessários ao app.
3. Registrar as versões de `system.img`, `vendor.img` e seus hashes.
4. Verificar condições de distribuição de todos os componentes incluídos.
5. Definir o procedimento de instalação do Certisign pelo canal oficial.
6. Definir se as imagens acompanharão cada instalador ou um artefato separado.
7. Se houver download, usar versão definida, verificação de integridade e retomada.
8. Separar imagens do sistema e dados persistentes do usuário.
9. Repetir cadastro, autenticação e reinício com a imagem escolhida.

- [ ] Registrar tamanho, origem, componentes e processo de atualização da imagem.
- [ ] Definir se haverá entrega offline dos componentes e seus limites.

**Conclusão:** imagem conhecida e testada, estratégia de distribuição documentada
e instalação oficial do Certisign definida. Uma imagem com cadastro pessoal
prévio nunca deve ser usada como artefato de distribuição.

### Fase 5 — Implementar configuração e gerenciamento em Rust

1. Separar os módulos do launcher conforme a complexidade real surgir.
2. Ampliar o diagnóstico para runtime, imagem, kernel, serviços, rede e app.
3. Modelar os estados: ausente, não configurado, configurando, pronto,
   executando e falha recuperável.
4. Criar configuração repetível: executar novamente não deve duplicar recursos
   nem apagar dados existentes.
5. Implementar as operações administrativas mínimas, separadas da interface.
6. Integrar serviço, D-Bus, permissões e regras de segurança necessárias.
7. Implementar início, abertura direta do Certisign e encerramento controlado.
8. Tratar falhas, operações concorrentes, tempo de espera e interrupção de setup.
9. Preservar dados durante atualização, reparo e coexistência com instalações externas.
10. Produzir diagnósticos úteis sem registrar PIN, senha, segredos ou e-tokens.

- [ ] Adicionar testes dos fluxos de configuração, falha e recuperação.
- [ ] Testar operações administrativas em ambientes descartáveis.

**Conclusão:** primeira configuração e uso repetido funcionam sem instruções
manuais de instalação de dependências, com recuperação de falhas previsível.

### Fase 6 — Criar a interface gráfica

1. Escolher a tecnologia após avaliar integração Linux e consumo adicional.
2. Implementar telas de diagnóstico, preparação, progresso e erro recuperável.
3. Orientar a instalação e ativação dentro do app oficial.
4. Oferecer ações claras de abrir Certisign e encerrar o ambiente.
5. Explicar o efeito do encerramento sobre notificações de autorização.
6. Exibir versões e disponibilizar diagnóstico sem dados sensíveis.
7. Integrar atalhos e testar acessibilidade e mensagens em português.

- [ ] Validar o fluxo em uma máquina sem ferramentas de desenvolvimento.
- [ ] Documentar o comportamento ao fechar a interface e a janela Android.

**Conclusão:** o fluxo normal pode ser concluído graficamente, com exceção da
autorização administrativa normal do sistema quando necessária.

### Fase 7 — Entregar o primeiro RPM

1. Criar o spec e incluir aplicação, runtime, recursos e integração compartilhada.
2. Declarar somente os requisitos do hospedeiro definidos na Fase 2.
3. Usar scripts de instalação curtos; setup demorado ou interativo fica no assistente.
4. Não executar outro gerenciador de pacotes dentro dos scripts de transação RPM.
5. Definir propriedade dos arquivos, atualização e remoção dos serviços.
6. Gerar o RPM e os materiais de licença/fontes necessários à distribuição.
7. Testar instalação limpa, configuração, autenticação, atualização e remoção.
8. Confirmar que a remoção não apaga dados de autenticação sem uma ação específica.
9. Verificar coexistência ou conflito explícito com Waydroid instalado separadamente.

- [ ] Publicar as evidências dos testes em Fedora na matriz de compatibilidade.
- [ ] Definir assinatura dos artefatos e verificação de integridade para entrega.

**Conclusão:** um RPM instalável entrega a base prevista e permite uso real no
Fedora suportado, sem preparação manual de dependências específicas.

### Fase 8 — Reutilizar a base no DEB

1. Criar os metadados DEB e adaptar a instalação dos mesmos recursos compartilhados.
2. Reutilizar o runtime da mesma arquitetura e compatibilidade de bibliotecas.
3. Isolar adaptações de serviços, políticas e caminhos necessárias ao hospedeiro.
4. Executar os mesmos cenários de instalação, uso, atualização e remoção.
5. Registrar quais diferenças foram inevitáveis e eliminar duplicação desnecessária.

- [ ] Validar autenticação real na distribuição DEB escolhida.
- [ ] Adicionar outras distribuições apenas após executar a matriz correspondente.

**Conclusão:** RPM e DEB usam a mesma base aplicável, com diferenças concentradas
nos instaladores e integrações do sistema.

### Fase 9 — Medir e otimizar o conjunto

1. Registrar hardware, versões, método de medição e cenário de cada teste.
2. Medir abertura após boot e reabertura com o ambiente já preparado.
3. Medir memória, CPU em repouso, CPU durante uso e tamanho instalado.
4. Separar o consumo do launcher, runtime e processos Android quando possível.
5. Definir metas a partir da medição inicial, sem números arbitrários.
6. Comparar encerramento após uso e permanência para notificações.
7. Reduzir componentes e serviços apenas com evidência de que são dispensáveis.
8. Repetir autenticação e persistência após cada alteração relevante.

- [ ] Criar `docs/performance.md` com resultados antes/depois.
- [ ] Registrar os ganhos e as limitações em cada ambiente testado.

**Conclusão:** melhorias mensuradas, metas atingidas ou limitações explicitadas,
sem regressão na autenticação oficial.

### Fase 10 — Estabelecer manutenção e releases

1. Automatizar compilação, verificações de código e geração dos pacotes.
2. Manter testes de instalação e ciclo de execução em ambientes limpos.
3. Usar máquinas virtuais com sessão gráfica e hardware real quando necessário;
   testes em contêiner não substituem a validação de GPU, kernel e autenticação.
4. Monitorar atualizações dos componentes incorporados e do aplicativo oficial.
5. Testar atualizações em uma combinação conhecida antes de ampliar a entrega.
6. Definir versionamento do produto, runtime e imagem Android.
7. Definir recuperação/retorno de versão sem presumir compatibilidade dos dados
   Android ou possibilidade de restaurar um token cadastrado.
8. Registrar componentes, hashes, fontes, licenças e evidências de cada release.
9. Documentar fim de suporte e revisão periódica das distribuições atendidas.

- [ ] Criar `docs/maintenance.md` e `docs/releases.md`.
- [ ] Estabelecer processo para correções urgentes e comunicação de mudanças.

**Conclusão:** uma nova versão pode ser construída, validada e entregue por
procedimento documentado, preservando a rastreabilidade da versão anterior.

## 7. Marcos e condições de avanço

| Marco | Entrega | Condição para avançar |
| --- | --- | --- |
| M0 — Base inicial | Launcher e roadmap | Código inicial verificado e documentação criada. |
| M1 — Viabilidade | Certisign funcionando no Waydroid de referência | Cadastro e autenticação reais aceitos. |
| M2 — Base compartilhada | Runtime e imagem rastreáveis | Funcionamento nas duas famílias de distribuição escolhidas. |
| M3 — Uso integrado | Configuração e interface | Uso sem preparação manual das dependências específicas. |
| M4 — Primeiro pacote | RPM | Instalação limpa e ciclo completo validados no Fedora. |
| M5 — Portabilidade | DEB | Mesma base validada na segunda distribuição. |
| M6 — Release mantida | Medições e processo de atualização | Evidências de desempenho e procedimento de manutenção. |

M0 está parcialmente concluído: o launcher, os documentos e o repositório Git
local existem; primeiro commit e licença do código próprio continuam pendentes.

## 8. Questões abertas

| Questão | Quando resolver | Impacto |
| --- | --- | --- |
| O app utilizado é exatamente o pacote configurado? | Fase 1 | Define o aplicativo que será validado. |
| O serviço aceita cadastro e autenticação no ambiente? | Fase 1 | Determina a viabilidade do produto. |
| O fluxo exige notificações ou recursos específicos? | Fase 1 | Define serviços e ciclo de execução. |
| Qual distribuição DEB e quais versões serão suportadas? | Fase 2 | Define a matriz e a base de compilação. |
| Quais dependências podem ser incorporadas de forma portável? | Fases 2–3 | Define o conteúdo real do runtime. |
| Como isolar a instalação e os dados de outro Waydroid? | Fases 2–5 | Evita conflitos com instalações existentes. |
| Quais componentes podem ser redistribuídos? | Fases 3–4 | Define inclusão de imagens, componentes Google e aplicativo. |
| Imagem incluída no instalador ou entregue separadamente? | Fase 4 | Afeta tamanho, primeira execução e suporte offline. |
| Qual tecnologia de interface gráfica? | Fase 6 | Afeta integração, acessibilidade e consumo. |
| Quais metas de desempenho são realistas? | Fase 9 | Depende das medições do conjunto. |

## 9. Como manter este documento atualizado

Após cada entrega relevante:

1. Atualizar a data no início do documento.
2. Marcar como concluído somente o item efetivamente entregue e verificado.
3. Atualizar a situação atual e o marco correspondente quando necessário.
4. Acrescentar uma entrada ao histórico abaixo, preservando as anteriores.
5. Apontar arquivos, comandos, versões e resultado que comprovem a entrega.
6. Registrar limitações e próximo passo quando uma etapa ficar parcial.
7. Documentar mudanças de arquitetura com motivo e impacto nas fases seguintes.

O roadmap registra o progresso. Resultados extensos de testes e medições devem
ficar em `docs/`, referenciados pela entrada correspondente. Não registrar
credenciais, dados do titular, QR codes de ativação ou e-tokens nesses arquivos.

Modelo para novas entradas:

```markdown
### AAAA-MM-DD — Título da entrega

- Estado: concluído / parcial / impedido por condição identificada.
- Fase ou marco: identificação correspondente.
- Realizado: mudança concreta.
- Evidências: arquivos, revisão/commit quando houver e verificações executadas.
- Resultado: o que foi demonstrado e quais limitações permanecem.
- Próximo passo: ação necessária para continuar.
```

## 10. Histórico de execução

### 2026-10-16 — Protótipo Rust inicial

- Estado: concluído como launcher inicial; autenticação real pendente.
- Fase ou marco: Fase 0 / parte do M0.
- Realizado: projeto Cargo, perfil de release, comandos `doctor`, `status`,
  `open` e `stop`, pacote oficial fixo e verificações de Wayland.
- Evidências: `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `tests/launcher.rs`
  e `README.md`.
- Verificações executadas na implementação: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test` (6 aprovados)
  e `cargo build --release`.
- Resultado: interface com um Waydroid simulado validada. O diagnóstico local
  encontrou socket Wayland e ausência de Waydroid. Não houve cadastro ou
  autenticação real, instalação do Android ou instalação do produto no sistema.
- Próximo passo: comprovar funcionamento do app oficial em um ambiente de referência.

### 2026-10-16 — Definição da direção de distribuição

- Estado: direção de arquitetura acordada; implementação pendente.
- Fase ou marco: planejamento das Fases 2–4, 7 e 8.
- Realizado: definida a preferência por runtime incorporado e versionado,
  reutilizado entre distribuições, com RPM inicial e DEB posterior.
- Motivo: reduzir a necessidade de manter receitas distintas para instalar
  dependências específicas em cada distribuição.
- Resultado: o runtime passa a ser responsabilidade do produto; compatibilidade
  com kernel, gráficos e serviços do hospedeiro continua precisando de validação.
- Limitações: não há runtime incorporado, scripts de configuração, imagem própria,
  RPM ou DEB gerados. A entrega completamente offline ainda precisa ser definida.
- Próximo passo: validar viabilidade antes de construir a distribuição integrada.

### 2026-10-16 — Criação do roadmap de acompanhamento

- Estado: concluído.
- Fase ou marco: documentação da Fase 0.
- Realizado: registrados objetivo, decisões, arquitetura, estrutura proposta,
  fases, critérios de conclusão, questões abertas e formato de histórico.
- Evidências: `roadmap.MD` e referência no `README.md`.
- Resultado: planejamento separado do estado implementado, com procedimento
  para documentar novas entregas ao longo do projeto.
- Próximo passo: iniciar a Fase 1 e registrar o resultado da validação real.

### 2026-10-18 — Início da validação em ambiente de referência

- Estado: parcial; preparação do Android em andamento.
- Fase ou marco: Fase 1 / M1, com avanço no controle de versão da Fase 0.
- Realizado: inicializado o Git local na branch `main`; inspecionados Fedora 44,
  kernel, Binder, Wayland, GPU AMD e espaço livre; instalado Waydroid 1.6.3 e
  dependências pelos repositórios oficiais do Fedora, com autorização do sistema.
- Evidências: [docs/validation.md](docs/validation.md), consulta dos RPMs,
  serviço `waydroid-container.service` ativo e download da imagem GAPPS iniciado.
- Ocorrência: o DNF invalidou uma transação offline de atualização anteriormente
  agendada; usuário informado sobre a necessidade de reagendamento.
- Resultado: ambiente de referência em preparação. A instalação de pacotes do
  Fedora nesta prova não substitui a futura construção do runtime incorporado.
- Limitações: não houve instalação do Certisign, cadastro ou autenticação real;
  primeiro commit, licença do código próprio e pacotes do produto continuam pendentes.
- Próximo passo: concluir o Android, preparar acesso à loja oficial e testar o app.

### 2026-10-18 — Definição dos sistemas de acesso a validar

- Estado: alvos registrados; testes pendentes.
- Fase ou marco: Fase 1 / M1.
- Realizado: registrados GERID, Projudi, PJe e eproc, conforme indicação do titular.
- Evidências: matriz em [docs/validation.md](docs/validation.md).
- Resultado: cada sistema terá registro próprio de aceitação do e-token,
  acesso pelo navegador e integração de assinatura quando aplicável.
- Próximo passo: identificar as instâncias no momento dos testes e registrar
  os resultados sem generalizar a aprovação de um sistema para os outros.

### 2026-10-18 — Levantamento preliminar das dependências de referência

- Estado: parcial; inventário transitivo e construção do runtime pendentes.
- Fase ou marco: levantamento de apoio às Fases 2 e 3, durante a preparação da Fase 1.
- Realizado: consultadas dependências dos RPMs instalados, arquivos de serviço,
  integração Binder e metadados oficiais das imagens anunciadas.
- Evidências: [docs/runtime-inventory.md](docs/runtime-inventory.md).
- Resultado: identificada exigência de `GLIBC_2.43` no LXC do Fedora instalado,
  reforçando a necessidade de uma base de compilação compatível com os sistemas-alvo.
- Próximo passo: concluir a validação do app e, antes de construir o runtime,
  escolher os sistemas suportados e completar o inventário.

### 2026-10-18 — Primeira inicialização do Android de referência

- Estado: preparação técnica concluída; instalação do app e autenticação pendentes.
- Fase ou marco: Fase 1 / M1.
- Realizado: concluídos downloads e verificações das imagens de sistema e vendor;
  iniciada a sessão Android; executado o diagnóstico Rust contra o Waydroid real;
  solicitada a abertura da Play Store na página do Certisign oficial.
- Evidências: [docs/validation.md](docs/validation.md), inicializador com saída
  de sucesso, mensagem `Android with user 0 is ready` e diagnóstico do launcher.
- Resultado: Play Store presente; Certisign ainda não instalado na consulta antes
  da pausa do titular. Na retomada, contêiner observado como `FROZEN`.
- Limitações: visualização da janela, login na loja, instalação do Certisign,
  cadastro e aceitação do e-token ainda dependem de confirmação/teste com o titular.
- Próximo passo: retomar a instalação oficial e registrar o resultado.

### 2026-10-18 — Recuperação da sessão e retomada do teste

- Estado: ambiente reaberto; instalação do Certisign pendente.
- Fase ou marco: Fase 1 / M1.
- Realizado: recuperado registro antigo de sessão com encerramento e nova
  inicialização; solicitada novamente a abertura da página do app na Play Store.
- Evidências: [docs/validation.md](docs/validation.md), nova mensagem de Android
  pronto e consulta de pacotes confirmando Play Store presente e Certisign ausente.
- Próximo passo: titular instalar o aplicativo oficial e informar o resultado
  ou a mensagem exibida, sem compartilhar credenciais ou códigos.

### 2026-10-18 — Certisign instalado e impedimento por emulador

- Estado: instalação concluída; M1 sem autenticação aceita.
- Fase ou marco: Fase 1 / M1.
- Realizado: titular instalou o app pela loja e apresentou a tela de restrição;
  consultados versão, ABI e propriedades do ambiente; feita análise estática
  limitada dos componentes instalados e executado o launcher Rust com o app presente.
- Evidências: [docs/emulator-block.md](docs/emulator-block.md) e
  [docs/validation.md](docs/validation.md).
- Resultado: app 1.4.1 contém referências a checagens locais de arquitetura/emulação;
  não há contorno validado nem evidência de autenticação real.
- Limitações: não foi determinado se há validação adicional pelo servidor.
  Não foram modificados o APK ou as propriedades do ambiente nesta investigação.
- Próximo passo: avaliar um experimento delimitado para identificar a barreira
  efetiva, com recuperação documentada e sem confundir abertura com autenticação.

### 2026-10-18 — Primeiro experimento reversível de configuração

- Estado: aplicado; resultado visual e autenticação pendentes.
- Fase ou marco: investigação da Fase 1 / M1.
- Realizado: criado auxiliar de laboratório com backup e restauração protegidos;
  aplicados `dalvik.vm.extra-opts=-Dos.arch=aarch64` e atualização offline da
  configuração; reiniciada a sessão e reaberto o aplicativo oficial.
- Evidências: [docs/experiment-os-arch.md](docs/experiment-os-arch.md),
  `scripts/experiment-os-arch.py` e `tests/test_os_arch_experiment.py`.
- Verificação: quatro testes de backup/restauração aprovados; opção observada
  na nova sessão; Android pronto; APKs base e x86-64 comparados e idênticos aos anteriores.
- Limitações: ler a opção não comprova a resposta Java ao app nem a superação
  do bloqueio. A observação da tela foi solicitada ao titular.
- Próximo passo: registrar a tela exibida; restaurar a configuração original se
  a hipótese falhar ou se surgirem problemas de funcionamento.

### 2026-10-18 — Carteira aberta após o ajuste experimental

- Estado: abertura confirmada; geração e aceitação do e-token pendentes.
- Fase ou marco: avanço parcial da Fase 1 / M1.
- Realizado: titular confirmou que o app abriu corretamente e enviou captura
  mostrando a carteira e a opção e-Token após o experimento de arquitetura Java.
- Evidências: relato e captura apresentados no chat; resultado registrado em
  [docs/experiment-os-arch.md](docs/experiment-os-arch.md). A captura e os dados
  dos certificados não foram armazenados no repositório.
- Resultado: restrição inicial ultrapassada na versão 1.4.1 e no ambiente testado,
  preservando o APK oficial. Não houve teste confirmado de autenticação por e-token.
- Próximo passo: titular verificar a geração do e-token e sua aceitação no GERID;
  depois validar persistência em nova sessão e após reinício do computador.

### 2026-10-18 — Primeiro teste de e-token/acesso confirmado

- Estado: teste inicial confirmado pelo titular; persistência pendente.
- Fase ou marco: avanço parcial da Fase 1 / M1.
- Realizado: solicitado teste de geração do e-token no app oficial e uso no GERID;
  titular respondeu que funcionou perfeitamente.
- Evidências: confirmação textual no chat, registrada em
  [docs/validation.md](docs/validation.md). Nenhum código ou credencial coletado.
- Resultado: primeiro teste funcional positivo na configuração experimental,
  preservando o APK oficial. A evidência é o relato do titular, não uma verificação
  automatizada da sessão autenticada.
- Limitações: detalhes do navegador e componentes de integração não foram
  informados; não se generaliza o resultado a todo navegador Linux ou a outros sistemas.
- Próximo passo: repetir o teste após nova sessão Android, depois após reboot
  em momento apropriado, e completar os detalhes do ambiente utilizado.

### 2026-10-18 — Reabertura para verificar persistência

- Estado: nova sessão iniciada; repetição do acesso aguardando o titular.
- Fase ou marco: teste de persistência da Fase 1 / M1.
- Realizado: encerrada e iniciada novamente a sessão Android; observados Android
  pronto e opção experimental mantida; reaberto o Certisign pelo launcher Rust.
- Evidências: [docs/validation.md](docs/validation.md), mensagem da nova sessão
  e consulta de `dalvik.vm.extra-opts`.
- Resultado: inicialização e abertura encaminhadas com sucesso; continuidade
  do cadastro e aceitação de novo e-token ainda não confirmadas.
- Próximo passo: obter o resultado do titular; validar reboot em momento apropriado.

### 2026-10-18 — Persistência entre sessões confirmada pelo titular

- Estado: teste entre sessões concluído; reboot pendente.
- Fase ou marco: avanço parcial da Fase 1 / M1.
- Realizado: após encerrar e iniciar novamente o Android e reabrir o Certisign,
  titular confirmou que os certificados continuam disponíveis e o e-token
  permanece aceito no mesmo acesso.
- Evidências: resposta explícita do titular ao teste de persistência,
  registrada em [docs/validation.md](docs/validation.md).
- Resultado: ajuste e funcionamento mantidos na nova sessão, sem novo cadastro
  relatado. Não foram coletados códigos ou credenciais.
- Próximo passo: repetir o acesso após reboot em momento apropriado; documentar
  o navegador e as integrações utilizadas e validar os demais sistemas.

### 2026-10-23 — Janela 9:16, segundo plano e publicação

- Estado: concluído.
- Fase ou marco: Fase 5 (início, abertura e encerramento) e Fase 0.
- Realizado: janela única 450×800 sem suspensão; sessão Android iniciada fora
  do terminal e lançamento após `sys.boot_completed`; reabertura da janela após
  fechá-la (o Waydroid passa a ignorar a tarefa fechada; o launcher Android
  limpa essa marca); atalho próprio e ocultação dos demais apps; renomeado para
  Redoma, licença GPL-3.0-or-later, SECURITY.md e publicação em
  `github.com/rodolfoaltenrath/redoma-linux`.
- Evidências: testes de integração; confirmação visual do titular.
- Resultado: multi-janela descartado (app reduzido dentro de moldura no GNOME);
  janela sem barra de título no GNOME permanece como limitação conhecida.

### 2026-10-24 — Assistente gráfico e binário único

- Estado: implementado; instalação do zero pendente de validação.
- Fase ou marco: Fases 5 e 6.
- Realizado: assistente GTK4/libadwaita com verificação do sistema, opções
  (ocultar apps, dock, autostart, área de transferência), instalação
  administrativa por `pkexec` com progresso, configuração, guia da Play Store
  com registro de dispositivo e abertura do app. Ajuste `os.arch` portado de
  Python para Rust, compatível com o backup existente; ferramenta Python removida.
- Evidências: 30 testes; helper administrativo executado no ambiente de
  referência pulando todas as etapas sem alterar `waydroid.cfg`; assistente
  percorrido até o fim e ícone confirmado pelo titular.
- Próximo passo: validar a instalação do zero numa máquina ou VM Fedora limpa;
  barra de título.

### 2026-10-24 — Primeiro RPM

- Estado: gerado e instalado no ambiente de referência.
- Fase ou marco: Fase 7.
- Realizado: `packaging/rpm/redoma.spec`, atalho e ícone de sistema em
  `packaging/common/` e `scripts/build-rpm.sh`. O pacote empacota o binário
  compilado fora do rpmbuild, declara `waydroid` e `polkit` e não executa
  configuração na transação.
- Evidências: rpmlint sem erros além de ortografia em português e falta de
  página de manual; instalação por `dnf`; ícone, reabertura e "Configurar
  Redoma" confirmados pelo titular usando `/usr/bin/redoma`.
- Próximo passo: instalação limpa, atualização e remoção numa máquina sem o
  ambiente de referência.

## 11. Referências técnicas

- [Aplicativo oficial Certisign](https://play.google.com/store/apps/details?id=com.certisign.certisign)
- [Código-fonte do Waydroid](https://github.com/waydroid/waydroid)
- [Licença do Waydroid](https://github.com/waydroid/waydroid/blob/main/LICENSE)
- [Instalação e abertura de aplicativos](https://docs.waydro.id/usage/install-and-run-android-applications)
- [Imagens personalizadas do Waydroid](https://docs.waydro.id/faq/using-custom-waydroid-images)
- [Compilação das imagens Android](https://docs.waydro.id/development/compile-waydroid-lineage-os-based-images)
- [Integrações instaladas pelo Waydroid](https://github.com/waydroid/waydroid/blob/main/Makefile)
- [Compatibilidade de aplicações Linux portáveis](https://docs.appimage.org/reference/best-practices.html)

Consultar novamente as versões atuais das referências durante a execução;
links para branches de desenvolvimento não substituem versões fixadas em releases.
