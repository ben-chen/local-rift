"""Control-card facts are checked before the engine may admit these cards."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class ControlSourceReview(unittest.TestCase):
    def test_original_faces_and_effective_facts_preserve_existing_recipes(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_control/source_review.json').read_text())
        expected = {
            'ogn-203-298': ('Possession', 8, 3,
                            '[Action] (Play on your turn or in showdowns.)\nChoose an enemy unit at a battlefield. Take control of it and recall it. (Send it to your base. This isn\'t a move.)'),
            'unl-140-219': ('Conscription', 5, 2,
                            'You may spend 5 XP as an additional cost to play this.\nChoose an enemy unit at a battlefield with 3 Might or less. If you paid the additional cost, choose any enemy unit at a battlefield instead. Take control of it, exhaust it, and recall it.'),
        }
        self.assertEqual({r['official_id'] for r in proof['records']}, set(expected))
        origin, archives = catalog.read_archives()
        raws = catalog.project_gallery(archives['gallery'])[0]
        inventory = catalog.make_catalog(origin, archives)
        for record in proof['records']:
            ident = record['official_id']
            with self.subTest(card=ident):
                name, energy, power, text = expected[ident]
                raw = next(p for p in raws if p['official_id'] == ident)
                self.assertEqual(record['raw_printing'], raw)
                self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
                self.assertEqual(raw['name'], name)
                printing = next(p for p in inventory['printings'] if p['official_id'] == ident)
                facts = printing['effective']
                self.assertEqual(record['effective'], facts)
                self.assertEqual((facts['types'], facts['super_types'], facts['domains'],
                                  facts['energy'], facts['power'], facts['might'], facts['tags'], facts['rules_text']),
                                 (['spell'], [], ['chaos'], energy, power, None, [], text))
                identity = next(c for c in inventory['cards'] if c['identity'] == ident)
                self.assertTrue(identity['standard_legal'])
                self.assertEqual(identity['errata'], [])
                self.assertEqual(printing['unresolved_gameplay_fields'], [])
                self.assertTrue(record['visual_review']['actual_full_face_viewed'])
                for key, value in record['visual_review']['printed_facts'].items():
                    self.assertEqual(facts[key], value, key)
                image = record['original_image']
                data = (catalog.ROOT / image['path']).read_bytes()
                self.assertEqual(image['url'], raw['image_url'])
                self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
                self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                self.assertEqual(list(struct.unpack('>II', data[16:24])), image['dimensions'])
                self.assertEqual(image['dimensions'], [744, 1039])
                if 'runtime_id' in record:
                    runtime = catalog.load_runtime_manifest()['cards'][record['runtime_id']]
                    self.assertEqual((runtime['name'], runtime['official_id']), (name, ident))
                    self.assertEqual((runtime['energy'], runtime['power'], runtime['official_text']),
                                     (energy, power, text))
                    thumb = record['runtime_image']
                    data = (catalog.ROOT / thumb['path']).read_bytes()
                    self.assertEqual((len(data), catalog.sha(data)), (thumb['bytes'], thumb['sha256']))
                    self.assertEqual(list(struct.unpack('>II', data[16:24])), thumb['dimensions'])
                    self.assertEqual(thumb['url'], runtime['art_url'])
                    self.assertEqual(thumb['url'].split('?')[0], image['url'].split('?')[0])
        runtime = catalog.load_runtime_manifest()
        pin = proof['preserved_runtime_prefix']
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'),
                                   ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[key][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
