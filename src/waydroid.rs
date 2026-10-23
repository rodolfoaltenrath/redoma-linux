//! Interface com o Waydroid: localização, sessão, propriedades e lançamento.

use std::env;
use std::fs::{self, File};
use std::io;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

pub const BOOT_TIMEOUT_SECONDS: u32 = 120;
pub const STOP_TIMEOUT_SECONDS: u32 = 30;
// Janela única em retrato 9:16. O modo multi-janela foi testado em 23/10/2026
// e exibiu o app reduzido dentro de uma moldura transparente no GNOME.
// Sem suspensão, fechar a janela não congela o Android nem as notificações.
pub const WINDOW_PROPERTIES: [(&str, &str); 4] = [
    ("persist.waydroid.multi_windows", "false"),
    ("persist.waydroid.width", "450"),
    ("persist.waydroid.height", "800"),
    ("persist.waydroid.suspend", "false"),
];

pub fn find_waydroid() -> Result<PathBuf, String> {
    let search_path = env::var_os("PATH").unwrap_or_default();
    for directory in env::split_paths(&search_path) {
        let candidate = directory.join("waydroid");
        if is_executable(&candidate) {
            return Ok(candidate);
        }
    }
    Err("Waydroid não encontrado no PATH. No Fedora: sudo dnf install waydroid".into())
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

pub fn wayland_socket() -> Result<PathBuf, String> {
    let display = env::var_os("WAYLAND_DISPLAY")
        .filter(|value| !value.is_empty())
        .ok_or("WAYLAND_DISPLAY ausente. Execute na sessão gráfica Wayland, sem sudo.")?;
    let display = PathBuf::from(display);
    let socket = if display.is_absolute() {
        display
    } else {
        let runtime = env::var_os("XDG_RUNTIME_DIR")
            .filter(|value| !value.is_empty())
            .ok_or("XDG_RUNTIME_DIR ausente. Execute na sessão gráfica, sem sudo.")?;
        PathBuf::from(runtime).join(display)
    };
    if !socket
        .metadata()
        .is_ok_and(|metadata| metadata.file_type().is_socket())
    {
        return Err(format!("socket Wayland indisponível: {}", socket.display()));
    }
    Ok(socket)
}

/// Ao fechar a janela, o Waydroid mantém o app em primeiro plano no Android,
/// mas passa a ignorá-lo até que ele saia da tela; relançar não traz a janela.
/// O launcher do Android nunca vira janela e limpa essa marca, então ele é
/// trazido à frente por um instante antes de abrir o Certisign novamente.
pub fn release_closed_window(waydroid: &Path) -> Result<(), String> {
    waydroid_output(
        waydroid,
        &[
            "app",
            "intent",
            "android.intent.action.APPLICATION_PREFERENCES",
            "",
        ],
    )?;
    sleep(Duration::from_secs(1));
    Ok(())
}

/// Inicia a sessão fora do grupo de processos do terminal, para que fechar o
/// terminal ou a janela não encerre o Android, e aguarda o fim do boot.
/// Retorna se a sessão já estava em execução.
pub fn ensure_session(waydroid: &Path) -> Result<bool, String> {
    let was_running = session_running(waydroid)?;
    if !was_running {
        eprintln!("Iniciando o Android em segundo plano.");
        let log = session_log();
        let mut command = Command::new(waydroid);
        command
            .args(["session", "start"])
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(log.as_ref().map_or_else(Stdio::null, clone_stdio))
            .stderr(log.as_ref().map_or_else(Stdio::null, clone_stdio));
        // SAFETY: setsid é async-signal-safe e não usa memória do processo pai.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command
            .spawn()
            .map_err(|error| format!("não foi possível iniciar a sessão Waydroid: {error}"))?;
        wait_until(BOOT_TIMEOUT_SECONDS, || session_running(waydroid))
            .map_err(|_| "a sessão Waydroid não iniciou a tempo.".to_string())?;
    }
    // Abrir o app antes do fim do boot falha sem mensagem de erro.
    wait_until(BOOT_TIMEOUT_SECONDS, || {
        Ok(waydroid_output(waydroid, &["prop", "get", "sys.boot_completed"])?.trim() == "1")
    })
    .map_err(|_| "o Android não concluiu a inicialização a tempo.".to_string())?;
    Ok(was_running)
}

fn clone_stdio(file: &File) -> Stdio {
    file.try_clone().map_or_else(|_| Stdio::null(), Stdio::from)
}

fn session_log() -> Option<File> {
    let directory = env::var_os("XDG_STATE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?
        .join("redoma");
    fs::create_dir_all(&directory).ok()?;
    File::create(directory.join("session.log")).ok()
}

pub fn session_running(waydroid: &Path) -> Result<bool, String> {
    let status = waydroid_output(waydroid, &["status"])?;
    Ok(status.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(key, value)| key == "Session" && value.trim() == "RUNNING")
    }))
}

pub fn wait_until(
    seconds: u32,
    mut ready: impl FnMut() -> Result<bool, String>,
) -> Result<(), String> {
    for _ in 0..seconds {
        if ready()? {
            return Ok(());
        }
        sleep(Duration::from_secs(1));
    }
    if ready()? {
        Ok(())
    } else {
        Err("tempo esgotado".into())
    }
}

pub fn waydroid_output(waydroid: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(waydroid)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("não foi possível executar o Waydroid: {error}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn replace_with_waydroid(args: &[&str]) -> Result<(), String> {
    replace(find_waydroid()?, args)
}

pub fn replace(executable: PathBuf, args: &[&str]) -> Result<(), String> {
    let error = Command::new(executable)
        .args(args)
        .env("LC_ALL", "C")
        .exec();
    Err(format!("não foi possível executar o Waydroid: {error}"))
}
