#!/usr/bin/python3
"""Ferramenta de laboratório; não integra o produto Rust ou seu instalador."""

import configparser
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import stat
import sys
import tempfile


CONFIG = Path('/var/lib/waydroid/waydroid.cfg')
BACKUP = Path('/var/lib/waydroid/certisign-os-arch-experiment')
PROPERTY = 'dalvik.vm.extra-opts'
VALUE = '-Dos.arch=aarch64'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_regular(path):
    if not stat.S_ISREG(path.lstat().st_mode):
        raise RuntimeError(f'Arquivo regular esperado: {path}')
    return path.read_bytes()


def replace_config(path, content):
    descriptor, name = tempfile.mkstemp(prefix='.certisign-experiment-', dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, 'wb') as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        # Preserva modo, atributos e contexto SELinux da configuração atual.
        shutil.copystat(path, temporary)
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def experiment(action, config=CONFIG, backup=BACKUP):
    current = read_regular(config)
    if not backup.exists():
        if action == 'restore':
            raise RuntimeError('Backup inexistente; nenhuma alteração aplicada.')
        backup.mkdir(mode=0o700)
    directory = backup.lstat()
    if (not stat.S_ISDIR(directory.st_mode) or directory.st_uid != os.geteuid()
            or directory.st_mode & 0o077):
        raise RuntimeError('Diretório de backup deve pertencer ao executor e ter modo 0700.')

    manifest = backup / 'manifest.json'
    original_path = backup / 'waydroid.cfg.original'
    expected_path = backup / 'waydroid.cfg.experimental'
    if manifest.exists():
        info = json.loads(read_regular(manifest))
        original = read_regular(original_path)
        expected = read_regular(expected_path)
        if digest(original) != info['original_sha256'] or digest(expected) != info['experimental_sha256']:
            raise RuntimeError('Backup alterado; operação interrompida.')
    else:
        if action != 'apply' or original_path.exists() or expected_path.exists():
            raise RuntimeError('Backup incompleto; preservar arquivos e revisar manualmente.')
        parser = configparser.ConfigParser(interpolation=None)
        parser.read_string(current.decode())
        if not parser.has_section('properties'):
            parser.add_section('properties')
        if parser.get('properties', PROPERTY, fallback='').strip():
            raise RuntimeError('Já existem opções extras da VM; não serão substituídas.')
        parser.set('properties', PROPERTY, VALUE)
        output = io.StringIO()
        parser.write(output)
        original, expected = current, output.getvalue().encode()
        info = {'original_sha256': digest(original), 'experimental_sha256': digest(expected)}
        for path, data in [(original_path, original), (expected_path, expected),
                           (manifest, (json.dumps(info, indent=2) + '\n').encode())]:
            with path.open('xb') as stream:
                stream.write(data)
            path.chmod(0o600)

    if digest(current) not in (info['original_sha256'], info['experimental_sha256']):
        raise RuntimeError('Configuração mudou desde o backup; nada será sobrescrito.')
    desired = expected if action == 'apply' else original
    if current != desired:
        replace_config(config, desired)
    print('Configuração experimental aplicada.' if action == 'apply' else 'Configuração original restaurada.')
    print('Aplicar ao runtime com: waydroid upgrade -o; depois iniciar uma nova sessão.')


if __name__ == '__main__':
    if sys.argv[1:] not in (['apply'], ['restore']):
        sys.exit('Uso: experiment-os-arch.py apply|restore')
    if os.geteuid() != 0:
        sys.exit('Execute pelo pkexec para alterar a configuração do Waydroid de referência.')
    # Impede dois experimentos simultâneos de atualizarem configuração e backup.
    import fcntl
    with open('/run/lock/certisign-os-arch-experiment.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        try:
            experiment(sys.argv[1])
        except (RuntimeError, OSError, ValueError, configparser.Error) as error:
            sys.exit(str(error))
