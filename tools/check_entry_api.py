#!/usr/bin/env python3
"""Authored legal Irelia series exercise both entry cards, both seats and all AI levels.

These are integration registrations, not sourced meta decks or a strength benchmark.
Exact entry timing, cost and restriction expectations are tested in Rust scenarios.
"""
import argparse
import copy
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
    assert [catalog['cards'][i]['official_id'] for i in [466, 467]] == ['unl-122-219', 'rad-026-167']
    definition = catalog['legends'][0]
    deck = copy.deepcopy(definition['training_deck'])
    assert all(card in definition['eligible_cards'] for card in [466, 467])
    deck['cards'] = [row for row in deck['cards'] if row[0] not in [466, 467]] + [[466, 3], [467, 3]]
    excess = sum(count for _, count in deck['cards']) - 40
    for row in deck['cards']:
        if row[0] not in [466, 467]:
            remove = min(excess, row[1] - int(row[0] == deck['chosen_champion']))
            row[1] -= remove
            excess -= remove
    assert excess == 0
    deck['cards'] = [row for row in deck['cards'] if row[1]]
    deck['name'] = 'Irelia / optional entry verification'
    deck['battlefield'] = 3
    registration = {'deck': deck, 'sideboard': [], 'battlefields': [3, 0, 13]}
    report = {'status': 'running', 'scope': __doc__, 'format_date': '2026-11-16',
              'registration': registration, 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                entries, restricted = {466: set(), 467: set()}, set()
                digest = hashlib.sha256()

                def trace(request, response):
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != seat:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    for unit in view['players'][seat]['units']:
                        card = unit['card_id']
                        if card in entries:
                            entries[card].add((game, unit['id']))
                        if card == 467 and unit.get('movement_restriction'):
                            assert unit['movement_restriction'] == {'scope': 'any_player', 'expires': 'end_of_turn'}
                            restricted.add((game, unit['id']))

                seats = [{'preset': 1}, {'preset': 1}]
                seats[seat] = registration
                outcome = run_match(binary, seats, 467100 + 10 * level_index + seat,
                                    level, seat, first_player=1-seat, trace=trace,
                                    format_date=report['format_date'])
                assert max(outcome['wins']) == 2 and len(outcome['history']) >= 2
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                                          'first_player': 1-seat, 'result': outcome,
                                          'guardian_entries': len(entries[466]), 'boar_entries': len(entries[467]),
                                          'restricted_boars_seen': len(restricted), 'trace_sha256': digest.hexdigest()})
                save()
                print(f'Completed {level}, seat {seat}: Guardian {len(entries[466])}, Boar {len(entries[467])}', flush=True)
        for seat in range(2):
            for key in ['guardian_entries', 'boar_entries']:
                assert sum(row[key] for row in report['records'] if row['irelia_seat'] == seat) > 0, (seat, key)
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
