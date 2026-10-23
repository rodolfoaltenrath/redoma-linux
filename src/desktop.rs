//! Integração com a área de trabalho: atalhos do menu e dock do GNOME.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::PACKAGE;

pub const DESKTOP_ID: &str = "redoma.desktop";
// app_id da janela criada pelo Waydroid; associa a janela ao atalho próprio.
const WINDOW_APP_ID: &str = "waydroid.com.certisign.certisign";

pub fn integrate_desktop_reporting() {
    match integrate_desktop() {
        Ok(0) => {}
        Ok(count) => eprintln!("Atalhos do menu ajustados: {count}"),
        Err(error) => eprintln!("Aviso: não foi possível ajustar os atalhos do menu: {error}"),
    }
}

pub fn data_home() -> Option<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
}

/// O Waydroid cria um atalho no menu do Linux para cada app Android e preserva
/// `NoDisplay` ao regenerá-los. Todos ficam ocultos, inclusive o do Certisign,
/// cujo `waydroid app launch` não reabre uma janela fechada; o atalho próprio
/// usa `open`. Os apps continuam instalados no Android.
pub fn integrate_desktop() -> io::Result<usize> {
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

pub fn hide_waydroid_entries(applications: &Path) -> io::Result<usize> {
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

pub fn desktop_entry(executable: &Path, icon: &Path) -> String {
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

/// Troca o favorito da dock do GNOME que aponta para o atalho do Waydroid.
pub fn replace_dock_favorite() {
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
