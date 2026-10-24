"""Preserve a completed cohort while admitting new legends and training lists.

The archived files are the exact C15 source, not regenerated proof hashes.
Runtime equivalence of the first eight recipes is checked independently against
C15 API output by tests/c16_legends.rs.
"""
import json
import import_catalog as catalog

PREDECESSOR = catalog.ROOT / 'assets/data/card_admissions/c16/predecessor'


def legacy_legends():
    archived = json.loads((PREDECESSOR / 'legends.json').read_text())['legends']
    current = json.loads((catalog.ROOT / 'assets/data/legends.json').read_text())['legends']
    if current[:len(archived)] != archived:
        raise AssertionError('A preserved legend identity changed')
    return archived


def legacy_legend_manifest_bytes():
    legacy_legends()
    return (PREDECESSOR / 'legends.json').read_bytes()


def legacy_recipe_source_bytes():
    return (PREDECESSOR / 'deck.rs').read_bytes()
