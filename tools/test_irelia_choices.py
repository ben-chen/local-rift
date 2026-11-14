"""Reviewed spell-choice cards and preservation of existing registrations."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog


class IreliaChoiceAdmissions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_choices/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(archives['gallery'])[0]}
        cls.snapshot = catalog.make_catalog(origin, archives)

    def test_jae_printed_facts_and_complete_source_face(self):
        record, = self.proof['records']
        self.assertEqual(record['official_id'], 'sfd-142-221')
        raw = self.raw[record['official_id']]
        self.assertEqual(record['raw_printing'], raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual((raw['energy'], raw['power'], raw['might']), (5, 2, 5))
        self.assertEqual((raw['types'], raw['domains'], raw['tags']), (['unit'], ['chaos'], ['Noxus']))
        self.assertEqual(raw['rules_text'], 'When you choose me with a spell, draw 1.')
        self.assertEqual(record['errata'], [])
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == record['official_id'])
        self.assertEqual(identity['official_ids'], record['printing_cohort'])
        self.assertTrue(identity['implementation']['playable_in_client'])
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        for field, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
            image = record[field]
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
            self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
            self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_previous_cards_and_all_seven_presets_remain_exact(self):
        runtime = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['preset_count']), (450, 7))
        for field, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[field][:pin[count]])), pin[digest])
        row = runtime['cards'][450]
        self.assertEqual(row['official_id'], 'sfd-142-221')
        self.assertEqual(row['official_text'], self.raw[row['official_id']]['rules_text'])
        self.assertEqual(row['art_url'], self.proof['records'][0]['runtime_image']['url'])


if __name__ == '__main__':
    unittest.main()
