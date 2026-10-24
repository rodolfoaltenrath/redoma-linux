//! Verificações do sistema e estado de cada etapa da instalação, para o
//! assistente bloquear ambientes sem suporte e pular o que já foi feito.

use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::os_arch;
use crate::waydroid::wayland_socket;

const WAYDROID_CONFIG: &str = "/var/lib/waydroid/waydroid.cfg";
const DEFAULT_IMAGES: &str = "/var/lib/waydroid/images";
// Imagens de sistema e vendor ocupam cerca de 3 GiB depois de extraídas.
pub const REQUIRED_FREE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

pub struct Check {
    pub label: &'static str,
    pub result: Result<String, String>,
}

/// Requisitos indispensáveis; qualquer falha impede a instalação.
pub fn system_checks() -> Vec<Check> {
    let initialized = state().initialized;
    vec![
        Check {
            label: "Distribuição",
            result: distribution(&fs::read_to_string("/etc/os-release").unwrap_or_default()),
        },
        Check {
            label: "Arquitetura",
            result: match std::env::consts::ARCH {
                "x86_64" => Ok("x86-64".into()),
                other => Err(format!("{other} ainda não suportada; apenas x86-64.")),
            },
        },
        Check {
            label: "Sessão gráfica",
            result: wayland_socket().map(|_| "Wayland".into()),
        },
        Check {
            label: "Kernel",
            result: if fs::read_to_string("/proc/filesystems")
                .is_ok_and(|filesystems| filesystems.lines().any(|line| line.ends_with("binder")))
            {
                Ok("Binder disponível".into())
            } else {
                Err("Kernel sem suporte a Binder (binderfs).".into())
            },
        },
        Check {
            label: "Espaço livre",
            result: if initialized {
                Ok("Imagens Android já instaladas".into())
            } else {
                free_space(Path::new("/var/lib"))
            },
        },
    ]
}

fn distribution(os_release: &str) -> Result<String, String> {
    let field = |key: &str| {
        os_release.lines().find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
                .map(|value| value.trim_matches('"').to_owned())
        })
    };
    let name = field("PRETTY_NAME").unwrap_or_else(|| "desconhecida".into());
    if field("ID").as_deref() == Some("fedora") {
        Ok(name)
    } else {
        Err(format!(
            "{name} ainda não é suportada; por enquanto, só Fedora."
        ))
    }
}

fn free_space(path: &Path) -> Result<String, String> {
    let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|e| e.to_string())?;
    // SAFETY: estrutura zerada é válida para statvfs preencher.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
        return Err("Não foi possível consultar o espaço livre.".into());
    }
    let free = stats.f_bavail * stats.f_frsize;
    let gib = |bytes: u64| bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if free >= REQUIRED_FREE_BYTES {
        Ok(format!("{:.0} GiB livres", gib(free)))
    } else {
        Err(format!(
            "{:.1} GiB livres; são necessários {:.0} GiB.",
            gib(free),
            gib(REQUIRED_FREE_BYTES)
        ))
    }
}

/// Etapas que exigem administrador, verificáveis sem privilégios.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub waydroid_installed: bool,
    pub initialized: bool,
    pub os_arch_applied: bool,
    pub clipboard_allowed: bool,
}

impl State {
    pub fn environment_ready(&self) -> bool {
        self.waydroid_installed && self.initialized && self.os_arch_applied
    }
}

pub fn state() -> State {
    let config = fs::read_to_string(WAYDROID_CONFIG).unwrap_or_default();
    State {
        waydroid_installed: Path::new("/usr/bin/waydroid").exists(),
        initialized: images_path(&config).is_some_and(|images| images.join("system.img").exists()),
        os_arch_applied: os_arch::is_applied(Path::new(WAYDROID_CONFIG)),
        clipboard_allowed: package_installed("python3-pyclip"),
    }
}

fn images_path(config: &str) -> Option<PathBuf> {
    if config.is_empty() {
        return None;
    }
    let configured = config.lines().find_map(|line| {
        line.split_once('=')
            .filter(|(key, _)| key.trim() == "images_path")
            .map(|(_, value)| PathBuf::from(value.trim()))
    });
    Some(configured.unwrap_or_else(|| PathBuf::from(DEFAULT_IMAGES)))
}

pub fn package_installed(package: &str) -> bool {
    Command::new("rpm")
        .args(["-q", "--quiet", package])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::{distribution, images_path};
    use std::path::PathBuf;

    #[test]
    fn accepts_only_fedora() {
        let fedora = "NAME=\"Fedora Linux\"\nID=fedora\nPRETTY_NAME=\"Fedora Linux 44 (Workstation Edition)\"\n";
        assert_eq!(
            distribution(fedora).unwrap(),
            "Fedora Linux 44 (Workstation Edition)"
        );
        let ubuntu = "ID=ubuntu\nID_LIKE=debian\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n";
        assert!(distribution(ubuntu).unwrap_err().contains("só Fedora"));
    }

    #[test]
    fn reads_the_configured_images_path() {
        assert_eq!(images_path(""), None);
        assert_eq!(
            images_path("[waydroid]\narch = x86_64\n"),
            Some(PathBuf::from("/var/lib/waydroid/images"))
        );
        assert_eq!(
            images_path("[waydroid]\nimages_path = /opt/img\n"),
            Some(PathBuf::from("/opt/img"))
        );
    }
}
