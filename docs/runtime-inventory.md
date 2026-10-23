# Inventário inicial do ambiente de referência

Data: 18/10/2026  
Estado: levantamento preliminar, anterior à construção do runtime incorporado.

Este documento registra evidências obtidas durante a instalação do Waydroid
de referência. Não constitui um manifesto completo, uma lista final de arquivos
redistribuíveis ou uma declaração de portabilidade entre distribuições.

## Dependências observadas

Fonte: `rpm -q --requires waydroid`, Waydroid `1.6.3-1.fc44` instalado.

| Componente | Papel a considerar no runtime |
| --- | --- |
| Python 3 | Interpretador usado pelo gerenciador Waydroid. |
| dbus-python | Comunicação com os serviços D-Bus. |
| gbinder-python >= 1.3 | Comunicação com os serviços Android por Binder. |
| PyGObject e GTK 3 | Integrações do gerenciador atual; avaliar quais partes são necessárias ao fluxo próprio. |
| LXC | Execução do contêiner Android. |
| dnsmasq | Serviços de rede usados pela configuração de referência. |
| iproute | Ferramentas de configuração da rede. |
| nftables | Configuração da rede na integração Fedora observada. |
| shell POSIX | Scripts e operações auxiliares. |
| waydroid-selinux | Política fornecida pelo Fedora quando aplicável ao sistema. |

As dependências transitivas também precisam ser inventariadas. A consulta do
LXC instalado já revelou `liblxc`, libc, libcap, libgcc, libseccomp, lxcfs,
OpenSSL e rsync. Copiar apenas os executáveis listados acima não produz uma
distribuição completa.

## Restrição concreta de compatibilidade binária

A saída de `rpm -q --requires lxc` para `lxc-6.0.6-1.fc44` inclui
`libc.so.6(GLIBC_2.43)(64bit)`.

Isso impede tratar os binários do Fedora como uma base universal para sistemas
com uma libc mais antiga. A construção do runtime deve escolher uma base
compatível com os sistemas suportados, compilar as dependências adequadamente
e verificar os símbolos exigidos pelos artefatos finais.

A instalação de referência é útil para comprovar o aplicativo; seus RPMs não
devem ser simplesmente extraídos e renomeados como runtime portável.

## Integrações do hospedeiro observadas

| Recurso | Evidência na instalação Fedora |
| --- | --- |
| Serviço do contêiner | `/usr/lib/systemd/system/waydroid-container.service` |
| Ativação D-Bus | `/usr/share/dbus-1/system-services/id.waydro.Container.service` |
| Política de autorização | `/usr/share/polkit-1/actions/id.waydro.Container.policy` |
| Montagem Binder | Unidade `dev-binderfs.mount`, solicitada pelo serviço do contêiner |
| Drivers e nós Binder | Verificações em `tools/helpers/drivers.py` do Waydroid instalado |

O serviço de referência executa `/usr/bin/waydroid container start` e publica
`id.waydro.Container`. A instalação incorporada precisará definir caminhos,
propriedade dos serviços e coexistência com uma instalação externa. Não basta
alterar o caminho usado pelo launcher Rust.

Nenhuma política de segurança do hospedeiro foi desativada para este levantamento.

## Imagens anunciadas pelos canais oficiais

Metadados consultados em 18/10/2026:

| Componente | Arquivo | Tamanho compactado |
| --- | --- | --- |
| system | `lineage-20.0-20260927-GAPPS-waydroid_x86_64-system.zip` | 1.191.198.089 bytes |
| vendor | `lineage-20.0-20260927-MAINLINE-waydroid_x86_64-vendor.zip` | 189.818.687 bytes |

SHA-256 anunciado para o arquivo de sistema:

```text
1d79df0b17ab8f79d66fd3b5f94500cc02f1abe149cf8caa391380092ede6e2a
```

SHA-256 anunciado para o arquivo de vendor:

```text
d911b8353f6c807b94790b1c41c67e863a9f3dcd1cf3ec0232351398235afd7a
```

Fontes dos metadados:

- [Canal GAPPS x86-64](https://ota.waydro.id/system/lineage/waydroid_x86_64/GAPPS.json)
- [Canal MAINLINE x86-64](https://ota.waydro.id/vendor/waydroid_x86_64/MAINLINE.json)

Os hashes acima identificam os arquivos ZIP anunciados, não as imagens
descompactadas. O inicializador instalado verifica os downloads contra os hashes
do canal antes de extrair os arquivos. O resultado observado da inicialização
e da execução deve ser consultado em [validation.md](validation.md).

## Próximas verificações

- Completar as dependências transitivas e identificar as efetivamente usadas.
- Definir a distribuição DEB e a compatibilidade mínima antes de compilar a base.
- Verificar bibliotecas compartilhadas, módulos Python e recursos externos.
- Inventariar fontes, licenças e obrigações de distribuição por componente.
- Avaliar execução sem os inicializadores gráficos do Waydroid, preservando
  as integrações de sessão necessárias.
- Definir separação entre runtime somente leitura e dados persistentes.
- Validar a arquitetura proposta em uma máquina sem os pacotes de referência.

Essas atividades complementam as Fases 2 e 3 do [roadmap](../roadmap.MD).
A autenticação real do Certisign continua sendo a condição para investir na
construção e distribuição completa do runtime.
