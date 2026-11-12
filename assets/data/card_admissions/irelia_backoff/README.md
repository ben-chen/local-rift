# Back Off — Irelia priority

The complete official UNL-042/219 face was visually inspected November 11, 2026.
It is a Calm spell costing 3 Energy and 0 Power, with Hidden, Action and Stun.
It stuns a unit, then independently draws one card if played from hand. The
accepted dated inventory contains one printing and no erratum or field correction.
`admissions.json` pins the original face, bounded runtime image, raw/effective
facts and the preceding 428-card/five-preset hashes. Runtime ID 428 follows
Siren's Song. This adds a practice-pool card, not a new verified event preset.

The draw reads the physical spell's finalized play origin. Hand qualifies;
Hidden, Flow from Trash and instructed Banish plays do not. Repeat executes the
conditional draw each time using that same original fact. A departed or protected
unit prevents the Stun instruction but not an eligible draw (359.3.e.5). A counter
prevents both instructions. The spell stays on the chain during all instructions,
so it cannot shuffle itself into a burnout draw.

Stun does not exhaust, deal damage or lower Might. An already-stunned unit may
be chosen, although no new Stun occurs; Stun expires at end-of-turn cleanup
(424.1). Ordinary targeting and Deflect apply to friendly and enemy units.

Hidden is separate from playing: pay one Any Power to hide at a controlled field
with an empty hidden slot. It does not trigger Play or draw. From the following
turn it can be played as a Reaction, ignoring its base cost; additional Repeat
or Deflect costs remain. Targets must be at that field and satisfy that restriction
again on resolution. Playing it normally from hand retains Action timing and its
printed cost (811.1, 811.3, 811.6).

Source: [Riot's official face](https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/a0e7f7fa55ae618f94fe48920f92406ddd7e3512-744x1039.png?accountingTag=RB)
and the [October 9 core rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf).
