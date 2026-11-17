#!/usr/bin/env python3
"""Temptation through registered Irelia matches at all three AI levels."""
from card_campaign import Campaign, campaign_main


SPEC = Campaign(
    scope=__doc__,
    deck_name='Irelia / Temptation integration',
    cards=((487, 'sfd-129-221'),),
    swaps=((4, 487),),
    base_seed=487200,
    samples=(2, 2, 2),
    target_counts={487: 1},
)


if __name__ == '__main__':
    campaign_main(SPEC, __file__)
