#!/usr/bin/env python3
"""Complete legal Irelia series with control cards, both seats and all AI levels.

Registrations are authored integration lists, not new sourced meta presets.
Target/cost/timing expectations also have independent Rust scenario coverage.
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
    assert [catalog['cards'][i]['official_id'] for i in [468, 469]] == ['ogn-203-298', 'unl-140-219']
    definition = catalog['legends'][0]
    assert all(card in definition['eligible_cards'] for card in [449, 468, 469])
    deck = copy.deepcopy(definition['training_deck'])
    # Same substitutions as the independent conservation campaign.
    for old, new in [(5, 468), (6, 469), (17, 449)]:
        row = next(row for row in deck['cards'] if row[0] == old)
        assert row[1] == 3
        row[0] = new
    deck['name'] = 'Irelia / control integration'
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
                plays = {468: set(), 469: set()}
                paid, taken, board_owners = set(), set(), {}
                digest = hashlib.sha256()

                def trace(request, response):
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != seat:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    for item in view['chain']:
                        if item['player'] == seat and item['card_id'] in plays and item.get('ability') is None:
                            plays[item['card_id']].add((game, item['id']))
                            if item['card_id'] == 469 and item['bonus']:
                                paid.add((game, item['id']))
                    for controller, player in enumerate(view['players']):
                        for unit in player['units']:
                            key = (game, unit['id'])
                            assert unit['controller'] == controller and unit['owner'] in [0, 1]
                            assert board_owners.setdefault(key, unit['owner']) == unit['owner']
                            if controller == seat and unit['owner'] != seat:
                                taken.add(key)

                seats = [{'preset': 0}, {'preset': 0}]
                seats[seat] = registration
                outcome = run_match(binary, seats, 469100 + 10 * level_index + seat,
                                    level, seat, first_player=1-seat, trace=trace,
                                    format_date=report['format_date'])
                assert max(outcome['wins']) == 2 and len(outcome['history']) >= 2
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                    'first_player': 1-seat, 'result': outcome,
                    'possession_plays': len(plays[468]), 'conscription_plays': len(plays[469]),
                    'paid_conscriptions': len(paid), 'distinct_taken_units': len(taken),
                    'trace_sha256': digest.hexdigest()})
                save()
                print(f'Completed {level}, seat {seat}: Possession {len(plays[468])}, Conscription {len(plays[469])}, taken {len(taken)}', flush=True)
        for seat in range(2):
            for key in ['possession_plays', 'conscription_plays', 'distinct_taken_units']:
                assert sum(row[key] for row in report['records'] if row['irelia_seat'] == seat) > 0, (seat, key)
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
