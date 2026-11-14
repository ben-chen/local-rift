"""Flurry's full printed face and official data; runtime facts and preserved presets."""
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

    def test_runtime_uses_exact_reviewed_facts_and_preserves_earlier_cards_and_presets(self):
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'unl-044-219')
        self.assertTrue(identity['standard_legal'])
        self.assertEqual(identity['errata'], [])
        self.assertTrue(identity['implementation']['playable_in_client'])
        card = self.runtime['cards'][463]
        self.assertEqual(card['official_id'], 'unl-044-219')
        self.assertEqual((card['energy'], card['power'], card['might']), (4, 2, 0))
        self.assertEqual(card['keywords'], ['Reaction'])
        self.assertEqual(card['domains'], ['Calm'])
        self.assertEqual(card['tags'], [])
        effective = self.proof['records'][0]['effective']
        self.assertEqual(card['official_text'], effective['rules_text'])
        self.assertEqual(card['official_text_html'], effective['rules_html'])
        image = self.proof['records'][0]['runtime_image']
        data = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertEqual(struct.unpack('>II', data[16:24]), (280, 391))
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (463, 7))
        for field, count, digest in [('cards', 'card_count', 'cards_sha256'),
                                     ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[field][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
