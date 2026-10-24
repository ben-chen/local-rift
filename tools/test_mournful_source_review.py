"""A printed-tag correction preserves its raw source and cannot admit gameplay."""
import copy
import gzip
import json
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog
import admission_history as history


class MournfulSourceReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.raw = catalog.project_gallery(cls.archives['gallery'])[0]
        cls.manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        cls.proof = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/reviews/mournful-spirit.json').read_text())
        cls.runtime = copy.deepcopy(catalog.load_runtime_manifest())
        cls.runtime['cards'] = cls.runtime['cards'][:372]
        with patch.object(catalog, 'load_runtime_manifest', return_value=cls.runtime), patch.object(catalog, "load_legend_manifest", return_value=history.legacy_legends()):
            cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def apply(self, manifest=None, raw=None):
        return review.apply_reviews(self.raw if raw is None else raw, root=catalog.ROOT,
            project_gallery=lambda data: catalog.project_gallery(data)[0], plain=catalog.plain,
            manifest=self.manifest if manifest is None else manifest)

    def test_only_effective_tags_change_and_source_review_does_not_admit_the_card(self):
        p = next(p for p in self.snapshot['printings'] if p['official_id'] == 'rad-126-167')
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'rad-126-167')
        self.assertEqual(p['tags'], ['Spirit'])
        self.assertEqual(p['effective']['tags'], ['Shadow Isles', 'Spirit'])
        self.assertEqual(p['effective'], self.proof['effective_after'])
        self.assertEqual([k for k in self.proof['effective_before']
            if self.proof['effective_before'][k] != self.proof['effective_after'][k]], ['tags'])
        self.assertEqual(p['resolution_ids'], ['rad-126-167:tags'])
        self.assertTrue(identity['standard_legal'])
        self.assertEqual(identity['official_ids'], ['rad-126-167'])
        self.assertEqual(identity['errata'], [])
        self.assertFalse(identity['implementation']['playable_in_client'])
        self.assertFalse(self.proof['enables_runtime_support'])
        c = self.snapshot['counts']
        self.assertEqual((c['implemented_main_cards'], c['implemented_legal_identities'],
                          c['unimplemented_legal_identities']), (372, 397, 682))

    def test_complete_raw_origin_and_independent_face_observation_are_pinned(self):
        p = self.proof
        raw = next(r for r in self.raw if r['official_id'] == 'rad-126-167')
        self.assertEqual(p['raw_printing'], raw)
        self.assertEqual(p['raw_printing_sha256'], review.printing_sha256(raw))
        origin = gzip.decompress((catalog.ROOT / p['origin_gallery']['archive']).read_bytes())
        self.assertEqual((len(origin), catalog.sha(origin)),
                         (p['origin_gallery']['uncompressed_bytes'], p['origin_gallery']['sha256']))
        self.assertEqual(catalog.project_gallery(origin)[0], self.raw)
        image = p['original_image']; payload = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual((len(payload), catalog.sha(payload)), (661398, image['sha256']))
        self.assertEqual(image['bytes'], len(payload))
        self.assertEqual(image['url'], raw['image_url'])
        observed = p['visual_observation']
        self.assertTrue(observed['face_visually_inspected'])
        self.assertEqual(observed['original_image_sha256'], image['sha256'])
        self.assertEqual(observed['printed_tags'], ['Shadow Isles', 'Spirit'])
        self.assertEqual(observed['printed_types'], ['unit'])
        self.assertEqual(observed['printed_domains'], ['order'])
        self.assertEqual((observed['printed_energy'], observed['printed_power'], observed['printed_might']), (4, None, 1))
        self.assertEqual(observed['collector_code'], 'RAD-126/167')
        self.assertEqual(observed['printed_rules_icons_expanded'], p['effective_after']['rules_text'])
        self.assertNotIn('/tmp/', json.dumps(p))

    def test_every_previous_source_cohort_and_runtime_recipe_is_preserved(self):
        p = self.proof; digest = lambda value: catalog.sha(review.canonical_bytes(value))
        for key, old in p['preserved_source_cohorts'].items():
            self.assertEqual(digest(self.manifest[key][:old['count']]), old['canonical_sha256'])
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        old = p['preserved_field_image_index']
        self.assertEqual(digest(images[:old['count']]), old['canonical_sha256'])
        v = p['preserved_variant_review']
        for path, key in [('assets/data/catalog_identity_reviews.json', 'manifest_sha256'),
                          ('assets/data/catalog_evidence/variant_images.json', 'image_index_sha256')]:
            self.assertEqual(catalog.sha((catalog.ROOT / path).read_bytes()), v[key])
        r = p['preserved_runtime']
        self.assertEqual(digest(self.runtime['cards']), r['cards_sha256'])
        self.assertEqual(digest(self.runtime['decks']), r['presets_sha256'])
        self.assertEqual(catalog.sha(history.legacy_legend_manifest_bytes()), r['legends_manifest_sha256'])
        self.assertEqual(catalog.sha(history.legacy_recipe_source_bytes()), r['training_recipe_source_sha256'])

    def test_changed_raw_prerequisites_fail_closed(self):
        for field, value in [('tags', ['Shadow Isles', 'Spirit']), ('might', 2), ('domains', ['chaos'])]:
            with self.subTest(field=field):
                raw = copy.deepcopy(self.raw)
                next(r for r in raw if r['official_id'] == 'rad-126-167')[field] = value
                with self.assertRaisesRegex(ValueError, 'Stale candidate printing hash'):
                    self.apply(raw=raw)

    def test_discovery_cannot_lose_its_resolution_or_substitute_its_image(self):
        for mode in ('resolution', 'image'):
            with self.subTest(mode=mode):
                m = copy.deepcopy(self.manifest)
                if mode == 'resolution':
                    m['resolutions'] = [r for r in m['resolutions'] if r['id'] != 'rad-126-167:tags']
                else:
                    r = next(r for r in m['resolutions'] if r['id'] == 'rad-126-167:tags')
                    r['evidence'][0]['sha256'] = '0' * 64
                with self.assertRaises(ValueError):
                    self.apply(manifest=m)


if __name__ == '__main__':
    unittest.main()
