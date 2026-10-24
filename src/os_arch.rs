//! Ajuste reversível da propriedade Java `os.arch` informada às apps Android.
//!
//! Porte de `scripts/experiment-os-arch.py`, com o mesmo diretório e formato de
//! backup, para que backups criados pela ferramenta Python continuem válidos.
//! Diagnóstico e motivação: `docs/emulator-block.md`.

use std::ffi::CString;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::path::Path;

use sha2::{Digest, Sha256};

pub const CONFIG: &str = "/var/lib/waydroid/waydroid.cfg";
pub const BACKUP: &str = "/var/lib/waydroid/certisign-os-arch-experiment";
const LOCK: &str = "/run/lock/redoma-os-arch.lock";
const PROPERTY: &str = "dalvik.vm.extra-opts";
const VALUE: &str = "-Dos.arch=aarch64";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Apply,
    Restore,
}

/// Indica se a configuração já informa `os.arch` modificado, independentemente
/// do backup: o Waydroid regrava o arquivo e altera seu hash.
pub fn is_applied(config: &Path) -> bool {
    fs::read_to_string(config).is_ok_and(|content| property(&content).as_deref() == Some(VALUE))
}

/// Executa a ação com exclusão mútua entre execuções simultâneas.
pub fn run_locked(action: Action) -> Result<&'static str, String> {
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(LOCK)
        .map_err(|error| format!("não foi possível criar a trava: {error}"))?;
    // SAFETY: descritor válido enquanto `lock` existir.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("outro ajuste de os.arch está em andamento.".into());
    }
    experiment(action, Path::new(CONFIG), Path::new(BACKUP))
}

pub fn experiment(action: Action, config: &Path, backup: &Path) -> Result<&'static str, String> {
    let current = read_regular(config)?;
    if !backup.exists() {
        if action == Action::Restore {
            return Err("Backup inexistente; nenhuma alteração aplicada.".into());
        }
        DirBuilder::new()
            .mode(0o700)
            .create(backup)
            .map_err(|error| error.to_string())?;
    }
    let directory = fs::symlink_metadata(backup).map_err(|error| error.to_string())?;
    // SAFETY: geteuid não tem pré-condições.
    let euid = unsafe { libc::geteuid() };
    if !directory.is_dir() || directory.uid() != euid || directory.mode() & 0o077 != 0 {
        return Err("Diretório de backup deve pertencer ao executor e ter modo 0700.".into());
    }

    let manifest = backup.join("manifest.json");
    let original_path = backup.join("waydroid.cfg.original");
    let expected_path = backup.join("waydroid.cfg.experimental");
    let (original, expected, info) = if manifest.exists() {
        let info = Manifest::parse(&String::from_utf8_lossy(&read_regular(&manifest)?))?;
        let original = read_regular(&original_path)?;
        let expected = read_regular(&expected_path)?;
        if digest(&original) != info.original || digest(&expected) != info.experimental {
            return Err("Backup alterado; operação interrompida.".into());
        }
        (original, expected, info)
    } else {
        if action != Action::Apply || original_path.exists() || expected_path.exists() {
            return Err("Backup incompleto; preservar arquivos e revisar manualmente.".into());
        }
        let text = String::from_utf8(current.clone())
            .map_err(|_| "Configuração do Waydroid não é UTF-8.".to_string())?;
        if property(&text).is_some_and(|value| !value.is_empty()) {
            return Err("Já existem opções extras da VM; não serão substituídas.".into());
        }
        let expected = with_property(&text).into_bytes();
        let info = Manifest {
            original: digest(&current),
            experimental: digest(&expected),
        };
        for (path, data) in [
            (&original_path, current.as_slice()),
            (&expected_path, expected.as_slice()),
            (&manifest, info.render().as_bytes()),
        ] {
            write_new(path, data)?;
        }
        (current.clone(), expected, info)
    };

    let current_digest = digest(&current);
    if current_digest != info.original && current_digest != info.experimental {
        return Err("Configuração mudou desde o backup; nada será sobrescrito.".into());
    }
    let desired = if action == Action::Apply {
        &expected
    } else {
        &original
    };
    if &current != desired {
        replace_config(config, desired).map_err(|error| error.to_string())?;
    }
    Ok(match action {
        Action::Apply => "Configuração experimental aplicada.",
        Action::Restore => "Configuração original restaurada.",
    })
}

