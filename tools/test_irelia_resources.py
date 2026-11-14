"""Resource-card facts, original artwork, current errata, and prior deck preservation."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog


class IreliaResourceAdmissions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_resources/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(archives['gallery'])[0]}
        cls.snapshot = catalog.make_catalog(origin, archives)
        cls.runtime = catalog.load_runtime_manifest()

    def test_three_faces_and_runtime_facts_match_official_sources(self):
        expected = [(451, 'ogn-044-298', 'Clockwork Keeper', 2, 2, 'Unit'),
                    (452, 'ogn-048-298', 'Meditation', 2, 0, 'Spell'),
                    (453, 'rad-029-167', 'Life from Stone', 1, 0, 'Spell')]
        self.assertEqual(len(self.proof['records']), len(expected))
        for record, (runtime_id, official_id, name, energy, might, kind) in zip(self.proof['records'], expected, strict=True):
            with self.subTest(card=name):
                raw = self.raw[official_id]
                self.assertEqual((record['runtime_id'], record['official_id']), (runtime_id, official_id))
                self.assertEqual(record['raw_printing'], raw)
                self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
                self.assertTrue(record['visual_review']['actual_full_face_viewed'])
                row = self.runtime['cards'][runtime_id]
                self.assertEqual((row['name'], row['energy'], row['power'], row['might'], row['kind']), (name, energy, 0, might, kind))
                self.assertEqual(row['domains'], ['Calm'])
                self.assertEqual(row['official_text_html'], raw['rules_html'])
                self.assertEqual(row['official_text'], record['runtime_rules_text'])
                self.assertEqual(row['art_url'], record['runtime_image']['url'])
                identity = next(c for c in self.snapshot['cards'] if c['identity'] == official_id)
                self.assertTrue(identity['standard_legal'])
                self.assertTrue(identity['implementation']['playable_in_client'])
                self.assertEqual(identity['errata'], record['errata'])
                for field, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
                    image = record[field]
                    data = (catalog.ROOT / image['path']).read_bytes()
                    self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
                    self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                    self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_keeper_uses_errata_while_preserving_original_printing(self):
        keeper, meditation, life = self.proof['records']
        self.assertEqual(keeper['runtime_rules_text'], 'You may pay [Calm Power] as an additional cost to play me.\nWhen you play me, if you paid the additional cost, draw 1.')
        self.assertNotEqual(keeper['runtime_rules_text'], keeper['raw_printing']['rules_text'])
        self.assertEqual(keeper['errata'][0]['text'].replace('[C]', '[Calm Power]'), keeper['runtime_rules_text'])
        for record in [meditation, life]:
            self.assertEqual(record['errata'], [])
            self.assertEqual(record['runtime_rules_text'], record['raw_printing']['rules_text'])

    def test_previous_cards_and_seven_presets_remain_exact(self):
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (451, 7))
        for field, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[field][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
