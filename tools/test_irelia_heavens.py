"""Consult's reviewed Spell type, complete face and unchanged earlier recipes."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog


class ConsultHeavensAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_heavens/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = next(p for p in catalog.project_gallery(archives['gallery'])[0] if p['official_id'] == 'rad-025-167')
        cls.snapshot = catalog.make_catalog(origin, archives)
        cls.runtime = catalog.load_runtime_manifest()

    def test_printing_review_images_and_complete_runtime_text(self):
        record, = self.proof['records']
        self.assertEqual(record['raw_printing'], self.raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(self.raw))
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        card = self.runtime['cards'][457]
        self.assertEqual((card['official_id'], card['name'], card['energy'], card['power'], card['kind']),
                         ('rad-025-167', 'Consult the Heavens', 1, 0, 'Spell'))
        self.assertEqual(card['domains'], ['Calm'])
        self.assertEqual(card['keywords'], ['Reaction', 'Shield 2', 'Predict'])
        self.assertEqual(card['official_text'], self.raw['rules_text'])
        self.assertEqual(card['official_text_html'], self.raw['rules_html'])
        printing = next(p for p in self.snapshot['printings'] if p['official_id'] == card['official_id'])
        self.assertEqual(record['effective'], printing['effective'])
        self.assertEqual(record['resolution_ids'], ['rad-025-167:types'])
        self.assertEqual(printing['effective']['types'], ['spell'])
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == card['official_id'])
        self.assertTrue(identity['standard_legal'] and identity['implementation']['playable_in_client'])
        self.assertEqual(identity['errata'], [])
        for key, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
            image = record[key]
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
            self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_preserved_cards_and_seven_presets_are_exact(self):
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (457, 7))
        for field, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[field][:pin[count]])), pin[digest])
