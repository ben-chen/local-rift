#!/usr/bin/env python3
"""Paired, reproducible match-policy comparison; not a real metagame ranking.

The Irelia seat alternates keep/adaptive with identical shuffle seeds, seat and
in-game difficulty. Rivals keep their lists. Non-Arito opponents use sourced
main decks with explicitly authored empty sideboards and field registrations.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

from series_bot import run_match


def registration(catalog, preset):
    if preset == 5:
        return {'preset': 5}
    p = catalog['presets'][preset]
    runes = [0] * 6
    for domain, count in zip(p['domains'], p['rune_counts']):
        runes[catalog['domains'].index(domain)] = count
    return {'deck': {'name': f'Benchmark {p["name"]}', 'legend': p['legend_id'],
                     'chosen_champion': p['chosen_champion'], 'cards': p['cards'],
                     'runes': runes, 'battlefield': 0},
            'sideboard': [], 'battlefields': [0, 13, 14]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--seeds', type=int, default=2)
    parser.add_argument('--seed-base', type=int, default=830000)
    parser.add_argument('--difficulty', choices=['easy', 'tactical', 'strategic'], default='strategic')
    args = parser.parse_args()
    if args.seeds < 1:
        parser.error('--seeds must be positive')
    api = args.api.resolve()
    response = subprocess.run([str(api)], input='{"command":"catalog"}\n', text=True,
                              capture_output=True, check=True, timeout=30)
    catalog = json.loads(response.stdout)['result']
    rows = []
    proof = {'binary_sha256': hashlib.sha256(api.read_bytes()).hexdigest(),
             'difficulty': args.difficulty, 'seed_base': args.seed_base,
             'scope': __doc__, 'results': rows}
    for opponent in [1, 2, 4, 5]:
        for seat in [0, 1]:
            for offset in range(args.seeds):
                seed = args.seed_base + opponent * 100 + offset
                regs = [registration(catalog, opponent)] * 2
                regs[seat] = {'preset': 5}
                for policy in ['keep', 'adaptive']:
                    lists = {}

                    def trace(request, response):
                        if request['command'] == 'observe' and request['player'] == seat:
                            view = response['result']['series']
                            if view['phase'] == 'game':
                                lists[view['game_number']] = view['deck']['cards']

                    policies = ['keep', 'keep']
                    policies[seat] = policy
                    start = time.monotonic()
                    result = run_match(api, regs, seed, args.difficulty,
                                       designated_player=offset % 2, trace=trace,
                                       setup_policies=policies)
                    changes = 0
                    ordered = sorted(lists.items())
                    for (_, before), (_, after) in zip(ordered, ordered[1:]):
                        old = dict(before)
                        changes += sum(max(0, n-old.get(card, 0)) for card, n in after)
                    row = {'opponent_preset': opponent, 'seat': seat, 'seed': seed,
                           'policy': policy, 'won': result['winner'] == seat,
                           'swapped_copies': changes, 'main_by_game': lists,
                           'seconds': round(time.monotonic()-start, 3), **result}
                    rows.append(row)
                    args.output.parent.mkdir(parents=True, exist_ok=True)
                    args.output.write_text(json.dumps(proof, indent=2)+'\n')
                    print(json.dumps({k: v for k, v in row.items() if k not in ('main_by_game', 'history')}), flush=True)
    proof['summary'] = {policy: {'matches': sum(r['policy'] == policy for r in rows),
                                'wins': sum(r['policy'] == policy and r['won'] for r in rows),
                                'games': sum(len(r['history']) for r in rows if r['policy'] == policy),
                                'swapped_copies': sum(r['swapped_copies'] for r in rows if r['policy'] == policy)}
                        for policy in ['keep', 'adaptive']}
    args.output.write_text(json.dumps(proof, indent=2)+'\n')
    print(json.dumps(proof['summary']), flush=True)


if __name__ == '__main__':
    main()
