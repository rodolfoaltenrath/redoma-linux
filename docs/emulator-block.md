# Bloqueio do Certisign no ambiente de referência

Data: 18/10/2026  
Estado: restrição inicial ultrapassada; acesso e persistência entre sessões confirmados pelo titular, reboot pendente.

## Resultado observado

O titular concluiu o acesso à Play Store, instalou o Certisign oficial e tentou
abri-lo. A tela enviada pelo titular informa:

> Não é possível continuar
>
> Este aplicativo não pode ser usado em emuladores por motivos de segurança.
>
> Para continuar, utilize um dispositivo compatível.

No teste inicial, o aplicativo instalado chegou a uma tela de restrição de dispositivo.
Naquele momento não havia evidência de funcionamento do e-token. O resultado
funcional posterior está registrado na seção do experimento abaixo.

## Metadados verificados no dispositivo

Obtidos por consulta filtrada a `dumpsys package com.certisign.certisign`:

| Campo | Valor |
| --- | --- |
| Pacote | `com.certisign.certisign` |
| Versão | `1.4.1` |
| versionCode | `127` |
| ABI principal | `x86_64` |
| minSdk / targetSdk | `29` / `36` |

A instalação contém um split nativo x86-64. Portanto, há código fornecido para
a arquitetura usada; o problema observado é a restrição exibida pelo app.

## Propriedades do ambiente consultadas

| Propriedade | Valor |
| --- | --- |
| `ro.product.model` | `WayDroid x86_64 Device` |
| `ro.product.manufacturer` | `Waydroid` |
| `ro.product.brand` | `waydroid` |
| `ro.product.device` | `waydroid_x86_64` |
| `ro.product.name` | `lineage_waydroid_x86_64` |
| `ro.hardware` | `unknown` |
| `ro.board.platform` | `waydroid` |
| `ro.product.cpu.abi` | `x86_64` |
| `ro.product.cpu.abilist` | `x86_64,x86` |
| `ro.build.version.release` | `13` |
| `ro.build.type` | `userdebug` |
| `ro.build.tags` | `test-keys` |
| `ro.debuggable` / `ro.secure` | `0` / `1` |

Não foram consultados identificadores pessoais, conteúdo de contas, chaves,
PIN, senhas ou e-tokens. Não foi coletado logcat bruto.

## Análise estática limitada

Foram examinadas cópias locais de `base.apk` e `split_config.x86_64.apk`
provenientes da instalação realizada pelo titular. As cópias de análise ficam
em `target/apk-inspection/`, fora do controle de versão, com acesso restrito.
O pacote instalado não foi modificado ou substituído.

Observações:

- Foram encontradas referências aos plugins Flutter `safe_device` e `device_info_plus`.
- No DEX instalado, o corpo do método ofuscado `Ls7/a;->onMethodCall` contém
  referências a `isRealDevice`, `os.arch`, `x86`, `i686`, `ro.kernel.qemu` e
  indicadores conhecidos de emulação.
- O código público atual do `safe_device` encaminha `isRealDevice` para uma
  checagem de emulador que inclui a arquitetura retornada por `System.getProperty("os.arch")`.
- A versão exata do plugin incorporado não foi identificada. A comparação com
  o código público é um indício; não substitui uma análise completa do fluxo executado.
- A presença de constantes e bibliotecas, isoladamente, não demonstra quais
  verificações adicionais são executadas pelo serviço Certisign.

Conclusão: há forte indício de detecção local de arquitetura/emulação. Alterar
somente propriedades de modelo ou fabricante não demonstra que essa verificação
deixará de falhar. O resultado do experimento posterior está registrado abaixo.

## Limites da conclusão

- Não foi comprovado que o Certisign exige Play Integrity neste fluxo.
- Não foi comprovado que a checagem local é a única barreira.
- Na investigação inicial não foram aplicados perfis de outro aparelho,
  alterações de arquitetura, patches no APK ou instrumentação do processo.
- O teste funcional e a persistência entre sessões foram confirmados pelo
  titular; ainda não há validação após reboot ou solução pronta para empacotamento.

Um eventual experimento posterior deverá ter alteração identificada, recuperação
documentada e teste completo até autenticação aceita. Passar pela tela inicial
não basta para declarar a Fase 1 concluída.

## Experimento posterior autorizado

Em 18/10/2026, após autorização do titular, foi aplicado um ajuste delimitado
na inicialização da VM Java, preservando o APK e a arquitetura nativa reais.
O backup, os testes, os comandos de restauração e as evidências estão em
[experiment-os-arch.md](experiment-os-arch.md). O titular confirmou a abertura
e apresentou a tela de carteira com a opção e-Token. Assim, a restrição inicial
foi ultrapassada nessa configuração. Em seguida, após a orientação para gerar
o e-token e usá-lo no GERID, o titular confirmou que funcionou perfeitamente.
Essa confirmação é um relato do teste real, sem captura de códigos pelo agente.

## Efeito no planejamento

A instalação, a abertura, o acesso e a persistência entre sessões foram confirmados.
M1 permanece parcial até repetir o acesso após reboot, registrar os detalhes do
teste e verificar as dependências do fluxo. Os demais sistemas de interesse
também continuam exigindo validação individual.

## Referências

- [safe_device — encaminhamento de isRealDevice](https://github.com/ufukhawk/safe_device/blob/main/android/src/main/java/com/xamdesign/safe_device/SafeDevicePlugin.java)
- [safe_device — checagens de emulador](https://github.com/ufukhawk/safe_device/blob/main/android/src/main/java/com/xamdesign/safe_device/Emulator/EmulatorCheck.java)
- [device_info_plus — informações e detecção do dispositivo](https://github.com/fluttercommunity/plus_plugins/blob/main/packages/device_info_plus/device_info_plus/android/src/main/kotlin/dev/fluttercommunity/plus/device_info/MethodCallHandlerImpl.kt)
- [Play Integrity — verificações de dispositivo e aplicativo](https://developer.android.com/google/play/integrity/verdicts)

As referências apontam para o código público consultado em 18/10/2026, não para
uma versão identificada de todos os componentes do APK instalado.
