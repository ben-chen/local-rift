#!/usr/bin/env python3
"""Run an untimed registered best-of-three through rift-api.

The coordinator is trusted to route each seat only its own observation. Replace
choose_sideboard and choose_field to experiment with external opponent policies.
The default sideboard policy keeps the list; this is an integration example,
not a claim of tournament-strength sideboarding.
"""
import argparse
import json
from pathlib import Path
import subprocess


def choose_sideboard(view):
    """Only this player's main deck/sideboard are present in this observation."""
    return view['series']['deck']['cards'], view['series']['deck']['chosen_champion']


def choose_field(view):
    return view['series']['available_battlefields'][0]


def run_match(api, registrations, seed=1, difficulty='tactical', designated_player=0,
              sideboard_policy=choose_sideboard, first_player=None, trace=None):
    process = subprocess.Popen([str(api)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               text=True, bufsize=1)
    revision = 0
    requests = 0

    def request(command, **args):
        nonlocal revision, requests
        payload = {'command': command, **args}
        if command not in ('observe', 'catalog', 'info', 'validate_registration'):
            payload['revision'] = revision
        process.stdin.write(json.dumps(payload) + '\n')
        process.stdin.flush()
        line = process.stdout.readline()
        if not line:
            raise RuntimeError('The game server stopped without a response.')
        response = json.loads(line)
        if trace:
            trace(payload, response)
        requests += 1
        if not response.get('ok'):
            raise RuntimeError(f'{command}: {response.get("error")}')
        revision = response['revision']
        return response['result']

    try:
        request('series_begin', registrations=registrations, seed=seed,
                difficulty=difficulty, designated_player=designated_player)
        for _ in range(12000):
            # Observations are requested separately; policies receive only their seat.
            views = [request('observe', player=p) for p in range(2)]
            state = views[0]['series']
            phase = state['phase']
            if phase == 'complete':
                return {'wins': state['wins'], 'winner': state['winner'],
                        'history': state['history'], 'requests': requests,
                        'seed': seed, 'difficulty': difficulty}
            if phase == 'sideboarding':
                for p in range(2):
                    if not state['sideboard_ready'][p]:
                        cards, chosen = sideboard_policy(views[p])
                        request('series_sideboard', player=p, cards=cards, chosen_champion=chosen)
            elif phase == 'battlefields':
                # Each choice uses the pre-reveal view: neither sees the other's field.
                for p in range(2):
                    if not state['battlefield_committed'][p]:
                        request('series_battlefield', player=p, battlefield=choose_field(views[p]))
            elif phase == 'play_order':
                chooser = state['designated_player']
                request('series_play_order', player=chooser,
                        first_player=chooser if first_player is None else first_player)
            elif phase == 'game':
                game = views[0]
                if game['phase'] == 'mulligan':
                    first = game['first_player']
                    for p in [first, 1-first]:
                        if not game['mulligans_used'][p]:
                            request('mulligan', player=p, indices=[])
                    request('start')
                else:
                    request('step_ai', player=game['priority'])
            else:
                raise RuntimeError(f'Unknown match phase: {phase}')
        raise RuntimeError('Match exceeded the example runner action budget.')
    finally:
        process.stdin.close()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        process.stdout.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', type=Path, default=Path('target/release/rift-api'))
    parser.add_argument('--registrations', type=Path,
                        help='JSON array of two registrations; defaults to the sourced Irelia mirror.')
    parser.add_argument('--seed', type=int, default=1)
    parser.add_argument('--difficulty', choices=['easy','tactical','strategic'], default='tactical')
    parser.add_argument('--designated-player', type=int, choices=[0,1], default=0,
                        help='Seat selected by the agreed random method, or by playoff seeding.')
    args = parser.parse_args()
    registrations = json.loads(args.registrations.read_text()) if args.registrations else [{'preset':5},{'preset':5}]
    result = run_match(args.api.resolve(), registrations, args.seed, args.difficulty, args.designated_player)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
