"""Independent archived printing and inspected face checks for Guards."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class GuardsAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/guards/admissions.json').read_text())
        origin, cls.archives = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(origin, cls.archives)

    def test_printed_face_and_effective_facts(self):
        record, = self.proof['records']
        pid = 'sfd-154-221'
        raw = next(p for p in catalog.project_gallery(self.archives['gallery'])[0] if p['official_id'] == pid)
        printing = next(p for p in self.snapshot['printings'] if p['official_id'] == pid)
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == pid)
        self.assertEqual(record['official_id'], pid)
        self.assertEqual(record['raw_printing'], raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(record['effective'], printing['effective'])
        self.assertEqual(record['printing_cohort'], identity['official_ids'])
        self.assertEqual(record['resolution_ids'], printing['resolution_ids'])
        self.assertEqual(record['errata'], identity['errata'])
        self.assertEqual(len(record['errata']), 1)
        self.assertIn('Then do this:', record['errata'][0]['text'])
        self.assertTrue(record['errata'][0]['text'].endswith('You may pay [C] to ready it.'))
        self.assertTrue(identity['standard_legal'])
        self.assertTrue(record['visual_review']['actual_full_face_viewed'])
        effective = record['effective']
        self.assertEqual((effective['energy'], effective['power']), (3, None))
        self.assertEqual(effective['domains'], ['order'])
        self.assertEqual(effective['types'], ['spell'])
        self.assertIn('[Hidden]', effective['rules_text'])
        self.assertNotIn('Then do this:', effective['rules_text'])
        self.assertNotEqual(record['operative_text'], effective['rules_text'])
        for field, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
            image = record[field]
            path = catalog.ROOT / image['path']
            self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
            data = path.read_bytes()
            self.assertEqual(catalog.sha(data), image['sha256'])
            self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_runtime_append_preserves_existing_cards_and_presets(self):
        manifest = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(pin['card_count'], 448)
        self.assertEqual(pin['preset_count'], 7)
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['cards'][:448])), pin['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['decks'])), pin['presets_sha256'])
        record, = self.proof['records']
        card = manifest['cards'][448]
        self.assertEqual(card['official_id'], record['official_id'])
        self.assertEqual(card['official_text'], record['operative_text'])
        self.assertEqual(card['official_text_html'], record['effective']['rules_html'])
        self.assertEqual(card['keywords'], ['Hidden'])
        expected = record['errata'][0]['text'].replace('[M]', 'Might').replace('[A]', '[Any Power]').replace('[0]', '[0 Energy]').replace('[C]', '[Order Power]')
        self.assertEqual(card['official_text'], expected)
        self.assertEqual(card['tags'], [])
        self.assertEqual(card['domains'], ['Order'])
        self.assertEqual((card['energy'], card['power'], card['might']), (3, 0, 0))
        self.assertEqual(card['art_url'], record['runtime_image']['url'])
