"""Master Yi expansion: official faces, current rules, append-only mappings."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog


class MasterYiAdmissions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/master_yi/admissions.json').read_text())
        cls.origin, cls.archives = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def test_official_face_facts_printing_cohort_and_no_invented_erratum(self):
        raw = {p['official_id']: p for p in catalog.project_gallery(self.archives['gallery'])[0]}
        effective = {p['official_id']: p for p in self.snapshot['printings']}
        self.assertEqual([r['official_id'] for r in self.proof['records']], ['ogn-145-298', 'unl-106-219'])
        for record in self.proof['records']:
            pid = record['official_id']
            self.assertEqual(record['raw_printing'], raw[pid])
            self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw[pid]))
            self.assertEqual(record['effective'], effective[pid]['effective'])
            self.assertEqual(record['resolution_ids'], effective[pid]['resolution_ids'])
            self.assertEqual(record['errata'], [])
            self.assertTrue(record['visual_review']['actual_full_face_viewed'])
            for key, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
                image = record[key]
                path = catalog.ROOT / image['path']
                self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
                data = path.read_bytes()
                self.assertEqual(catalog.sha(data), image['sha256'])
                self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)
            identity = next(c for c in self.snapshot['cards'] if c['identity'] == pid)
            self.assertEqual(identity['official_ids'], record['printing_cohort'])
            self.assertTrue(identity['standard_legal'])

    def test_previous_cards_and_presets_unchanged_and_runtime_matches_the_face(self):
        manifest = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(pin['card_count'], 441)
        self.assertEqual(pin['preset_count'], 7)
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['cards'][:441])), pin['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['decks'][:7])), pin['presets_sha256'])
        for runtime_id, record in enumerate(self.proof['records'], start=441):
            card = manifest['cards'][runtime_id]
            self.assertEqual(card['official_id'], record['official_id'])
            self.assertEqual(card['kind'], 'Spell')
            self.assertEqual(card['domains'], ['Body'])
            self.assertEqual(card['keywords'], ['Reaction'])
            self.assertEqual(card['tags'], [])
            for key in ['energy', 'power', 'might']:
                self.assertEqual(card[key], record['effective'][key] or 0)
            self.assertEqual(card['official_text'], record['effective']['rules_text'])
            self.assertEqual(card['official_text_html'], record['effective']['rules_html'])
            self.assertEqual(card['art_url'], record['runtime_image']['url'])
