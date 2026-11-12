# Gust Monk — accepted admission

Accepted November 11, 2026. The complete official
VEN-101/166 face was individually inspected November 11, 2026. It is a Chaos
unit with 2 Energy, no Power cost, 2 Might, and Bandle City/Yordle tags. The
accepted gallery has one printing, no source conflicts and no erratum for it.
All predecessor source reviews and five preset recipes remain unchanged.

The optional 1 Energy is an additional cost of playing the unit. Paying it
causes a conditional play trigger. That ability chooses one unit, then banishes
one card from either player's trash as its base cost before finalization and
responses (Core 204.1.c, 355.10.c, 383.2–3, 404.1.b.1, 405–407). The trash card
is a cost, not a target; the chosen unit may be friendly, enemy, or Gust Monk.
Deflect is an additional ability cost and cannot use spell-only Power.

The controller may decline the incurred banish cost under 405.2, even though
the additional Energy was already paid. The completed unit play remains.
Once paid, a counter or lost target does not refund the banished card. If the
ability resolves on a legal target, it grants Assault 2 this turn. This adds
Might only while the unit is attacking, stacks with other Assault grants, and
expires at turn cleanup. It does not give Gust Monk intrinsic Assault.

The native client separates the extra play cost, unit target, and public
trash-cost tray. Full previews and Beginner/Advanced advice explain the timing.
The bot API exposes the same cost and action. A staged payment tracks the
physical trash copy if other cards leave first; removing that copy or recycling
the trash cannot silently substitute another. Undo/refusal restore generated
resources and the pending payment boundary without undoing the unit play.

The primary [Arito deck list](https://topdeck.gg/deck/convergence-4/3Mb0hLsUTwgBYrpAauGOaRvDhnq1)
and [Convergence IV standings](https://topdeck.gg/bracket/convergence-4) were read
on November 11: second place, 8–2–0, in a 272-player event on October 31, with
three Orianna and one Gust Monk. Preset integration and exact-list match testing
remain separate. These test games use an explicit authored 40-card variant.

[Source evidence](admissions.json),
[official face](https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/93a194c0c1f055c0b3bc3c0717cbe488749fed92-744x1039.png?accountingTag=RB),
[Core Rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf),
[Vendetta rules notes](https://playriftbound.com/en-us/news/announcements/core-rules-vendetta-patch-notes/),
and [Vendetta errata](https://playriftbound.com/en-us/news/announcements/vendetta-errata-updates/).

The [validation record](../../../../docs/validation/irelia_gust.json) records complete suites, games, API behavior, native interface checks, source review and remaining limits.
