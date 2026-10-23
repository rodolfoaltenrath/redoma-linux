use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread::sleep;
use std::time::Duration;

const PACKAGE: &str = "com.certisign.certisign";
const DESKTOP_ID: &str = "redoma.desktop";
// app_id da janela criada pelo Waydroid; associa a janela ao atalho próprio.
const WINDOW_APP_ID: &str = "waydroid.com.certisign.certisign";
const BOOT_TIMEOUT_SECONDS: u32 = 120;
const STOP_TIMEOUT_SECONDS: u32 = 30;
// Janela única em retrato 9:16. O modo multi-janela foi testado em 23/10/2026
// e exibiu o app reduzido dentro de uma moldura transparente no GNOME.
// Sem suspensão, fechar a janela não congela o Android nem as notificações.
const WINDOW_PROPERTIES: [(&str, &str); 4] = [
    ("persist.waydroid.multi_windows", "false"),
    ("persist.waydroid.width", "450"),
    ("persist.waydroid.height", "800"),
    ("persist.waydroid.suspend", "false"),
];
const HELP: &str = "redoma — app oficial Certisign no Linux, isolado em contêiner Waydroid

Uso: redoma <comando>

  doctor   Verifica Waydroid e acesso ao compositor Wayland
  status   Mostra o estado informado pelo Waydroid
  setup    Configura janela 9:16, Android em segundo plano e oculta outros apps
  open     Abre o app oficial (inicia a sessão em segundo plano se necessário)
  stop     Encerra a sessão Waydroid inteira, incluindo outros apps

O cadastro e a autenticação acontecem no app oficial.
Após 'setup', fechar a janela mantém o Android ativo em segundo plano.
Use 'stop' para encerrar; notificações não chegam enquanto estiver parado.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Erro: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let action = args.next().unwrap_or_else(|| OsString::from("--help"));
    if args.next().is_some() {
        return Err("argumentos extras não são aceitos. Use --help.".into());
    }
    match action.to_str() {
        Some("--help" | "-h" | "help") => {
            print!("{HELP}");
            Ok(())
        }
        Some("doctor") => doctor(),
        Some("status") => replace_with_waydroid(&["status"]),
        Some("setup") => setup(),
        Some("open") => {
            let waydroid = find_waydroid()?;
            wayland_socket()?;
            let was_running = ensure_session(&waydroid)?;
            integrate_desktop_reporting();
            if was_running {
                release_closed_window(&waydroid)?;
            }
            eprintln!("Abrindo o app oficial Certisign ({PACKAGE}).");
            eprintln!(
                "Fechar a janela mantém o Android em segundo plano; use 'stop' para encerrar."
            );
            // A sessão já roda destacada do terminal; o Waydroid só traz a janela.
            replace(waydroid, &["app", "launch", PACKAGE])
        }
        Some("stop") => replace_with_waydroid(&["session", "stop"]),
        _ => Err("comando desconhecido. Use --help.".into()),
    }
}

