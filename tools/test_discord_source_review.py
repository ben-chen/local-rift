"""Printed source facts, independent of the runtime implementation."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class DiscordSourceReview(unittest.TestCase):
    def test_source_face_facts_and_prior_recipes(self):
        proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_discord/source_review.json').read_text())
        record, = proof['records']
        origin, archives = catalog.read_archives()
        raw = next(p for p in catalog.project_gallery(archives['gallery'])[0]
                   if p['official_id'] == 'ven-107-166')
        self.assertEqual(record['raw_printing'], raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
        inventory = catalog.make_catalog(origin, archives)
        printing = next(p for p in inventory['printings'] if p['official_id'] == raw['official_id'])
        self.assertEqual(record['effective'], printing['effective'])
        facts = record['effective']
        self.assertEqual((facts['types'], facts['domains'], facts['energy'], facts['power'], facts['might']),
                         (['spell'], ['chaos'], 1, 1, None))
        self.assertEqual(facts['rules_text'], "Return any number of enemy Order ([Order Power]) units with total Might 5 or less to their owners' hands.")
        identity = next(c for c in inventory['cards'] if c['identity'] == raw['official_id'])
        self.assertTrue(identity['standard_legal'])
        self.assertEqual(identity['errata'], [])
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        for key, value in record['visual_review']['printed_facts'].items():
            self.assertEqual(facts[key], value, key)
        image = record['original_image']
        data = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual(image['url'], raw['image_url'])
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039))
        runtime = catalog.load_runtime_manifest()
        pin = proof['preserved_runtime_prefix']
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[key][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
