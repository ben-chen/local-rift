#!/usr/bin/env python3
"""A reproducible legal-action bot against Local Rift's built-in AI.

Build rift-api first. No third-party Python packages are required. This script
is the trusted coordinator; only choose_action is the custom player policy.
"""
import argparse
import json
from pathlib import Path
import random
import subprocess


def choose_action(view, suggestions, rng):
    """Replace this function with a policy receiving ONLY its own player view.

    Suggestions are not exhaustive. Return any structured action supported by
    docs/API.md; the server validates it before committing. This starter bot
    samples the legal convenience suggestions, including end-turn actions.
    """
    del view  # A stronger policy can inspect its hand, board, and pending choice.
    if not suggestions:
        raise RuntimeError("No suggested action is available for the acting seat")
    return rng.choice(suggestions)["action"]


class Match:
    def __init__(self, binary):
        self.process = subprocess.Popen(
            [str(Path(binary).resolve())],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            bufsize=1,
        )
        self.revision = 0

    def request(self, command, **fields):
        if command in {"reset", "mulligan", "start", "apply", "validate", "quote_payment", "step_ai"}:
            fields["revision"] = self.revision
        request = {"command": command, **fields}
        self.process.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError("rift-api closed before returning a response")
        response = json.loads(line)
        self.revision = response["revision"]
        if not response["ok"]:
            raise RuntimeError(f"{response['error']['code']}: {response['error']['message']}")
        return response["result"]

    def close(self):
        self.process.stdin.close()
        self.process.wait(timeout=5)
        self.process.stdout.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/rift-api")
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--deck", type=int, choices=range(4), default=0)
    parser.add_argument("--opponent", type=int, choices=range(4), default=3)
    parser.add_argument("--custom-deck", type=Path, help="Your saved custom deck JSON file")
    parser.add_argument("--custom-opponent", type=Path, help="Opponent's saved custom deck JSON file")
    parser.add_argument("--first-player", type=int, choices=(0, 1), default=0)
    parser.add_argument("--difficulty", choices=("easy", "tactical", "strategic"), default="tactical")
    parser.add_argument("--max-actions", type=int, default=10000)
    args = parser.parse_args()
    if not 0 <= args.seed < 2**64:
        parser.error("--seed must be an unsigned 64-bit integer")
    if args.max_actions <= 0:
        parser.error("--max-actions must be positive")
    custom_decks = []
    for path in (args.custom_deck, args.custom_opponent):
        try:
            custom_decks.append(json.loads(path.read_text(encoding="utf-8")) if path else None)
        except (OSError, ValueError) as error:
            parser.error(f"Cannot read custom deck {path}: {error}")

    rng = random.Random(args.seed)
    match = Match(args.binary)
    try:
        match.request("reset", seed=args.seed, decks=[args.deck, args.opponent],
                      difficulty=args.difficulty, first_player=args.first_player,
                      custom_decks=custom_decks)
        # Both players keep their opening hands. A policy can instead inspect its
        # own opening observation and replace up to two indices once.
        match.request("mulligan", player=0, indices=[])
        match.request("mulligan", player=1, indices=[])
        match.request("start")
        for count in range(args.max_actions + 1):
            view = match.request("observe", player=0)
            if view["phase"] == "finished":
                scores = [player["score"] for player in view["players"]]
                print(json.dumps({"seed": args.seed, "winner": view["winner"],
                                  "scores": scores, "actions": count, "turns": view["turn"]}))
                return
            if count == args.max_actions:
                raise RuntimeError("Action limit reached before match completion")
            if view["priority"] == 0:
                suggestions = match.request("suggest", player=0)["actions"]
                action = choose_action(view, suggestions, rng)
                match.request("apply", player=0, action=action)
            else:
                match.request("step_ai", player=1)
    finally:
        match.close()


if __name__ == "__main__":
    main()
