"""Official faces for the four remaining cards in the reviewed Irelia sample."""
import hashlib
import json
import struct
import unittest

import import_catalog as catalog


class FieldOptionsSource(unittest.TestCase):
    def test_reviewed_faces_and_existing_recipes(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_field_options/source_review.json').read_text())
        origin, archives = catalog.read_archives()
        inventory = catalog.make_catalog(origin, archives)
        expected = {
            'ven-164-166': ('Sandswept Tomb', ['battlefield'], None, None, None,
                'Each spell that chooses one or more units here that are friendly to it costs [Any Power] less.'),
            'ven-115-166': ('Ocean Drake', ['unit'], 8, 2, 7,
                "You may play me to an open battlefield.\nWhen you play me, you may return a non-Dragon unit to its owner's hand."),
            'rad-046-167': ('Resplendent Rostrum', ['gear'], 8, 1, None,
                '[Exhaust]: Draw 1 and channel 1 rune exhausted.\nWhen I become exhausted, give a friendly unit +3 Might this turn.'),
            'ogn-176-298': ('Sneaky Deckhand', ['unit'], 3, None, 2,
                'You may play me to an open battlefield.'),
        }
        self.assertEqual({r['identity'] for r in proof['records']}, set(expected))
        for record in proof['records']:
            ident = record['identity']
            with self.subTest(card=ident):
                printing = next(p for p in inventory['printings'] if p['official_id'] == ident)
                facts = printing['effective']
                self.assertEqual(record['effective'], facts)
                self.assertEqual((printing['name'], facts['types'], facts['energy'], facts['power'], facts['might'], facts['rules_text']), expected[ident])
                self.assertTrue(record['actual_full_face_viewed'])
                image = record['original_image']
                data = (catalog.ROOT / image['path']).read_bytes()
                self.assertEqual(image['url'], printing['image_url'])
                self.assertEqual(hashlib.sha256(data).hexdigest(), image['sha256'])
                self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                self.assertEqual(list(struct.unpack('>II', data[16:24])), image['dimensions'])
                identity = next(c for c in inventory['cards'] if c['identity'] == ident)
                self.assertTrue(identity['standard_legal'])
                self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
                self.assertEqual(identity['errata'], [])
        runtime = catalog.load_runtime_manifest()
        fields = catalog.load_battlefield_manifest()
        for name, data in [('main_cards', runtime['cards']), ('presets', runtime['decks']), ('battlefields', fields)]:
            pin = proof['preserved_runtime_prefix'][name]
            encoded = json.dumps(data[:pin['count']], sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
            self.assertEqual(hashlib.sha256(encoded).hexdigest(), pin['sha256'])
        tomb = next(r for r in proof['records'] if r['identity'] == 'ven-164-166')
        self.assertEqual(fields[21]['official_text'], tomb['effective']['rules_text'])
        thumb = tomb['runtime_image']
        data = (catalog.ROOT / thumb['path']).read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), thumb['sha256'])
        self.assertEqual(struct.unpack('>II', data[16:24]), (360, 258))


if __name__ == '__main__':
    unittest.main()
