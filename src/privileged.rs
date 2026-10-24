//! Ações administrativas executadas por `pkexec redoma privileged <ação>`.
//!
//! A lista de ações é fechada: nenhum pacote, caminho ou comando vem dos
//! argumentos. O progresso é informado à interface por linhas no stdout:
//! `STEP <id> <texto>`, `SKIP <id> <texto>`, `PROGRESS <0-100|-> <texto>`,
//! `ANDROID_ID <id>` e `DONE`. Falhas encerram com código diferente de zero e
//! a mensagem no stderr.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::checks;
use crate::os_arch::{self, Action};

const DNF: &str = "/usr/bin/dnf";
const RPM: &str = "/usr/bin/rpm";
const SYSTEMCTL: &str = "/usr/bin/systemctl";
const WAYDROID: &str = "/usr/bin/waydroid";
const CONTAINER_SERVICE: &str = "waydroid-container.service";
const CLIPBOARD_PACKAGE: &str = "python3-pyclip";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clipboard {
    Allow,
    Block,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    InstallEnvironment { clipboard: Clipboard },
    AndroidId,
    RestoreOsArch,
}

pub fn parse(args: &[&str]) -> Result<Request, String> {
    match args {
        ["install-environment", "--clipboard=allow"] => Ok(Request::InstallEnvironment {
            clipboard: Clipboard::Allow,
        }),
        ["install-environment", "--clipboard=block"] => Ok(Request::InstallEnvironment {
            clipboard: Clipboard::Block,
        }),
        ["android-id"] => Ok(Request::AndroidId),
        ["restore-os-arch"] => Ok(Request::RestoreOsArch),
        _ => Err("ação administrativa desconhecida.".into()),
    }
}

pub fn run(request: Request) -> Result<(), String> {
    // SAFETY: geteuid não tem pré-condições.
    if unsafe { libc::geteuid() } != 0 {
        return Err("execute pelo pkexec; esta ação exige administrador.".into());
    }
    match request {
        Request::InstallEnvironment { clipboard } => install_environment(clipboard)?,
        Request::AndroidId => emit(&format!("ANDROID_ID {}", android_id()?)),
        Request::RestoreOsArch => {
            emit(&format!(
                "STEP os-arch {}",
                os_arch::run_locked(Action::Restore)?
            ));
            regenerate_properties()?;
        }
    }
    emit("DONE");
    Ok(())
}

fn install_environment(clipboard: Clipboard) -> Result<(), String> {
    let mut install = Vec::new();
    if !installed("waydroid") {
        install.push("waydroid");
    }
    let clipboard_installed = installed(CLIPBOARD_PACKAGE);
    if clipboard == Clipboard::Allow && !clipboard_installed {
        install.push(CLIPBOARD_PACKAGE);
    }
    if install.is_empty() {
        emit("SKIP packages Pacotes já instalados");
    } else {
        emit(&format!("STEP packages Instalando {}", install.join(", ")));
        let mut args = vec!["install", "-y"];
        args.extend(&install);
        stream(DNF, &args, &[], |line| emit(&format!("PROGRESS - {line}")))?;
    }
    if clipboard == Clipboard::Block && clipboard_installed {
        if required_by_others(CLIPBOARD_PACKAGE) {
            emit("SKIP clipboard python3-pyclip é usado por outro pacote; mantido");
        } else {
            emit("STEP clipboard Removendo sincronização da área de transferência");
            stream(DNF, &["remove", "-y", CLIPBOARD_PACKAGE], &[], |line| {
                emit(&format!("PROGRESS - {line}"))
            })?;
        }
    }

    let enabled = succeeds(SYSTEMCTL, &["is-enabled", "--quiet", CONTAINER_SERVICE]);
    let active = succeeds(SYSTEMCTL, &["is-active", "--quiet", CONTAINER_SERVICE]);
    if enabled && active {
        emit("SKIP service Serviço do contêiner ativo");
    } else {
        emit("STEP service Ativando o serviço do contêiner");
        stream(
            SYSTEMCTL,
            &["enable", "--now", CONTAINER_SERVICE],
            &[],
            |_| {},
        )?;
    }

    if checks::state().initialized {
        emit("SKIP images Imagens Android já instaladas");
    } else {
        emit("STEP images Baixando o Android com Google Play");
        // Sem buffer, para o progresso chegar enquanto o download acontece.
        stream(
            WAYDROID,
            &["init", "-s", "GAPPS"],
            &[("PYTHONUNBUFFERED", "1")],
            |line| match download_progress(line) {
                Some((done, total)) => emit(&format!(
                    "PROGRESS {} Baixando {done:.0} de {total:.0} MB",
                    (done / total * 100.0).clamp(0.0, 100.0) as u32
                )),
                None => emit(&format!("PROGRESS - {line}")),
            },
        )?;
    }

    if os_arch::is_applied(Path::new(os_arch::CONFIG)) {
        emit("SKIP os-arch Ajuste de compatibilidade já aplicado");
    } else {
        emit("STEP os-arch Aplicando ajuste de compatibilidade");
        emit(&format!(
            "PROGRESS - {}",
            os_arch::run_locked(Action::Apply)?
        ));
        regenerate_properties()?;
    }
    Ok(())
}

