#!/usr/bin/env python3
"""Play legal authored Irelia/Kha'Zix registrations through the JSON API.

These are coverage lists, not tournament presets. Record actual Kha'Zix entries,
Attack/Defend chain items and observed XP resolution separately from match wins.
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
    response = subprocess.run([str(binary)], input='{"command":"catalog"}\n',
                              capture_output=True, text=True, check=True, timeout=30)
    catalog = json.loads(response.stdout)['result']
    card = next(row for row in catalog['cards'] if row['id'] == 449)
    assert card['name'] == "Kha'Zix, Mutating Horror"
    assert card['official_id'] == 'unl-143-219'
    assert card['tags'] == ["Kha'Zix", 'The Void']
    preset = catalog['presets'][0]
    cards = copy.deepcopy(preset['cards'])
    # Replace three Scuttle Crabs; retain the chosen Irelia champion and all
    # other registered main-deck cards. No changes to the shipped preset.
    assert [row for row in cards if row[0] == 15] == [[15, 3]]
    cards = [row for row in cards if row[0] != 15] + [[449, 3]]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {'deck': {'name': "Irelia / Kha'Zix rules coverage", 'legend': preset['legend_id'],
                             'chosen_champion': preset['chosen_champion'], 'cards': cards,
                             'runes': runes, 'battlefield': preset['registration']['battlefields'][0]},
                    'sideboard': preset['registration']['sideboard'],
                    'battlefields': preset['registration']['battlefields']}
    report = {'status': 'running', 'scope': 'Authored legal Irelia list versus registered Master Yi; historical snapshot, no strength claim.',
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'registration': registration, 'records': []}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                for starter in range(2):
                    seen_units, seen_triggers, resolved = set(), {}, set()
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
                        game_number = len(view['series']['history'])
                        for owner, player in enumerate(view['players']):
                            for unit in player['units']:
                                if unit['card_id'] == 449:
                                    seen_units.add((game_number, owner, unit['id']))
                        for item in view['chain']:
                            if item['card_id'] == 449 and item['ability'] and item['ability']['type'] == 'card_trigger':
                                seen_triggers[(game_number, item['id'])] = item['ability']['rule_index']
                        if previous and previous['chain'] and len(previous['series']['history']) == game_number:
                            top = previous['chain'][-1]
                            key = (game_number, top['id'])
                            if key in seen_triggers and top['id'] not in {item['id'] for item in view['chain']}:
                                owner = top['player']
                                if view['players'][owner]['xp'] == previous['players'][owner]['xp'] + 2:
                                    resolved.add(key)
                        previous = view

                    registrations = [{'preset': 1}, {'preset': 1}]
                    registrations[seat] = registration
                    result = run_match(binary, registrations, 449100 + level_index * 10 + seat * 2 + starter,
                                       level, seat, first_player=starter, trace=trace)
                    record = {'difficulty': level, 'irelia_seat': seat, 'first_player': starter,
                              'result': result, 'khazix_units_seen': len(seen_units),
                              'attack_triggers_seen': sum(index == 0 for index in seen_triggers.values()),
                              'defend_triggers_seen': sum(index == 1 for index in seen_triggers.values()),
                              'observed_xp_resolutions': len(resolved), 'trace_sha256': digest.hexdigest()}
                    report['records'].append(record)
                    save()
                    assert max(result['wins']) == 2 and len(result['history']) >= 2
                    assert seen_units, record
                    print(f'Completed {level}, Irelia seat {seat}, starter {starter}: {len(resolved)} observed XP resolutions', flush=True)
        # A legal game may never offer an isolated enemy. Keep those zero-use
        # records visible; require actual resolution across each level/seat's
        # two starting-player cases, and both trigger types in the campaign.
        for level in ['easy', 'tactical', 'strategic']:
            for seat in range(2):
                assert sum(row['observed_xp_resolutions'] for row in report['records']
                           if row['difficulty'] == level and row['irelia_seat'] == seat) > 0, (level, seat)
        assert sum(row['attack_triggers_seen'] for row in report['records']) > 0
        assert sum(row['defend_triggers_seen'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
