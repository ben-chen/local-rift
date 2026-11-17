"""Reviewed official recovery faces, playable data, and existing recipe preservation."""
import hashlib
import json
import struct
import unittest

import import_catalog as catalog


class RecoverySources(unittest.TestCase):
    def test_reviewed_faces_runtime_facts_and_existing_recipes(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_recovery/source_review.json').read_text())
        origin, archives = catalog.read_archives()
        inventory = catalog.make_catalog(origin, archives)
        runtime = catalog.load_runtime_manifest()
        expected = [
            (473, 'ogn-174-298', 'Sai Scout', 6, 0, 5,
             "[Vision] (When you play me, look at the top card of your Main Deck. You may recycle it.)\nYou may play me to an open battlefield."),
            (474, 'ogn-165-298', 'Cemetery Attendant', 3, 1, 3,
             'When you play me, return a unit from your trash to your hand.'),
            (475, 'ogs-010-024', 'Annie, Stubborn', 4, 1, 3,
             'When you play me, return a spell from your trash to your hand.'),
        ]
        self.assertEqual(len(proof['records']), len(expected))
        for record, (index, ident, name, energy, power, might, text) in zip(proof['records'], expected, strict=True):
            with self.subTest(card=name):
                printing = next(p for p in inventory['printings'] if p['official_id'] == ident)
                identity = next(c for c in inventory['cards'] if c['identity'] == ident)
                facts = printing['effective']
                self.assertEqual(record['effective'], facts)
                self.assertEqual((record['runtime_id'], record['identity'], record['name']), (index, ident, name))
                self.assertTrue(record['actual_full_face_viewed'])
                self.assertTrue(identity['standard_legal'])
                self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
                self.assertEqual(identity['errata'], [])
                card = runtime['cards'][index]
                self.assertEqual((card['official_id'], card['name'], card['kind'], card['energy'], card['power'], card['might'], card['official_text']),
                                 (ident, name, 'Unit', energy, power, might, text))
                self.assertEqual((facts['types'], facts['domains'], card['domains']), (['unit'], ['chaos'], ['Chaos']))
                self.assertEqual(card['tags'], facts['tags'])
                self.assertEqual(card['official_text_html'], facts['rules_html'])
                self.assertEqual(record['original_image']['url'], printing['image_url'])
                self.assertEqual(record['runtime_image']['url'], card['art_url'])
                for field, size in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
                    image = record[field]
                    data = (catalog.ROOT / image['path']).read_bytes()
                    self.assertEqual(hashlib.sha256(data).hexdigest(), image['sha256'])
                    self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                    self.assertEqual(struct.unpack('>II', data[16:24]), size)
        for name, data in [('main_cards', runtime['cards']), ('presets', runtime['decks'])]:
            pin = proof['preserved_runtime_prefix'][name]
            encoded = json.dumps(data[:pin['count']], sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
            self.assertEqual(hashlib.sha256(encoded).hexdigest(), pin['sha256'])


if __name__ == '__main__':
    unittest.main()
