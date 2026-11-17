"""Twenty-four legal Irelia recovery series: four deck families, three AIs, both seats."""
import argparse
import copy
import hashlib
import itertools
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
    cards = [476, 477, 478, 479]
    assert [catalog['cards'][i]['official_id'] for i in cards] == ['sfd-035-221', 'ogn-196-298', 'ogn-198-298', 'ven-103-166']
    definition = catalog['legends'][0]
    assert all(card in definition['eligible_cards'] for card in cards)
    # Exercise the two recovery families separately, together, and with generated
    # Power. Keep all source presets intact and retain one fixed seed per AI/seat.
    families = {
        'units': [(5, 476), (6, 477)],
        'spells': [(7, 478), (8, 479)],
        'combined': [(5, 476), (6, 477), (7, 478), (8, 479)],
        'generated_power': [(5, 476), (6, 477), (7, 478), (8, 479), (15, 42)],
    }
    registrations = {}
    for family, replacements in families.items():
        deck = copy.deepcopy(definition['training_deck'])
        for old, new in replacements:
            row = next(row for row in deck['cards'] if row[0] == old)
            assert row[1] == (2 if old == 8 else 3)
            row[0] = new
        deck['name'] = f'Irelia / trash recovery / {family}'
        registrations[family] = {'deck': deck, 'sideboard': [], 'battlefields': [13, 0, 14]}
    report = {'status': 'running', 'scope': __doc__, 'format_date': '2026-11-16',
              'registrations': registrations, 'records': [],
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for level_index, level in enumerate(['easy', 'tactical', 'strategic']):
            for seat, family in itertools.product(range(2), families):
                entries = {card: set() for card in cards}
                choices = {card: set() for card in cards}
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
                        if item['player'] == seat and item['card_id'] in [478, 479] and item.get('ability') is None:
                            entries[item['card_id']].add((game, item['id']))
                            if item['targets']:
                                choices[item['card_id']].add((game, item['id']))
                        if item['player'] == seat and item['card_id'] in choices and (item.get('ability') or {}).get('type') in ['return_chosen', 'play_chosen']:
                            choices[item['card_id']].add((game, item['id']))

                seats = [{'preset': 0}, {'preset': 0}]
                seats[seat] = registrations[family]
                result = run_match(binary, seats, 477200 + 100 * level_index + seat, level, seat,
                                   first_player=1-seat, trace=trace, format_date=report['format_date'])
                assert max(result['wins']) == 2 and len(result['history']) >= 2
                row = {'difficulty': level, 'irelia_seat': seat, 'family': family,
                       'first_player': 1-seat, 'result': result,
                       'card_entries': {str(c): len(entries[c]) for c in cards},
                       'trash_targets': {str(c): len(choices[c]) for c in choices},
                       'trace_sha256': digest.hexdigest()}
                report['records'].append(row)
                save()
                print(f'Completed {level}, seat {seat}, {family}: {row["card_entries"]}; trash targets {row["trash_targets"]}', flush=True)
        for seat in range(2):
            for card in cards:
                assert sum(row['card_entries'][str(card)] for row in report['records'] if row['irelia_seat'] == seat) > 0, (seat, card)
        for card in cards:
            assert sum(row['trash_targets'][str(card)] for row in report['records']) > 0, card
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
