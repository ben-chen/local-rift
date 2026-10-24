"""Exact C9 face evidence and append-only registry/source-review boundaries."""
import json
import unittest

import catalog_review as review
import import_catalog as catalog


class C9AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.data = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.manifest, cls.data)
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c9/admissions.json').read_text())

    def test_all_original_faces_match_pinned_raw_and_effective_facts(self):
        runtime = catalog.load_runtime_manifest()['cards']
        raw = {p['official_id']: p for p in catalog.project_gallery(self.data['gallery'])[0]}
        prints = {p['official_id']: p for p in self.snapshot['printings']}
        identities = {p['identity']: p for p in self.snapshot['cards']}
        self.assertEqual(len(self.proof['records']), 26)
        self.assertEqual(catalog.sha(self.data['gallery']),
                         self.proof['origin_gallery']['uncompressed_sha256'])
        for index, record in enumerate(self.proof['records'], 308):
            with self.subTest(index=index):
                pid = record['official_id']
                effective = prints[pid]['effective']
                self.assertEqual(record['runtime_id'], index)
                self.assertEqual(record['raw_printing'], raw[pid])
                self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw[pid]))
                self.assertEqual(record['effective'], effective)
                self.assertEqual(record['existing_field_resolution_ids'], prints[pid]['resolution_ids'])
                self.assertEqual(identities[pid]['official_ids'], [pid])
                self.assertEqual(identities[pid]['errata'], [])
                self.assertTrue(identities[pid]['standard_legal'])
                self.assertEqual(identities[pid]['source_review']['unresolved_variant_fields'], [])
                self.assertEqual(identities[pid]['source_review']['unresolved_gameplay_fields'], [])
                self.assertEqual(identities[pid]['implementation']['runtime_id'], index)
                for key in ['original_image', 'runtime_image']:
                    image = record[key]
                    self.assertFalse(image['path'].startswith('/'))
                    payload = (catalog.ROOT / image['path']).read_bytes()
                    self.assertEqual((len(payload), catalog.sha(payload)),
                                     (image['bytes'], image['sha256']))
                    self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))
                self.assertEqual(record['original_image']['url'], raw[pid]['image_url'])
                self.assertEqual(record['runtime_image']['url'], runtime[index]['art_url'])
                observation = record['visual_observation']
                self.assertTrue(observation['face_visually_inspected'])
                self.assertTrue(all(observation['comparison_to_current_effective'].values()))
                self.assertEqual(observation['printed_domains'], effective['domains'])
                self.assertEqual(set(observation['printed_tags_in_face_order']), set(effective['tags']))
                self.assertEqual(observation['printed_rules_readable'].casefold(),
                                 effective['rules_text'].casefold())
                self.assertEqual(observation['printed_equipment_readable'], effective['equipment_text'])
                text = effective['rules_text']
                if effective['equipment_text']:
                    text += f"\nAttached unit (+{effective['might_bonus'] or 0} Might): {effective['equipment_text']}"
                self.assertEqual(runtime[index]['official_text'], text)
        self.assertEqual(self.proof['records'][-1]['original_image']['path'],
                         'assets/data/catalog_evidence/images/rad-003-167.png')

    def test_old_runtime_entries_presets_legends_and_source_reviews_are_preserved(self):
        root = catalog.ROOT
        runtime = catalog.load_runtime_manifest()
        preserved = self.proof['preserved_runtime_prefix']
        digest = lambda value: catalog.sha(review.canonical_bytes(value))
        self.assertEqual(preserved['card_count'], 308)
        self.assertEqual(digest(runtime['cards'][:308]), preserved['cards_sha256'])
        self.assertEqual(digest(runtime['decks']), preserved['presets_sha256'])
        self.assertEqual(catalog.sha((root / 'assets/data/legends.json').read_bytes()),
                         preserved['legends_manifest_sha256'])
        p = self.proof['preserved_source_reviews']
        resolutions = json.loads((root / 'assets/data/catalog_resolutions.json').read_text())
        identity_reviews = json.loads((root / 'assets/data/catalog_identity_reviews.json').read_text())
        images = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        for values, count, key in [
            (resolutions['resolutions'], 112, 'field_resolutions_sha256'),
            (resolutions['raw_printings'], 69, 'raw_printings_sha256'),
            (resolutions['historical_issues'], 37, 'historical_issues_sha256'),
            (images, 69, 'field_image_index_sha256'),
            (identity_reviews['reviews'], 29, 'identity_reviews_sha256'),
        ]:
            self.assertEqual(digest(values[:count]), p[key])
        checkpoint = self.proof['source_only_checkpoint']
        self.assertEqual((checkpoint['main_cards'], checkpoint['mapped_legal'],
                          checkpoint['remaining_legal']), (308, 333, 746))
        self.assertFalse(checkpoint['tracker_playable'])


if __name__ == '__main__':
    unittest.main()