/// Aplica ao Android as opções do `waydroid.cfg`; o Waydroid para e reinicia
/// o contêiner sozinho se ele estiver em execução.
fn regenerate_properties() -> Result<(), String> {
    emit("STEP upgrade Atualizando a configuração do Android");
    stream(WAYDROID, &["upgrade", "-o"], &[], |line| {
        emit(&format!("PROGRESS - {line}"))
    })
}

/// Identificador usado no registro de dispositivo do Google Play. Exige a
/// sessão Android ativa e a Play Store aberta ao menos uma vez.
fn android_id() -> Result<String, String> {
    let output = Command::new(WAYDROID)
        // Variáveis exigidas pelo sqlite3 do Android, conforme a documentação
        // do Waydroid sobre certificação no Google Play.
        .args([
            "shell",
            "--",
            "env",
            "ANDROID_RUNTIME_ROOT=/apex/com.android.runtime",
            "ANDROID_DATA=/data",
            "ANDROID_TZDATA_ROOT=/apex/com.android.tzdata",
            "ANDROID_I18N_ROOT=/apex/com.android.i18n",
            "sqlite3",
            "/data/data/com.google.android.gsf/databases/gservices.db",
            "select value from main where name = 'android_id';",
        ])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("não foi possível executar o Waydroid: {error}"))?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() && !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) {
        Ok(id)
    } else {
        Err("ID do Google indisponível. Inicie o Android e abra a Play Store uma vez.".into())
    }
}

fn installed(package: &str) -> bool {
    succeeds(RPM, &["-q", "--quiet", package])
}

fn required_by_others(package: &str) -> bool {
    Command::new(RPM)
        .args(["-q", "--whatrequires", package])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| output.status.success())
}

fn succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Executa o programa repassando cada linha de saída; `\r` também separa
/// linhas, pois o Waydroid redesenha o progresso na mesma linha.
fn stream(
    program: &str,
    args: &[&str],
    environment: &[(&str, &str)],
    mut on_line: impl FnMut(&str),
) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .envs(environment.iter().copied())
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("não foi possível executar {program}: {error}"))?;
    let (sender, receiver) = mpsc::channel();
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|source| {
        let sender = sender.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(source);
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                match read_segment(&mut reader, &mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let line = String::from_utf8_lossy(&buffer).trim().to_owned();
                        if !line.is_empty() && sender.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        })
    })
    .collect();
    drop(sender);
    let mut last = String::new();
    for line in receiver {
        on_line(&line);
        last = line;
    }
    for reader in readers {
        let _ = reader.join();
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} falhou ({status}): {last}"))
    }
}

fn read_segment(reader: &mut impl BufRead, buffer: &mut Vec<u8>) -> io::Result<usize> {
    let mut total = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(total);
        }
        match available.iter().position(|&b| b == b'\n' || b == b'\r') {
            Some(end) => {
                buffer.extend_from_slice(&available[..end]);
                reader.consume(end + 1);
                return Ok(total + end + 1);
            }
            None => {
                let length = available.len();
                buffer.extend_from_slice(available);
                reader.consume(length);
                total += length;
            }
        }
    }
}

/// Lê `[Downloading] 123.45 MB/1234.5 MB ...`, formato de `tools/helpers/http.py`.
fn download_progress(line: &str) -> Option<(f64, f64)> {
    let rest = line.trim().strip_prefix("[Downloading]")?;
    let (done, rest) = rest.split_once("MB/")?;
    let total = rest.split_whitespace().next()?;
    let done: f64 = done.trim().parse().ok()?;
    let total: f64 = total.parse().ok()?;
    (total > 0.0).then_some((done, total))
}

fn emit(line: &str) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_fixed_actions() {
        assert_eq!(
            parse(&["install-environment", "--clipboard=block"]),
            Ok(Request::InstallEnvironment {
                clipboard: Clipboard::Block
            })
        );
        assert_eq!(parse(&["android-id"]), Ok(Request::AndroidId));
        for rejected in [
            &[][..],
            &["install-environment"][..],
            &["install-environment", "--clipboard=allow", "extra"][..],
            &["run", "/bin/sh"][..],
            &["android-id", "--package=x"][..],
        ] {
            assert!(parse(rejected).is_err(), "{rejected:?}");
        }
    }

    #[test]
    fn parses_waydroid_download_progress() {
        assert_eq!(
            download_progress("[Downloading]  512.5 MB/1024.0 MB      3.2 MB/s(approx.)"),
            Some((512.5, 1024.0))
        );
        assert_eq!(download_progress("[Downloading] 0 MB/0 MB"), None);
        assert_eq!(download_progress("Validating system image"), None);
    }

    #[test]
    fn splits_output_on_carriage_returns() {
        let mut reader = BufReader::new(&b"a\rb\nc"[..]);
        let mut segments = Vec::new();
        let mut buffer = Vec::new();
        while read_segment(&mut reader, &mut buffer).unwrap() > 0 {
            segments.push(String::from_utf8(buffer.clone()).unwrap());
            buffer.clear();
        }
        assert_eq!(segments, ["a", "b", "c"]);
    }
}
