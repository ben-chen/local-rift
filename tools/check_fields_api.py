#!/usr/bin/env python3
"""Complete legal Irelia series with each new battlefield, both seats and all AI levels.

These authored registrations test integration, not metagame prevalence or AI strength.
Exact rules expectations and response windows are covered by the Rust scenarios.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    result = subprocess.run([str(binary)], input='{"command":"catalog"}\n',
                            text=True, capture_output=True, check=True, timeout=30)
    catalog = json.loads(result.stdout)['result']
    fields = [18, 19, 20]
    assert [catalog['battlefields'][i]['name'] for i in fields] == [
        'Forgotten Monument', 'Back-Alley Bar', 'The Candlelit Sanctum']
    preset = catalog['presets'][0]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    report = {'status': 'running', 'scope': __doc__, 'format_date': '2026-11-16',
              'records': [], 'registrations': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for field in fields:
            registration = {
                'deck': {'name': 'Irelia / battlefield verification',
                         'legend': preset['legend_id'], 'chosen_champion': preset['chosen_champion'],
                         'cards': preset['cards'], 'runes': runes, 'battlefield': field},
                'sideboard': preset['registration']['sideboard'],
                'battlefields': [field] + [other for other in fields if other != field],
            }
            report['registrations'].append(registration)
            for index, level in enumerate(['easy', 'tactical', 'strategic']):
                for seat in range(2):
                    present, looks, bar_triggers = set(), set(), set()
                    digest = hashlib.sha256()

                    def trace(request, response):
                        digest.update(json.dumps([request, response], sort_keys=True,
                                                 separators=(',', ':')).encode())
                        if request['command'] != 'observe':
                            return
                        view = response.get('result', {})
                        if view.get('series', {}).get('phase') != 'game':
                            return
                        game = len(view['series']['history'])
                        if field in view['battlefields']:
                            present.add(game)
                        for restriction in view.get('battlefield_scoring', []):
                            for player in restriction['players']:
                                remaining = max(3 - player['turns_started'], 0)
                                assert player['turns_until_unlocked'] == remaining
                                assert player['scoring_unlocked'] == (remaining == 0)
                        decision = view.get('decision') or {}
                        if decision.get('purpose') == 'look_recycle':
                            assert decision['source']['battlefield_id'] == 20
                            assert ('cards' in decision) == (decision['player'] == request['player'])
                            looks.add((game, decision['choice_id'], decision['player']))
                        for item in view['chain']:
                            ability = item.get('ability') or {}
                            if ability.get('type') == 'back_alley_might':
                                assert item['targets'] == []
                                assert item['source']['battlefield_id'] == 19
                                bar_triggers.add((game, item['id']))

                    registrations = [{'preset': 1}, {'preset': 1}]
                    registrations[seat] = registration
                    outcome = run_match(binary, registrations, field * 10000 + index * 10 + seat,
                                        level, seat, first_player=1-seat, trace=trace,
                                        format_date=report['format_date'])
                    assert max(outcome['wins']) == 2 and len(outcome['history']) >= 2
                    assert present, 'The selected battlefield never entered a game.'
                    report['records'].append({'field': field, 'difficulty': level, 'irelia_seat': seat,
                                              'first_player': 1-seat, 'result': outcome,
                                              'games_with_field': len(present), 'private_looks': len(looks),
                                              'bar_triggers': len(bar_triggers),
                                              'trace_sha256': digest.hexdigest()})
                    save()
                    print(f'Completed field {field}, {level}, seat {seat}: '
                          f'{len(outcome["history"])} games, {len(looks)} looks, '
                          f'{len(bar_triggers)} movement triggers', flush=True)
        assert sum(r['private_looks'] for r in report['records']) > 0
        assert sum(r['bar_triggers'] for r in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
