"""Testes do backup/restauração em diretórios temporários, sem privilégios."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


spec = importlib.util.spec_from_file_location(
    'experiment', Path(__file__).resolve().parents[1] / 'scripts/experiment-os-arch.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ConfigurationExperimentTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.config = self.root / 'waydroid.cfg'
        self.backup = self.root / 'backup'
        self.original = b'[waydroid]\narch = x86_64\n\n[properties]\n'
        self.config.write_bytes(self.original)
        self.config.chmod(0o640)

    def test_repeated_apply_preserves_original_and_restore_is_exact(self):
        module.experiment('apply', self.config, self.backup)
        changed = self.config.read_bytes()
        self.assertIn(b'dalvik.vm.extra-opts = -Dos.arch=aarch64', changed)
        module.experiment('apply', self.config, self.backup)
        self.assertEqual(self.config.read_bytes(), changed)
        self.assertEqual((self.backup / 'waydroid.cfg.original').read_bytes(), self.original)
        module.experiment('restore', self.config, self.backup)
        module.experiment('restore', self.config, self.backup)
        self.assertEqual(self.config.read_bytes(), self.original)
        self.assertEqual(self.config.stat().st_mode & 0o777, 0o640)

    def test_restore_does_not_overwrite_unrelated_changes(self):
        module.experiment('apply', self.config, self.backup)
        changed_elsewhere = self.config.read_bytes() + b'new.setting = keep\n'
        self.config.write_bytes(changed_elsewhere)
        with self.assertRaisesRegex(RuntimeError, 'mudou desde o backup'):
            module.experiment('restore', self.config, self.backup)
        self.assertEqual(self.config.read_bytes(), changed_elsewhere)

    def test_existing_vm_options_are_preserved(self):
        self.config.write_bytes(self.original + b'dalvik.vm.extra-opts = -Xmx256m\n')
        before = self.config.read_bytes()
        with self.assertRaisesRegex(RuntimeError, 'opções extras'):
            module.experiment('apply', self.config, self.backup)
        self.assertEqual(self.config.read_bytes(), before)

    def test_modified_backup_is_rejected(self):
        module.experiment('apply', self.config, self.backup)
        before = self.config.read_bytes()
        (self.backup / 'waydroid.cfg.original').write_bytes(b'changed')
        with self.assertRaisesRegex(RuntimeError, 'Backup alterado'):
            module.experiment('restore', self.config, self.backup)
        self.assertEqual(self.config.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