/// Formato gravado pelo `json.dumps(..., indent=2)` da ferramenta Python.
struct Manifest {
    original: String,
    experimental: String,
}

impl Manifest {
    fn parse(text: &str) -> Result<Self, String> {
        let field = |key: &str| -> Result<String, String> {
            let after = text
                .split_once(&format!("\"{key}\""))
                .map(|(_, rest)| rest)
                .ok_or("Manifesto do backup inválido.")?;
            let value = after
                .split('"')
                .nth(1)
                .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or("Manifesto do backup inválido.")?;
            Ok(value.to_ascii_lowercase())
        };
        Ok(Self {
            original: field("original_sha256")?,
            experimental: field("experimental_sha256")?,
        })
    }

    fn render(&self) -> String {
        format!(
            "{{\n  \"original_sha256\": \"{}\",\n  \"experimental_sha256\": \"{}\"\n}}\n",
            self.original, self.experimental
        )
    }
}

fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_regular(path: &Path) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("Arquivo regular esperado: {}", path.display()));
    }
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn write_new(path: &Path, data: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    file.write_all(data)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// Valor de `dalvik.vm.extra-opts` na seção `[properties]`, se presente.
fn property(content: &str) -> Option<String> {
    let mut in_properties = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_properties = trimmed == "[properties]";
        } else if in_properties
            && let Some((key, value)) = trimmed.split_once(['=', ':'])
            && key.trim() == PROPERTY
        {
            return Some(value.trim().to_owned());
        }
    }
    None
}

/// Inclui a opção na seção `[properties]`, criando a seção se necessário.
fn with_property(content: &str) -> String {
    let line = format!("{PROPERTY} = {VALUE}");
    let mut lines: Vec<&str> = content.lines().collect();
    match lines.iter().position(|line| line.trim() == "[properties]") {
        Some(header) => lines.insert(header + 1, &line),
        None => {
            if lines.last().is_some_and(|last| !last.trim().is_empty()) {
                lines.push("");
            }
            lines.push("[properties]");
            lines.push(&line);
        }
    }
    let mut updated = lines.join("\n");
    updated.push('\n');
    updated
}

