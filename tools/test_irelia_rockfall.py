"""Rockfall's printed facts and actual face, independent of the runtime rule."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog


class RockfallAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_rockfall/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = next(p for p in catalog.project_gallery(archives['gallery'])[0] if p['official_id'] == 'sfd-216-221')
        cls.snapshot = catalog.make_catalog(origin, archives)

    def test_printed_facts_images_and_legal_identity(self):
        record = self.proof['record']
        self.assertEqual(record['raw_printing'], self.raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(self.raw))
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        field = catalog.load_battlefield_manifest()[17]
        self.assertEqual((field['source_id'], field['name'], field['official_html']),
                         ('sfd-216-221', 'Rockfall Path', "<p>Units can't be played here.</p>"))
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'sfd-216-221')
        self.assertTrue(identity['standard_legal'])
        self.assertTrue(identity['implementation']['playable_in_client'])
        self.assertEqual(identity['errata'], [])
        self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
        for key, dimensions in [('original_image', (1039, 744)), ('runtime_image', (360, 258))]:
            image = record[key]
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
            self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
            self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_existing_fields_cards_and_presets_remain_exact(self):
        pin = self.proof['preserved_runtime']
        runtime = catalog.load_runtime_manifest()
        for key, data in [('cards', runtime['cards']), ('presets', runtime['decks']),
                          ('battlefields', catalog.load_battlefield_manifest()[:17])]:
            self.assertEqual(catalog.sha(review.canonical_bytes(data)), pin[key + '_sha256'])


if __name__ == '__main__':
    unittest.main()
