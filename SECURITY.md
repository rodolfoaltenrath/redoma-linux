# Política de segurança

O Redoma executa o aplicativo oficial Certisign num contêiner Android
(Waydroid) no Linux. Como o projeto envolve certificados digitais ICP-Brasil,
falhas de segurança são tratadas com prioridade e em sigilo até a correção.

## Como relatar

**Não abra issue pública** para falhas de segurança.

Use o relato privado do GitHub:
<https://github.com/rodolfoaltenrath/redoma-linux/security/advisories/new>

Inclua, se possível:

- descrição do problema e do impacto;
- passos para reproduzir;
- versão do Redoma (commit), distribuição, versão do Waydroid e do Certisign;
- sugestão de correção, se houver.

**Nunca envie** PIN, senha, e-token, certificado, código de autenticação,
CPF ou capturas de tela que mostrem esses dados. Nenhum relato precisa deles.

## Escopo

| Componente | Onde relatar |
| --- | --- |
| Código deste repositório (`src/`, `tests/`) e documentação | Aqui, pelo relato privado acima. |
| Configuração aplicada pelo Redoma (propriedades, atalhos, sessão) e ações administrativas (`redoma privileged`) | Aqui. Falhas que permitam escalar privilégio têm prioridade máxima. |
| Aplicativo Certisign, certificados, e-token ou serviços da Certisign | Diretamente à Certisign. |
| Waydroid, contêiner LXC ou imagem Android | Ao [projeto Waydroid](https://github.com/waydroid/waydroid/security). |

Se não souber onde a falha se encaixa, relate aqui e ajudaremos a encaminhar.

## Versões atendidas

O projeto é um protótipo sem versões publicadas. Correções são feitas apenas
na branch `main`.

## Resposta

O projeto é mantido por voluntários, sem prazo garantido. O objetivo é
confirmar o recebimento em poucos dias, avaliar o impacto e publicar a
correção com um aviso de segurança, dando crédito a quem relatou, se desejar.

## Modelo de segurança

O que o Redoma faz e não faz, o modelo de isolamento do contêiner e o que é
alterado no sistema estão na seção [Segurança do README](README.md#segurança).
Em resumo: o Redoma não recebe credenciais, não tem telemetria e não modifica
o app oficial. Usa privilégios de administrador apenas pela janela de senha do
sistema (`pkexec`), para ações fixas listadas em `src/privileged.rs`. O
contêiner isola arquivos, processos e rede, mas compartilha o kernel do Linux;
não equivale a uma máquina virtual.
