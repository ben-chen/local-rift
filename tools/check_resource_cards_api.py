#!/usr/bin/env python3
"""Complete Irelia matches exercising Keeper, Meditation and Life from Stone.

The authored list measures behavior reached in real AI games, not meta strength.
Zero-use cases remain in the report; aggregate gates require actual resolutions.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match

FORMAT_DATE = '2026-11-13'
FEATURED = {451: 'Clockwork Keeper', 452: 'Meditation', 453: 'Life from Stone'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    raw = subprocess.run([str(binary)], input='{"command":"catalog"}\n', text=True,
                         capture_output=True, check=True, timeout=30)
    catalog = json.loads(raw.stdout)['result']
    for card_id, name in FEATURED.items():
        assert catalog['cards'][card_id]['name'] == name
    preset = catalog['presets'][0]
    replaced = {15: 451, 6: 452, 7: 453}
    assert {card: count for card, count in preset['cards'] if card in replaced} == {15: 3, 6: 3, 7: 3}
    cards = [[replaced.get(card, card), count] for card, count in preset['cards']]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {
        'deck': {'name': 'Irelia / resource-card coverage', 'legend': preset['legend_id'],
                 'chosen_champion': preset['chosen_champion'], 'cards': cards, 'runes': runes,
                 'battlefield': preset['registration']['battlefields'][0]},
        'sideboard': preset['registration']['sideboard'],
        'battlefields': preset['registration']['battlefields'],
    }
    report = {'status': 'running', 'scope': __doc__.strip(), 'registration': registration,
              'format_date': FORMAT_DATE, 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                seen = {card: set() for card in FEATURED}
                resolved = {card: set() for card in FEATURED}
                paid_meditations = set()
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
                        if unit['card_id'] == 451:
                            seen[451].add((game, unit['id']))
                    for item in view['chain']:
                        if item['player'] == seat and item['card_id'] in FEATURED:
                            card = item['card_id']
                            if card != 451:
                                seen[card].add((game, item['id']))
                            if card == 452 and item['bonus']:
                                paid_meditations.add((game, item['id']))
                    if previous and previous['chain'] and len(previous['series']['history']) == game:
                        top = previous['chain'][-1]
                        card = top['card_id']
                        if top['player'] == seat and card in FEATURED and top['id'] not in {item['id'] for item in view['chain']}:
                            before, after = previous['players'][seat], view['players'][seat]
                            draws = 2 if card == 452 and top['bonus'] else 1
                            executions = max(1, len(top.get('executions', [])))
                            draws *= executions
                            channel_ok = card != 453 or sum(after['runes']) == sum(before['runes']) + executions
                            if (channel_ok and after['hand_count'] == before['hand_count'] + draws
                                    and after['deck_count'] == before['deck_count'] - draws):
                                resolved[card].add((game, top['id']))
                    previous = view

                registrations = [{'preset': 1}, {'preset': 1}]
                registrations[seat] = registration
                result = run_match(binary, registrations, 451100 + 10 * level_index + seat,
                                   level, seat, first_player=1-seat, trace=trace, format_date=FORMAT_DATE)
                counts = {str(card): {'seen': len(seen[card]), 'resolutions': len(resolved[card])} for card in FEATURED}
                report['records'].append({'difficulty': level, 'irelia_seat': seat, 'first_player': 1-seat,
                                          'result': result, 'cards': counts,
                                          'paid_meditations_seen': len(paid_meditations),
                                          'trace_sha256': digest.hexdigest()})
                save()
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                print(f'Completed {level}, Irelia seat {seat}: {counts}', flush=True)
        for card in FEATURED:
            assert sum(row['cards'][str(card)]['resolutions'] for row in report['records']) > 0, f'No observed resolution for {FEATURED[card]}'
        assert sum(row['paid_meditations_seen'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
