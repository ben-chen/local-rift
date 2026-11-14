"""Independent gallery facts and immutable artwork for Royal Entourage."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog


class RoyalEntourageAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_entourage/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = next(p for p in catalog.project_gallery(archives['gallery'])[0] if p['official_id'] == 'sfd-039-221')
        cls.snapshot = catalog.make_catalog(origin, archives)
        cls.runtime = catalog.load_runtime_manifest()

    def test_printed_facts_images_and_legal_identity(self):
        record, = self.proof['records']
        self.assertEqual(record['raw_printing'], self.raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(self.raw))
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        card = self.runtime['cards'][455]
        self.assertEqual((card['official_id'], card['name'], card['energy'], card['power'], card['might'], card['kind']),
                         ('sfd-039-221', 'Royal Entourage', 3, 1, 4, 'Unit'))
        self.assertEqual(card['domains'], ['Calm'])
        self.assertEqual(card['tags'], ['Shurima'])
        self.assertEqual(card['keywords'], [])
        self.assertEqual(card['official_text_html'], self.raw['rules_html'])
        self.assertEqual(card['official_text'], 'When you play me, ready or exhaust a legend.')
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'sfd-039-221')
        self.assertTrue(identity['standard_legal'])
        self.assertTrue(identity['implementation']['playable_in_client'])
        self.assertEqual(identity['errata'], [])
        self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
        for field, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
            image = record[field]
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
            self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
            self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_previous_cards_and_seven_presets_remain_exact(self):
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (455, 7))
        for field, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[field][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
