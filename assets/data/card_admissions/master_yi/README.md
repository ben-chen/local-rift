# Master Yi opponent expansion

Unyielding Spirit (OGN-145/298, runtime 441), Repulse (UNL-106/219, runtime 442),
and Alpha Strike (UNL-192/219, runtime 443) have completed their release checks.
The complete SWAGALISK Los Angeles registration is being verified separately.
See [Spirit/Repulse acceptance](../../../../docs/validation/master_yi_reactions.json),
[Alpha/equipment acceptance](../../../../docs/validation/alpha_equipment.json),
and [Alpha Strike's rules audit](../../../../docs/ALPHA_STRIKE_AUDIT.md).

The complete official face, reviewed gallery fields, printing cohort and artwork
hashes are recorded in `admissions.json`. The original 441 main cards and seven
preset recipes are preserved by their prior manifest hashes. None of these cards has a
current reviewed erratum. Riot's retired Origins FAQ is historical context and
does not supply an invented text override.

Core Rules 418.6 distinguishes spell/ability damage from damage explicitly dealt
by units. An ability created by a unit or gear is still an ability damage source.
Core Rules 438 applies prevention after bonuses and treats fully prevented damage
as no damage dealt. This effect applies to both players and units entering later.
It expires in ending cleanup after ending triggers; combat and Challenge still
deal damage, existing damage remains, and independent instructions still resolve.

The shared damage calculation, action-level tests, both API views and native
interaction checks cover the addition. Beginner and Advanced commentary describe
the global protection and its limits. AI estimates pending-chain consequences
from its public-information belief state, stops at unresolved choices and avoids
spending pure damage resources while prevention is active. This is a bounded
practice policy, not evidence of tournament-strength play.

Repulse chooses a friendly battlefield unit first, then an enemy spell or ability
that chooses that unit and no other friendly unit. Core 359.3.e.9.a explicitly
counts mistargeted units that remain on the board. Moving another chosen unit to
base therefore does not enable Repulse; leaving for a non-board zone breaks the
relationship, and a newly played physical copy does not restore it. Every Repeat
choice belongs to the same chain item. Not So Fast now shares this full physical
target tracking. The current Core text supplies this rule directly; the retired
Unleashed FAQ is only historical corroboration.

Sources: [official card gallery](https://playriftbound.com/en-us/card-gallery/),
[current rules hub](https://playriftbound.com/en-us/rules-hub/),
[Origins errata](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/),
[Los Angeles deck registrations](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/).
