"""Independently reviewed Kha'Zix facts; existing runtime identities stay fixed."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class IreliaKhazixAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_khazix/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = {row['official_id']: row for row in catalog.project_gallery(archives['gallery'])[0]}
        cls.snapshot = catalog.make_catalog(origin, archives)

    def test_both_original_faces_and_effective_facts_are_reviewed(self):
        record, = self.proof['records']
        pid = 'unl-143-219'
        identity = next(row for row in self.snapshot['cards'] if row['identity'] == pid)
        printing = next(row for row in self.snapshot['printings'] if row['official_id'] == pid)
        self.assertEqual(identity['official_ids'], ['unl-143-219', 'unl-143a-219'])
        self.assertEqual(record['printing_cohort'], identity['official_ids'])
        self.assertEqual(record['raw_printing'], self.raw[pid])
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(self.raw[pid]))
        self.assertEqual(record['effective'], printing['effective'])
        self.assertEqual((record['resolution_ids'], record['errata']), ([], []))
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        self.assertTrue(identity['standard_legal'])
        self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
        effect = 'When I attack or defend, if an enemy unit is alone here, give me +2 Might this turn and gain 2 XP.'
        for member in identity['official_ids']:
            row = self.raw[member]
            self.assertEqual((row['energy'], row['power'], row['might']), (4, 1, 4))
            self.assertEqual((row['domains'], row['types'], row['super_types']), (['chaos'], ['unit'], ['champion']))
            self.assertEqual(row['tags'], ["Kha'Zix", 'The Void'])
            self.assertTrue(row['rules_text'].startswith('[Ambush]'))
            self.assertTrue(row['rules_text'].endswith(effect))
        for field, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
            image = record[field]
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual(catalog.sha(data), image['sha256'])
            self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_admission_preserves_existing_cards_and_seven_tournament_lists(self):
        manifest = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (449, 7))
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['cards'][:449])), pin['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['decks'][:7])), pin['presets_sha256'])
        row = manifest['cards'][449]
        self.assertEqual(row['official_id'], 'unl-143-219')
        self.assertEqual((row['energy'], row['power'], row['might']), (4, 1, 4))
        self.assertEqual((row['domains'], row['kind'], row['keywords']), (['Chaos'], 'Unit', ['Ambush']))
        self.assertEqual(row['tags'], ["Kha'Zix", 'The Void'])
        self.assertEqual(row['official_text'], self.raw[row['official_id']]['rules_text'])
        self.assertEqual(row['art_url'], self.proof['records'][0]['runtime_image']['url'])


if __name__ == '__main__':
    unittest.main()
