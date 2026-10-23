mod desktop;
mod waydroid;

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

use desktop::{integrate_desktop_reporting, replace_dock_favorite};
use waydroid::{
    STOP_TIMEOUT_SECONDS, WINDOW_PROPERTIES, ensure_session, find_waydroid, release_closed_window,
    replace, replace_with_waydroid, session_running, wait_until, waydroid_output, wayland_socket,
};

pub const PACKAGE: &str = "com.certisign.certisign";
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
