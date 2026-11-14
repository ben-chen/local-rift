#!/usr/bin/env python3
"""Complete registered Irelia matches exercising Crumbling Sands and public history.

This authored coverage deck is not a sourced tournament preset or a strength benchmark.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match

SANDS = 456
FORMAT_DATE = '2026-11-14'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    raw = subprocess.run([str(binary)], input='{"command":"catalog"}\n', text=True,
                         capture_output=True, check=True, timeout=30)
    catalog = json.loads(raw.stdout)['result']
    assert catalog['cards'][SANDS]['official_id'] == 'ven-039-166'
    preset = catalog['presets'][0]
    assert [row for row in preset['cards'] if row[0] == 15] == [[15, 3]]
    cards = [row for row in preset['cards'] if row[0] != 15] + [[SANDS, 3]]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {
        'deck': {'name': 'Irelia / Crumbling Sands coverage', 'legend': preset['legend_id'],
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
                played, countered, no_effect, in_hand = set(), set(), set(), set()
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
                    if SANDS in view['players'][seat]['hand']:
                        in_hand.add((game, view['turn']))
                    history = view['play_history']['finalized_spells']
                    flat = history[0] + history[1]
                    assert len(flat) == len(set(flat)), 'A physical spell must be recorded once'
                    for item in view['chain']:
                        if item['ability'] is None:
                            assert item['id'] in flat, 'Finalized spell missing from public history'
                        if item['card_id'] == SANDS and item['ability'] is None:
                            played.add((game, item['id']))
                    same_game = previous and len(previous['series']['history']) == game
                    if same_game and previous['turn'] == view['turn']:
                        for owner in range(2):
                            assert set(previous['play_history']['finalized_spells'][owner]) <= set(history[owner])
                    if same_game and previous['chain']:
                        top = previous['chain'][-1]
                        live = {item['id'] for item in view['chain']}
                        if top['card_id'] == SANDS and top['ability'] is None and top['id'] not in live:
                            target = top['targets'][0]['chain_id']
                            if target in {item['id'] for item in previous['chain']}:
                                eligible = any(spell != target for spell in
                                               previous['play_history']['finalized_spells'][1-top['player']])
                                assert (target not in live) == eligible, 'Sands resolution disagrees with spell history'
                                (countered if eligible else no_effect).add((game, top['id']))
                    previous = view

                registrations = [{'preset': 1}, {'preset': 1}]
                registrations[seat] = registration
                result = run_match(binary, registrations, 456100 + 10 * level_index + seat,
                                   level, seat, first_player=1-seat, trace=trace, format_date=FORMAT_DATE)
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                                          'first_player': 1-seat, 'result': result,
                                          'spells_played': len(played),
                                          'observed_counters': len(countered),
                                          'observed_no_effect_resolutions': len(no_effect),
                                          'turns_with_sands_in_hand': len(in_hand),
                                          'trace_sha256': digest.hexdigest()})
                save()
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                print(f'Completed {level}, Irelia seat {seat}: {len(played)} plays, {len(countered)} counters, {len(no_effect)} no-ops', flush=True)
        # Keep zero-use records; completion and exercised rules are separate facts.
        assert sum(row['observed_counters'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