/// Troca atômica preservando modo e atributos estendidos, incluindo o
/// contexto SELinux (`security.selinux`) da configuração atual.
fn replace_config(path: &Path, content: &[u8]) -> io::Result<()> {
    let directory = path.parent().unwrap_or(Path::new("/"));
    let temporary = directory.join(format!(".redoma-os-arch-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(content)?;
        file.sync_all()?;
        let metadata = fs::metadata(path)?;
        file.set_permissions(fs::Permissions::from_mode(metadata.mode() & 0o7777))?;
        copy_xattrs(path, &file)?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn copy_xattrs(source: &Path, target: &File) -> io::Result<()> {
    let source = CString::new(source.as_os_str().as_bytes())?;
    // SAFETY: ponteiros válidos e tamanhos informados conforme listxattr(2)/getxattr(2).
    unsafe {
        let size = libc::listxattr(source.as_ptr(), std::ptr::null_mut(), 0);
        if size < 0 {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(libc::ENOTSUP) {
                Ok(())
            } else {
                Err(error)
            };
        }
        let mut names = vec![0u8; size as usize];
        let size = libc::listxattr(source.as_ptr(), names.as_mut_ptr().cast(), names.len());
        if size < 0 {
            return Err(io::Error::last_os_error());
        }
        for name in names[..size as usize]
            .split(|&byte| byte == 0)
            .filter(|n| !n.is_empty())
        {
            let name = CString::new(name)?;
            let length = libc::getxattr(source.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0);
            if length < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut value = vec![0u8; length as usize];
            let length = libc::getxattr(
                source.as_ptr(),
                name.as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
            );
            if length < 0
                || libc::fsetxattr(
                    target.as_raw_fd(),
                    name.as_ptr(),
                    value.as_ptr().cast(),
                    length as usize,
                    0,
                ) != 0
            {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    const ORIGINAL: &[u8] = b"[waydroid]\narch = x86_64\n\n[properties]\n";

    struct Fixture {
        root: std::path::PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "redoma-os-arch-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let fixture = Self { root };
            fs::write(fixture.config(), ORIGINAL).unwrap();
            fs::set_permissions(fixture.config(), fs::Permissions::from_mode(0o640)).unwrap();
            fixture
        }

        fn config(&self) -> std::path::PathBuf {
            self.root.join("waydroid.cfg")
        }

        fn backup(&self) -> std::path::PathBuf {
            self.root.join("backup")
        }

        fn run(&self, action: Action) -> Result<&'static str, String> {
            experiment(action, &self.config(), &self.backup())
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn repeated_apply_preserves_original_and_restore_is_exact() {
        let fixture = Fixture::new();
        fixture.run(Action::Apply).unwrap();
        let changed = fs::read(fixture.config()).unwrap();
        assert!(
            String::from_utf8_lossy(&changed).contains("dalvik.vm.extra-opts = -Dos.arch=aarch64")
        );
        assert!(is_applied(&fixture.config()));
        fixture.run(Action::Apply).unwrap();
        assert_eq!(fs::read(fixture.config()).unwrap(), changed);
        assert_eq!(
            fs::read(fixture.backup().join("waydroid.cfg.original")).unwrap(),
            ORIGINAL
        );
        fixture.run(Action::Restore).unwrap();
        fixture.run(Action::Restore).unwrap();
        assert_eq!(fs::read(fixture.config()).unwrap(), ORIGINAL);
        assert_eq!(
            fs::metadata(fixture.config()).unwrap().mode() & 0o777,
            0o640
        );
        assert!(!is_applied(&fixture.config()));
    }

    #[test]
    fn restore_does_not_overwrite_unrelated_changes() {
        let fixture = Fixture::new();
        fixture.run(Action::Apply).unwrap();
        let mut changed_elsewhere = fs::read(fixture.config()).unwrap();
        changed_elsewhere.extend_from_slice(b"new.setting = keep\n");
        fs::write(fixture.config(), &changed_elsewhere).unwrap();
        assert!(
            fixture
                .run(Action::Restore)
                .unwrap_err()
                .contains("mudou desde o backup")
        );
        assert_eq!(fs::read(fixture.config()).unwrap(), changed_elsewhere);
    }

    #[test]
    fn existing_vm_options_are_preserved() {
        let fixture = Fixture::new();
        let mut before = ORIGINAL.to_vec();
        before.extend_from_slice(b"dalvik.vm.extra-opts = -Xmx256m\n");
        fs::write(fixture.config(), &before).unwrap();
        assert!(
            fixture
                .run(Action::Apply)
                .unwrap_err()
                .contains("opções extras")
        );
        assert_eq!(fs::read(fixture.config()).unwrap(), before);
    }

    #[test]
    fn modified_backup_is_rejected() {
        let fixture = Fixture::new();
        fixture.run(Action::Apply).unwrap();
        let before = fs::read(fixture.config()).unwrap();
        fs::write(fixture.backup().join("waydroid.cfg.original"), b"changed").unwrap();
        assert!(
            fixture
                .run(Action::Restore)
                .unwrap_err()
                .contains("Backup alterado")
        );
        assert_eq!(fs::read(fixture.config()).unwrap(), before);
    }

    #[test]
    fn restores_a_backup_created_by_the_python_tool() {
        let fixture = Fixture::new();
        // Bytes no formato do configparser e do json.dumps(indent=2) do Python.
        let experimental = b"[waydroid]\narch = x86_64\n\n[properties]\ndalvik.vm.extra-opts = -Dos.arch=aarch64\n\n";
        let backup = fixture.backup();
        DirBuilder::new().mode(0o700).create(&backup).unwrap();
        fs::write(backup.join("waydroid.cfg.original"), ORIGINAL).unwrap();
        fs::write(backup.join("waydroid.cfg.experimental"), experimental).unwrap();
        fs::write(
            backup.join("manifest.json"),
            format!(
                "{{\n  \"original_sha256\": \"{}\",\n  \"experimental_sha256\": \"{}\"\n}}\n",
                digest(ORIGINAL),
                digest(experimental)
            ),
        )
        .unwrap();
        fs::write(fixture.config(), experimental).unwrap();
        assert!(is_applied(&fixture.config()));
        fixture.run(Action::Restore).unwrap();
        assert_eq!(fs::read(fixture.config()).unwrap(), ORIGINAL);
    }

    #[test]
    fn creates_the_properties_section_when_missing() {
        assert_eq!(
            with_property("[waydroid]\narch = x86_64\n"),
            "[waydroid]\narch = x86_64\n\n[properties]\ndalvik.vm.extra-opts = -Dos.arch=aarch64\n"
        );
    }

    #[test]
    fn digest_matches_the_sha256_reference() {
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
