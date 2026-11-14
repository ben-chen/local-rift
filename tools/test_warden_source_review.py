"""Warden's printed restrictions and source identity before runtime admission."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class WardenSourceReview(unittest.TestCase):
    def test_runtime_preserves_printed_facts_and_prior_recipes(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_warden/source_review.json').read_text())
        runtime = catalog.load_runtime_manifest()
        card = runtime['cards'][464]
        record, = proof['records']
        self.assertEqual((card['official_id'], card['kind'], card['energy'], card['power'], card['might']),
                         ('ogn-070-298', 'Unit', 6, 1, 5))
        self.assertEqual((card['domains'], card['tags'], card['keywords']), (['Calm'], ['Demacia'], []))
        self.assertEqual(card['official_text'], record['effective']['rules_text'])
        self.assertEqual(card['official_text_html'], record['effective']['rules_html'])
        image = record['runtime_image']
        data = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual(card['art_url'], image['url'])
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertEqual(struct.unpack('>II', data[16:24]), (280, 391))
        pin = proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (464, 7))
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[key][:pin[count]])), pin[digest])

    def test_complete_source_and_printed_restrictions(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_warden/source_review.json').read_text())
        record, = proof['records']
        origin, archives = catalog.read_archives()
        raw = next(p for p in catalog.project_gallery(archives['gallery'])[0]
                   if p['official_id'] == 'ogn-070-298')
        self.assertEqual(record['raw_printing'], raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
        inventory = catalog.make_catalog(origin, archives)
        printing = next(p for p in inventory['printings'] if p['official_id'] == raw['official_id'])
        self.assertEqual(record['effective'], printing['effective'])
        self.assertEqual(record['resolution_ids'], [])
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        for key, value in record['visual_review']['printed_facts'].items():
            self.assertEqual(record['effective'][key], value, key)
        facts = record['effective']
        self.assertEqual((facts['energy'], facts['power'], facts['might']), (6, 1, 5))
        self.assertEqual((facts['types'], facts['domains'], facts['tags']),
                         (['unit'], ['calm'], ['Demacia']))
        self.assertEqual(facts['rules_text'], "While I'm at a battlefield, opponents can only play units to their base.\nWhile I'm at a battlefield, spells and abilities can't ready enemy units and gear.")
        image = record['original_image']
        data = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual(image['url'], raw['image_url'])
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039))


if __name__ == '__main__':
    unittest.main()
