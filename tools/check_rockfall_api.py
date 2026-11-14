#!/usr/bin/env python3
"""Complete authored Irelia matches with Rockfall; not a tournament-strength claim."""
import argparse
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
    raw = subprocess.run([str(binary)], input='{"command":"catalog"}\n', text=True,
                         capture_output=True, check=True, timeout=30)
    catalog = json.loads(raw.stdout)['result']
    assert catalog['battlefields'][17]['official_text'] == "Units can't be played here."
    preset = catalog['presets'][0]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {
        'deck': {'name': 'Irelia / Rockfall coverage', 'legend': preset['legend_id'],
                 'chosen_champion': preset['chosen_champion'], 'cards': preset['cards'],
                 'runes': runes, 'battlefield': 17},
        'sideboard': preset['registration']['sideboard'], 'battlefields': [17, 13, 14],
    }
    report = {'status': 'running', 'scope': __doc__, 'registration': registration,
              'format_date': '2026-11-14', 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                previous, previous_game = {}, None
                field_games, arrivals = set(), set()
                digest = hashlib.sha256()

                def trace(request, response):
                    nonlocal previous, previous_game
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != seat:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    lanes = {i for i, field in enumerate(view['battlefields']) if field == 17}
                    if lanes:
                        field_games.add(game)
                    units = {u['id']: u['lane'] for p in view['players'] for u in p['units']}
                    if previous_game == game:
                        for unit_id, lane in units.items():
                            if lane in lanes and (unit_id not in previous or previous[unit_id] != lane):
                                assert unit_id in previous, 'A unit was played directly to Rockfall.'
                                arrivals.add((game, unit_id, view['turn']))
                    previous, previous_game = units, game

                registrations = [{'preset': 1}, {'preset': 1}]
                registrations[seat] = registration
                result = run_match(binary, registrations, 170100 + 10 * level_index + seat,
                                   level, seat, first_player=1-seat, trace=trace,
                                   format_date=report['format_date'])
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                assert field_games, 'Rockfall was never selected.'
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                    'result': result, 'rockfall_games': sorted(field_games),
                    'arrivals_by_movement': len(arrivals), 'trace_sha256': digest.hexdigest()})
                save()
                print(f'Completed {level}, Irelia seat {seat}: {len(arrivals)} Rockfall arrivals', flush=True)
        assert sum(row['arrivals_by_movement'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
