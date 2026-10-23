use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

// Registra uma chamada por linha e simula sessão e propriedades apenas com
// comandos internos do shell, pois o PATH dos testes contém só este diretório.
const FAKE_WAYDROID: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$CALL_LOG"
state="$STATE_DIR/session"
case "$1 $2" in
  "status ")
    session=STOPPED
    [ -e "$state" ] && read -r session < "$state"
    printf 'Session:\t%s\n' "$session" ;;
  "session start") printf 'RUNNING\n' > "$state" ;;
  "session stop") printf 'STOPPED\n' > "$state" ;;
  "prop get")
    if [ "$3" = sys.boot_completed ]; then printf '1\n'
    elif [ -e "$STATE_DIR/$3" ]; then read -r value < "$STATE_DIR/$3"; printf '%s\n' "$value"
    fi ;;
  "prop set") printf '%s\n' "$4" > "$STATE_DIR/$3" ;;
  "app launch") exit "${BACKEND_EXIT:-0}" ;;
esac
"#;

struct Fixture {
    directory: PathBuf,
    _socket: UnixListener,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "redoma-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let executable = directory.join("waydroid");
        // Registra apenas a interface de processo usada pelo launcher.
        fs::write(&executable, FAKE_WAYDROID).unwrap();
        fs::set_permissions(executable, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = UnixListener::bind(directory.join("wayland-test")).unwrap();
        Self {
            directory,
            _socket: socket,
        }
    }

    fn command(&self, action: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_redoma"));
        command
            .arg(action)
            .env("PATH", &self.directory)
            .env("XDG_RUNTIME_DIR", &self.directory)
            .env("WAYLAND_DISPLAY", "wayland-test")
            .env("CALL_LOG", self.directory.join("calls"))
            .env("STATE_DIR", &self.directory)
            .env("HOME", &self.directory)
            .env_remove("XDG_DATA_HOME")
            .env_remove("XDG_STATE_HOME");
        command
    }

    fn start_session(&self) {
        fs::write(self.directory.join("session"), "RUNNING\n").unwrap();
    }

    fn applications(&self) -> PathBuf {
        let applications = self.directory.join(".local/share/applications");
        fs::create_dir_all(&applications).unwrap();
        applications
    }

    fn calls(&self) -> String {
        fs::read_to_string(self.directory.join("calls")).unwrap_or_default()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn successful(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn opens_only_the_official_certisign_package() {
    let fixture = Fixture::new();
    fixture.start_session();
    successful(fixture.command("open").output().unwrap());
    assert_eq!(
        fixture.calls(),
        "status\nprop get sys.boot_completed\n\
         app intent android.intent.action.APPLICATION_PREFERENCES \n\
         app launch com.certisign.certisign\n"
    );
}

#[test]
fn starts_a_detached_session_and_waits_for_boot_before_launching() {
    let fixture = Fixture::new();
    successful(fixture.command("open").output().unwrap());
    let calls = fixture.calls();
    let start = calls.find("session start").expect(&calls);
    let boot = calls.find("prop get sys.boot_completed").expect(&calls);
    let launch = calls.find("app launch").expect(&calls);
    assert!(start < boot && boot < launch, "{calls}");
    // Sessão recém-iniciada não tem janela fechada a liberar.
    assert!(!calls.contains("app intent"), "{calls}");
}

#[test]
fn hides_waydroid_entries_and_installs_own_shortcut() {
    let fixture = Fixture::new();
    fixture.start_session();
    let applications = fixture.applications();
    let entry = "[Desktop Entry]\nName=App\n";
    fs::write(
        applications.join("waydroid.com.google.android.contacts.desktop"),
        entry,
    )
    .unwrap();
    fs::write(
        applications.join("waydroid.com.certisign.certisign.desktop"),
        entry,
    )
    .unwrap();
    fs::write(applications.join("firefox.desktop"), entry).unwrap();
    successful(fixture.command("open").output().unwrap());
    let read = |name: &str| fs::read_to_string(applications.join(name)).unwrap();
    assert!(read("waydroid.com.google.android.contacts.desktop").contains("NoDisplay=true"));
    assert!(read("waydroid.com.certisign.certisign.desktop").contains("NoDisplay=true"));
    assert_eq!(read("firefox.desktop"), entry);
    let own = read("redoma.desktop");
    assert!(own.contains("\" open\n"), "{own}");
    assert!(
        own.contains("StartupWMClass=waydroid.com.certisign.certisign\n"),
        "{own}"
    );
}

#[test]
fn setup_applies_window_properties_and_restarts_once() {
    let fixture = Fixture::new();
    fixture.start_session();
    successful(fixture.command("setup").output().unwrap());
    let calls = fixture.calls();
    for property in [
        "persist.waydroid.multi_windows false",
        "persist.waydroid.width 450",
        "persist.waydroid.height 800",
        "persist.waydroid.suspend false",
    ] {
        assert!(calls.contains(&format!("prop set {property}\n")), "{calls}");
    }
    assert_eq!(calls.matches("session stop").count(), 1, "{calls}");

    fs::remove_file(fixture.directory.join("calls")).unwrap();
    successful(fixture.command("setup").output().unwrap());
    let calls = fixture.calls();
    assert!(
        !calls.contains("prop set") && !calls.contains("session stop"),
        "{calls}"
    );
}

#[test]
fn refuses_to_launch_without_a_wayland_socket() {
    let fixture = Fixture::new();
    let output = fixture
        .command("open")
        .env("WAYLAND_DISPLAY", "missing")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(fixture.calls().is_empty());
}

#[test]
fn stops_the_session_without_requiring_a_display() {
    let fixture = Fixture::new();
    successful(
        fixture
            .command("stop")
            .env_remove("WAYLAND_DISPLAY")
            .output()
            .unwrap(),
    );
    assert_eq!(fixture.calls(), "session stop\n");
}

#[test]
fn preserves_backend_failure_status() {
    let fixture = Fixture::new();
    fixture.start_session();
    let output = fixture
        .command("open")
        .env("BACKEND_EXIT", "17")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(17));
}

#[test]
fn doctor_does_not_start_android() {
    let fixture = Fixture::new();
    successful(fixture.command("doctor").output().unwrap());
    assert!(fixture.calls().is_empty());
}

#[test]
fn rejects_package_overrides_without_invoking_waydroid() {
    let fixture = Fixture::new();
    let output = fixture
        .command("open")
        .arg("another.package")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(fixture.calls().is_empty());
}
