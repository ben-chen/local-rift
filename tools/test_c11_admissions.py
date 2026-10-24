"""C11's four face admissions and unchanged earlier source boundaries."""
import copy
import json
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog


class C11AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.data = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.manifest, cls.data)
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c11/admissions.json').read_text())

    def test_exact_original_faces_raw_effective_and_runtime_facts(self):
        runtime = catalog.load_runtime_manifest()['cards']
        raw = {p['official_id']: p for p in catalog.project_gallery(self.data['gallery'])[0]}
        prints = {p['official_id']: p for p in self.snapshot['printings']}
        identities = {p['identity']: p for p in self.snapshot['cards']}
        self.assertEqual(len(self.proof['records']), 4)
        self.assertEqual(catalog.sha(self.data['gallery']), self.proof['origin_gallery']['uncompressed_sha256'])
        for index, record in enumerate(self.proof['records'], 345):
            with self.subTest(index=index):
                pid = record['official_id']
                effective = prints[pid]['effective']
                identity = identities[pid]
                self.assertEqual(record['runtime_id'], index)
                self.assertEqual(record['raw_printing'], raw[pid])
                self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw[pid]))
                self.assertEqual(record['effective'], effective)
                self.assertEqual(record['existing_field_resolution_ids'], [])
                self.assertEqual(prints[pid]['resolution_ids'], [])
                self.assertEqual(identity['official_ids'], [pid])
                self.assertEqual(identity['errata'], [])
                self.assertTrue(identity['standard_legal'])
                self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
                self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
                self.assertEqual(identity['implementation']['runtime_id'], index)
                for key in ['original_image', 'runtime_image']:
                    image = record[key]
                    self.assertFalse(image['path'].startswith('/'))
                    payload = (catalog.ROOT / image['path']).read_bytes()
                    self.assertEqual((len(payload), catalog.sha(payload)), (image['bytes'], image['sha256']))
                    self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))
                self.assertLess(record['runtime_image']['bytes'], 300_000)
                self.assertEqual(record['original_image']['url'], raw[pid]['image_url'])
                self.assertEqual(record['runtime_image']['url'], runtime[index]['art_url'])
                observation = record['visual_observation']
                self.assertTrue(observation['face_visually_inspected'])
                self.assertEqual(observation['uncertainties'], [])
                self.assertEqual(observation['official_id'], pid)
                self.assertEqual(observation['name'], raw[pid]['name'])
                self.assertEqual(observation['collector_code'], raw[pid]['public_code'])
                self.assertEqual(observation['original_image_sha256'], record['original_image']['sha256'])
                for face_key, effective_key in [
                    ('printed_types', 'types'), ('printed_supertypes', 'super_types'),
                    ('printed_domains', 'domains'), ('printed_tags', 'tags'),
                    ('printed_energy', 'energy'), ('printed_power', 'power'),
                    ('printed_might_bonus', 'might_bonus'),
                    ('printed_rules_icons_expanded', 'rules_text'),
                    ('printed_equipment_icons_expanded', 'equipment_text'),
                ]:
                    self.assertEqual(observation[face_key], effective[effective_key], face_key)
                self.assertEqual(observation['printed_might'], effective['might'] if effective['types'] == ['unit'] else None)
                self.assertEqual(runtime[index]['might'], (effective['might_bonus'] or 0) if effective['types'] == ['gear'] else effective['might'])
                self.assertEqual(runtime[index]['official_text'], effective['rules_text'])
                self.assertEqual(runtime[index]['official_text_html'], effective['rules_html'])
                self.assertEqual(runtime[index]['power'], effective['power'] or 0)
                self.assertEqual(runtime[index]['energy'], effective['energy'] or 0)
        self.assertEqual(sum(r['original_image']['bytes'] for r in self.proof['records']), 3_198_855)
        self.assertEqual(sum(r['runtime_image']['bytes'] for r in self.proof['records']), 1_065_020)

    def test_previous_cards_lists_recipes_and_all_source_reviews_remain_unchanged(self):
        root = catalog.ROOT
        runtime = catalog.load_runtime_manifest()
        preserved = self.proof['preserved_runtime_prefix']
        digest = lambda value: catalog.sha(review.canonical_bytes(value))
        self.assertEqual(preserved['card_count'], 345)
        self.assertEqual(digest(runtime['cards'][:345]), preserved['cards_sha256'])
        self.assertEqual(digest(runtime['decks']), preserved['presets_sha256'])
        self.assertEqual(catalog.sha((root / 'assets/data/legends.json').read_bytes()), preserved['legends_manifest_sha256'])
        self.assertEqual(catalog.sha((root / 'src/deck.rs').read_bytes()), preserved['training_recipe_source_sha256'])
        p = self.proof['preserved_source_reviews']
        resolutions = json.loads((root / 'assets/data/catalog_resolutions.json').read_text())
        identities = json.loads((root / 'assets/data/catalog_identity_reviews.json').read_text())
        images = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        for values, count, key in [
            (resolutions['resolutions'], 114, 'field_resolutions_sha256'),
            (resolutions['raw_printings'], 70, 'raw_printings_sha256'),
            (resolutions['historical_issues'], 39, 'historical_issues_sha256'),
            (images, 70, 'field_image_index_sha256'),
            (identities['reviews'], 29, 'identity_reviews_sha256'),
        ]:
            # Later source-only reviews append records; this admission pins its exact original cohort.
            self.assertGreaterEqual(len(values), count)
            self.assertEqual(digest(values[:count]), p[key])
        self.assertEqual(self.proof['pre_admission_counts'], {'main_cards': 345, 'mapped_legal': 370, 'remaining_legal': 709})

    def test_heirloom_keeps_raw_stat_and_empty_effect_panel_without_a_source_correction(self):
        record = self.proof['records'][3]
        raw = record['raw_printing']
        self.assertEqual(raw['official_id'], 'unl-158-219')
        self.assertEqual((raw['might'], raw['might_bonus']), (2, 2))
        self.assertIsNone(record['visual_observation']['printed_might'])
        self.assertEqual(record['visual_observation']['printed_might_bonus'], 2)
        self.assertEqual(raw['equipment_html'], '')
        self.assertEqual(raw['equipment_text'], '')
        self.assertEqual(record['effective']['rules_text'],
                         'When you play this, gain 1 XP.\n[Equip] — Spend 1 XP (Pay the cost: Attach this to a unit you control.)')
        self.assertEqual(record['existing_field_resolution_ids'], [])
        self.assertIn('attached +2 bonus', record['interpretation_note'])
        self.assertEqual(catalog.load_runtime_manifest()['cards'][348]['might'], 2)

    def test_importer_rejects_changed_new_card_source_facts(self):
        runtime = catalog.load_runtime_manifest()
        for key, changed in [
            ('kind', 'Spell'), ('energy', 9), ('power', 2), ('might', 6),
            ('domains', ['Fury']), ('tags', ['Invented']),
            ('official_text_html', '<p>Invented.</p>'),
            ('equipment_effect_html', '<p>Invented.</p>'),
        ]:
            with self.subTest(field=key):
                mutated = copy.deepcopy(runtime)
                mutated['cards'][345][key] = changed
                with patch.object(catalog, 'load_runtime_manifest', return_value=mutated):
                    with self.assertRaisesRegex(ValueError, 'Runtime main-card .*mismatch'):
                        catalog.make_catalog(self.manifest, self.data)


if __name__ == '__main__':
    unittest.main()
