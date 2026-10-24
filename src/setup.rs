//! Configuração sem privilégios, compartilhada pelo comando `setup` e pelo
//! assistente gráfico: sessão Android, janela, atalhos, dock e autostart.

use crate::desktop::{integrate_desktop, set_autostart, set_dock_pin};
use crate::settings::Options;
use crate::waydroid::{
    WINDOW_PROPERTIES, ensure_session, find_waydroid, restart_session, waydroid_output,
    wayland_socket,
};

pub fn configure(options: &Options, mut report: impl FnMut(&str)) -> Result<(), String> {
    let waydroid = find_waydroid()?;
    wayland_socket()?;
    report("Iniciando o Android em segundo plano");
    ensure_session(&waydroid)?;
    let mut changed = false;
    for (key, value) in WINDOW_PROPERTIES {
        if waydroid_output(&waydroid, &["prop", "get", key])?.trim() != value {
            waydroid_output(&waydroid, &["prop", "set", key, value])?;
            report(&format!("Propriedade ajustada: {key}={value}"));
            changed = true;
        }
    }
    if changed {
        // Tamanho e modo da janela só valem para a próxima sessão.
        report("Reiniciando o Android para aplicar a janela 9:16");
        restart_session(&waydroid)?;
    }
    report("Ajustando atalhos do menu");
    integrate_desktop(options.hide_apps)
        .map_err(|error| format!("não foi possível ajustar os atalhos: {error}"))?;
    set_dock_pin(options.pin_dock);
    set_autostart(options.autostart)
        .map_err(|error| format!("não foi possível ajustar o início automático: {error}"))?;
    options
        .save()
        .map_err(|error| format!("não foi possível salvar as opções: {error}"))
}
