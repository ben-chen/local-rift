# Deathgrip — admission evidence

Development admission 447 is the single SFD-163/221 printing: Order Spell,
2 Energy, no Power, Reaction. The original complete 744×1039 face was inspected;
its printed wording is kept separately from the operative text and runtime art.

The [official Spiritforged erratum](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-spiritforged-errata/)
and [FAQ](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-spiritforged-faq/)
make both distinct friendly units targets chosen before responses. The first is
killed during resolution, not as a cost. Actual death is required for the Might
bonus, while the separate Draw1 still happens when either target is invalid or
the death is replaced. Core359.3.e.13 supplies the immediately preceding Might
when a successful instruction changes the target's zone; Core359.3.e.14.b uses
Deathgrip as its explicit example of the linked actual-death requirement.

The continuation waits through ordered death replacements. It records Might
with live gear and auras immediately before actual removal, floors negative
references to zero, preserves both target slots and keeps each Repeat execution
independent. Temporary Might uses the common turn expiry. The action boundary
restores the whole attempted resolution or replacement choice on capacity errors.

The JSON API and native client use the same ordered targeting rules. Native
prompts explain which unit dies and which receives Might, with full corrected
text in the inspector. Beginner/Advanced advice uses only the player's own hand.
The authored 40-card Kennen substitution used for testing is not a sourced preset;
all seven tournament recipes and the preceding 447 card definitions stay pinned.

Verification remains in progress. This admission does not assert complete Azir
support, a complete Standard pool, or competitive AI strength.
