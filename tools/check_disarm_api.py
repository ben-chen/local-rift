#!/usr/bin/env python3
"""Complete legal Irelia/Siren versus Kai'Sa/Mechs series; authored coverage lists."""
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
    package = [card for card in range(458, 461) if card in definition['eligible_cards']]
    assert package == ([460] if legend == 0 else [458, 459])
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
    deck['name'] = definition['champion'] + ' / Disarm coverage'
    deck['battlefield'] = 0
    return {'deck': deck, 'sideboard': [], 'battlefields': [0, 3, 13]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    raw = subprocess.run([str(binary)], input='{"command":"catalog"}\n', text=True,
                         capture_output=True, check=True, timeout=30)
    catalog = json.loads(raw.stdout)['result']
    assert [catalog['cards'][i]['official_id'] for i in range(458, 461)] == [
        'rad-054-167', 'rad-055-167', 'rad-097-167']
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
                played, triggers, reductions = set(), set(), set()
                previous, previous_game = None, None
                digest = hashlib.sha256()

                def trace(request, response):
                    nonlocal previous, previous_game
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != 0:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    units = {(owner, unit['id']): unit for owner, player in enumerate(view['players'])
                             for unit in player['units']}
                    for (owner, _), unit in units.items():
                        if not unit['token'] and 458 <= unit['card_id'] <= 460:
                            played.add((owner, unit['card_id']))
                            assert unit['keywords']['disarm'] >= 1
                            assert unit['keywords']['disarm_exact'] == str(unit['keywords']['disarm'])
                    for item in view['chain']:
                        keyword = (item.get('ability') or {}).get('context', {}).get('keyword')
                        if keyword and keyword['type'] == 'disarm':
                            assert keyword['value'] > 0 and keyword['value_exact'] == str(keyword['value'])
                            triggers.add((game, item['id']))
                    if previous_game == game and previous and previous['chain']:
                        item = previous['chain'][-1]
                        keyword = (item.get('ability') or {}).get('context', {}).get('keyword')
                        if keyword and keyword['type'] == 'disarm' and item['id'] not in {x['id'] for x in view['chain']}:
                            for target in item['targets']:
                                key = (target['player'], target['unit_id'])
                                old = next((u for u in previous['players'][key[0]]['units'] if u['id'] == key[1]), None)
                                new = units.get(key)
                                if old and new and new['temporary_buff'] == old['temporary_buff'] - keyword['value']:
                                    reductions.add((game, item['id'], key))
                    previous, previous_game = view, game

                seats = registrations if seat == 0 else registrations[::-1]
                result = run_match(binary, seats, 458100 + 10 * level_index + seat,
                                   level, seat, first_player=1-seat, trace=trace, format_date=report['format_date'])
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                                          'first_player': 1-seat, 'result': result,
                                          'cards_seen': sorted(played), 'disarm_triggers': len(triggers),
                                          'observed_reductions': len(reductions), 'trace_sha256': digest.hexdigest()})
                save()
                print(f'Completed {level}, Irelia seat {seat}: {len(triggers)} Disarm triggers, {len(reductions)} reductions', flush=True)
        for seat in range(2):
            assert {card for row in report['records'] for owner, card in row['cards_seen'] if owner == seat} == {458, 459, 460}
        assert sum(row['disarm_triggers'] for row in report['records']) > 0
        assert sum(row['observed_reductions'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
