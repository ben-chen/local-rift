"""Complete card faces, source-only Brawl correction, and preserved admissions."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog
from catalog_test_support import assert_review_inventory


class TurnKeywordsSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/turn_keywords/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(archives['gallery'])[0]}
        cls.snapshot = catalog.make_catalog(origin, archives)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}
        cls.runtime = catalog.load_runtime_manifest()

    def test_all_three_full_faces_match_complete_reviewed_facts(self):
        self.assertEqual({r['official_id'] for r in self.proof['records']},
                         {'ogn-057-298', 'unl-056-219', 'rad-004-167'})
        for rec in self.proof['records']:
            pid = rec['official_id']
            with self.subTest(printing=pid):
                self.assertEqual(rec['raw_printing'], self.raw[pid])
                self.assertEqual(rec['raw_printing_sha256'], review.printing_sha256(self.raw[pid]))
                self.assertEqual(rec['effective'], self.printings[pid]['effective'])
                self.assertEqual(rec['resolution_ids'], self.printings[pid]['resolution_ids'])
                self.assertTrue(rec['visual_review']['actual_full_face_viewed'])
                for field, value in rec['visual_review']['printed_facts'].items():
                    self.assertEqual(value, rec['effective'][field], field)
                image = rec['original_image']
                self.assertEqual(image['url'], self.raw[pid]['image_url'])
                data = (catalog.ROOT / image['path']).read_bytes()
                self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
                self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039))

    def test_brawl_is_fury_and_source_correction_never_admits_runtime_support(self):
        pid = 'rad-004-167'
        self.assertEqual(self.raw[pid]['domains'], ['chaos'])
        self.assertEqual(self.printings[pid]['effective']['domains'], ['fury'])
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == pid)
        self.assertFalse(identity['implementation']['playable_in_client'])
        self.assertNotIn(pid, {c['official_id'] for c in self.runtime['cards']})
        assert_review_inventory(self, self.snapshot)

    def test_preserves_all_earlier_source_reviews_runtime_cards_and_presets(self):
        manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        for field, pin in self.proof['preserved_source_prefix'].items():
            values = images if field == 'images' else manifest[field]
            self.assertEqual(catalog.sha(review.canonical_bytes(values[:pin['count']])), pin['sha256'], field)
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (461, 7))
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[key][:pin[count]])), pin[digest])

    def test_block_and_yuumi_runtime_text_art_and_facts_are_complete(self):
        expected = {461: ('ogn-057-298', 'Spell', 2, 0, 0), 462: ('unl-056-219', 'Unit', 3, 1, 1)}
        for rec in self.proof['records']:
            if 'runtime_id' not in rec:
                continue
            i = rec['runtime_id']
            card = self.runtime['cards'][i]
            self.assertEqual((card['official_id'], card['kind'], card['energy'], card['power'], card['might']), expected[i])
            self.assertEqual(card['domains'], ['Calm'])
            self.assertEqual(card['tags'], rec['effective']['tags'])
            self.assertEqual(card['official_text'], rec['effective']['rules_text'])
            self.assertEqual(card['official_text_html'], rec['effective']['rules_html'])
            image = rec['runtime_image']
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
            self.assertEqual(struct.unpack('>II', data[16:24]), (280, 391))
        self.assertEqual(self.runtime['cards'][461]['keywords'], ['Hidden', 'Action', 'Shield 3', 'Tank'])
        self.assertEqual(self.runtime['cards'][462]['keywords'], [], 'Yuumi grants Tank; she does not have it intrinsically')


if __name__ == '__main__':
    unittest.main()
