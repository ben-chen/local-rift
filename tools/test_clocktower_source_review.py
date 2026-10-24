"""Exact-face corrections preserve raw source and never admit a card by themselves."""
import copy
import gzip
import json
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog


class ClocktowerSourceReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.raw = catalog.project_gallery(cls.archives['gallery'])[0]
        cls.manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        cls.proof = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/reviews/clocktower-guardian.json').read_text())
        cls.runtime = copy.deepcopy(catalog.load_runtime_manifest())
        cls.runtime['cards'] = cls.runtime['cards'][:384]
        legends = catalog.load_legend_manifest()[:10]
        with patch.object(catalog, 'load_runtime_manifest', return_value=cls.runtime), patch.object(catalog, 'load_legend_manifest', return_value=legends):
            cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def apply(self, manifest=None, raw=None):
        return review.apply_reviews(self.raw if raw is None else raw, root=catalog.ROOT,
            project_gallery=lambda data: catalog.project_gallery(data)[0], plain=catalog.plain,
            manifest=self.manifest if manifest is None else manifest)

    def test_only_effective_domain_and_tag_change_without_admitting_gameplay(self):
        p = next(p for p in self.snapshot['printings'] if p['official_id'] == 'rad-067-167')
        card = next(c for c in self.snapshot['cards'] if c['identity'] == 'rad-067-167')
        self.assertEqual((p['domains'], p['tags']), (['fury'], ['Mech']))
        self.assertEqual((p['effective']['domains'], p['effective']['tags']), (['mind'], ['Shurima']))
        self.assertEqual(p['effective'], self.proof['effective_after'])
        changed = {k for k in self.proof['effective_before']
                   if self.proof['effective_before'][k] != self.proof['effective_after'][k]}
        self.assertEqual(changed, {'domains', 'tags'})
        self.assertEqual(p['resolution_ids'], ['rad-067-167:domains', 'rad-067-167:tags'])
        self.assertTrue(card['standard_legal'])
        self.assertEqual(card['official_ids'], ['rad-067-167'])
        self.assertEqual(card['errata'], [])
        self.assertFalse(card['implementation']['playable_in_client'])
        self.assertFalse(self.proof['enables_runtime_support'])
        c = self.snapshot['counts']
        self.assertEqual((c['implemented_main_cards'], c['implemented_legal_identities'], c['unimplemented_legal_identities']), (384, 411, 668))

    def test_complete_raw_origin_and_independent_face_observation_are_pinned(self):
        p = self.proof
        raw = next(r for r in self.raw if r['official_id'] == 'rad-067-167')
        self.assertEqual(p['raw_printing'], raw)
        self.assertEqual(p['raw_printing_sha256'], review.printing_sha256(raw))
        origin = gzip.decompress((catalog.ROOT / p['origin_gallery']['archive']).read_bytes())
        self.assertEqual((len(origin), catalog.sha(origin)), (p['origin_gallery']['uncompressed_bytes'], p['origin_gallery']['sha256']))
        self.assertEqual(catalog.project_gallery(origin)[0], self.raw)
        image = p['original_image']; payload = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual((len(payload), catalog.sha(payload)), (1108541, image['sha256']))
        self.assertEqual(image['bytes'], len(payload))
        self.assertEqual(image['url'], raw['image_url'])
        o = p['visual_observation']
        self.assertTrue(o['face_visually_inspected'])
        self.assertEqual(o['original_image_sha256'], image['sha256'])
        self.assertEqual((o['printed_types'], o['printed_domains'], o['printed_tags']), (['unit'], ['mind'], ['Shurima']))
        self.assertEqual((o['printed_energy'], o['printed_power'], o['printed_might']), (8, 2, 8))
        self.assertEqual(o['collector_code'], 'RAD-067/167')
        self.assertEqual(o['printed_rules_icons_expanded'], "When you play me or when I hold, [Predict 3].\n[Deathknell] Banish the top card of your Main Deck. You may play it, ignoring its Energy cost.")
        self.assertNotIn('/tmp/', json.dumps(p))

    def test_earlier_source_cohorts_and_runtime_are_preserved(self):
        p = self.proof; digest = lambda v: catalog.sha(review.canonical_bytes(v))
        for key, old in p['preserved_source_cohorts'].items():
            self.assertEqual(digest(self.manifest[key][:old['count']]), old['canonical_sha256'])
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        old = p['preserved_field_image_index']
        self.assertEqual(digest(images[:old['count']]), old['canonical_sha256'])
        for path, key in [('assets/data/catalog_identity_reviews.json', 'manifest_sha256'), ('assets/data/catalog_evidence/variant_images.json', 'image_index_sha256')]:
            self.assertEqual(catalog.sha((catalog.ROOT / path).read_bytes()), p['preserved_variant_review'][key])
        old = p['preserved_runtime']
        self.assertEqual(digest(self.runtime['cards']), old['cards_sha256'])
        self.assertEqual(digest(self.runtime['decks']), old['presets_sha256'])
        self.assertEqual(catalog.sha((catalog.ROOT / 'assets/data/legends.json').read_bytes()), old['legends_manifest_sha256'])
        self.assertEqual(catalog.sha((catalog.ROOT / 'src/deck.rs').read_bytes()), old['training_recipe_source_sha256'])

    def test_changed_raw_prerequisites_are_rejected(self):
        for field, value in [('domains', ['mind']), ('tags', ['Shurima']), ('might', 9)]:
            with self.subTest(field=field):
                raw = copy.deepcopy(self.raw)
                next(r for r in raw if r['official_id'] == 'rad-067-167')[field] = value
                with self.assertRaisesRegex(ValueError, 'Stale candidate printing hash'):
                    self.apply(raw=raw)

    def test_each_discovery_requires_its_resolution_and_exact_image(self):
        for field in ('domains', 'tags'):
            for mode in ('resolution', 'image'):
                with self.subTest(field=field, mode=mode):
                    m = copy.deepcopy(self.manifest)
                    if mode == 'resolution':
                        m['resolutions'] = [r for r in m['resolutions'] if r['id'] != f'rad-067-167:{field}']
                    else:
                        next(r for r in m['resolutions'] if r['id'] == f'rad-067-167:{field}')['evidence'][0]['sha256'] = '0' * 64
                    with self.assertRaises(ValueError):
                        self.apply(manifest=m)


if __name__ == '__main__':
    unittest.main()
