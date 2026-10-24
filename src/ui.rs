//! Assistente gráfico de instalação (GTK4 + libadwaita).
//!
//! O trabalho demorado roda em threads e chega à interface por canais. Ações
//! de administrador são executadas por `pkexec redoma privileged ...`, cujo
//! progresso é lido do stdout no protocolo descrito em `privileged.rs`.

use std::cell::{Cell, RefCell};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::thread;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::checks;
use crate::settings::Options;
use crate::setup;
use crate::waydroid;

const APP_ID: &str = "io.github.rodolfoaltenrath.Redoma";
const UNCERTIFIED_URL: &str = "https://www.google.com/android/uncertified";

/// Ação executada ao entrar numa página do assistente.
type Hook = Rc<dyn Fn()>;

pub fn run() -> Result<(), String> {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| Wizard::new(app).start());
    // Os argumentos já foram tratados pelo Redoma; o GTK não deve interpretá-los.
    if app.run_with_args::<&str>(&[]) == glib::ExitCode::SUCCESS {
        Ok(())
    } else {
        Err("o assistente foi encerrado com erro.".into())
    }
}

enum Event {
    Line(String),
    Finished(Result<(), String>),
}

/// Executa uma ação administrativa fechada e repassa cada linha do stdout.
fn run_privileged(args: &[&str]) -> async_channel::Receiver<Event> {
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let (sender, receiver) = async_channel::unbounded();
    thread::spawn(move || {
        let result = (|| {
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let mut child = Command::new("pkexec")
                .arg(executable)
                .arg("privileged")
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| format!("não foi possível executar o pkexec: {error}"))?;
            let mut stderr = child.stderr.take().expect("stderr configurado");
            let errors = thread::spawn(move || {
                let mut text = String::new();
                let _ = stderr.read_to_string(&mut text);
                text
            });
            let stdout = child.stdout.take().expect("stdout configurado");
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = sender.send_blocking(Event::Line(line));
            }
            let status = child.wait().map_err(|error| error.to_string())?;
            let errors = errors.join().unwrap_or_default();
            match status.code() {
                Some(0) => Ok(()),
                // Códigos do pkexec para autorização recusada ou cancelada.
                Some(126 | 127) => Err("A autorização de administrador foi cancelada.".into()),
                _ => Err(errors
                    .lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .map(|line| line.trim_start_matches("Erro: ").to_owned())
                    .unwrap_or_else(|| format!("a ação administrativa falhou ({status})."))),
            }
        })();
        let _ = sender.send_blocking(Event::Finished(result));
    });
    receiver
}

/// Executa uma tarefa sem privilégios em outra thread, com mensagens de progresso.
fn run_task(
    task: impl FnOnce(&dyn Fn(&str)) -> Result<(), String> + Send + 'static,
) -> async_channel::Receiver<Event> {
    let (sender, receiver) = async_channel::unbounded();
    thread::spawn(move || {
        let report = |line: &str| {
            let _ = sender.send_blocking(Event::Line(line.to_owned()));
        };
        let result = task(&report);
        let _ = sender.send_blocking(Event::Finished(result));
    });
    receiver
}

fn in_background<T: Send + 'static>(
    task: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) + 'static,
) {
    glib::spawn_future_local(async move {
        if let Ok(value) = gio::spawn_blocking(task).await {
            done(value);
        }
    });
}

fn with_waydroid<T>(
    action: impl FnOnce(&std::path::Path) -> Result<T, String>,
) -> Result<T, String> {
    action(&waydroid::find_waydroid()?)
}

fn button(label: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .halign(gtk::Align::Center)
        .css_classes(["suggested-action", "pill"])
        .build()
}

fn secondary_button(label: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .halign(gtk::Align::Center)
        .css_classes(["pill"])
        .build()
}

fn text(label: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(label)
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build()
}

fn page(icon: &str, title: &str, description: &str, children: &[&gtk::Widget]) -> adw::StatusPage {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    for child in children {
        content.append(*child);
    }
    adw::StatusPage::builder()
        .icon_name(icon)
        .title(title)
        .description(description)
        .child(
            &adw::Clamp::builder()
                .maximum_size(480)
                .child(&content)
                .build(),
        )
        .build()
}

