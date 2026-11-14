# Guards!

Admission for SFD-154/221, runtime 448. The rules implementation passed native
and installed package acceptance. Final acceptance of the combined token-art
release is tracked in [the validation record](../../../../docs/validation/guards_tokens.json).
This adds a card for custom practice, not a sourced Azir deck. The preceding
448 cards and seven tournament recipes are preserved.

The original face and gallery record show 3 Energy, no Power, Order and Hidden.
The gallery's missing Power field remains null in the archived printing; the
runtime zero is checked against the actual face. The original and operative
wording are stored separately: the official [Unleashed erratum](https://playriftbound.com/en-us/news/rules-and-releases/unleashed-errata-updates/)
makes the optional Ready a reflexive ability.

The October 9 Core Rules give the sequence used here:

- CR354.3: the instructed Soldier play becomes pending until the spell finishes.
- CR388–389: the subsequent reflexive ability becomes a separate pending item.
- CR404.1.b.1: its initial optional Order payment is a base cost paid before
  responses, not a cost deferred until the Ready effect resolves.
- CR355.10.d: “it” identifies the exact created token without choosing it. This
  does not add a unit target, Deflect payment, or an Irelia choose trigger.
- CR811.1.d.5: Hidden fixes the token's destination to that battlefield. Hidden
  waives the original spell cost, not the later ability's Order payment.

Repeated spell executions retain independent physical-token links. A refused
token play leaves no referenced unit; a departed token cannot be replaced with
an older or newly created Soldier. No cost is refunded when a paid Ready later
does nothing. If the spell itself is countered, neither instruction happens.

The payment panel identifies the Soldier and lets the player pay or decline.
Mote's optional Beginner and Advanced comments explain the separate payment and
response opportunity without reading private opponent cards. The same choices
and public physical-unit reference are available through the bot API.

Focused tests cover both seats, Hidden, Repeat, the response window, absence,
countering the original spell, refused token plays, stale API submissions, and
AI choices. Existing spell-only and targeted-ability counters correctly reject
the untargeted Ready ability; this is not evidence for an as-yet unimplemented
general ability counter. Complete games and native controls are checked
separately before publication.
