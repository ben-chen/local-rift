#!/usr/bin/env python3
"""Exercise Jae in complete legal Irelia matches, retaining observed draw triggers.

This authored coverage list is not a tournament preset or a strength benchmark.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match

FORMAT_DATE = '2026-11-13'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    raw = subprocess.run([str(binary)], input='{"command":"catalog"}\n',
                         text=True, capture_output=True, check=True, timeout=30)
    catalog = json.loads(raw.stdout)['result']
    jae = catalog['cards'][450]
    assert jae['official_id'] == 'sfd-142-221' and jae['name'] == 'Jae Medarda'
    preset = catalog['presets'][0]
    assert [row for row in preset['cards'] if row[0] == 15] == [[15, 3]]
    cards = [row for row in preset['cards'] if row[0] != 15] + [[450, 3]]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {
        'deck': {'name': 'Irelia / Jae spell-choice coverage', 'legend': preset['legend_id'],
                 'chosen_champion': preset['chosen_champion'], 'cards': cards, 'runes': runes,
                 'battlefield': preset['registration']['battlefields'][0]},
        'sideboard': preset['registration']['sideboard'],
        'battlefields': preset['registration']['battlefields'],
    }
    report = {'status': 'running', 'scope': __doc__.strip(), 'registration': registration,
              'format_date': FORMAT_DATE,
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'records': []}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                units, triggers, draws = set(), set(), set()
                previous = None
                digest = hashlib.sha256()

                def trace(request, response):
                    nonlocal previous
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != 0:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        previous = None
                        return
                    game = len(view['series']['history'])
                    for unit in view['players'][seat]['units']:
                        if unit['card_id'] == 450:
                            units.add((game, unit['id']))
                    for item in view['chain']:
                        if item['card_id'] == 450 and item['ability']:
                            assert item['ability']['type'] == 'card_trigger'
                            triggers.add((game, item['id']))
                    if previous and previous['chain'] and len(previous['series']['history']) == game:
                        top = previous['chain'][-1]
                        key = (game, top['id'])
                        if key in triggers and top['id'] not in {item['id'] for item in view['chain']}:
                            before, after = previous['players'][seat], view['players'][seat]
                            if (after['hand_count'] == before['hand_count'] + 1
                                    and after['deck_count'] == before['deck_count'] - 1):
                                draws.add(key)
                    previous = view

                registrations = [{'preset': 1}, {'preset': 1}]
                registrations[seat] = registration
                result = run_match(binary, registrations, 450100 + level_index * 10 + seat,
                                   level, seat, first_player=1-seat, trace=trace,
                                   format_date=FORMAT_DATE)
                record = {'difficulty': level, 'irelia_seat': seat, 'first_player': 1-seat,
                          'result': result, 'jae_units_seen': len(units),
                          'draw_triggers_seen': len(triggers), 'observed_draw_resolutions': len(draws),
                          'trace_sha256': digest.hexdigest()}
                report['records'].append(record)
                save()
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                print(f'Completed {level}, Irelia seat {seat}: {len(draws)} observed Jae draws', flush=True)
        # Preserve every zero-use case; the aggregate gate only establishes
        # that complete matches actually reached the new behavior at least once.
        assert sum(row['observed_draw_resolutions'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
