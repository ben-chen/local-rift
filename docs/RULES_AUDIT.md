# Independent rules audit

Reviewed on **October 24, 2026**. This is an evidence log, not a certification that Local Rift implements every legal Riftbound interaction. The executable catalog still covers a subset of the full legal card pool.

The audit started from commit `167ad7e` and continued against the shared working tree. The findings below were reproduced through public `Game` actions before being fixed. Small fixtures construct board positions directly; they do not verify deck construction or card conservation.

## Authoritative references

- Riot's [Rules Hub](https://playriftbound.com/en-us/rules-hub/), checked on the review date, links the [October 9, 2026 Core Rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf).
- The [Radiance rules update, effective October 16](https://playriftbound.com/en-us/news/announcements/core-rules-radiance-patch-notes-effective-october-16-2026/), supplies current clarifications about pending items and delayed effects.
- Current card wording and previous errata provenance remain in [SOURCES.md](SOURCES.md) and `assets/data/decks.json`.

## Confirmed findings and fixes

| ID | Reproduction and prior result | Correction | Rules |
|---|---|---|---|
| RA-001 | Discipline drew from an empty deck with a nonempty trash while the opponent had seven points. The opponent won before the replacement draw completed. | The first burnout finishes its instruction; ordinary victory is checked at cleanup. A subsequent repeated empty-deck burnout retains its special immediate victory. | 432.2.d, 432.3.c.1, 323.1 |
| RA-002 | End Turn immediately removed Tornado Warrior's Empower without exposing a response window. | A delayed Disempower ability enters the chain during the Ending Step. Expiration and the next turn wait. Its controller remains Tornado's controller, even when the empowered object belongs to an opponent. | 317.1–317.2, 391.2 |
| RA-003 | Falling Star killed a friendly Pyke and an enemy unit in one cleanup. The dead Pyke incorrectly generated Gold. | Pyke must remain on the board after the simultaneous death event to see the enemy death. | 383.2.c.2 |
| RA-004 | Kennen's conquest ability exposed its trash-card choice only after opponents had responded. | The spell is chosen during finalization. Flow is granted on resolution only if that same object remains in the trash. Removing another card adjusts the tracked index; recycling the chosen card invalidates it. | 355.5, 355.9.a, 355.10.a.1, 359.3.e.4 |
| RA-005 | First Mate's play effect and an opposing Vex trigger were pending. Choosing Irelia for First Mate caused the newer Irelia trigger to finalize ahead of the older Vex trigger. | Pending groups preserve their age. The controller still orders simultaneous triggers, but newly generated groups cannot pass an older pending group. | 337.1, 337.1.b, 383.3.d |

The first four failures were observed before correction. The fifth ordering failure was also reproduced independently rather than inferred solely from inspection.

## Regression evidence

[tests/rules_audit.rs](../tests/rules_audit.rs) contains seven public-API tests:

1. First burnout completes the draw before cleanup victory.
2. Tornado's scheduled Disempower waits for responses.
3. Pyke cannot see an enemy death when Pyke leaves simultaneously.
4. Kennen chooses its Flow target before finalization completes.
5. A newer Irelia trigger follows an older pending Vex trigger.
6. A Tornado effect created after this turn's ending window does not expire on the next turn instead.
7. Burnout invalidates Kennen's chosen trash object even if an identical card later occupies the same index.

All seven passed after the fixes. The library suite passed **156 tests**, and the library passed Clippy with warnings denied at the end of this batch. These checks establish the named examples, not universal rules equivalence. Full-match simulation, rendering, and API transport checks are maintained separately.

## Payment review

Read `src/payment.rs` and the hand-play, optional-cost, trash-play, Deflect, Hidden, and ability-payment entry points in `src/game.rs`. The exact-payment wrapper validates on a disposable clone and commits only after all selected sources are consumed. Restrictive card-domain obligations precede flexible Deflect obligations. Spell-only power is excluded from non-spell payments. No additional atomicity or matching-domain defect was confirmed in this review; [tests/payments.rs](../tests/payments.rs) exercises the concrete mixed-source cases.

This review also identified a narrower rune-choice gap: domain-only recycling always preferred exhausted runes. That gap was subsequently closed by `Action::RecycleRune { domain, exhausted }`, which can recycle a ready rune without implicitly exhausting it for energy, while retaining an exhausted rune of the same domain. The implementation and its successful/rejected-action regressions were independently reviewed. All **nine payment tests** and **ten earlier conformance tests** passed after this addition. Automatic payment remains a convenience, while explicit source selection and rune preparation provide the corresponding player choices.

## Open questions and next audit targets

These items are **not resolved by the seven passing examples**:

- **Multiple Flow costs:** rule 829.1.c.3 permits choosing between different Flow instances. The current trash-play path prefers Kennen's granted cost over a printed Flow cost and has no selector for the alternative. This is a confirmed choice gap, although the granted cost is cheaper or equal for the bundled overlapping cards.
- **Separate events within one resolving item:** the new batching fix preserves older pending items during finalization. A broader audit must still distinguish triggers arising from separate instructions and pending permanent/token plays during the same resolution; a single cleanup is not itself proof that all those triggers were simultaneous.
- **Several staged battlefields:** rules 323.11–323.12 give the turn player battlefield selection within the relevant stage. A follow-up trace found a bundled example: keep Discipline pending while the other player Ambushes Rengar into each of two enemy-controlled fields. After the chain resolves, the turn player must choose which staged combat starts; the current fallback takes field 0. That fallback also uses the chain initiator as attacker instead of the player who applied Contested (190.3.a.1, 345, 466.2.c.1). This trace still needs executable regressions and a fix.
- **Expanded card pool:** new keywords, replacement interactions, granted abilities, zone changes, and cards not in the original presets require dedicated executable behavior and tests. Importing their official text is not implementation coverage.

The audit read the original 69 card texts and the execution paths for targeting, pending triggers, death replacement, scoring, turn start/end, movement, combat, and resource payment. It did not exhaustively enumerate every pair or longer combination of those cards, and it did not audit the full currently legal card pool.
