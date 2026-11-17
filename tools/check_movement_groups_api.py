#!/usr/bin/env python3
"""Movement groups through registered Irelia matches at all three AI levels."""
from card_campaign import Campaign, campaign_main


SPEC = Campaign(
    scope=__doc__,
    deck_name='Irelia / Movement groups integration',
    cards=((488, 'sfd-043-221'), (489, 'unl-054-219'), (490, 'unl-124-219')),
    swaps=((4, 489), (14, 488), (16, 490)),
    base_seed=489200,
    samples=(8, 2, 2),
    target_counts={490: 1},
)


if __name__ == '__main__':
    campaign_main(SPEC, __file__)
