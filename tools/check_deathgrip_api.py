"""Complete JSON games with a legal authored Kennen substitution, not a new preset."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

binary = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2]).resolve()
process = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                           text=True, bufsize=1)
revision = 0
trace = hashlib.sha256()
rows = []
coverage = {level: [0, 0] for level in ['easy', 'tactical', 'strategic']}


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
    preset = catalog['presets'][2]
    deck = catalog['legends'][preset['legend_id']]['training_deck']
    assert deck['cards'] == preset['cards'] and [39, 3] in deck['cards']
    deck['name'] = 'Kennen / Deathgrip verification'
    deck['cards'] = [[447 if card == 39 else card, count] for card, count in deck['cards']]
    assert request('validate_deck', deck=deck)['card_count'] == 40
    for level, difficulty in enumerate(coverage):
        for opponent in range(len(catalog['presets'])):
            for seat in range(2):
                for starter in range(2):
                    trace = hashlib.sha256()
                    ids, custom = [2, opponent], [deck, None]
                    if seat:
                        ids.reverse()
                        custom.reverse()
                    seed = 447000 + level * 100 + opponent * 4 + seat * 2 + starter
                    request('reset', decks=ids, custom_decks=custom, seed=seed,
                            difficulty=difficulty, first_player=starter)
                    request('start')
                    seen = set()
                    for step in range(2400):
                        public = request('observe', player=seat)
                        if public['players'][1-seat]['hand'] is not None:
                            choice = public['decision']
                            assert choice and choice['type'] in ['reveal_hand', 'sabotage', 'recycle_revealed_hand']
                            assert choice['opponent'] == 1-seat
                        for item in public['chain']:
                            if item['player'] == seat and item['card_id'] == 447 and item['ability'] is None:
                                seen.add(item['id'])
                                assert not item['from_hidden']
                                assert len(item['targets']) == 2
                                assert item['targets'][0] != item['targets'][1]
                                assert all(t['player'] == seat for t in item['targets'])
                        if public['winner'] is not None:
                            break
                        request('step_ai', player=public['priority'])
                    else:
                        raise AssertionError((difficulty, opponent, seat, starter, 'unfinished'))
                    coverage[difficulty][seat] += len(seen)
                    rows.append({'difficulty': difficulty, 'opponent': opponent, 'seat': seat,
                                 'starter': starter, 'seed': seed, 'steps': step,
                                 'winner': public['winner'], 'scores': [p['score'] for p in public['players']],
                                 'deathgrip_plays': len(seen),
                                 'trace_sha256': trace.hexdigest()})
                    result = {'status': 'running', 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                              'games': rows, 'coverage': coverage}
                    output.write_text(json.dumps(result, indent=2) + '\n')
                    print(difficulty, opponent, seat, starter, 'plays', len(seen), flush=True)
    assert all(count > 0 for seats in coverage.values() for count in seats), coverage
    result['status'] = 'passed'
    result['scope'] = '84 complete legal authored Kennen games across every preset, seat, starter and AI level; actual Deathgrip plays required at every level and seat. This is not a tournament-strength assessment.'
    output.write_text(json.dumps(result, indent=2) + '\n')
    print('PASS', len(rows), 'games', coverage, flush=True)
finally:
    process.stdin.close()
    process.wait(timeout=5)
    process.stdout.close()
