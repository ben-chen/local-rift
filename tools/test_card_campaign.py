"""Campaign evidence must detect missing coverage and preserve failed traces."""
from contextlib import redirect_stdout
import copy
from dataclasses import replace
import hashlib
import io
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch

from card_campaign import Campaign, registration, run_campaign


class CardCampaignTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.folder = Path(folder.name)
        self.binary = self.folder / 'binary'
        self.binary.write_bytes(b'fixed-test-binary')
        self.runner = self.folder / 'campaign.py'
        self.runner.write_text('# fixed test campaign\n')
        self.output = self.folder / 'report.json'
        self.spec = Campaign('test', 'authored', ((7, 'test-007'),), ((4, 7),),
                             1000, samples=(1, 1, 1), target_counts={7: 1})
        self.catalog = {'cards': [{'official_id': f'test-{i:03d}'} for i in range(8)],
                        'legends': [{'eligible_cards': [7],
                                     'training_deck': {'name': 'original', 'cards': [[4, 2], [1, 3]]}}]}

    def fake_match(self, binary, decks, seed, level, designated, *, trace, **kwargs):
        self.assertEqual(decks[0]['deck']['cards'], [[7, 2], [1, 3]])
        self.assertEqual(kwargs['first_player'], designated)
        self.assertEqual(kwargs['format_date'], '2026-11-17')
        request, response = self.event()
        trace(request, response)
        trace(request, response)  # Re-observing a chain must not count another cast.
        return {'wins': [2, 0], 'winner': 0, 'history': [{'winner': 0}, {'winner': 0}],
                'seed': seed, 'difficulty': level}

    @staticmethod
    def event():
        return {'command': 'observe', 'player': 0}, {
            'result': {'series': {'phase': 'game', 'history': []},
                       'chain': [{'id': 1 + p, 'card_id': 7, 'player': p, 'targets': [3]}
                                 for p in range(2)]}}

    def run_fake(self, matcher=None, spec=None):
        with patch('card_campaign.subprocess.run', return_value=types.SimpleNamespace(
                stdout=json.dumps({'result': self.catalog}))), \
                patch('card_campaign.run_match', side_effect=matcher or self.fake_match), \
                redirect_stdout(io.StringIO()):
            return run_campaign(spec or self.spec, self.binary, self.output, self.runner)

    def report(self):
        return json.loads(self.output.read_text())

    def test_preserves_seeds_trace_hashes_and_unique_physical_cast_counts(self):
        before = copy.deepcopy(self.catalog)
        report = self.run_fake()
        self.assertEqual(self.catalog, before)
        self.assertEqual(report['status'], 'passed')
        self.assertEqual([row['result']['seed'] for row in report['records']],
                         [1000, 1010, 1100, 1110, 1200, 1210])
        encoded = json.dumps(list(self.event()), sort_keys=True, separators=(',', ':')).encode()
        for row in report['records']:
            self.assertEqual(row['casts'], [[1], [1]])
            self.assertEqual(row['trace_sha256'], hashlib.sha256(encoded * 2).hexdigest())
        self.assertNotIn('active_series', report)
        self.assertFalse(list(self.folder.glob('*.jsonl')))

    def test_wrong_source_identity_fails_before_playing_and_records_failure(self):
        self.catalog['cards'][7]['official_id'] = 'wrong-card'
        with self.assertRaisesRegex(AssertionError, 'identities differ'):
            self.run_fake(lambda *args, **kwargs: self.fail('Must reject the catalog before playing.'))
        self.assertEqual(self.report()['status'], 'failed')
        self.assertEqual(self.report()['records'], [])

    def test_required_cast_coverage_cannot_pass_from_wins_alone(self):
        def no_casts(*args, trace, **kwargs):
            trace({'command': 'observe'}, {'result': {'series': {'phase': 'sideboarding'}}})
            return {'wins': [2, 0], 'history': [{}, {}]}

        with self.assertRaisesRegex(AssertionError, 'No cast observed: easy, seat 0, card 7'):
            self.run_fake(no_casts)
        report = self.report()
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(len(report['records']), 6)
        self.assertEqual(report['active_series']['seed'], 1210)
        self.assertTrue(Path(report['failed_trace']['path']).is_file())

    def test_cast_exemptions_require_reasons_and_retain_actual_zero_counts(self):
        with self.assertRaisesRegex(AssertionError, 'explicit reason'):
            replace(self.spec, cast_exemptions={('strategic', 7): ''}).validate()

        def strategy_may_decline(binary, decks, seed, level, designated, *, trace, **kwargs):
            if level == 'strategic':
                trace({'command': 'observe'}, {'result': {'series': {'phase': 'sideboarding'}}})
                return {'wins': [2, 0], 'history': [{}, {}]}
            return self.fake_match(binary, decks, seed, level, designated, trace=trace, **kwargs)

        report = self.run_fake(strategy_may_decline, replace(
            self.spec, cast_exemptions={('strategic', 7): 'Authored choices tested separately.'}))
        self.assertEqual(report['status'], 'passed')
        self.assertEqual([row['casts'] for row in report['records'][-2:]], [[[0], [0]], [[0], [0]]])
        self.assertIn('Authored choices tested separately.', report['coverage_limits'][0])

    def test_bad_target_retains_the_actual_request_response_and_seed(self):
        def bad_target(*args, trace, **kwargs):
            request, response = self.event()
            response['result']['chain'][0]['targets'] = []
            trace(request, response)
            self.fail('The target assertion must fail inside the trace callback.')

        with self.assertRaisesRegex(AssertionError, 'unexpected target count'):
            self.run_fake(bad_target)
        report = self.report()
        self.assertEqual(report['active_series']['seed'], 1000)
        path = Path(report['failed_trace']['path'])
        trace = [json.loads(line) for line in path.read_text().splitlines()]
        self.assertEqual(trace[0][0], {'command': 'observe', 'player': 0})
        self.assertEqual(trace[0][1]['result']['chain'][0]['targets'], [])
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), report['failed_trace']['sha256'])

    def test_interruption_preserves_completed_records_and_active_series(self):
        calls = 0

        def interrupted(*args, trace, **kwargs):
            nonlocal calls
            calls += 1
            if calls == 2:
                trace(*self.event())
                raise KeyboardInterrupt('test interruption')
            return self.fake_match(*args, trace=trace, **kwargs)

        with self.assertRaises(KeyboardInterrupt):
            self.run_fake(interrupted)
        report = self.report()
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(len(report['records']), 1)
        self.assertEqual(report['active_series']['seed'], 1010)
        self.assertTrue(Path(report['failed_trace']['path']).is_file())

    def test_existing_evidence_is_never_overwritten(self):
        self.output.write_text('previous evidence')
        with self.assertRaises(FileExistsError):
            self.run_fake()
        self.assertEqual(self.output.read_text(), 'previous evidence')

    def test_changed_binary_cannot_produce_accepted_evidence(self):
        def changed(*args, **kwargs):
            result = self.fake_match(*args, **kwargs)
            self.binary.write_bytes(b'changed-during-campaign')
            return result

        with self.assertRaisesRegex(AssertionError, 'changed during verification'):
            self.run_fake(changed)
        self.assertEqual(self.report()['status'], 'failed')
        self.assertIn('failed_trace', self.report())

    def test_registration_rejects_missing_copies_and_never_mutates_source(self):
        before = copy.deepcopy(self.catalog)
        registration(self.spec, self.catalog)
        self.assertEqual(self.catalog, before)
        self.catalog['legends'][0]['training_deck']['cards'][0][1] = 1
        with self.assertRaisesRegex(AssertionError, 'exactly two copies'):
            registration(self.spec, self.catalog)


if __name__ == '__main__':
    unittest.main()
