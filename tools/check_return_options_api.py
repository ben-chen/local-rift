#!/usr/bin/env python3
"""Return options through registered Irelia matches at all three AI levels."""
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
    cards = [482, 483]
    assert [catalog['cards'][i]['official_id'] for i in cards] == ['ogn-170-298', 'unl-134-219']
    definition = catalog['legends'][0]
    assert all(card in definition['eligible_cards'] for card in cards)
    deck = copy.deepcopy(definition['training_deck'])
    for old, new in [(5, 482), (7, 483)]:
        row = next(row for row in deck['cards'] if row[0] == old)
        assert row[1] == 3
        row[0] = new
    deck['name'] = 'Irelia / return options integration'
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
            for first in range(2):
                for sample in range(8):
                    casts = [[set() for _ in cards] for _ in range(2)]
                    digest = hashlib.sha256()

                    def trace(request, response):
                        digest.update(json.dumps([request, response], sort_keys=True, separators=(',', ':')).encode())
                        if request['command'] != 'observe':
                            return
                        view = response.get('result', {})
                        if view.get('series', {}).get('phase') != 'game':
                            return
                        game = len(view['series']['history'])
                        for item in view['chain']:
                            if item['card_id'] in cards and item.get('ability') is None:
                                assert len(item['targets']) == 1
                                casts[item['player']][item['card_id'] - 482].add((game, item['id']))

                    result = run_match(binary, [registration, registration], 482200 + 100 * level_index + 10 * first + sample,
                                       level, first, first_player=first, trace=trace,
                                       format_date=report['format_date'])
                    assert max(result['wins']) == 2 and len(result['history']) >= 2
                    row = {'difficulty': level, 'first_player': first, 'sample': sample, 'result': result,
                           'casts': [[len(values) for values in seat] for seat in casts],
                           'trace_sha256': digest.hexdigest()}
                    report['records'].append(row)
                    save()
                    print(f'Completed {level}, start {first}: casts={row["casts"]}', flush=True)
        for level in ['easy', 'tactical', 'strategic']:
            for seat in range(2):
                for card in range(2):
                    assert sum(row['casts'][seat][card] for row in report['records'] if row['difficulty'] == level) > 0, (level, seat, card)
        report['status'] = 'passed'
    except Exception as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
