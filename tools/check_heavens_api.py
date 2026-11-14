#!/usr/bin/env python3
"""Complete Irelia practice series with Consult; authored coverage, not a meta list."""
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
    assert catalog['cards'][457]['official_id'] == 'rad-025-167'
    preset = catalog['presets'][0]
    assert [row for row in preset['cards'] if row[0] == 8] == [[8, 2]]
    cards = [row for row in preset['cards'] if row[0] != 8] + [[457, 2]]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {
        'deck': {'name': 'Irelia / Consult coverage', 'legend': preset['legend_id'],
                 'chosen_champion': preset['chosen_champion'], 'cards': cards, 'runes': runes,
                 'battlefield': preset['registration']['battlefields'][0]},
        'sideboard': preset['registration']['sideboard'],
        'battlefields': preset['registration']['battlefields'],
    }
    report = {'status': 'running', 'scope': __doc__, 'registration': registration,
              'format_date': '2026-11-14', 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                played, grants, predictions = set(), set(), set()
                digest = hashlib.sha256()

                def trace(request, response):
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != seat:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    sources = [item for item in view['chain'] if item['card_id'] == 457 and item['ability'] is None]
                    for item in sources:
                        played.add((game, item['id']))
                    if (view.get('decision') or {}).get('type') == 'predict' and sources:
                        predictions.add((game, sources[-1]['id']))
                    for player in view['players']:
                        for unit in player['units']:
                            grant = unit.get('turn_shield')
                            if grant:
                                assert grant['amount'] > 0 and grant['amount'] % 2 == 0
                                assert grant['amount_exact'] == str(grant['amount'])
                                assert grant['expires'] == 'end_of_turn'
                                assert unit['keywords']['shield'] >= grant['amount']
                                grants.add((game, view['turn'], unit['id'], grant['amount']))

                registrations = [{'preset': 1}, {'preset': 1}]
                registrations[seat] = registration
                result = run_match(binary, registrations, 457100 + 10 * level_index + seat,
                                   level, seat, first_player=1-seat, trace=trace, format_date=report['format_date'])
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                                          'first_player': 1-seat, 'result': result,
                                          'observed_plays': len(played), 'observed_grants': len(grants),
                                          'observed_predict_sources': len(predictions),
                                          'trace_sha256': digest.hexdigest()})
                save()
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                print(f'Completed {level}, Irelia seat {seat}: {len(played)} plays, {len(grants)} grants', flush=True)
        assert sum(row['observed_grants'] for row in report['records']) > 0
        assert sum(row['observed_predict_sources'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