struct Wizard {
    window: adw::ApplicationWindow,
    stack: gtk::Stack,
    options: Cell<Options>,
    entered: RefCell<Vec<(&'static str, Hook)>>,
}

impl Wizard {
    fn new(app: &adw::Application) -> Rc<Self> {
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::SlideLeftRight)
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&stack));
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Redoma — configuração")
            .default_width(560)
            .default_height(700)
            .content(&toolbar)
            .build();
        let mut options = Options::load().unwrap_or_default();
        options.clipboard = checks::state().clipboard_allowed;
        Rc::new(Self {
            window,
            stack,
            options: Cell::new(options),
            entered: RefCell::new(Vec::new()),
        })
    }

    fn start(self: &Rc<Self>) {
        self.add_checks();
        self.add_options();
        self.add_install();
        self.add_configure();
        self.add_play();
        self.add_done();
        self.window.present();
        self.go("checks");
    }

    fn add(&self, name: &'static str, widget: &impl IsA<gtk::Widget>, on_enter: Option<Hook>) {
        self.stack.add_named(widget, Some(name));
        if let Some(on_enter) = on_enter {
            self.entered.borrow_mut().push((name, on_enter));
        }
    }

    fn go(&self, name: &str) {
        self.stack.set_visible_child_name(name);
        let hook = self
            .entered
            .borrow()
            .iter()
            .find(|(page, _)| *page == name)
            .map(|(_, hook)| hook.clone());
        if let Some(hook) = hook {
            hook();
        }
    }

    fn add_checks(self: &Rc<Self>) {
        let group = adw::PreferencesGroup::new();
        let mut supported = true;
        for check in checks::system_checks() {
            let (icon, subtitle) = match check.result {
                Ok(detail) => ("emblem-ok-symbolic", detail),
                Err(problem) => {
                    supported = false;
                    ("dialog-error-symbolic", problem)
                }
            };
            let row = adw::ActionRow::builder()
                .title(check.label)
                .subtitle(subtitle)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name(icon));
            group.add(&row);
        }
        let next = button("Continuar");
        next.set_sensitive(supported);
        let wizard = self.clone();
        next.connect_clicked(move |_| wizard.go("options"));
        let description = if supported {
            "O Redoma executa o app oficial Certisign num contêiner Android isolado. \
             Seu computador atende aos requisitos."
        } else {
            "Este computador ainda não é suportado. Veja os itens marcados abaixo."
        };
        let page = page(
            "computer-symbolic",
            "Bem-vindo ao Redoma",
            description,
            &[group.upcast_ref(), next.upcast_ref()],
        );
        self.add("checks", &page, None);
    }

    fn add_options(self: &Rc<Self>) {
        let current = self.options.get();
        let switch = |title: &str, subtitle: &str, active: bool| {
            adw::SwitchRow::builder()
                .title(title)
                .subtitle(subtitle)
                .active(active)
                .build()
        };
        let hide_apps = switch(
            "Ocultar outros apps Android",
            "Contatos, Mensagens, Play Store e afins somem do menu; continuam instalados.",
            current.hide_apps,
        );
        let pin_dock = switch(
            "Fixar na dock",
            "Ícone Certisign nos favoritos do GNOME.",
            current.pin_dock,
        );
        let autostart = switch(
            "Iniciar o Android com o computador",
            "Notificações de autorização chegam sem abrir o app. Usa memória o tempo todo.",
            current.autostart,
        );
        let clipboard = switch(
            "Compartilhar área de transferência",
            "Copiar e colar entre Linux e Android. Desligado é mais seguro: evite copiar senhas.",
            current.clipboard,
        );
        let group = adw::PreferencesGroup::new();
        for row in [&hide_apps, &pin_dock, &autostart, &clipboard] {
            group.add(row);
        }
        let next = button("Continuar");
        let wizard = self.clone();
        next.connect_clicked(move |_| {
            wizard.options.set(Options {
                hide_apps: hide_apps.is_active(),
                pin_dock: pin_dock.is_active(),
                autostart: autostart.is_active(),
                clipboard: clipboard.is_active(),
            });
            wizard.go("install");
        });
        let page = page(
            "preferences-system-symbolic",
            "Opções",
            "Escolha como o Redoma se integra ao seu sistema. Dá para mudar depois.",
            &[group.upcast_ref(), next.upcast_ref()],
        );
        self.add("options", &page, None);
    }

    fn add_install(self: &Rc<Self>) {
        let status = text("");
        let progress = gtk::ProgressBar::builder().show_text(true).build();
        let detail = text("");
        detail.add_css_class("dim-label");
        detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let action = button("Instalar");
        let page = page(
            "system-software-install-symbolic",
            "Ambiente Android",
            "Instala o Waydroid, baixa o Android com Google Play (cerca de 1 GB) e aplica \
             o ajuste de compatibilidade. Será pedida a senha de administrador uma vez.",
            &[
                status.upcast_ref(),
                progress.upcast_ref(),
                detail.upcast_ref(),
                action.upcast_ref(),
            ],
        );
        let installing = Rc::new(Cell::new(false));
        let ready = Rc::new(Cell::new(false));

        let wizard = self.clone();
        let (status_c, progress_c, detail_c, ready_c) = (
            status.clone(),
            progress.clone(),
            detail.clone(),
            ready.clone(),
        );
        let action_c = action.clone();
        let on_enter: Hook = Rc::new(move || {
            let state = checks::state();
            let clipboard = wizard.options.get().clipboard;
            let done = state.environment_ready() && state.clipboard_allowed == clipboard;
            ready_c.set(done);
            detail_c.set_label("");
            if done {
                status_c.set_label("O ambiente Android já está instalado.");
                progress_c.set_fraction(1.0);
                progress_c.set_text(Some("Concluído"));
                action_c.set_label("Continuar");
            } else {
                status_c.set_label("Pronto para instalar.");
                progress_c.set_fraction(0.0);
                progress_c.set_text(Some(""));
                action_c.set_label("Instalar");
            }
            action_c.set_sensitive(true);
        });

        let wizard = self.clone();
        action.connect_clicked(move |action| {
            if ready.get() {
                wizard.go("configure");
                return;
            }
            if installing.replace(true) {
                return;
            }
            action.set_sensitive(false);
            status.set_label("Aguardando autorização de administrador…");
            let clipboard = if wizard.options.get().clipboard {
                "--clipboard=allow"
            } else {
                "--clipboard=block"
            };
            let receiver = run_privileged(&["install-environment", clipboard]);
            let (wizard, status, progress, detail, action, installing, ready) = (
                wizard.clone(),
                status.clone(),
                progress.clone(),
                detail.clone(),
                action.clone(),
                installing.clone(),
                ready.clone(),
            );
            glib::spawn_future_local(async move {
                while let Ok(event) = receiver.recv().await {
                    match event {
                        Event::Line(line) => {
                            show_progress(&line, &status, &progress, &detail);
                        }
                        Event::Finished(Ok(())) => {
                            ready.set(true);
                            status.set_label("Ambiente Android instalado.");
                            progress.set_fraction(1.0);
                            progress.set_text(Some("Concluído"));
                            detail.set_label("");
                            action.set_label("Continuar");
                            action.set_sensitive(true);
                            wizard.go("configure");
                        }
                        Event::Finished(Err(error)) => {
                            status.set_label(&format!("Não foi possível concluir: {error}"));
                            action.set_label("Tentar novamente");
                            action.set_sensitive(true);
                        }
                    }
                }
                installing.set(false);
            });
        });
        self.add("install", &page, Some(on_enter));
    }

    fn add_configure(self: &Rc<Self>) {
        let spinner = adw::Spinner::builder().height_request(48).build();
        let status = text("");
        let retry = secondary_button("Tentar novamente");
        retry.set_visible(false);
        let page = page(
            "emblem-system-symbolic",
            "Configurando",
            "Iniciando o Android, ajustando a janela 9:16 e os atalhos.",
            &[
                spinner.upcast_ref(),
                status.upcast_ref(),
                retry.upcast_ref(),
            ],
        );
        let wizard = self.clone();
        let retry_c = retry.clone();
        let on_enter: Hook = Rc::new(move || {
            retry_c.set_visible(false);
            spinner.set_visible(true);
            let options = wizard.options.get();
            let receiver = run_task(move |report| setup::configure(&options, report));
            let (wizard, status, spinner, retry) = (
                wizard.clone(),
                status.clone(),
                spinner.clone(),
                retry_c.clone(),
            );
            glib::spawn_future_local(async move {
                while let Ok(event) = receiver.recv().await {
                    match event {
                        Event::Line(line) => status.set_label(&line),
                        Event::Finished(Ok(())) => wizard.go("play"),
                        Event::Finished(Err(error)) => {
                            spinner.set_visible(false);
                            status.set_label(&format!("Não foi possível configurar: {error}"));
                            retry.set_visible(true);
                        }
                    }
                }
            });
        });
        let wizard = self.clone();
        retry.connect_clicked(move |_| wizard.go("configure"));
        self.add("configure", &page, Some(on_enter));
    }

    fn add_play(self: &Rc<Self>) {
        let status = text("Verificando se o Certisign já está instalado…");
        let store = button("Abrir Certisign na Play Store");
        let next = button("Continuar");
        next.set_visible(false);

        let uncertified = adw::PreferencesGroup::builder()
            .title("A Play Store diz que o dispositivo não é certificado?")
            .description(
                "Registre este Android na sua conta Google: o Redoma obtém o ID do dispositivo \
                 (pede a senha de administrador) e abre a página oficial do Google. Cole o ID, \
                 confirme e, após alguns minutos, reinicie o Android.",
            )
            .build();
        let id_row = adw::ActionRow::builder()
            .title("ID do dispositivo")
            .subtitle("Ainda não obtido")
            .build();
        let get_id = gtk::Button::builder()
            .label("Obter ID")
            .valign(gtk::Align::Center)
            .build();
        let copy = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copiar")
            .valign(gtk::Align::Center)
            .sensitive(false)
            .build();
        id_row.add_suffix(&get_id);
        id_row.add_suffix(&copy);
        let register = adw::ActionRow::builder()
            .title("Abrir a página de registro do Google")
            .activatable(true)
            .build();
        register.add_suffix(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
        let restart = adw::ActionRow::builder()
            .title("Reiniciar o Android")
            .subtitle("Depois de registrar, para a Play Store reconhecer o dispositivo.")
            .activatable(true)
            .build();
        for row in [&id_row, &register, &restart] {
            uncertified.add(row);
        }

        let page = page(
            "folder-download-symbolic",
            "Instalar o Certisign",
            "Entre na sua conta Google e instale o app oficial pela Play Store. O Redoma \
             percebe sozinho quando a instalação terminar.",
            &[
                status.upcast_ref(),
                store.upcast_ref(),
                next.upcast_ref(),
                uncertified.upcast_ref(),
            ],
        );

        let installed = Rc::new(Cell::new(false));
        let polling = Rc::new(Cell::new(false));
        let check: Hook = {
            let (status, store, next, installed) = (
                status.clone(),
                store.clone(),
                next.clone(),
                installed.clone(),
            );
            Rc::new(move || {
                let (status, store, next, installed) = (
                    status.clone(),
                    store.clone(),
                    next.clone(),
                    installed.clone(),
                );
                in_background(
                    || with_waydroid(waydroid::app_installed),
                    move |result| {
                        if result == Ok(true) {
                            installed.set(true);
                            status.set_label("Certisign instalado.");
                            store.set_visible(false);
                            next.set_visible(true);
                        } else if !installed.get() {
                            status.set_label("Aguardando a instalação do Certisign na Play Store…");
                        }
                    },
                );
            })
        };
        let on_enter: Hook = {
            let (wizard, check, installed, polling) = (
                self.clone(),
                check.clone(),
                installed.clone(),
                polling.clone(),
            );
            Rc::new(move || {
                check();
                if polling.replace(true) {
                    return;
                }
                let (wizard, check, installed, polling) = (
                    wizard.clone(),
                    check.clone(),
                    installed.clone(),
                    polling.clone(),
                );
                glib::timeout_add_seconds_local(3, move || {
                    let visible = wizard.stack.visible_child_name().as_deref() == Some("play");
                    if installed.get() || !visible {
                        polling.set(false);
                        return glib::ControlFlow::Break;
                    }
                    check();
                    glib::ControlFlow::Continue
                });
            })
        };

        store.connect_clicked(|_| {
            in_background(|| with_waydroid(waydroid::open_play_store), |_| {});
        });
        let wizard = self.clone();
        next.connect_clicked(move |_| wizard.go("done"));

        let android_id = Rc::new(RefCell::new(String::new()));
        {
            let (id_row, copy, android_id) = (id_row.clone(), copy.clone(), android_id.clone());
            get_id.connect_clicked(move |get_id| {
                get_id.set_sensitive(false);
                id_row.set_subtitle("Aguardando autorização de administrador…");
                let receiver = run_privileged(&["android-id"]);
                let (id_row, copy, get_id, android_id) = (
                    id_row.clone(),
                    copy.clone(),
                    get_id.clone(),
                    android_id.clone(),
                );
                glib::spawn_future_local(async move {
                    while let Ok(event) = receiver.recv().await {
                        match event {
                            Event::Line(line) => {
                                if let Some(id) = line.strip_prefix("ANDROID_ID ") {
                                    *android_id.borrow_mut() = id.to_owned();
                                    id_row.set_subtitle(id);
                                    copy.set_sensitive(true);
                                }
                            }
                            Event::Finished(Err(error)) => id_row.set_subtitle(&error),
                            Event::Finished(Ok(())) => {}
                        }
                    }
                    get_id.set_sensitive(true);
                });
            });
        }
        copy.connect_clicked(move |copy| copy.clipboard().set_text(&android_id.borrow()));
        register.connect_activated(|_| {
            let _ = gio::AppInfo::launch_default_for_uri(
                UNCERTIFIED_URL,
                None::<&gio::AppLaunchContext>,
            );
        });
        {
            let status = status.clone();
            restart.connect_activated(move |restart| {
                restart.set_sensitive(false);
                status.set_label("Reiniciando o Android…");
                let (restart, status) = (restart.clone(), status.clone());
                in_background(
                    || with_waydroid(waydroid::restart_session),
                    move |result| {
                        restart.set_sensitive(true);
                        status.set_label(&match result {
                            Ok(()) => "Android reiniciado. Abra a Play Store novamente.".to_owned(),
                            Err(error) => format!("Não foi possível reiniciar: {error}"),
                        });
                    },
                );
            });
        }
        self.add("play", &page, Some(on_enter));
    }

    fn add_done(self: &Rc<Self>) {
        let open = button("Abrir Certisign");
        let close = secondary_button("Fechar");
        let page = page(
            "emblem-ok-symbolic",
            "Tudo pronto",
            "Abra o Certisign e conclua o cadastro do dispositivo no app oficial. \
             Para voltar a este assistente, clique com o botão direito no ícone e \
             escolha \"Configurar Redoma\".",
            &[open.upcast_ref(), close.upcast_ref()],
        );
        let wizard = self.clone();
        open.connect_clicked(move |_| {
            // Esconde o assistente e espera a animação de fechamento do GNOME
            // terminar antes de a janela do Android surgir; abrir as duas ao
            // mesmo tempo deixou uma sombra residual na tela.
            wizard.window.set_visible(false);
            let wizard = wizard.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
                if let Ok(executable) = std::env::current_exe() {
                    let _ = Command::new(executable)
                        .arg("open")
                        .stdin(Stdio::null())
                        .spawn();
                }
                wizard.window.close();
            });
        });
        let wizard = self.clone();
        close.connect_clicked(move |_| wizard.window.close());
        self.add("done", &page, None);
    }
}

/// Interpreta uma linha do protocolo de `privileged.rs`.
fn show_progress(
    line: &str,
    status: &gtk::Label,
    progress: &gtk::ProgressBar,
    detail: &gtk::Label,
) {
    let mut parts = line.splitn(3, ' ');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("STEP"), Some(_), Some(text)) => {
            status.set_label(text);
            detail.set_label("");
            progress.set_text(Some(""));
            progress.pulse();
        }
        (Some("SKIP"), Some(_), Some(text)) => detail.set_label(&format!("✓ {text}")),
        (Some("PROGRESS"), Some("-"), text) => {
            detail.set_label(text.unwrap_or_default());
            progress.pulse();
        }
        (Some("PROGRESS"), Some(percent), text) => {
            if let Ok(percent) = percent.parse::<f64>() {
                progress.set_fraction(percent / 100.0);
            }
            progress.set_text(text);
        }
        _ => {}
    }
}
