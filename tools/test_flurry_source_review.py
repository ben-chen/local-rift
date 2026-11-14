"""Flurry's full printed face and official data; no gameplay admission implied."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class FlurrySourceReview(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_flurry/source_review.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = next(p for p in catalog.project_gallery(archives['gallery'])[0]
                       if p['official_id'] == 'unl-044-219')
        cls.snapshot = catalog.make_catalog(origin, archives)
        cls.runtime = catalog.load_runtime_manifest()

    def test_complete_source_and_actual_full_face_are_preserved(self):
        record, = self.proof['records']
        self.assertEqual(record['raw_printing'], self.raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(self.raw))
        printing = next(p for p in self.snapshot['printings'] if p['official_id'] == 'unl-044-219')
        self.assertEqual(record['effective'], printing['effective'])
        self.assertEqual(record['resolution_ids'], [])
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        for key, value in record['visual_review']['printed_facts'].items():
            self.assertEqual(record['effective'][key], value, key)
        self.assertEqual((record['effective']['energy'], record['effective']['power']), (4, 2))
        image = record['original_image']
        data = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual(image['url'], self.raw['image_url'])
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039))

    def test_source_review_does_not_admit_a_runtime_card_or_a_preset(self):
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'unl-044-219')
        self.assertTrue(identity['standard_legal'])
        self.assertEqual(identity['errata'], [])
        self.assertFalse(identity['implementation']['playable_in_client'])
        self.assertNotIn('unl-044-219', {c['official_id'] for c in self.runtime['cards']})
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (463, 7))
        for field, count, digest in [('cards', 'card_count', 'cards_sha256'),
                                     ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[field][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
