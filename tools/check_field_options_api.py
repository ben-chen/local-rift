#!/usr/bin/env python3
"""Complete authored Irelia field-option series through the standalone API.

Both seats and all three AI levels use legal registrations and ordinary actions.
These lists test integration; they are not sourced meta presets. Natural card
usage is reported, not forced. Independent rules scenarios and legal scripted
openings cover expensive Rostrum when an unassisted policy does not choose it.
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
                              text=True, capture_output=True, check=True, timeout=30)
    catalog = json.loads(response.stdout)['result']
    cards = [470, 471, 472]
    assert [catalog['cards'][i]['official_id'] for i in cards] == [
        'ogn-176-298', 'ven-115-166', 'rad-046-167']
    assert catalog['battlefields'][21]['name'] == 'Sandswept Tomb'
    definition = catalog['legends'][0]
    assert all(card in definition['eligible_cards'] for card in cards)
    deck = copy.deepcopy(definition['training_deck'])
    for old, new in zip([5, 6, 17], cards):
        row = next(row for row in deck['cards'] if row[0] == old)
        assert row[1] == 3
        row[0] = new
    deck['name'] = 'Irelia / field options integration'
    deck['battlefield'] = 21
    registration = {'deck': deck, 'sideboard': [], 'battlefields': [21, 3, 13]}
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
                entries = {card: set() for card in cards}
                uses, field_games = set(), set()
                digest = hashlib.sha256()

                def trace(request, response):
                    digest.update(json.dumps([request, response], sort_keys=True,
                                             separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != seat:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    if 21 in view['battlefields']:
                        field_games.add(game)
                    for kind in ['units', 'gear']:
                        for obj in view['players'][seat][kind]:
                            if not obj['token'] and obj['card_id'] in entries:
                                entries[obj['card_id']].add((game, obj['id']))
                    for item in view['chain']:
                        ability = item.get('ability') or {}
                        if (item['player'] == seat and ability.get('type') == 'activated'
                                and ability.get('card_id') == 472):
                            uses.add((game, item['id']))

                seats = [{'preset': 0}, {'preset': 0}]
                seats[seat] = registration
                outcome = run_match(binary, seats, 472200 + 100 * level_index + seat,
                                    level, seat, first_player=1-seat, trace=trace,
                                    format_date=report['format_date'])
                assert max(outcome['wins']) == 2 and len(outcome['history']) >= 2
                assert field_games, 'registered Sandswept Tomb was never selected'
                row = {'difficulty': level, 'irelia_seat': seat, 'first_player': 1-seat,
                       'result': outcome, 'card_entries': {str(c): len(entries[c]) for c in cards},
                       'rostrum_activations': len(uses), 'games_with_tomb': len(field_games),
                       'trace_sha256': digest.hexdigest()}
                report['records'].append(row)
                save()
                print(f'Completed {level}, seat {seat}: {row["card_entries"]}; '
                      f'Rostrum activations {len(uses)}', flush=True)
        for seat in range(2):
            for card in [470, 471]:
                assert sum(row['card_entries'][str(card)] for row in report['records']
                           if row['irelia_seat'] == seat) > 0, (seat, card)
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
