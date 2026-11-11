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


C27_PREDECESSOR = catalog.ROOT / 'assets/data/card_admissions/c28/predecessor'


def legacy_battlefields():
    """Use the exact eleven-field registry of admissions before C28."""
    archived = json.loads((C27_PREDECESSOR / 'battlefields.json').read_text())['battlefields']
    if catalog.load_battlefield_manifest()[:len(archived)] != archived:
        raise AssertionError('A preserved battlefield source record changed')
    return archived


def legacy_variant_evidence_bytes(path):
    """Preserve old whole-file pins while checking their live evidence prefixes."""
    if path == 'assets/data/catalog_evidence/variant_images.json':
        archived_path = C27_PREDECESSOR / 'variant_images.json'
        old = json.loads(archived_path.read_text())
        current = json.loads((catalog.ROOT / path).read_text())
        if current[:len(old)] != old:
            raise AssertionError('A preserved variant image record changed')
    elif path == 'assets/data/catalog_identity_reviews.json':
        archived_path = C27_PREDECESSOR / 'catalog_identity_reviews.json'
        old = json.loads(archived_path.read_text())
        current = json.loads((catalog.ROOT / path).read_text())
        if current['reviews'][:len(old['reviews'])] != old['reviews']:
            raise AssertionError('A preserved identity review changed')
        metadata = lambda value: {k: v for k, v in value.items() if k not in ('reviews', 'image_index')}
        if metadata(current) != metadata(old):
            raise AssertionError('A preserved identity authority changed')
        if current['image_index']['local_path'] != old['image_index']['local_path']:
            raise AssertionError('A preserved identity image-index location changed')
        legacy_variant_evidence_bytes('assets/data/catalog_evidence/variant_images.json')
    else:
        raise ValueError('Not a historical identity evidence path')
    return archived_path.read_bytes()


def pre_rengar_recipe_source_bytes():
    """Exact pre-fifth-preset source; behavior is pinned by tests/rengar_preset.rs.

    Adding the preset needs selection/default-field plumbing in deck.rs. The
    original ten recipe JSON outputs are compared independently at runtime.
    """
    folder = catalog.ROOT / 'assets/data/preset_admissions/rengar_2026_09'
    proof = json.loads((folder / 'admission.json').read_text())
    data = (catalog.ROOT / proof['predecessor_training_recipe_source']).read_bytes()
    if catalog.sha(data) != proof['predecessor_training_recipe_sha256']:
        raise AssertionError('The preserved pre-Rengar training source changed')
    return data
