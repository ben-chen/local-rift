#!/usr/bin/env python3
"""Legal Irelia Block/Yuumi practice series through the external JSON API."""
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
        package = [461, 462]
        assert all(card in definition['eligible_cards'] for card in package)
        deck['cards'] = [row for row in deck['cards'] if row[0] not in package]
        deck['cards'].extend([[card, 3] for card in package])
        excess = sum(count for _, count in deck['cards']) - 40
        for row in deck['cards']:
            if row[0] in package:
                continue
            remove = min(excess, row[1] - int(row[0] == deck['chosen_champion']))
            row[1] -= remove
            excess -= remove
        assert excess == 0
        deck['cards'] = [row for row in deck['cards'] if row[1]]
    deck['name'] = definition['champion'] + ' / turn keyword verification'
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
    assert [catalog['cards'][i]['official_id'] for i in [461, 462]] == ['ogn-057-298', 'unl-056-219']
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
                block, yuumi, shield, tank = set(), set(), set(), set()
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
                            if 'turn_shield' in unit:
                                effect = unit['turn_shield']
                                assert effect['amount_exact'] == str(effect['amount'])
                                assert effect['expires'] == 'end_of_turn'
                                if effect['amount'] >= 3:
                                    shield.add((game, owner, unit['id']))
                            if 'turn_tank' in unit:
                                assert unit['turn_tank'] == {'expires': 'end_of_turn'}
                                assert unit['keywords']['tank'] is True
                                tank.add((game, owner, unit['id']))
                    for item in view['chain']:
                        if item['card_id'] == 461 and not item.get('ability'):
                            block.add((game, item['player'], item['id']))
                        if item['card_id'] == 462 and item.get('ability'):
                            yuumi.add((game, item['player'], item['id']))

                seats = registrations if seat == 0 else registrations[::-1]
                outcome = run_match(binary, seats, 461100 + 10 * level_index + seat,
                                    level, seat, first_player=1-seat, trace=trace,
                                    format_date=report['format_date'])
                assert max(outcome['wins']) == 2 and len(outcome['history']) >= 2
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                                          'first_player': 1-seat, 'result': outcome,
                                          'block_casts': len(block), 'yuumi_triggers': len(yuumi),
                                          'shield_grants': len(shield), 'tank_grants': len(tank),
                                          'trace_sha256': digest.hexdigest()})
                save()
                print(f'Completed {level}, Irelia seat {seat}: Block {len(block)}, Yuumi {len(yuumi)}, granted Tank {len(tank)}', flush=True)
        for seat in range(2):
            rows = [row for row in report['records'] if row['irelia_seat'] == seat]
            for field in ['block_casts', 'yuumi_triggers', 'shield_grants', 'tank_grants']:
                assert sum(row[field] for row in rows) > 0, (seat, field)
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
