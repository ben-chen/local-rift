# Alpha Strike development audit

Updated November 12, 2026. Alpha Strike, Brutalizer and Hand Hammer have passed
native, source, Nix and installed-client acceptance. The release maps 480 legal
identities, including 446 main cards. See [the acceptance record](validation/alpha_equipment.json).
Card-pool counts and these tests do not establish complete Standard parity.

The [current rules hub](https://playriftbound.com/en-us/rules-hub/) was rechecked
on November 12 and still identifies the October 9 Core Rules. The full official
face and source hashes are preserved in
[the admission record](../assets/data/card_admissions/master_yi/admissions.json).
The retired Unleashed FAQ is not treated as a current rules override.

## Implemented and focused-tested

- Core 355.8–9 and 355.14: finalize a friendly source and distinct enemy
  battlefield recipients before responses. The initial number of recipients
  cannot exceed the source's referenced Might. Costs include every chosen
  enemy's Deflect and either of the printed hybrid Power domains.
- Core 355.14.e–i: divide damage only during resolution, using positive integer
  amounts and all available damage. If Might falls below the valid recipient
  count, retain exactly that many recipients. No combat lethal-first order
  applies. Allocation errors preserve the choice and game state.
- Core 359.3.e/f: recheck the original physical copies and current Might. A source
  can move within the board; an enemy recipient in base is no longer eligible.
  Leaving the board breaks identity even if the same printed card re-enters.
- Core 418.6.b.3: explicitly dealt unit damage bypasses Unyielding Spirit's
  spell/ability prevention and spell damage bonuses. Stun's combat contribution
  is not the referenced Might.
- Core 388–389 and 429.5: the draft observes actual cleanup deaths and creates
  separate XP abilities. Replaced deaths do not grant XP, tokens can, and a
  later spell's kills are not carried into this spell's rewards.
- Repeat executions preserve their own source, recipients and allocation
  choices. The physical parent stays on the chain until all executions finish;
  no response window opens between allocations.
- JSON observations expose a public `split_damage` decision with an exact
  decimal total. `assign_damage` accepts the same exact numeric form as combat,
  with split-specific validation. Previews are read-only. Wrong-player, stale,
  duplicate, foreign, zero, incomplete and malformed allocations are rejected
  without committing state. Hidden hands remain private.
- The client supports drag-to-source, successive enemy choices, undoing the
  source and dependent choices, editing individual amounts, invalid-confirmation
  recovery, reduced-total recipient selection, full inspection, and unobtrusive
  advice at both levels. Tray labels identify the battlefield and unit position.
- Automatic target recommendations compare smaller recipient sets to avoid
  spreading damage too thinly to kill a unit. Explicit player choices remain
  legal. A focused open-state position verifies useful kills at all three AI
  levels in both seats, as well as sweeping multiple low-Might recipients.

Initial zero-Might/no-recipient behavior is derived from the mandatory split
instruction together with 355.8, 355.13 and 355.14: this card does not specify
"up to" or "any number," so the draft requires at least one recipient with a
legal positive allocation at finalization. A later reduction to zero resolves
without dealing damage. This interpretation is recorded separately from the
explicit reduced-Might example in 355.14.h.1.

Focused checks are in [engine tests](../src/alpha_strike_tests.rs),
[protocol tests](../src/api/alpha_tests.rs), and the native
[interaction smoke](../src/smoke/alpha.rs). Controlled state fixtures test
identity and arithmetic boundaries; they are not tournament deck histories.

Earlier snapshots passed library/native/source checks, 288 authored complete
games against both Irelia presets, 288 external API games, and four native UI
runs at two sizes. Those results predate the physical-parent correction below.
The corrected combined source now passes 29 focused engine/protocol checks,
including both actors, cleanup cascades, death replacement and separate XP
response windows. Its broader acceptance remains pending. The separately
published Spirit/Repulse source passed all 1,774 Rust tests, Nix acceptance and
installed-client checks; see [its receipt](validation/master_yi_reactions.json).

## Repeat and physical-parent attribution

The prior draft stored damaged recipients per execution. The corrected design
stores their union against the resolving physical spell, together with the
number of non-ignored reward clauses. This interpretation is derived from the
current Core Rules; no Alpha-specific Repeat ruling was found.

1. Core 053.2 makes the spell the referent of its self-reference. Core 820.1.d
   repeats instructions on the same chain item; it does not create a new spell.
2. Core 321 defers cleanup until that whole chain item has finished. Core
   429.5.c attributes cleanup kills to the preceding resolved spell or ability
   that dealt damage to those units. The damage dealer for this card remains
   the chosen unit for prevention and bonuses under Core 418.6.b.3.
3. Consequently, each executed reward clause observes the parent's actual
   attributed deaths. Two executions killing two distinct units create four
   separate one-XP abilities. Two executions together killing one unit create
   two. Core 388–389 keeps those rewards independently respondable.
4. Under Core 359.3.e.14, an ignored earlier linked instruction also ignores its
   later clause. A departed source or no legal recipients therefore creates no
   reward clause. A legal source reduced to zero Might negates the Deal without
   ignoring the clause. The clause can still observe another execution's kills.

The [Radiance patch notes](https://playriftbound.com/en-us/news/announcements/core-rules-radiance-patch-notes-effective-october-16-2026/)
support the ignored-versus-negated distinction. They do not give an
Alpha-specific example. Tests verify the implemented interpretation and cannot
serve as independent rules authority.

The observer persists through cascading cleanup and replacement choices, then
expires before another chain item resolves. It counts each damaged physical
victim once per clause, excluding replaced deaths and undamaged cascade
victims. Regressions cover Taric's Shield leaving through death or recall,
both execution orders, a sole automatic Guardian Angel replacement, and two
gear copies requiring an explicit choice. Protocol regressions verify that
lethal damage from the first execution does not remove units before the second
allocation, rejected/stale commands leave both views unchanged, and reactions
can resolve above XP rewards without granting XP early.

The later [attribution checkpoint](validation/hidden_blade.json) replaces this
observer’s private recipient list with a shared physical Deal/Kill ledger. Alpha
retains its physical parent and number of non-ignored reward clauses. Unit-dealt
packets identify the unit; cleanup credit identifies the causing spell. The
remaining origin, cost and replacement gaps are listed in the rules audit’s
damage-attribution follow-up; passing this checkpoint does not close those gaps.

## Acceptance and remaining work

The release passes 1,815 Rust tests and 282 source checks, 1,176 preset simulations,
432 standalone API games with identical native/installed traces, and eight
installed UI runs with 40 captures at two window sizes. The packaged runtime
matches the frozen tested source; only release documentation and proof status
labels differ, as recorded in the acceptance record.

Review the physical-parent derivation above if a new official ruling or Core
revision appears. The remaining damage-source gaps are explicit in
[the rules audit](RULES_AUDIT.md). Complete SWAGALISK registration acceptance is
recorded separately in [Master Yi validation](validation/master_yi_registration.json);
the existing main-deck recipe remains unchanged.
No Azir preset or legend is enabled by this release.

No expert-strength claim is made for the allocation heuristic or AI opponents.
