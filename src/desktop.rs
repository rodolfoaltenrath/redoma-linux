//! Integração com a área de trabalho: atalhos do menu, dock do GNOME e
//! início automático.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::PACKAGE;

pub const DESKTOP_ID: &str = "redoma.desktop";
/// Ícone instalado pelo pacote em `hicolor/scalable/apps`.
pub const APP_ICON: &str = "io.github.rodolfoaltenrath.Redoma";
// app_id da janela criada pelo Waydroid; associa a janela ao atalho próprio.
const WINDOW_APP_ID: &str = "waydroid.com.certisign.certisign";
// Atalho do Certisign gerado pelo Waydroid: sempre oculto, pois seu
// `waydroid app launch` não reabre uma janela fechada.
const WAYDROID_CERTISIGN_ENTRY: &str = "waydroid.com.certisign.certisign.desktop";
// Sobreposição local do atalho de sistema que abre a interface Android completa.
const FULL_UI_ENTRY: &str = "Waydroid.desktop";
// Atalhos ocultados pelo Redoma, para reexibir apenas esses depois.
const HIDDEN_LIST: &str = "redoma/hidden-entries";

pub fn integrate_desktop_reporting(hide_apps: bool) {
    match integrate_desktop(hide_apps) {
        Ok(0) => {}
        Ok(count) => eprintln!("Atalhos do menu ajustados: {count}"),
        Err(error) => eprintln!("Aviso: não foi possível ajustar os atalhos do menu: {error}"),
    }
}

fn xdg_home(variable: &str, fallback: &str) -> Option<PathBuf> {
    env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
}

pub fn data_home() -> Option<PathBuf> {
    xdg_home("XDG_DATA_HOME", ".local/share")
}

pub fn config_home() -> Option<PathBuf> {
    xdg_home("XDG_CONFIG_HOME", ".config")
}

/// O Waydroid cria um atalho no menu do Linux para cada app Android e preserva
/// `NoDisplay` ao regenerá-los. Com `hide_apps`, os atalhos dos demais apps
/// ficam ocultos; sem, os que o Redoma ocultou voltam a aparecer. Os apps
/// continuam instalados no Android.
pub fn integrate_desktop(hide_apps: bool) -> io::Result<usize> {
    let (Some(data_home), Some(config_home)) = (data_home(), config_home()) else {
        return Ok(0);
    };
    let applications = data_home.join("applications");
    fs::create_dir_all(&applications)?;
    let mut changed = 0;
    let own_entry = applications.join(DESKTOP_ID);
    // O ícone do app só existe depois que o Certisign é instalado no Android;
    // até lá, o atalho usa o ícone do Redoma e é atualizado no próximo `open`.
    let app_icon = data_home.join(format!("waydroid/data/icons/{PACKAGE}.png"));
    let icon = if app_icon.exists() {
        app_icon.to_string_lossy().into_owned()
    } else {
        APP_ICON.to_owned()
    };
    let content = desktop_entry(&env::current_exe()?, &icon);
    if fs::read_to_string(&own_entry).ok().as_deref() != Some(content.as_str()) {
        fs::write(&own_entry, content)?;
        changed += 1;
    }
    let certisign = applications.join(WAYDROID_CERTISIGN_ENTRY);
    if let Ok(entry) = fs::read_to_string(&certisign)
        && let Some(updated) = with_no_display(&entry)
    {
        fs::write(&certisign, updated)?;
        changed += 1;
    }
    let list = config_home.join(HIDDEN_LIST);
    changed += if hide_apps {
        hide_waydroid_entries(&applications, &list)?
    } else {
        show_hidden_entries(&applications, &list)?
    };
    Ok(changed)
}

fn hide_waydroid_entries(applications: &Path, list: &Path) -> io::Result<usize> {
    let mut hidden: Vec<String> = fs::read_to_string(list)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let before = hidden.len();
    for entry in fs::read_dir(applications)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("waydroid.")
            || !name.ends_with(".desktop")
            || name == WAYDROID_CERTISIGN_ENTRY
        {
            continue;
        }
        if let Some(updated) = with_no_display(&fs::read_to_string(&path)?) {
            fs::write(&path, updated)?;
            hidden.push(name.to_owned());
        }
    }
    let full_ui = applications.join(FULL_UI_ENTRY);
    let system_full_ui = Path::new("/usr/share/applications").join(FULL_UI_ENTRY);
    if !full_ui.exists() && system_full_ui.exists() {
        let content = fs::read_to_string(system_full_ui)?;
        fs::write(full_ui, with_no_display(&content).unwrap_or(content))?;
        hidden.push(FULL_UI_ENTRY.to_owned());
    }
    let count = hidden.len() - before;
    if count > 0 {
        if let Some(parent) = list.parent() {
            fs::create_dir_all(parent)?;
        }
        hidden.sort();
        hidden.dedup();
        fs::write(list, hidden.join("\n") + "\n")?;
    }
    Ok(count)
}

