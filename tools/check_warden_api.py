#!/usr/bin/env python3
"""Legal Irelia/Warden practice series through the external JSON API."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match


def registration(catalog, legend):
    definition = catalog['legends'][legend]
    deck = copy.deepcopy(definition['training_deck'])
    if legend == 0:
        assert 464 in definition['eligible_cards']
        deck['cards'] = [row for row in deck['cards'] if row[0] != 464] + [[464, 3]]
        excess = sum(count for _, count in deck['cards']) - 40
        for row in deck['cards']:
            if row[0] != 464:
                remove = min(excess, row[1] - int(row[0] == deck['chosen_champion']))
                row[1] -= remove
                excess -= remove
        assert excess == 0
        deck['cards'] = [row for row in deck['cards'] if row[1]]
    deck['name'] = definition['champion'] + ' / Warden verification'
    deck['battlefield'] = 0
    return {'deck': deck, 'sideboard': [], 'battlefields': [0, 3, 13]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    result = subprocess.run([str(binary)], input='{"command":"catalog"}\n',
                            text=True, capture_output=True, check=True, timeout=30)
    catalog = json.loads(result.stdout)['result']
    assert catalog['cards'][464]['official_id'] == 'ogn-070-298'
    registrations = [registration(catalog, legend) for legend in [0, 3]]
    report = {'status': 'running', 'scope': __doc__, 'format_date': '2026-11-14',
              'registrations': registrations, 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                seen, active = set(), set()
                digest = hashlib.sha256()

                def trace(request, response):
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != 0:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    for owner, player in enumerate(view['players']):
                        for unit in player['units']:
                            if unit['card_id'] == 464 and not unit['token']:
                                seen.add((game, owner, unit['id']))
                        expected = []
                        for unit in view['players'][1-owner]['units']:
                            if unit['card_id'] == 464 and not unit['token'] and unit['lane'] is not None:
                                active.add((game, 1-owner, unit['id']))
                                for kind in ['unit_play_base_only', 'unit_gear_ready_by_effect']:
                                    expected.append({'kind': kind, 'source': {
                                        'player': 1-owner, 'unit_id': unit['id'],
                                        'card_id': 464, 'lane': unit['lane']},
                                        'duration': 'while_source_at_battlefield'})
                        assert player.get('board_restrictions', []) == expected

                seats = registrations if seat == 0 else registrations[::-1]
                outcome = run_match(binary, seats, 464100 + 10 * level_index + seat,
                                    level, seat, first_player=1-seat, trace=trace,
                                    format_date=report['format_date'])
                assert max(outcome['wins']) == 2 and len(outcome['history']) >= 2
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                                          'first_player': 1-seat, 'result': outcome,
                                          'wardens_seen': len(seen), 'active_wardens': len(active),
                                          'trace_sha256': digest.hexdigest()})
                save()
                print(f'Completed {level}, Irelia seat {seat}: Warden {len(seen)}, active {len(active)}', flush=True)
        for seat in range(2):
            rows = [row for row in report['records'] if row['irelia_seat'] == seat]
            for field in ['wardens_seen', 'active_wardens']:
                assert sum(row[field] for row in rows) > 0, (seat, field)
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