fn find_waydroid() -> Result<PathBuf, String> {
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

fn wayland_socket() -> Result<PathBuf, String> {
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

fn doctor() -> Result<(), String> {
    let backend = find_waydroid();
    let display = wayland_socket();
    match &backend {
        Ok(path) => println!("Waydroid: {}", path.display()),
        Err(message) => println!("Waydroid: {message}"),
    }
    match &display {
        Ok(path) => println!("Wayland: {}", path.display()),
        Err(message) => println!("Wayland: {message}"),
    }
    println!("App esperado: {PACKAGE}");
    println!("Este diagnóstico não valida a instalação do app nem a autenticação Certisign.");
    if backend.is_err() || display.is_err() {
        Err("pré-requisitos incompletos; veja o diagnóstico acima.".into())
    } else {
        println!("Próximo passo: consulte 'status' e teste 'open'.");
        Ok(())
    }
}

fn setup() -> Result<(), String> {
    let waydroid = find_waydroid()?;
    wayland_socket()?;
    ensure_session(&waydroid)?;
    let mut changed = false;
    for (key, value) in WINDOW_PROPERTIES {
        if waydroid_output(&waydroid, &["prop", "get", key])?.trim() != value {
            waydroid_output(&waydroid, &["prop", "set", key, value])?;
            println!("Propriedade ajustada: {key}={value}");
            changed = true;
        }
    }
    if changed {
        // Tamanho e modo da janela só valem para a próxima sessão.
        println!("Reiniciando a sessão Android para aplicar a janela.");
        waydroid_output(&waydroid, &["session", "stop"])?;
        wait_until(STOP_TIMEOUT_SECONDS, || Ok(!session_running(&waydroid)?))
            .map_err(|_| "a sessão Android não encerrou a tempo.".to_string())?;
        ensure_session(&waydroid)?;
    }
    integrate_desktop_reporting();
    replace_dock_favorite();
    println!("Configuração concluída. Use o ícone Certisign ou 'open'.");
    Ok(())
}

/// Ao fechar a janela, o Waydroid mantém o app em primeiro plano no Android,
/// mas passa a ignorá-lo até que ele saia da tela; relançar não traz a janela.
/// O launcher do Android nunca vira janela e limpa essa marca, então ele é
/// trazido à frente por um instante antes de abrir o Certisign novamente.
fn release_closed_window(waydroid: &Path) -> Result<(), String> {
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
fn ensure_session(waydroid: &Path) -> Result<bool, String> {
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
                if setsid() == -1 {
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

unsafe extern "C" {
    fn setsid() -> i32;
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

fn session_running(waydroid: &Path) -> Result<bool, String> {
    let status = waydroid_output(waydroid, &["status"])?;
    Ok(status.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(key, value)| key == "Session" && value.trim() == "RUNNING")
    }))
}

fn wait_until(seconds: u32, mut ready: impl FnMut() -> Result<bool, String>) -> Result<(), String> {
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

fn waydroid_output(waydroid: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(waydroid)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("não foi possível executar o Waydroid: {error}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn integrate_desktop_reporting() {
    match integrate_desktop() {
        Ok(0) => {}
        Ok(count) => eprintln!("Atalhos do menu ajustados: {count}"),
        Err(error) => eprintln!("Aviso: não foi possível ajustar os atalhos do menu: {error}"),
    }
}

fn data_home() -> Option<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
}

/// O Waydroid cria um atalho no menu do Linux para cada app Android e preserva
/// `NoDisplay` ao regenerá-los. Todos ficam ocultos, inclusive o do Certisign,
/// cujo `waydroid app launch` não reabre uma janela fechada; o atalho próprio
/// usa `open`. Os apps continuam instalados no Android.
fn integrate_desktop() -> io::Result<usize> {
    let Some(data_home) = data_home() else {
        return Ok(0);
    };
    let applications = data_home.join("applications");
    fs::create_dir_all(&applications)?;
    let mut changed = 0;
    let own_entry = applications.join(DESKTOP_ID);
    let content = desktop_entry(
        &env::current_exe()?,
        &data_home.join(format!("waydroid/data/icons/{PACKAGE}.png")),
    );
    if fs::read_to_string(&own_entry).ok().as_deref() != Some(content.as_str()) {
        fs::write(&own_entry, content)?;
        changed += 1;
    }
    Ok(changed + hide_waydroid_entries(&applications)?)
}

fn hide_waydroid_entries(applications: &Path) -> io::Result<usize> {
    let mut hidden = 0;
    for entry in fs::read_dir(applications)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("waydroid.") || !name.ends_with(".desktop") {
            continue;
        }
        if let Some(updated) = with_no_display(&fs::read_to_string(&path)?) {
            fs::write(&path, updated)?;
            hidden += 1;
        }
    }
    // O atalho "Waydroid" abre a interface completa com todos os apps.
    let full_ui = applications.join("Waydroid.desktop");
    let system_full_ui = Path::new("/usr/share/applications/Waydroid.desktop");
    if !full_ui.exists() && system_full_ui.exists() {
        let content = fs::read_to_string(system_full_ui)?;
        fs::write(full_ui, with_no_display(&content).unwrap_or(content))?;
        hidden += 1;
    }
    Ok(hidden)
}

fn desktop_entry(executable: &Path, icon: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Certisign\n\
         Comment=App oficial Certisign via Waydroid\n\
         Exec={} open\n\
         Icon={}\n\
         Terminal=false\n\
         Categories=Office;\n\
         StartupWMClass={WINDOW_APP_ID}\n",
        desktop_exec_argument(&executable.to_string_lossy()),
        icon.display(),
    )
}

/// Aspas e escapes da especificação Desktop Entry para um argumento de `Exec`.
fn desktop_exec_argument(argument: &str) -> String {
    let mut quoted = String::from("\"");
    for character in argument.chars() {
        match character {
            // Escape do Exec seguido do escape de string do arquivo.
            '"' | '`' | '$' | '\\' => {
                quoted.push_str("\\\\");
                quoted.push(character);
            }
            '%' => quoted.push_str("%%"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

/// Troca o favorito da dock do GNOME que aponta para o atalho do Waydroid.
fn replace_dock_favorite() {
    let waydroid_id = format!("'waydroid.{PACKAGE}.desktop'");
    let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.shell", "favorite-apps"])
        .stderr(Stdio::null())
        .output()
    else {
        return;
    };
    let favorites = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || !favorites.contains(&waydroid_id) {
        return;
    }
    let updated = if favorites.contains(&format!("'{DESKTOP_ID}'")) {
        favorites
            .replace(&format!("{waydroid_id}, "), "")
            .replace(&format!(", {waydroid_id}"), "")
    } else {
        favorites.replace(&waydroid_id, &format!("'{DESKTOP_ID}'"))
    };
    let replaced = Command::new("gsettings")
        .args(["set", "org.gnome.shell", "favorite-apps", updated.trim()])
        .status()
        .is_ok_and(|status| status.success());
    if replaced {
        println!("Favorito da dock trocado para o atalho {DESKTOP_ID}.");
    }
}

/// Retorna o atalho com `NoDisplay=true` no grupo principal, ou `None` se já estiver oculto.
fn with_no_display(content: &str) -> Option<String> {
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let header = lines
        .iter()
        .position(|line| line.trim() == "[Desktop Entry]")?;
    let end = lines[header + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .map_or(lines.len(), |offset| header + 1 + offset);
    match lines[header + 1..end].iter().position(|line| {
        line.split_once('=')
            .is_some_and(|(key, _)| key.trim() == "NoDisplay")
    }) {
        Some(offset) if lines[header + 1 + offset].trim().ends_with("=true") => return None,
        Some(offset) => lines[header + 1 + offset] = "NoDisplay=true".into(),
        None => lines.insert(header + 1, "NoDisplay=true".into()),
    }
    let mut updated = lines.join("\n");
    updated.push('\n');
    Some(updated)
}

fn replace_with_waydroid(args: &[&str]) -> Result<(), String> {
    replace(find_waydroid()?, args)
}

fn replace(executable: PathBuf, args: &[&str]) -> Result<(), String> {
    let error = Command::new(executable)
        .args(args)
        .env("LC_ALL", "C")
        .exec();
    Err(format!("não foi possível executar o Waydroid: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{desktop_exec_argument, with_no_display};

    #[test]
    fn quotes_exec_paths_per_desktop_entry_spec() {
        assert_eq!(desktop_exec_argument("/opt/a b/x"), "\"/opt/a b/x\"");
        assert_eq!(desktop_exec_argument("/p/$x%"), "\"/p/\\\\$x%%\"");
    }

    #[test]
    fn hides_an_entry_without_no_display() {
        let entry =
            "[Desktop Entry]\nName=Contacts\n\n[Desktop Action app-settings]\nName=App Settings\n";
        assert_eq!(
            with_no_display(entry).unwrap(),
            "[Desktop Entry]\nNoDisplay=true\nName=Contacts\n\n[Desktop Action app-settings]\nName=App Settings\n"
        );
    }

    #[test]
    fn replaces_a_visible_no_display_value() {
        let entry = "[Desktop Entry]\nNoDisplay=false\nName=Contacts\n";
        assert_eq!(
            with_no_display(entry).unwrap(),
            "[Desktop Entry]\nNoDisplay=true\nName=Contacts\n"
        );
    }

    #[test]
    fn leaves_hidden_entries_and_other_groups_untouched() {
        assert!(with_no_display("[Desktop Entry]\nNoDisplay=true\n").is_none());
        let entry = "[Desktop Entry]\nName=Settings\n[Desktop Action a]\nNoDisplay=true\n";
        assert!(
            with_no_display(entry)
                .unwrap()
                .starts_with("[Desktop Entry]\nNoDisplay=true\n")
        );
    }
}
