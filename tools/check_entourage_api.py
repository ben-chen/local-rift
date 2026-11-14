#!/usr/bin/env python3
"""Complete Irelia matches exercising legend targeting and late readiness choices.

The authored Royal Entourage list tests coverage, not tournament strength.
"""
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
    assert catalog['cards'][455]['official_id'] == 'sfd-039-221'
    preset = catalog['presets'][0]
    assert [row for row in preset['cards'] if row[0] == 15] == [[15, 3]]
    cards = [row for row in preset['cards'] if row[0] != 15] + [[455, 3]]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {
        'deck': {'name': 'Irelia / Royal Entourage coverage', 'legend': preset['legend_id'],
                 'chosen_champion': preset['chosen_champion'], 'cards': cards, 'runes': runes,
                 'battlefield': preset['registration']['battlefields'][0]},
        'sideboard': preset['registration']['sideboard'],
        'battlefields': preset['registration']['battlefields'],
    }
    report = {'status': 'running', 'scope': __doc__.strip(), 'registration': registration,
              'format_date': '2026-11-14', 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat in range(2):
                targeted, resolved, owners = set(), set(), set()
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
                    for item in view['chain']:
                        if item['card_id'] == 455 and item['player'] == seat:
                            target, = item['targets']
                            assert target['type'] == 'legend'
                            assert target['legend_id'] == view['players'][target['player']]['legend_id']
                            targeted.add((game, item['id']))
                            owners.add(target['player'])
                    choice = view.get('decision') or {}
                    if choice.get('type') == 'legend_readiness':
                        assert choice['timing'] == 'resolution'
                        assert not choice['optional']
                        assert (game, choice['choice_id']) in targeted
                        top = view['chain'][-1]
                        assert top['id'] == choice['choice_id']
                        assert top['targets'] == [choice['target']]
                        assert len(choice['options']) == 2
                    if previous and len(previous['series']['history']) == game:
                        old = previous.get('decision') or {}
                        if old.get('type') == 'legend_readiness' and choice != old:
                            assert old['choice_id'] not in {item['id'] for item in view['chain']}
                            resolved.add((game, old['choice_id']))
                    previous = view

                registrations = [{'preset': 1}, {'preset': 1}]
                registrations[seat] = registration
                result = run_match(binary, registrations, 455100 + 10 * level_index + seat,
                                   level, seat, first_player=1-seat, trace=trace, format_date=report['format_date'])
                report['records'].append({'difficulty': level, 'irelia_seat': seat,
                    'first_player': 1-seat, 'result': result, 'finalized_targets': len(targeted),
                    'resolved_choices': len(resolved), 'target_owners': sorted(owners),
                    'trace_sha256': digest.hexdigest()})
                save()
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                print(f'Completed {level}, Irelia seat {seat}: {len(targeted)} targets, {len(resolved)} resolutions', flush=True)
        assert sum(row['resolved_choices'] for row in report['records']) > 0
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
