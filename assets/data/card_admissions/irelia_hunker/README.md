# Hunker Down — Irelia priority

The complete official RAD-034/167 face was inspected November 11, 2026. It is a
1-Energy, 0-Power Calm Reaction with one archived printing and no listed erratum.
The existing reviewed type correction remains separate from the unchanged raw
gallery record. The original image, bounded runtime art, effective facts and
preserved preceding 425-card/five-preset hashes are pinned in `admissions.json`.
Runtime ID 425 follows Ravenbloom Prefect. This card is an Irelia practice-pool
addition; no community list is relabeled as a verified tournament preset.

Ready a friendly unit, prevent that physical unit from moving for the rest of the
turn, then draw one. Already-ready units are legal choices: they still receive the
restriction and the draw still happens. Irelia's Choose trigger is separate from
her Ready trigger, which requires an actual exhausted-to-ready transition.

Core 358.3.a permits movement spells to choose a restricted unit. Their Move
instruction is negated; independent instructions still execute. Hunker Down
blocks movement by either player, unlike Vex's controller-only restriction.
Core 055 and 359.3.e.5–8 preserve instructions unrelated to an illegal target:
Hunker Down still draws if its chosen unit leaves play before resolution.
Countering the spell prevents all its instructions.

Basic movement, grouped movement, forced moves, Tideturner's two moves and Smoke
and Mirrors obey the restriction. An unrestricted second unit still moves to the
original destination. Recall/return-to-hand and banish are not Move. A replayed
card has a new physical identity and does not inherit the restriction.

The native board shows CANNOT MOVE with a hover explanation. The JSON observation
retains `movement_locked` and adds `movement_restriction` only for an object-wide
restriction. Beginner and Advanced companion tips explain the rule and positioning
tradeoff without taking input focus.
