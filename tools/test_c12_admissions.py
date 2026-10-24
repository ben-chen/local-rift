"""C12 source facts, artwork provenance, preserved cohorts and legal deck homes."""
import copy
import json
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog

ORDER = ['ogn-180-298', 'ogn-069-298', 'rad-095-167', 'unl-076-219',
         'unl-165-219', 'unl-083-219', 'unl-069-219', 'ogn-094-298',
         'ogn-106-298', 'unl-084-219', 'unl-048-219', 'unl-078-219', 'ogn-075-298']
BASE = 349


class C12AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.data = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.manifest, cls.data)
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c12/admissions.json').read_text())

    def test_complete_raw_effective_runtime_and_original_face_facts(self):
        runtime = catalog.load_runtime_manifest()['cards']
        raw = {p['official_id']: p for p in catalog.project_gallery(self.data['gallery'])[0]}
        prints = {p['official_id']: p for p in self.snapshot['printings']}
        identities = {p['identity']: p for p in self.snapshot['cards']}
        self.assertFalse(self.proof.get('draft_not_for_import', False))
        self.assertEqual([r['official_id'] for r in self.proof['records']], ORDER)
        self.assertEqual(catalog.sha(self.data['gallery']), self.proof['origin_gallery']['uncompressed_sha256'])
        for index, record in enumerate(self.proof['records'], BASE):
            with self.subTest(index=index):
                pid = record['official_id']
                e = prints[pid]['effective']
                identity = identities[pid]
                self.assertEqual(record['runtime_id'], index)
                self.assertEqual(record['raw_printing'], raw[pid])
                self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw[pid]))
                self.assertEqual(record['effective'], e)
                self.assertEqual(record['existing_field_resolution_ids'], prints[pid]['resolution_ids'])
                self.assertEqual(identity['official_ids'], [pid])
                self.assertEqual(identity['errata'], [])
                self.assertTrue(identity['standard_legal'])
                self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
                self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
                self.assertEqual(identity['implementation']['runtime_id'], index)
                self.assertTrue(identity['implementation']['playable_in_client'])
                for key in ['original_image', 'runtime_image']:
                    image = record[key]
                    self.assertFalse(image['path'].startswith('/'))
                    payload = (catalog.ROOT / image['path']).read_bytes()
                    self.assertEqual((len(payload), catalog.sha(payload)), (image['bytes'], image['sha256']))
                    self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))
                self.assertLess(record['runtime_image']['bytes'], 300_000)
                self.assertEqual(record['original_image']['url'], raw[pid]['image_url'])
                self.assertEqual(record['runtime_image']['url'], runtime[index]['art_url'])
                o = record['visual_observation']
                self.assertTrue(o['face_visually_inspected'])
                self.assertEqual(o['uncertainties'], [])
                self.assertEqual((o['official_id'], o['name'], o['collector_code']),
                                 (pid, raw[pid]['name'], raw[pid]['public_code']))
                self.assertEqual(o['original_image_sha256'], record['original_image']['sha256'])
                for face, effective in [
                    ('printed_types', 'types'), ('printed_supertypes', 'super_types'),
                    ('printed_domains', 'domains'), ('printed_tags', 'tags'),
                    ('printed_energy', 'energy'), ('printed_power', 'power'),
                    ('printed_might', 'might'), ('printed_might_bonus', 'might_bonus'),
                    ('printed_rules_icons_expanded', 'rules_text'),
                    ('printed_equipment_icons_expanded', 'equipment_text'),
                ]:
                    self.assertEqual(o[face], e[effective], face)
                self.assertEqual(runtime[index]['official_id'], pid)
                self.assertEqual(runtime[index]['official_text'], e['rules_text'])
                self.assertEqual(runtime[index]['official_text_html'], e['rules_html'])
                self.assertEqual(runtime[index]['tags'], e['tags'])
                self.assertEqual(runtime[index]['power'], e['power'] or 0)
                self.assertEqual(runtime[index]['energy'], e['energy'] or 0)
        self.assertEqual(sum(r['original_image']['bytes'] for r in self.proof['records']), 10_782_362)

    def test_original_cards_lists_and_complete_source_review_cohorts_are_preserved(self):
        root = catalog.ROOT
        runtime = catalog.load_runtime_manifest()
        p = self.proof['preserved_runtime_prefix']
        digest = lambda v: catalog.sha(review.canonical_bytes(v))
        self.assertEqual(p['card_count'], BASE)
        self.assertEqual(digest(runtime['cards'][:BASE]), p['cards_sha256'])
        self.assertEqual(digest(runtime['decks']), p['presets_sha256'])
        self.assertEqual(catalog.sha((root / 'assets/data/legends.json').read_bytes()), p['legends_manifest_sha256'])
        self.assertEqual(catalog.sha((root / 'src/deck.rs').read_bytes()), p['training_recipe_source_sha256'])
        p = self.proof['preserved_source_reviews']
        r = json.loads((root / 'assets/data/catalog_resolutions.json').read_text())
        identities = json.loads((root / 'assets/data/catalog_identity_reviews.json').read_text())
        images = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        for values, count, key in [
            (r['resolutions'], 115, 'field_resolutions_sha256'),
            (r['raw_printings'], 71, 'raw_printings_sha256'),
            (r['historical_issues'], 40, 'historical_issues_sha256'),
            (images, 71, 'field_image_index_sha256'),
            (identities['reviews'], 29, 'identity_reviews_sha256'),
        ]:
            # Later source reviews append entries; this admission pins its original cohort.
            self.assertGreaterEqual(len(values), count)
            self.assertEqual(digest(values[:count]), p[key])
        self.assertEqual(self.proof['pre_admission_counts'], {'main_cards': 349, 'mapped_legal': 374, 'remaining_legal': 705})

    def test_momentary_uses_prior_effective_tags_and_reuses_source_original(self):
        record = self.proof['records'][2]
        self.assertEqual(record['official_id'], 'rad-095-167')
        self.assertEqual(record['raw_printing']['tags'], [])
        self.assertEqual(record['effective']['tags'], ['Shadow Isles', 'Demon'])
        self.assertEqual(record['existing_field_resolution_ids'], ['rad-095-167:tags'])
        self.assertEqual(record['original_image']['path'], 'assets/data/catalog_evidence/images/rad-095-167.png')
        self.assertFalse((catalog.ROOT / 'assets/data/card_admissions/c12/originals/rad-095-167.png').exists())
        self.assertEqual([r['official_id'] for r in self.proof['records'] if r['existing_field_resolution_ids']], ['rad-095-167'])

    def test_sprite_token_references_do_not_become_constructed_cards(self):
        ids = {r['official_id'] for r in catalog.load_runtime_manifest()['cards']}
        self.assertTrue(ids.isdisjoint({'ogn-274-298', 'unl-t07'}))
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'ogn-274-298')
        self.assertTrue(identity['auxiliary'])
        self.assertFalse(identity['standard_legal'])
        c = self.snapshot['counts']
        self.assertEqual((c['implemented_main_cards'], c['implemented_legal_identities'], c['unimplemented_legal_identities']), (362, 387, 692))
        self.assertEqual(c['standard_legal_identities'], 1079)

    def test_fountain_is_rules_panel_gear_without_equipment_or_unit_might(self):
        record = self.proof['records'][11]
        self.assertEqual(record['official_id'], 'unl-078-219')
        e = record['effective']
        self.assertEqual(e['types'], ['gear'])
        self.assertEqual(e['tags'], [])
        self.assertIsNone(e['might'])
        self.assertIsNone(e['might_bonus'])
        self.assertEqual(e['equipment_text'], '')
        self.assertEqual(e['equipment_html'], '')
        self.assertIn('[Temporary]', e['rules_text'])
        self.assertIn('[Deathknell][>] Repeat this gear\'s play effect.', e['rules_text'])
        self.assertEqual(catalog.load_runtime_manifest()['cards'][BASE + 11]['might'], 0)

    def test_every_new_card_has_legal_existing_homes_without_champion_eligibility(self):
        legends = json.loads((catalog.ROOT / 'assets/data/legends.json').read_text())['legends']
        for record in self.proof['records']:
            e = record['effective']
            self.assertNotIn('champion', [s.lower() for s in e['super_types']])
            self.assertTrue(any(set(e['domains']) <= {d.lower() for d in l['domains']} for l in legends))
        self.assertEqual(len(legends), 8)

    def test_runtime_source_drift_is_rejected(self):
        runtime = catalog.load_runtime_manifest()
        for index, key, value in [
            (BASE, 'kind', 'Unit'), (BASE, 'energy', 9), (BASE, 'power', 0),
            (BASE + 2, 'tags', []), (BASE + 2, 'domains', ['Order']),
            (BASE + 3, 'might', 3), (BASE + 11, 'equipment_effect_html', '<p>Invented.</p>'),
            (BASE + 12, 'official_text_html', '<p>Invented.</p>'),
        ]:
            with self.subTest(index=index, field=key):
                changed = copy.deepcopy(runtime)
                changed['cards'][index][key] = value
                with patch.object(catalog, 'load_runtime_manifest', return_value=changed):
                    with self.assertRaisesRegex(ValueError, 'Runtime main-card .*mismatch'):
                        catalog.make_catalog(self.manifest, self.data)


if __name__ == '__main__':
    unittest.main()
