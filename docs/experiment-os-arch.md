# Experimento 1 — Arquitetura informada ao Java

Data: 18/10/2026  
Estado: abertura, e-token/acesso e persistência entre sessões confirmados pelo titular; reboot pendente.

## Hipótese

A análise registrada em [emulator-block.md](emulator-block.md) encontrou
referências a `System.getProperty("os.arch")`, `x86` e `i686` no método ligado
ao plugin `safe_device`. O experimento testa se alterar a propriedade Java na
inicialização permite avançar além da tela de emulador.

Não se altera a arquitetura real do processador, o código nativo instalado,
o APK, a assinatura do app ou a resposta de serviços de atestação.

## Alteração delimitada

Em `/var/lib/waydroid/waydroid.cfg`, seção `[properties]`:

```ini
dalvik.vm.extra-opts = -Dos.arch=aarch64
```

Não foram alteradas propriedades de fabricante, modelo, fingerprint, ABI,
bootloader, root ou segurança. Não foram instalados frameworks de instrumentação.

A implementação Android 13 consultada aceita opções extras da VM por
`dalvik.vm.extra-opts`. A inicialização de `java.lang.System` permite que
propriedades recebidas pela VM sobreponham determinados valores iniciais.
Isso sustenta a hipótese; ler a opção Android de volta não comprova, isoladamente,
o valor retornado ao aplicativo nem a superação do bloqueio.

Fontes consultadas:

- [AOSP AndroidRuntime.cpp](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/android13-release/core/jni/AndroidRuntime.cpp)
- [AOSP System.java](https://android.googlesource.com/platform/libcore/+/refs/heads/android13-release/ojluni/src/main/java/java/lang/System.java)

## Backup e ferramenta de laboratório

A ferramenta [experiment-os-arch.py](../scripts/experiment-os-arch.py) é um
auxiliar de desenvolvimento, separado da aplicação Rust e de seu instalador.
Usa a biblioteca padrão Python já disponível no ambiente de referência.

Backup protegido, no hospedeiro:

```text
/var/lib/waydroid/certisign-os-arch-experiment/
├── waydroid.cfg.original
├── waydroid.cfg.experimental
└── manifest.json
```

O diretório tem modo 0700 e os arquivos têm modo 0600. O manifesto registra
hashes das duas configurações. A ferramenta preserva o backup na reaplicação
e recusa sobrescrever alterações posteriores não reconhecidas. A troca do
arquivo preserva permissões, atributos e contexto SELinux da configuração.

Foram aprovados quatro testes com arquivos temporários, sem privilégios:

- Reaplicação mantém o backup e restauração recupera bytes e modo originais.
- Restauração recusa sobrescrever alterações externas posteriores.
- Opções extras já existentes na VM são preservadas por recusa da alteração.
- Backup modificado é rejeitado.

Comando de teste:

```sh
python3 -m unittest discover -s tests -p 'test_*.py'
```

## Execução observada

1. Aplicada a configuração pelo auxiliar com autorização administrativa do Fedora.
2. Encerrada a sessão Android pelos comandos normais do Waydroid.
3. Executado `waydroid upgrade -o`, sem baixar novas imagens.
4. Confirmada a presença da opção no arquivo `waydroid_base.prop` gerado.
5. Iniciada uma nova sessão por unidade temporária do gerenciador systemd do usuário:

   ```sh
   systemd-run --user --unit=certisign-reference-session --collect \
     /usr/bin/waydroid session start
   ```

6. Observada a mensagem `Android with user 0 is ready` na nova inicialização.
7. `waydroid prop get dalvik.vm.extra-opts` retornou `-Dos.arch=aarch64`.
8. Reaberto o app por `./target/release/certisign-linux open`.
9. Solicitada ao titular a confirmação da tela exibida, sem entrada de credenciais.

A reaplicação exigiu autorização administrativa própria do Fedora, confirmada
pelo titular. Durante a espera houve uma inicialização intermediária sem a nova
opção; ela foi encerrada antes de executar a sessão usada neste teste.

## Resultado e condição de avanço

| Verificação | Resultado |
| --- | --- |
| Backup e proteção contra sobrescrita | Implementados; quatro testes aprovados. |
| Configuração gerada pelo Waydroid | Opção experimental presente. |
| Inicialização Android com a opção | Android pronto observado na nova sessão. |
| APK base e split x86-64 | Comparados com as cópias anteriores ao experimento; idênticos. |
| Tela de emulador desapareceu | Confirmado pelo titular em 18/10/2026; captura mostra a carteira do app. |
| Geração e aceitação do e-token | Primeiro teste confirmado pelo titular em resposta à orientação de testar o GERID. |
| Persistência entre sessões | Confirmada pelo titular: certificados disponíveis e e-token aceito após nova sessão Android. |
| Persistência após reboot | Pendente; computador não reiniciado. |

O titular informou que o app abriu corretamente e enviou uma captura mostrando
a tela de carteira e a opção e-Token. A imagem não foi salva no repositório,
e os detalhes dos certificados não são reproduzidos neste registro.

Isso confirma avanço além da restrição de emulador no ambiente e na versão
testados. A observação é consistente com a hipótese de detecção local por
arquitetura, mas não comprova que todas as verificações do aplicativo se resumam
a esse mecanismo. Posteriormente, em resposta à orientação para gerar o e-token
e usá-lo no GERID, o titular confirmou funcionamento perfeito. O resultado foi
registrado como relato do titular, sem captura dos códigos ou da sessão de acesso.
O titular também confirmou certificados disponíveis e novo e-token aceito após
encerrar e iniciar novamente a sessão Android. Persistência após reboot e
atualizações permanece pendente; o resultado não constitui homologação ou suporte oficial.

## Restauração

Estes comandos encerram a sessão Android inteira. Executar no diretório do projeto:

```sh
waydroid session stop
pkexec /usr/bin/python3 /home/altenrath/certisign-android/scripts/experiment-os-arch.py restore
pkexec /usr/bin/waydroid upgrade -o
systemd-run --user --unit=certisign-reference-session --collect \
  /usr/bin/waydroid session start
```

Depois de o Android iniciar:

```sh
./target/release/certisign-linux open
```

Se a ferramenta reportar mudança na configuração após o backup, não forçar a
cópia do arquivo antigo: primeiro revisar a diferença para preservar alterações
legítimas. A restauração não apaga aplicativos nem dados Android.
