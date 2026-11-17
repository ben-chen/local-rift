"""Six unassisted legal Irelia recovery series through the ordinary JSON API."""
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
    response = subprocess.run([str(binary)], input='{"command":"catalog"}\n', text=True,
                              capture_output=True, check=True, timeout=30)
    catalog = json.loads(response.stdout)['result']
    cards = [473, 474, 475]
    assert [catalog['cards'][i]['official_id'] for i in cards] == ['ogn-174-298', 'ogn-165-298', 'ogs-010-024']
    definition = catalog['legends'][0]
    assert all(card in definition['eligible_cards'] for card in cards)
    deck = copy.deepcopy(definition['training_deck'])
    for old, new in zip([5, 6, 17], cards):
        row = next(row for row in deck['cards'] if row[0] == old)
        assert row[1] == 3
        row[0] = new
    deck['name'] = 'Irelia / recovery integration'
    registration = {'deck': deck, 'sideboard': [], 'battlefields': [13, 0, 14]}
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
                choices = {card: set() for card in [474, 475]}
                digest = hashlib.sha256()

                def trace(request, response):
                    digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                    if request['command'] != 'observe' or request['player'] != seat:
                        return
                    view = response.get('result', {})
                    if view.get('series', {}).get('phase') != 'game':
                        return
                    game = len(view['series']['history'])
                    for unit in view['players'][seat]['units']:
                        if not unit['token'] and unit['card_id'] in entries:
                            entries[unit['card_id']].add((game, unit['id']))
                    for item in view['chain']:
                        if item['player'] == seat and (item.get('ability') or {}).get('type') == 'return_chosen':
                            choices[item['card_id']].add((game, item['id']))

                seats = [{'preset': 0}, {'preset': 0}]
                seats[seat] = registration
                result = run_match(binary, seats, 475200 + 100 * level_index + seat, level, seat,
                                   first_player=1-seat, trace=trace, format_date=report['format_date'])
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                row = {'difficulty': level, 'irelia_seat': seat, 'first_player': 1-seat, 'result': result,
                       'card_entries': {str(c): len(entries[c]) for c in cards},
                       'recovery_targets': {str(c): len(choices[c]) for c in choices},
                       'trace_sha256': digest.hexdigest()}
                report['records'].append(row)
                save()
                print(f'Completed {level}, seat {seat}: {row["card_entries"]}; recoveries {row["recovery_targets"]}', flush=True)
        for seat in range(2):
            for card in cards:
                assert sum(row['card_entries'][str(card)] for row in report['records'] if row['irelia_seat'] == seat) > 0, (seat, card)
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
