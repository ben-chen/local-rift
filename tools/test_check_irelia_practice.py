"""An iteration check must expose misses and reject incomplete/failed evidence."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import check_irelia_practice as checks


class PracticeChecks(unittest.TestCase):
    def setUp(self):
        self.report = {'positions': 1, 'records': [
            {'position': 'ready-to-win', 'seat': seat, 'difficulty': level,
             'must_pass': not (level == 'Easy' and seat == 0),
             'success': not (level == 'Easy' and seat == 0)}
            for level in ['Easy', 'Tactical', 'Strategic'] for seat in range(2)]}

    def test_known_miss_is_visible_without_claiming_every_objective_passed(self):
        self.report['summary'] = [{'successes': 999}]  # Recompute from actual cases.
        result = checks.decision_summary(self.report)
        self.assertEqual((result['cases'], result['required_passed']), (6, 5))
        self.assertEqual(result['levels'][0]['successes'], 1)
        self.assertEqual(result['levels'][0]['misses'], [{'position': 'ready-to-win', 'seat': 0}])

    def test_rejects_failed_required_case_missing_seat_and_duplicate_case(self):
        failed = copy.deepcopy(self.report)
        failed['records'][0]['must_pass'] = True
        missing = copy.deepcopy(self.report)
        missing['records'].pop()
        duplicate = copy.deepcopy(self.report)
        duplicate['records'].append(duplicate['records'][0])
        for report in [failed, missing, duplicate, {'positions': 0, 'records': []}]:
            with self.subTest(report=report), self.assertRaises(ValueError):
                checks.decision_summary(report)

    def test_rejects_nonboolean_outcomes(self):
        self.report['records'][0]['success'] = 'false'
        with self.assertRaises(ValueError):
            checks.decision_summary(self.report)

    def test_junit_requires_each_suite_benchmarks_and_no_skips_or_failures(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'junit.xml'
            tree = ET.Element('testsuites', tests='8', skipped='0', failures='0', errors='0', time='1.5')
            suite = ET.SubElement(tree, 'testsuite')
            for name in sorted(checks.SUITES):
                ET.SubElement(suite, 'testcase', name='fixture', classname=name)
            for name in ['measured_irelia_decisions_and_companion',
                         'waiting_for_irelia_reset_does_not_ignore_an_enemy_response']:
                ET.SubElement(suite, 'testcase', name='practice_benchmark::' + name,
                              classname='local-rift::bin/local-rift')
            ET.ElementTree(tree).write(path)
            self.assertEqual(checks.test_summary(path)['passed'], 8)
            for tag in ['failure', 'error', 'skipped']:
                bad = copy.deepcopy(tree)
                ET.SubElement(bad.find('.//testcase'), tag)
                ET.ElementTree(bad).write(path)
                with self.subTest(tag=tag), self.assertRaises(ValueError):
                    checks.test_summary(path)
            for case in [suite[0], suite[-1]]:
                bad = copy.deepcopy(tree)
                bad.find('testsuite').remove(next(row for row in bad.find('testsuite') if row.attrib == case.attrib))
                ET.ElementTree(bad).write(path)
                with self.assertRaises(ValueError):
                    checks.test_summary(path)

    def test_failed_test_process_retains_failure_and_never_starts_api(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / 'repo'
            (root / '.config').mkdir(parents=True)
            (root / '.config/nextest.toml').write_text('[profile.ci.junit]\npath = "junit.xml"\n')
            output = Path(folder) / 'run'
            with patch.object(checks, 'ROOT', root), \
                    patch.object(checks, 'source_identity', return_value={'revision': 'test'}), \
                    patch('sys.argv', ['check', '--output', str(output), '--api']), \
                    patch.object(checks.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1)) as run:
                with self.assertRaises(subprocess.CalledProcessError):
                    checks.main()
                self.assertEqual(run.call_count, 1)
            status = json.loads((output / 'status.json').read_text())
            self.assertEqual(status['status'], 'failed')
            self.assertEqual(status['stage'], 'rules-and-decisions')
            self.assertEqual(status['api'], 'pending')
            self.assertEqual(status['steps'][0]['exit_code'], 1)


if __name__ == '__main__':
    unittest.main()
