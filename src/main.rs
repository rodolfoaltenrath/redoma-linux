mod checks;
mod desktop;
mod os_arch;
mod privileged;
mod settings;
mod setup;
mod ui;
mod waydroid;

use std::env;
use std::process::ExitCode;

use desktop::integrate_desktop_reporting;
use settings::Options;
use waydroid::{
    ensure_session, find_waydroid, release_closed_window, replace, replace_with_waydroid,
    wayland_socket,
};

pub const PACKAGE: &str = "com.certisign.certisign";
const HELP: &str = "redoma — app oficial Certisign no Linux, isolado em contêiner Waydroid

Uso: redoma [comando]

Sem comando, abre o Certisign; se o ambiente ainda não estiver pronto,
abre o assistente gráfico de instalação.

  wizard   Abre o assistente gráfico de instalação e configuração
  doctor   Verifica Waydroid e acesso ao compositor Wayland
  status   Mostra o estado informado pelo Waydroid
  setup    Configura janela 9:16, Android em segundo plano e oculta outros apps
  open     Abre o app oficial (inicia a sessão em segundo plano se necessário)
  stop     Encerra a sessão Waydroid inteira, incluindo outros apps
  background  Inicia o Android em segundo plano, sem abrir janela

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
    let args = env::args_os()
        .skip(1)
        .map(|arg| arg.into_string())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "argumentos devem ser texto UTF-8.".to_string())?;
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        // Ações administrativas: lista fechada, validada em `privileged::parse`.
        ["privileged", request @ ..] => return privileged::run(privileged::parse(request)?),
        [_, _, ..] => return Err("argumentos extras não são aceitos. Use --help.".into()),
        _ => {}
    }
    match args.first().copied() {
        None => default_action(),
        Some("wizard") => ui::run(),
        Some("--help" | "-h" | "help") => {
            print!("{HELP}");
            Ok(())
        }
        Some("doctor") => doctor(),
        Some("status") => replace_with_waydroid(&["status"]),
        Some("setup") => setup(),
        Some("background") => {
            let waydroid = find_waydroid()?;
            wayland_socket()?;
            ensure_session(&waydroid)?;
            integrate_desktop_reporting(Options::load().unwrap_or_default().hide_apps);
            Ok(())
        }
        Some("open") => open(),
        Some("stop") => replace_with_waydroid(&["session", "stop"]),
        _ => Err("comando desconhecido. Use --help.".into()),
    }
}

/// Ação do ícone: abre o app, ou o assistente se a configuração estiver incompleta.
fn default_action() -> Result<(), String> {
    if Options::load().is_none() || !checks::state().environment_ready() {
        return ui::run();
    }
    open().inspect_err(|message| {
        // Aberto pelo ícone, o erro não teria onde aparecer.
        let _ = std::process::Command::new("notify-send")
            .args([
                "--app-name=Redoma",
                "Não foi possível abrir o Certisign",
                message,
            ])
            .status();
    })
}

fn open() -> Result<(), String> {
    let waydroid = find_waydroid()?;
    wayland_socket()?;
    let was_running = ensure_session(&waydroid)?;
    integrate_desktop_reporting(Options::load().unwrap_or_default().hide_apps);
    if was_running {
        release_closed_window(&waydroid)?;
    }
    eprintln!("Abrindo o app oficial Certisign ({PACKAGE}).");
    eprintln!("Fechar a janela mantém o Android em segundo plano; use 'stop' para encerrar.");
    // A sessão já roda destacada do terminal; o Waydroid só traz a janela.
    replace(waydroid, &["app", "launch", PACKAGE])
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
    setup::configure(&Options::load().unwrap_or_default(), |step| {
        println!("{step}")
    })?;
    println!("Configuração concluída. Use o ícone Certisign ou 'open'.");
    Ok(())
}
