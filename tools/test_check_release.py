"""Release gates must reject incomplete runs, changed traces and empty plans."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from check_release import compare_records, main, require_executable, validate_plan, verify_ui_output


class ReleaseChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        baseline = self.folder / 'baseline.json'
        baseline.write_text(json.dumps({'status': 'passed', 'games': [{'winner': 0}]}))
        self.plan = {
            'revision': 'a' * 40,
            'ui': [{'name': 'core-small', 'args': ['--ui-smoke', '--small']}],
            'api': [{'name': 'card-api', 'script': 'check_guards_api.py',
                     'baseline': str(baseline), 'records_key': 'games'}],
        }

    def test_missing_display_tool_fails_before_git_build_or_output_creation(self):
        plan = self.folder / 'plan.json'
        plan.write_text(json.dumps(self.plan))
        output = self.folder / 'results'
        with patch('sys.argv', ['check_release.py', '--plan', str(plan), '--output', str(output),
                                '--xvfb', str(self.folder / 'missing-xvfb')]), \
                patch('check_release.subprocess.check_output') as process:
            with self.assertRaisesRegex(FileNotFoundError, 'missing-xvfb'):
                main()
            process.assert_not_called()
        self.assertFalse(output.exists())

    def test_executable_preflight_rejects_nonexecutable_files(self):
        executable = self.folder / 'display-tool'
        executable.write_text('#!/bin/sh\nexit 0\n')
        executable.chmod(0o600)
        with self.assertRaises(FileNotFoundError):
            require_executable(str(executable))
        executable.chmod(0o700)
        self.assertEqual(require_executable(str(executable)), str(executable))

    def test_rejects_empty_or_unpinned_acceptance_before_building(self):
        validate_plan(self.plan)
        for key, value in [('revision', 'HEAD'), ('ui', []), ('api', [])]:
            with self.subTest(key=key):
                plan = {**self.plan, key: value}
                with self.assertRaises(ValueError):
                    validate_plan(plan)

    def test_requires_distinct_cases_and_real_smoke_modes(self):
        # C24 is the shipped Deploy/Command control scenario, including small windows.
        for args in [['--ui-smoke-c24'], ['--ui-smoke-c24', '--small']]:
            plan = copy.deepcopy(self.plan)
            plan['ui'][0]['args'] = args
            validate_plan(plan)
        for change in [{'name': 'card-api'}, {'name': '../escape'}, {'args': ['--help']},
                       {'args': ['--ui-smoke-c24', '--arbitrary-option']},
                       {'captures': ['RIFT_BOARD', 'RIFT_BOARD']},
                       {'env': {'LD_LIBRARY_PATH': '/host/libraries'}}]:
            with self.subTest(change=change):
                plan = copy.deepcopy(self.plan)
                plan['ui'][0].update(change)
                with self.assertRaises(ValueError):
                    validate_plan(plan)

    def test_requires_nonempty_passing_baseline_and_real_series_games(self):
        plan = copy.deepcopy(self.plan)
        plan['ui'][0].update(args=['--ui-smoke-series'], minimum_games=2)
        validate_plan(plan)
        plan['ui'][0]['minimum_games'] = 0
        with self.assertRaises(ValueError):
            validate_plan(plan)
        for baseline in [{'status': 'running', 'games': [1]}, {'status': 'passed', 'games': []}]:
            Path(self.plan['api'][0]['baseline']).write_text(json.dumps(baseline))
            with self.assertRaises(ValueError):
                validate_plan(self.plan)

    def test_compares_full_records_not_only_winners(self):
        native = {'status': 'passed', 'games': [{'winner': 0, 'trace_sha256': 'abc'}]}
        compare_records(native, copy.deepcopy(native), 'games')
        changed = copy.deepcopy(native)
        changed['games'][0]['trace_sha256'] = 'def'
        with self.assertRaises(AssertionError):
            compare_records(native, changed, 'games')
        for result in [{'status': 'running', 'games': native['games']},
                       {'status': 'passed', 'games': []}]:
            with self.assertRaises(AssertionError):
                compare_records(native, result, 'games')

    def test_requires_completion_marker_exact_captures_and_game_count(self):
        log = self.folder / 'run.log'
        case = {'captures': ['RIFT_BOARD'], 'minimum_games': 2}
        log.write_text('Native UI completed 2 actual games\n')
        with self.assertRaises(AssertionError):
            verify_ui_output(self.folder, case)
        log.write_text('UI smoke: all native click checks passed.\nNative UI completed 2 actual games\n')
        wrong = self.folder / 'different.png'
        wrong.write_bytes(b'\x89PNG\r\n\x1a\n')
        with self.assertRaises(AssertionError):
            verify_ui_output(self.folder, case)
        wrong.rename(self.folder / 'rift_board.png')
        self.assertEqual(verify_ui_output(self.folder, case)['games'], 2)
        with self.assertRaises(AssertionError):
            verify_ui_output(self.folder, {**case, 'minimum_games': 3})


if __name__ == '__main__':
    unittest.main()
