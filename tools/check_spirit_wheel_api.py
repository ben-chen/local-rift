#!/usr/bin/env python3
"""Exercise Spirit Wheel in complete registered Irelia matches at all AI levels.

The authored coverage list is not a tournament preset or a strength benchmark.
Track payment before responses separately from observed draw resolutions.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match

FORMAT_DATE = '2026-11-13'
WHEEL = 454


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    raw = subprocess.run([str(binary)], input='{"command":"catalog"}\n', text=True,
                         capture_output=True, check=True, timeout=30)
    catalog = json.loads(raw.stdout)['result']
    assert catalog['cards'][WHEEL]['official_id'] == 'sfd-144-221'
    preset = catalog['presets'][0]
    assert [row for row in preset['cards'] if row[0] == 15] == [[15, 3]]
    cards = [row for row in preset['cards'] if row[0] != 15] + [[WHEEL, 3]]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {
        'deck': {'name': 'Irelia / Spirit Wheel coverage', 'legend': preset['legend_id'],
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
                gears, paid, drawn, in_hand = set(), set(), set(), set()
                previous = None
                digest = hashlib.sha256()

                def trace(request, response):
                    nonlocal previous
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != seat:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        previous = None
                        return
                    game = len(view['series']['history'])
                    if WHEEL in view['players'][seat]['hand']:
                        in_hand.add((game, view['turn']))
                    board = {gear['id']: gear for gear in view['players'][seat]['gear']}
                    gears.update((game, gear['id']) for gear in board.values() if gear['card_id'] == WHEEL)
                    same_game = previous and len(previous['series']['history']) == game
                    for item in view['chain']:
                        key = (game, item['id'])
                        if item['player'] != seat or item['card_id'] != WHEEL or key in paid:
                            continue
                        assert item['ability']['type'] == 'card_trigger'
                        source = item['ability']['context']['source']['gear_id']
                        assert not board[source]['ready'], 'Source cost must be paid before responses'
                        if same_game:
                            before = previous['players'][seat]
                            old_gear = next(gear for gear in before['gear'] if gear['id'] == source)
                            assert old_gear['ready']
                            assert view['players'][seat]['energy'] == before['energy'] - 1
                            assert view['players'][seat]['hand_count'] == before['hand_count']
                        paid.add(key)
                    if same_game and previous['chain']:
                        top = previous['chain'][-1]
                        key = (game, top['id'])
                        if key in paid and top['id'] not in {item['id'] for item in view['chain']}:
                            before, after = previous['players'][seat], view['players'][seat]
                            if (after['hand_count'] == before['hand_count'] + 1
                                    and after['deck_count'] == before['deck_count'] - 1):
                                drawn.add(key)
                    previous = view

                registrations = [{'preset': 1}, {'preset': 1}]
                registrations[seat] = registration
                result = run_match(binary, registrations, 454100 + 10 * level_index + seat,
                                   level, seat, first_player=1-seat, trace=trace, format_date=FORMAT_DATE)
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                                          'first_player': 1-seat, 'result': result,
                                          'wheels_seen': len(gears), 'paid_triggers': len(paid),
                                          'turns_with_wheel_in_hand': len(in_hand),
                                          'observed_draw_resolutions': len(drawn),
                                          'trace_sha256': digest.hexdigest()})
                save()
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                print(f'Completed {level}, Irelia seat {seat}: {len(paid)} paid, {len(drawn)} draws', flush=True)
        # Retain zero-use cases; the aggregate proves exercise, not expert play.
        assert sum(row['observed_draw_resolutions'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
