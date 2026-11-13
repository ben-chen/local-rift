"""Complete games through an external JSONL bot, with real allocation commands."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

binary = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2]).resolve()
process = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                           text=True, bufsize=1)
revision = 0
trace = hashlib.sha256()
counts = {}
rows = []
coverage = {level: {'casts': [0, 0], 'allocations': [0, 0]}
            for level in ['easy', 'tactical', 'strategic']}
READ_ONLY = {'catalog', 'observe', 'info', 'validate_deck', 'suggest'}

def request(command, *, expected_error=False, at_revision=None, **fields):
    global revision
    payload = {'command': command, **fields}
    if command not in READ_ONLY:
        payload['revision'] = revision if at_revision is None else at_revision
    process.stdin.write(json.dumps(payload, separators=(',', ':')) + '\n')
    process.stdin.flush()
    line = process.stdout.readline()
    assert line, 'API ended without a response'
    response = json.loads(line)
    trace.update(json.dumps([payload, response], sort_keys=True,
                            separators=(',', ':')).encode())
    counts[command] = counts.get(command, 0) + 1
    assert response['ok'] is not expected_error, (payload, response)
    if expected_error:
        assert response['revision'] == revision, (payload, response)
    else:
        revision = response['revision']
    return response['result'] if response['ok'] else response['error']

def save(status):
    output.write_text(json.dumps({
        'status': status, 'binary': str(binary),
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'games': rows, 'coverage': coverage,
        'scope': 'Authored Master Yi main replaces3Discipline with3Alpha; both Irelia presets, '
                 'both seats/starters,three levels. Manual exact allocations and read-only '
                 'previews on actual dealt hands. This does not certify Repeat kill-credit semantics.'
    }, indent=2) + '\n')

try:
    catalog = request('catalog')
    deck = copy.deepcopy(catalog['legends'][catalog['presets'][1]['legend_id']]['training_deck'])
    deck['name'] = 'Master Yi / Alpha allocation API verification'
    deck['cards'] = [[443 if card == 7 else card, count] for card, count in deck['cards']]
    assert request('validate_deck', deck=deck)['card_count'] == 40
    assert catalog['cards'][443]['play_targets']['damage_assignment']['timing'] == 'resolution'
    for level, difficulty in enumerate(['easy', 'tactical', 'strategic']):
        for opponent in [0, 5]:
            for seat in range(2):
                for starter in range(2):
                    for sample in range(12):
                        trace = hashlib.sha256()
                        counts = {}
                        previews = 0
                        allocations = 0
                        seen = set()
                        started = time.monotonic()
                        ids, custom = [1, opponent], [deck, None]
                        if seat:
                            ids.reverse()
                            custom.reverse()
                        seed = 443100 + sample * 4096 + level * 100 + opponent * 10 + seat * 2 + starter
                        request('reset', decks=ids, custom_decks=custom, seed=seed,
                                difficulty=difficulty, first_player=starter)
                        request('start')
                        for step in range(2400):
                            view = request('observe', player=seat)
                            if view['players'][1-seat]['hand'] is not None:
                                decision = view['decision']
                                assert decision and decision['type'] in [
                                    'reveal_hand', 'sabotage', 'recycle_revealed_hand']
                                assert decision['opponent'] == 1-seat
                            for item in view['chain']:
                                if item['card_id'] == 443 and item['ability'] is None:
                                    seen.add(item['id'])
                            if view['winner'] is not None:
                                break
                            actor = view['priority']
                            decision = view['decision']
                            if decision and decision['type'] == 'split_damage':
                                assert actor == seat and decision['player'] == seat
                                other = request('observe', player=1-seat)
                                assert decision == other['decision']
                                total = int(decision['total_exact'])
                                keep = decision['required_recipients']
                                assert not decision['lethal_first']
                                assert decision['minimum_per_recipient'] == 1
                                assert 1 <= keep <= min(total, len(decision['targets']))
                                # Use a valid player-authored split instead of stepping the AI.
                                # Retain the last eligible copies and put excess on the first.
                                targets = decision['targets'][-keep:]
                                amounts = [total-keep+1] + [1]*(keep-1)
                                action = {'type': 'assign_damage', 'assignments': [
                                    [target['unit_id'], str(amount)]
                                    for target, amount in zip(targets, amounts)]}
                                current = revision
                                request('validate', player=actor, action=action)
                                request('apply', player=1-actor, action=action, expected_error=True)
                                request('apply', player=actor, action=action, at_revision=current-1,
                                        expected_error=True)
                                request('apply', player=actor,
                                        action={'type':'assign_damage','assignments':[]}, expected_error=True)
                                assert revision == current
                                assert request('observe', player=seat) == view
                                assert request('observe', player=1-seat) == other
                                request('apply', player=actor, action=action)
                                assert revision == current+1
                                allocations += 1
                                continue
                            if (actor == seat and decision is None and not view['chain']
                                    and 443 in view['players'][seat]['hand'] and previews < 3):
                                current = revision
                                action = {'type': 'play_card_targets',
                                          'hand_index':view['players'][seat]['hand'].index(443),
                                          'lane':None, 'targets':[]}
                                first = request('target_candidates', player=seat, execution=0, action=action)
                                if first['candidates']:
                                    source = first['candidates'][0]
                                    assert source['player'] == seat
                                    action['targets'] = [source]
                                    second = request('target_candidates', player=seat, execution=0, action=action)
                                    assert all(target['player'] == 1-seat for target in second['candidates'])
                                    if second['candidates']:
                                        action['targets'].append(second['candidates'][0])
                                        request('target_candidates', player=seat, execution=0, action=action)
                                        previews += 1
                                assert revision == current
                                assert request('observe', player=seat) == view
                            request('step_ai', player=actor)
                        else:
                            raise AssertionError((difficulty, opponent, seat, starter, sample, 'unfinished game'))
                        coverage[difficulty]['casts'][seat] += len(seen)
                        coverage[difficulty]['allocations'][seat] += allocations
                        rows.append({'difficulty':difficulty, 'opponent':opponent, 'seat':seat,
                                     'starter':starter, 'sample':sample, 'seed':seed, 'actions':step,
                                     'winner':view['winner'], 'scores':[p['score'] for p in view['players']],
                                     'casts':len(seen), 'allocations':allocations,
                                     'target_previews':previews, 'requests':counts,
                                     'trace_sha256':trace.hexdigest(),
                                     'seconds':round(time.monotonic()-started, 2)})
                        save('running')
                        print(len(rows), difficulty, opponent, seat, starter, sample,
                              'casts',len(seen),'splits',allocations,'previews',previews, flush=True)
    assert all(count > 0 for level in coverage.values() for metric in level.values() for count in metric), coverage
    assert all(any(row['target_previews'] for row in rows if row['seat'] == seat) for seat in range(2))
    save('passed')
    print('PASS',len(rows),'games',coverage,flush=True)
finally:
    process.stdin.close()
    process.wait(timeout=5)
    process.stdout.close()