fn show_hidden_entries(applications: &Path, list: &Path) -> io::Result<usize> {
    let Ok(hidden) = fs::read_to_string(list) else {
        return Ok(0);
    };
    let mut shown = 0;
    for name in hidden
        .lines()
        .filter(|name| !name.is_empty() && !name.contains('/'))
    {
        let path = applications.join(name);
        if name == FULL_UI_ENTRY {
            if path.exists() {
                fs::remove_file(&path)?;
                shown += 1;
            }
        } else if let Ok(content) = fs::read_to_string(&path)
            && let Some(updated) = without_no_display(&content)
        {
            fs::write(&path, updated)?;
            shown += 1;
        }
    }
    fs::remove_file(list)?;
    Ok(shown)
}

pub fn desktop_entry(executable: &Path, icon: &str) -> String {
    let exec = desktop_exec_argument(&executable.to_string_lossy());
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Certisign\n\
         Comment=App oficial Certisign via Waydroid\n\
         Exec={exec}\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories=Office;\n\
         StartupWMClass={WINDOW_APP_ID}\n\
         Actions=configure;\n\
         \n\
         [Desktop Action configure]\n\
         Name=Configurar Redoma\n\
         Exec={exec} wizard\n",
    )
}

/// Aspas e escapes da especificação Desktop Entry para um argumento de `Exec`.
pub fn desktop_exec_argument(argument: &str) -> String {
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

/// Fixa ou desafixa o atalho na dock do GNOME, removendo o favorito antigo
/// que apontava para o atalho do Waydroid. Sem GNOME, não faz nada.
pub fn set_dock_pin(pinned: bool) {
    let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.shell", "favorite-apps"])
        .stderr(Stdio::null())
        .output()
    else {
        return;
    };
    if !output.status.success() {
        return;
    }
    if let Some(updated) = updated_favorites(&String::from_utf8_lossy(&output.stdout), pinned) {
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.shell", "favorite-apps", &updated])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Recebe a lista no formato do `gsettings get` e devolve a nova, se mudou.
fn updated_favorites(current: &str, pinned: bool) -> Option<String> {
    let current = current.trim();
    let items: Vec<String> = current
        .trim_start_matches("@as")
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().trim_matches('\'').to_owned())
        .filter(|item| !item.is_empty())
        .collect();
    let mut updated = Vec::new();
    for item in &items {
        let replacement = if item == WAYDROID_CERTISIGN_ENTRY {
            DESKTOP_ID
        } else {
            item.as_str()
        };
        if replacement == DESKTOP_ID && !pinned {
            continue;
        }
        if !updated
            .iter()
            .any(|existing: &String| existing == replacement)
        {
            updated.push(replacement.to_owned());
        }
    }
    if pinned && !updated.iter().any(|item| item == DESKTOP_ID) {
        updated.push(DESKTOP_ID.to_owned());
    }
    (updated != items).then(|| {
        let quoted: Vec<String> = updated.iter().map(|item| format!("'{item}'")).collect();
        format!("[{}]", quoted.join(", "))
    })
}

/// Inicia o Android em segundo plano no login, para as notificações de
/// autorização chegarem sem abrir o app.
pub fn set_autostart(enabled: bool) -> io::Result<()> {
    let Some(config_home) = config_home() else {
        return Ok(());
    };
    let path = config_home.join("autostart").join(DESKTOP_ID);
    if enabled {
        fs::create_dir_all(config_home.join("autostart"))?;
        fs::write(
            path,
            format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=Redoma (Android em segundo plano)\n\
                 Exec={} background\n\
                 NoDisplay=true\n\
                 X-GNOME-Autostart-enabled=true\n",
                desktop_exec_argument(&env::current_exe()?.to_string_lossy())
            ),
        )
    } else {
        match fs::remove_file(path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

/// Remove `NoDisplay=true` do grupo principal; `None` se não houver.
fn without_no_display(content: &str) -> Option<String> {
    let mut in_entry = false;
    let mut removed = false;
    let lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_entry = trimmed == "[Desktop Entry]";
            } else if in_entry && trimmed == "NoDisplay=true" {
                removed = true;
                return false;
            }
            true
        })
        .collect();
    removed.then(|| lines.join("\n") + "\n")
}

/// Retorna o atalho com `NoDisplay=true` no grupo principal, ou `None` se já estiver oculto.
pub fn with_no_display(content: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::{desktop_exec_argument, updated_favorites, with_no_display, without_no_display};

    #[test]
    fn pins_replaces_and_unpins_dock_favorites() {
        assert_eq!(
            updated_favorites(
                "['firefox.desktop', 'waydroid.com.certisign.certisign.desktop']",
                true
            )
            .unwrap(),
            "['firefox.desktop', 'redoma.desktop']"
        );
        assert_eq!(
            updated_favorites("@as []", true).unwrap(),
            "['redoma.desktop']"
        );
        assert!(updated_favorites("['redoma.desktop']", true).is_none());
        assert_eq!(
            updated_favorites("['redoma.desktop', 'code.desktop']", false).unwrap(),
            "['code.desktop']"
        );
        assert!(updated_favorites("['code.desktop']", false).is_none());
    }

    #[test]
    fn shows_an_entry_hidden_by_redoma() {
        let hidden = with_no_display("[Desktop Entry]\nName=Contacts\n").unwrap();
        assert_eq!(
            without_no_display(&hidden).unwrap(),
            "[Desktop Entry]\nName=Contacts\n"
        );
        assert!(without_no_display("[Desktop Entry]\nName=Contacts\n").is_none());
    }

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
