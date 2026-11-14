"""Preserve a completed cohort while admitting new legends and training lists.

The archived files are the exact C15 source, not regenerated proof hashes.
Runtime equivalence of the first eight recipes is checked independently against
C15 API output by tests/c16_legends.rs.
"""
import json
from contextlib import contextmanager
from unittest.mock import patch
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


def pre_registration_carddefs_source():
    """Project only the additive preset schema out of old whole-prefix pins.

    These historical checks hash the cards.rs prelude as well as CardDef values.
    Preset registration changes that prelude, but no card definition. Remove
    only the two exact additions; keep every card byte and all other prelude
    bytes checked by the original, unchanged proof hashes. Runtime preset and
    training-list equivalence is covered separately by the Rust recipe tests.
    """
    source = (catalog.ROOT / 'src/cards.rs').read_text()
    additions = [
        "#[derive(Clone, Copy, Debug)]\npub struct PresetRegistration {\n"
        "    pub pilot: &'static str,\n"
        "    pub event: &'static str,\n"
        "    pub date: &'static str,\n"
        "    pub placing: u16,\n"
        "    pub battlefields: &'static [usize],\n"
        "    pub sideboard: &'static [(CardId, u8)],\n}\n\n",
        "    /// Separately verified event registration; absent on older preset records.\n"
        "    pub registration: Option<PresetRegistration>,\n",
    ]
    for addition in additions:
        if source.count(addition) != 1:
            raise AssertionError('The additive preset registration schema changed')
        source = source.replace(addition, '', 1)
    return source


RIVEN_PREDECESSOR = catalog.ROOT / 'assets/data/card_admissions/riven/predecessor'


def pre_riven_evidence_bytes(path):
    """Check the unchanged historical records, then return their original bytes.

    New legends and source reviews append records. Original admission hashes
    still check the archived whole files; every archived record and authority
    must also remain present in the current files.
    """
    filename = path.removeprefix('assets/data/')
    allowed = {
        'legends.json': (['legends'], []),
        'catalog_resolutions.json': (
            ['resolutions', 'raw_printings', 'historical_issues'],
            ['evidence_manifest_sha256', 'stats']),
        'catalog_identity_reviews.json': (['reviews'], ['image_index']),
        'catalog_evidence/images.json': None,
        'catalog_evidence/variant_images.json': None,
    }
    if filename not in allowed or path != 'assets/data/' + filename:
        raise ValueError('Not a pre-Riven evidence path')
    payload = (RIVEN_PREDECESSOR / filename).read_bytes()
    archived = json.loads(payload)
    current = json.loads((catalog.ROOT / path).read_bytes())
    spec = allowed[filename]
    if spec is None:
        if current[:len(archived)] != archived:
            raise AssertionError(f'A preserved evidence record changed: {path}')
    else:
        arrays, derived = spec
        for key in arrays:
            if current[key][:len(archived[key])] != archived[key]:
                raise AssertionError(f'A preserved {key} record changed: {path}')
        authority = lambda value: {k: v for k, v in value.items() if k not in arrays + derived}
        if authority(current) != authority(archived):
            raise AssertionError(f'A preserved authority changed: {path}')
        if filename == 'catalog_resolutions.json':
            images = pre_riven_evidence_bytes(archived['evidence_manifest_path'])
            if catalog.sha(images) != archived['evidence_manifest_sha256']:
                raise AssertionError('The archived field image-index hash changed')
        elif filename == 'catalog_identity_reviews.json':
            if current['image_index']['local_path'] != archived['image_index']['local_path']:
                raise AssertionError('The identity image-index location changed')
            pre_riven_evidence_bytes(archived['image_index']['local_path'])
    return payload


def pre_riven_legends():
    return json.loads(pre_riven_evidence_bytes('assets/data/legends.json'))['legends']


@contextmanager
def pre_riven_runtime(runtime):
    """Pair a historical card pool with its ten legends and seventeen fields.

    C28's admission tests independently pin the original eleven fields and
    verify the six additions against their source records. Later battlefield
    admissions must not silently enlarge these historical coverage fixtures.
    Older cohorts can still supply their explicit eleven-field override.
    """
    if len(runtime['cards']) > 435:
        raise AssertionError('Use the current legend registry for post-Riven cards')
    legends = pre_riven_legends()
    fields = catalog.load_battlefield_manifest()[:17]
    # A later preset cannot refer to a legend/champion absent from this fixture.
    # Preserve the exact earlier recipes; only omit the subsequently added list.
    runtime = {**runtime, 'decks': [d for d in runtime['decks']
                                   if d['legend_id'] != 'rad-149-167']}
    with patch.object(catalog, 'load_runtime_manifest', return_value=runtime), \
            patch.object(catalog, 'load_legend_manifest', return_value=legends), \
            patch.object(catalog, 'load_battlefield_manifest', return_value=fields):
        yield
