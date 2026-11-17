#!/usr/bin/env python3
"""Exercise shared binary answers in complete Irelia games through JSONL."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

example_path = Path(__file__).resolve().parents[1] / 'examples' / 'random_bot.py'
spec = importlib.util.spec_from_file_location('binary_choices_example', example_path)
example = importlib.util.module_from_spec(spec)
spec.loader.exec_module(example)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    report = {'status': 'running', 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'example_sha256': hashlib.sha256(example_path.read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for first in range(2):
                for sample in range(2):
                    seed = 486000 + level_index * 100 + first * 10 + sample
                    match = example.Match(binary)
                    digest = hashlib.sha256()
                    counts = [{}, {}]

                    def call(command, **fields):
                        before = match.revision
                        result = match.request(command, **fields)
                        digest.update(json.dumps([command, fields, before, match.revision, result],
                                                 sort_keys=True, separators=(',', ':')).encode())
                        return result

                    try:
                        call('reset', seed=seed, decks=[0, 0], difficulty=level, first_player=first)
                        for player in range(2):
                            call('mulligan', player=player, indices=[])
                        call('start')
                        for step in range(10000):
                            public = call('observe', player=0)
                            if public['phase'] == 'finished':
                                report['records'].append({
                                    'seed': seed, 'difficulty': level, 'first_player': first,
                                    'winner': public['winner'], 'turn': public['turn'],
                                    'scores': [p['score'] for p in public['players']],
                                    'steps': step, 'questions': counts, 'trace_sha256': digest.hexdigest(),
                                })
                                save()
                                break
                            actor = public['priority']
                            view = public if actor == 0 else call('observe', player=actor)
                            kind = (view.get('decision') or {}).get('type')
                            if kind in example.BINARY_DECISIONS:
                                revision = match.revision
                                choices = call('binary_choices', player=actor)
                                assert match.revision == revision
                                assert call('observe', player=actor) == view
                                assert choices['player'] == actor and choices['scope'] == 'choose'
                                assert [o['action'] for o in choices['options']] == [
                                    {'type': 'choose', 'accept': True}, {'type': 'choose', 'accept': False}]
                                enabled = [o for o in choices['options'] if o['enabled']]
                                assert enabled
                                for option in enabled:
                                    assert option['unavailable_reason'] is None
                                    call('validate', player=actor, action=option['action'])
                                    assert match.revision == revision
                                counts[actor][kind] = counts[actor].get(kind, 0) + 1
                                call('apply', player=actor, action=enabled[0]['action'])
                            else:
                                call('step_ai', player=actor)
                        else:
                            raise AssertionError('Game did not finish within the action limit.')
                    finally:
                        match.close()
        assert len(report['records']) == 12
        for actor in range(2):
            assert sum(r['questions'][actor].get('legend', 0) for r in report['records']) > 0
        report['status'] = 'passed'
    except BaseException as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
