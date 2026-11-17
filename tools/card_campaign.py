"""Shared runner for authored card integrations through the real JSONL API.

Card suites supply identities, deck substitutions and sampling requirements.
The runner preserves per-series results and complete trace digests. On failure,
it also retains the active (or last completed) series as a JSONL transcript.
"""
import argparse
import copy
from dataclasses import dataclass, field
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match


LEVELS = ('easy', 'tactical', 'strategic')


@dataclass(frozen=True)
class Campaign:
    scope: str
    deck_name: str
    cards: tuple[tuple[int, str], ...]
    swaps: tuple[tuple[int, int], ...]
    base_seed: int
    samples: tuple[int, int, int] = (2, 2, 2)
    format_date: str = '2026-11-17'
    target_counts: dict[int, int] = field(default_factory=dict)
    cast_exemptions: dict[tuple[str, int], str] = field(default_factory=dict)

    def validate(self):
        ids = [card for card, _ in self.cards]
        require(ids and len(ids) == len(set(ids)), 'Card identities must be nonempty and unique.')
        require(len(self.samples) == len(LEVELS) and all(type(n) is int and n > 0 for n in self.samples),
                'Every difficulty must have a positive sample count.')
        require(set(self.target_counts) <= set(ids), 'Target checks must name a campaign card.')
        require(all(type(n) is int and n >= 0 for n in self.target_counts.values()),
                'Target counts must be nonnegative integers.')
        require(len({old for old, _ in self.swaps}) == len(self.swaps), 'Deck substitutions must be unique.')
        require(sorted(new for _, new in self.swaps) == sorted(ids), 'Every campaign card must enter the deck once.')
        for (level, card), reason in self.cast_exemptions.items():
            require(level in LEVELS and card in ids and isinstance(reason, str) and reason.strip(),
                    'A cast exemption needs a known difficulty, card and explicit reason.')


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def registration(spec, catalog):
    spec.validate()
    cards = [card for card, _ in spec.cards]
    require([catalog['cards'][card]['official_id'] for card in cards] == [ident for _, ident in spec.cards],
            'Runtime card identities differ from the reviewed campaign.')
    definition = catalog['legends'][0]
    require(all(card in definition['eligible_cards'] for card in cards), 'Campaign cards are not eligible for Irelia.')
    deck = copy.deepcopy(definition['training_deck'])
    for old, new in spec.swaps:
        rows = [row for row in deck['cards'] if row[0] == old]
        require(len(rows) == 1 and rows[0][1] == 2, f'Expected exactly two copies of source card {old}.')
        rows[0][0] = new
    deck['name'] = spec.deck_name
    return {'deck': deck, 'sideboard': [], 'battlefields': [13, 0, 14]}


def run_campaign(spec, binary, output, runner):
    spec.validate()
    binary, output, runner = Path(binary).resolve(), Path(output).resolve(), Path(runner).resolve()
    active_trace = output.with_suffix('.active-trace.jsonl')
    failed_trace = output.with_suffix('.failed-trace.jsonl')
    if active_trace.exists() or failed_trace.exists():
        raise FileExistsError('Prior trace evidence exists; use a fresh output path.')
    report = {'status': 'running', 'scope': spec.scope, 'format_date': spec.format_date,
              'records': [], 'binary_sha256': sha(binary), 'runner_sha256': sha(runner),
              'campaign_runner_sha256': sha(Path(__file__)),
              'series_runner_sha256': sha(Path(__file__).with_name('series_bot.py'))}
    if spec.cast_exemptions:
        report['coverage_limits'] = [f'{level} / {card}: {reason}'
                                     for (level, card), reason in spec.cast_exemptions.items()]
    # Refuse to overwrite an earlier result, including failed or partial evidence.
    with output.open('x') as stream:
        json.dump(report, stream, indent=2)

    def save():
        output.write_text(json.dumps(report, indent=2) + '\n')

    try:
        response = subprocess.run([str(binary)], input='{"command":"catalog"}\n', text=True,
                                  capture_output=True, check=True, timeout=30)
        deck = registration(spec, json.loads(response.stdout)['result'])
        report['registration'] = deck
        cards = [card for card, _ in spec.cards]
        for level_index, level in enumerate(LEVELS):
            for first in range(2):
                for sample in range(spec.samples[level_index]):
                    seed = spec.base_seed + 100 * level_index + 10 * first + sample
                    report['active_series'] = {'difficulty': level, 'first_player': first,
                                               'sample': sample, 'seed': seed}
                    save()
                    casts = [[set() for _ in cards] for _ in range(2)]
                    digest = hashlib.sha256()
                    with active_trace.open('w') as transcript:
                        def trace(request, response):
                            encoded = json.dumps([request, response], sort_keys=True, separators=(',', ':'))
                            transcript.write(encoded + '\n')
                            digest.update(encoded.encode())
                            if request['command'] != 'observe':
                                return
                            view = response.get('result', {})
                            if view.get('series', {}).get('phase') != 'game':
                                return
                            game = len(view['series']['history'])
                            for item in view['chain']:
                                card = item['card_id']
                                if card in cards and item.get('ability') is None:
                                    if card in spec.target_counts:
                                        require(len(item['targets']) == spec.target_counts[card],
                                                f'Card {card} has an unexpected target count.')
                                    casts[item['player']][cards.index(card)].add((game, item['id']))

                        result = run_match(binary, [deck, deck], seed, level, first,
                                           first_player=first, trace=trace, format_date=spec.format_date)
                    require(max(result['wins']) == 2 and len(result['history']) >= 2,
                            'Registered match did not complete a best-of-three series.')
                    row = {'difficulty': level, 'first_player': first, 'sample': sample, 'result': result,
                           'casts': [[len(values) for values in seat] for seat in casts],
                           'trace_sha256': digest.hexdigest()}
                    report['records'].append(row)
                    save()
                    print(f'Completed {level}, start {first}: casts={row["casts"]}', flush=True)
        for level in LEVELS:
            for seat in range(2):
                for index, card in enumerate(cards):
                    if (level, card) not in spec.cast_exemptions:
                        require(sum(row['casts'][seat][index] for row in report['records']
                                    if row['difficulty'] == level) > 0,
                                f'No cast observed: {level}, seat {seat}, card {card}.')
        require(sha(binary) == report['binary_sha256'] and sha(runner) == report['runner_sha256']
                and sha(Path(__file__)) == report['campaign_runner_sha256']
                and sha(Path(__file__).with_name('series_bot.py')) == report['series_runner_sha256'],
                'Binary or campaign code changed during verification.')
        report['status'] = 'passed'
        report.pop('active_series')
        active_trace.unlink()
    except BaseException as error:
        report.update(status='failed', error=f'{type(error).__name__}: {error}')
        if active_trace.exists():
            active_trace.replace(failed_trace)
            report['failed_trace'] = {'path': str(failed_trace), 'sha256': sha(failed_trace),
                                      'scope': 'Active or last completed series; completed records remain above.'}
        raise
    finally:
        save()
    return report


def campaign_main(spec, runner):
    parser = argparse.ArgumentParser(description=spec.scope)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    run_campaign(spec, args.binary, args.output, runner)
