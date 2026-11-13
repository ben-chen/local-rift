#!/usr/bin/env python3
"""Play complete sourced Master Yi matches through the external JSON protocol."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys

from series_bot import run_match


def pool(rows):
    result = Counter()
    for card, count in rows:
        result[card] += count
    return result


def manual_sideboard(view):
    state = view['series']
    deck = state['deck']
    if deck['legend'] != 1:
        return deck['cards'], deck['chosen_champion']
    cards = [row for row in state['registration']['deck']['cards'] if row[0] != 7]
    cards += [[441, 1], [442, 1], [443, 1]]
    return cards, deck['chosen_champion']


def main():
    binary = Path(sys.argv[1]).resolve()
    output = Path(sys.argv[2]).resolve()
    rows = []
    report = {
        'status': 'running',
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'scope': 'Both sourced Irelia opponents, both Master Yi seats and opening choosers, '
                 'all three levels; manual legal sideboard exchanges and adaptive setup. '
                 'This verifies match flow, not tournament playing strength.',
        'matches': rows,
    }

    def save():
        output.write_text(json.dumps(report, indent=2) + '\n')

    try:
        for policy in ['manual', 'adaptive']:
            for level, difficulty in enumerate(['easy', 'tactical', 'strategic']):
                for opponent in [0, 5]:
                    for yi in range(2):
                        for chooser in range(2):
                            presets = [1, opponent] if yi == 0 else [opponent, 1]
                            registrations = [{'preset': p} for p in presets]
                            originals = {}
                            seen = set()
                            played = Counter()
                            phases = set()
                            digest = hashlib.sha256()
                            snapshots = 0

                            def inspect(request, response):
                                nonlocal snapshots
                                digest.update(json.dumps([request, response], sort_keys=True,
                                                         separators=(',', ':')).encode())
                                if request['command'] != 'observe' or not response['ok']:
                                    return
                                view = response['result']
                                seat = request['player']
                                state = view['series']
                                phases.add(state['phase'])
                                original = originals.setdefault(seat, state['registration'])
                                assert state['registration'] == original
                                assert pool(state['deck']['cards'] + state['sideboard']) == pool(
                                    original['deck']['cards'] + original['sideboard'])
                                assert sum(n for _, n in state['deck']['cards']) == 40
                                assert not {'deck', 'hand', 'registration', 'sideboard'} & set(
                                    state['players'][1 - seat])
                                if state['phase'] == 'battlefields':
                                    assert state['players'][1 - seat]['selected_battlefield'] is None
                                if state['phase'] == 'game':
                                    for item in view['chain']:
                                        key = (state['game_number'], item['id'])
                                        if (item['player'] == yi and item['card_id'] in [441, 442, 443]
                                                and item['ability'] is None and key not in seen):
                                            seen.add(key)
                                            played[item['card_id']] += 1
                                snapshots += 1

                            seed = 443950 + (policy == 'adaptive') * 1000 + level * 100 + opponent * 10 + yi * 2 + chooser
                            result = run_match(binary, registrations, seed, difficulty, chooser,
                                               sideboard_policy=manual_sideboard, trace=inspect,
                                               setup_policies=['adaptive', 'adaptive'] if policy == 'adaptive' else None)
                            assert result['wins'][result['winner']] == 2
                            assert {'sideboarding', 'battlefields', 'play_order', 'game', 'complete'} <= phases
                            rows.append({**result, 'policy': policy, 'presets': presets,
                                         'yi_seat': yi, 'designated_player': chooser,
                                         'new_card_plays': dict(played), 'snapshots_checked': snapshots,
                                         'trace_sha256': digest.hexdigest()})
                            save()
                            print(policy, difficulty, presets, chooser, len(result['history']), dict(played), flush=True)
        assert len(rows) == 48
        assert all(any(row['new_card_plays'].get(card, 0) for row in rows) for card in [441, 442, 443])
        report['status'] = 'passed'
    except BaseException:
        report['status'] = 'failed'
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
