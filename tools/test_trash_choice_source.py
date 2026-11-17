"""Printed public-trash effects and the precise runtime admission boundary."""
import hashlib
import json
import struct
import unittest

import import_catalog as catalog


class TrashChoiceSources(unittest.TestCase):
    def test_reviewed_faces_and_runtime_cards(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_trash_targets/source_review.json').read_text())
        origin, archives = catalog.read_archives()
        inventory = catalog.make_catalog(origin, archives)
        runtime = catalog.load_runtime_manifest()
        expected = [
            ('sfd-035-221', 'Guardian of the Passage', 6, 0, 6, ['calm'],
             'When I hold, you may return a unit or gear from your trash to your hand.'),
            ('ogn-196-298', 'Soulgorger', 8, 2, 5, ['chaos'],
             'When you play me, you may play a unit from your trash, ignoring its Energy cost. (You must still pay its Power cost.)'),
            ('ogn-198-298', 'The Harrowing', 6, 2, None, ['chaos'],
             'Play a unit from your trash, ignoring its Energy cost. (You must still pay its Power cost.)'),
            ('ven-103-166', 'Shadows of the Past', 3, 1, None, ['chaos'],
             "Return up to 2 units from trashes to their owners' hands."),
        ]
        for record, facts in zip(proof['records'], expected, strict=True):
            ident, name, energy, power, might, domains, text = facts
            with self.subTest(card=name):
                printing = next(p for p in inventory['printings'] if p['official_id'] == ident)
                identity = next(c for c in inventory['cards'] if c['identity'] == ident)
                effective = printing['effective']
                self.assertEqual(record['effective'], effective)
                self.assertEqual((record['identity'], record['name']), (ident, name))
                self.assertEqual((effective['energy'], effective['power'] or 0, effective['might'], effective['domains'], effective['rules_text']),
                                 (energy, power, might, domains, text))
                self.assertTrue(record['actual_full_face_viewed'])
                self.assertTrue(identity['standard_legal'])
                self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
                self.assertEqual(identity['errata'], [])
                images = [('original_image', (744, 1039))]
                index = 476 + expected.index(facts)
                self.assertEqual(record['runtime_id'], index)
                card = runtime['cards'][index]
                kind = 'Unit' if might is not None else 'Spell'
                self.assertEqual((card['official_id'], card['name'], card['kind'], card['energy'], card['power'], card['might'], card['official_text']),
                                 (ident, name, kind, energy, power, might or 0, text))
                self.assertEqual(card['domains'], [domain.title() for domain in domains])
                self.assertEqual(card['tags'], effective['tags'])
                self.assertEqual(card['official_text_html'], effective['rules_html'])
                self.assertEqual(card['art_url'], record['runtime_image']['url'])
                images.append(('runtime_image', (280, 391)))
                self.assertEqual(record['original_image']['url'], printing['image_url'])
                for field, dimensions in images:
                    image = record[field]
                    data = (catalog.ROOT / image['path']).read_bytes()
                    self.assertEqual(hashlib.sha256(data).hexdigest(), image['sha256'])
                    self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                    self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)
        for name, data in [('main_cards', runtime['cards']), ('presets', runtime['decks'])]:
            pin = proof['preserved_runtime_prefix'][name]
            encoded = json.dumps(data[:pin['count']], sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
            self.assertEqual(hashlib.sha256(encoded).hexdigest(), pin['sha256'])


if __name__ == '__main__':
    unittest.main()
