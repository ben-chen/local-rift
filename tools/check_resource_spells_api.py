#!/usr/bin/env python3
"""Resource spells through registered Irelia matches at all three AI levels."""
from card_campaign import Campaign, campaign_main


SPEC = Campaign(
    scope=__doc__,
    deck_name='Irelia / Resource spells integration',
    cards=((491, 'rad-028-167'), (492, 'rad-100-167'), (493, 'ogn-201-298'), (494, 'ogn-071-298')),
    swaps=((4, 491), (14, 492), (16, 493), (21, 494)),
    base_seed=494300,
    samples=(8, 2, 8),
    target_counts={491: 1},
    cast_exemptions={
        ('strategic', 494): 'Party Favors is symmetric and may be declined; explicit opponent-choice scenarios cover all levels. Actual casts remain in the report.',
    },
)


if __name__ == '__main__':
    campaign_main(SPEC, __file__)
