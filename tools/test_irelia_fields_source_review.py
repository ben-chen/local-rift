"""Independent printed facts for three additional Irelia battlefield leads."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class IreliaBattlefieldSourceReview(unittest.TestCase):
    def test_original_faces_and_effective_facts_preserve_existing_runtime(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_fields/source_review.json').read_text())
        expected = {
            'sfd-209-221': ('Forgotten Monument', "Players can't score here until their third turn.", (1039, 744)),
            'ogn-277-298': ('Back-Alley Bar', 'When a unit moves from here, give it +1 Might this turn.', (1038, 744)),
            'ogn-291-298': ('The Candlelit Sanctum', "When you conquer here, look at the top two cards of your Main Deck. You may recycle one or both of them. Put those you don't back in any order.", (1038, 744)),
        }
        self.assertEqual({r['official_id'] for r in proof['records']}, set(expected))
        origin, archives = catalog.read_archives()
        raws = catalog.project_gallery(archives['gallery'])[0]
        inventory = catalog.make_catalog(origin, archives)
        for record in proof['records']:
            ident = record['official_id']
            with self.subTest(card=ident):
                name, text, size = expected[ident]
                raw = next(p for p in raws if p['official_id'] == ident)
                self.assertEqual(record['raw_printing'], raw)
                self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
                self.assertEqual(raw['name'], name)
                printing = next(p for p in inventory['printings'] if p['official_id'] == ident)
                facts = printing['effective']
                self.assertEqual(record['effective'], facts)
                self.assertEqual((facts['types'], facts['domains'], facts['rules_text']),
                                 (['battlefield'], ['colorless'], text))
                for key in ['energy', 'power', 'might']:
                    self.assertIsNone(facts[key])
                card = next(c for c in inventory['cards'] if c['identity'] == ident)
                self.assertTrue(card['standard_legal'])
                self.assertEqual(card['errata'], [])
                image = record['original_image']
                data = (catalog.ROOT / image['path']).read_bytes()
                self.assertEqual(image['url'], raw['image_url'])
                self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
                self.assertEqual(struct.unpack('>II', data[16:24]), size)
                self.assertTrue(record['visual_review']['actual_full_face_viewed'])
                for key, value in record['visual_review']['printed_facts'].items():
                    self.assertEqual(value, facts[key])
                runtime_image = record['runtime_image']
                thumbnail = (catalog.ROOT / runtime_image['path']).read_bytes()
                self.assertEqual(catalog.sha(thumbnail), runtime_image['sha256'])
                self.assertEqual(struct.unpack('>II', thumbnail[16:24]), (360, 258))
                self.assertEqual(runtime_image['url'], raw['image_url'] + '&w=360')
                self.assertEqual(runtime_image['path'], f"assets/art/battlefield_{record['runtime_id']}.png")
        runtime = catalog.load_runtime_manifest()
        runtime['battlefields'] = json.loads((catalog.ROOT / 'assets/data/battlefields.json').read_text())['battlefields']
        pin = proof['preserved_runtime_prefix']
        for record in proof['records']:
            field = runtime['battlefields'][record['runtime_id']]
            self.assertEqual(field['source_id'], record['official_id'])
            self.assertEqual(field['official_text'], record['effective']['rules_text'])
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'),
                                   ('decks', 'preset_count', 'presets_sha256'),
                                   ('battlefields', 'battlefield_count', 'battlefields_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[key][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
