"""CI must never skip executable, source, or generated-catalog changes."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from ci_scope import changed_paths, documentation_only, validate_documents


class CiScopeTests(unittest.TestCase):
    def test_documentation_and_receipts_can_use_fast_check(self):
        self.assertTrue(documentation_only(['README.md', 'docs/RULES.md', 'docs/validation/deathgrip.json']))

    def test_any_executable_data_catalog_or_unknown_change_requires_full_checks(self):
        for name in ['src/game.rs', 'assets/art/card_1.png', 'assets/data/decks.json',
                     'docs/CARD_COVERAGE.md', 'docs/fixture.json', 'tools/ci_scope.py',
                     '.github/workflows/ci.yml', 'Cargo.lock', 'flake.nix', 'new-file']:
            with self.subTest(name=name):
                self.assertFalse(documentation_only(['README.md', name]))
        self.assertFalse(documentation_only([]))

    def test_missing_history_and_new_branch_require_full_checks(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.assertIsNone(changed_paths(root, None, 'a' * 40))
            self.assertIsNone(changed_paths(root, '0' * 40, 'a' * 40))
            self.assertIsNone(changed_paths(root, 'b' * 40, 'a' * 40))

    def test_rename_of_code_to_documentation_retains_deleted_code_path(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.DEVNULL, text=True).strip()
            git('init')
            git('config', 'user.name', 'CI test')
            git('config', 'user.email', 'ci@example.invalid')
            (root / 'src').mkdir()
            (root / 'src/old.rs').write_text('// former source\n')
            git('add', '.')
            git('commit', '-m', 'initial')
            base = git('rev-parse', 'HEAD')
            (root / 'README.md').write_text('// former source\n')
            (root / 'src/old.rs').unlink()
            git('add', '.')
            git('commit', '-m', 'rename')
            paths = changed_paths(root, base, git('rev-parse', 'HEAD'))
            self.assertEqual(set(paths), {'README.md', 'src/old.rs'})
            self.assertFalse(documentation_only(paths))

    def test_invalid_document_receipt_fails_even_without_rust_tests(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            path = root / 'docs/validation/result.json'
            path.parent.mkdir(parents=True)
            path.write_text('{broken')
            with self.assertRaises(json.JSONDecodeError):
                validate_documents(root, ['docs/validation/result.json'])
            path.write_text('{"status":"passed"}')
            validate_documents(root, ['docs/validation/result.json'])
            path.unlink()
            validate_documents(root, ['docs/validation/result.json'])
