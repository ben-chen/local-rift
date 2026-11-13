"""Real JSON protocol games with authored Irelia substitutions and live gear checks."""
import hashlib, json, subprocess, sys, time
from pathlib import Path

binary = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2]).resolve()
process = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)
revision = 0
rows = []
coverage = {str(card): {level: [0, 0] for level in ['easy', 'tactical', 'strategic']} for card in [444, 445]}
trace = hashlib.sha256()

def request(command, **fields):
    global revision
    payload = {'command': command, **fields}
    if command not in ['catalog', 'observe', 'info', 'validate_deck']:
        payload['revision'] = revision
    process.stdin.write(json.dumps(payload, separators=(',', ':')) + '\n')
    process.stdin.flush()
    line = process.stdout.readline()
    assert line, 'API ended without responding'
    response = json.loads(line)
    trace.update(json.dumps([payload, response], sort_keys=True, separators=(',', ':')).encode())
    assert response['ok'], (payload, response)
    revision = response['revision']
    return response['result']

try:
    catalog = request('catalog')
    deck = catalog['legends'][catalog['presets'][0]['legend_id']]['training_deck']
    deck['name'] = 'Irelia / conditional equipment verification'
    assert [3, 2] in deck['cards'] and [21, 2] in deck['cards']
    deck['cards'] = [[{3: 444, 21: 445}.get(card, card), count] for card, count in deck['cards']]
    assert request('validate_deck', deck=deck)['card_count'] == 40
    for level, difficulty in enumerate(['easy', 'tactical', 'strategic']):
        for opponent in [0, 5]:
            for seat in range(2):
                for starter in range(2):
                    for sample in range(6):
                        trace = hashlib.sha256()
                        started = time.monotonic()
                        checked = 0
                        seen = set()
                        ids, custom = [0, opponent], [deck, None]
                        if seat:
                            ids.reverse(); custom.reverse()
                        seed = 444445 + sample * 4096 + level * 100 + opponent * 10 + seat * 2 + starter
                        request('reset', decks=ids, custom_decks=custom, seed=seed, difficulty=difficulty, first_player=starter)
                        request('start')
                        for step in range(2400):
                            public = request('observe', player=seat)
                            if public['players'][1-seat]['hand'] is not None:
                                decision = public['decision']
                                assert decision and decision['type'] in ['reveal_hand', 'sabotage', 'recycle_revealed_hand'] and decision['opponent'] == 1-seat
                            player = public['players'][seat]
                            equipment = [g for g in player['gear'] if g['card_id'] in [444, 445]]
                            if equipment:
                                before_revision = revision
                                other_view = request('observe', player=1-seat)
                                assert other_view['players'][seat]['gear'] == player['gear']
                                assert revision == before_revision
                            for gear in equipment:
                                assert isinstance(gear['attached_this_turn'], bool)
                                if gear['attached_to'] is None:
                                    assert not gear['attached_this_turn']
                                    continue
                                seen.add(gear['card_id'])
                                unit = next(u for u in player['units'] if u['id'] == gear['attached_to'])
                                contribution = next(c for c in unit['passive_contributions'] if c['source'].get('gear_id') == gear['id'])
                                expected = gear['attached_this_turn'] if gear['card_id'] == 444 else (
                                    unit['lane'] is not None and sum(u['lane'] == unit['lane'] for u in player['units']) == 2)
                                assert contribution['active'] == expected, (gear, unit, contribution)
                                assert contribution['computed_might_exact'] == ('2' if expected else '0')
                                checked += 1
                                if expected:
                                    coverage[str(gear['card_id'])][difficulty][seat] += 1
                            if public['winner'] is not None:
                                break
                            request('step_ai', player=public['priority'])
                        else:
                            raise AssertionError((difficulty, opponent, seat, starter, sample, 'unfinished'))
                        row = {'difficulty': difficulty, 'opponent': opponent, 'seat': seat, 'starter': starter,
                            'sample': sample, 'seed': seed, 'steps': step, 'winner': public['winner'],
                            'scores': [p['score'] for p in public['players']], 'attached_cards': sorted(seen),
                            'live_bonus_checks': checked, 'trace_sha256': trace.hexdigest(), 'seconds': round(time.monotonic()-started, 2)}
                        rows.append(row)
                        result = {'status': 'running', 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                            'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'games': rows, 'coverage': coverage}
                        output.write_text(json.dumps(result, indent=2)+'\n')
                        print(difficulty, opponent, seat, starter, sample, 'checks', checked, flush=True)
    assert all(count > 0 for by_level in coverage.values() for by_seat in by_level.values() for count in by_seat)
    result['status'] = 'passed'
    result['scope'] = '144 complete authored Irelia games, all levels/opponents/seats/starters; both gear conditions exercised in every level/seat. Public exact-copy attachment observations and private opponent hands checked.'
    output.write_text(json.dumps(result, indent=2)+'\n')
    print('PASS', len(rows), 'games', coverage, flush=True)
finally:
    process.stdin.close()
    process.wait(timeout=5)
    process.stdout.close()
