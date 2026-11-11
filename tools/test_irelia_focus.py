"""Official Decree of Focus facts and preservation of the tested C28 catalog."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog

class IreliaFocusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_focus/admission.json').read_text())
        cls.origin, cls.archives = catalog.read_archives()

    def test_exact_official_face_and_gallery_agree(self):
        proof = self.proof
        raw = next(p for p in catalog.project_gallery(self.archives['gallery'])[0] if p['official_id'] == 'ven-040-166')
        self.assertEqual(proof['raw_printing'], raw)
        self.assertEqual(proof['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(proof['effective'], {k: raw[k] for k in review.GAMEPLAY_FIELDS})
        self.assertEqual(proof['origin_gallery_sha256'], catalog.sha(self.archives['gallery']))
        self.assertTrue(proof['visual_review']['actual_full_face_viewed'])
        self.assertEqual(proof['visual_review']['image_sha256'], proof['original_image']['sha256'])
        for key, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
            info = proof[key]; path = catalog.ROOT / info['path']
            self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
            data = path.read_bytes()
            self.assertEqual((catalog.sha(data), len(data)), (info['sha256'], info['bytes']))
            self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_runtime_matches_source_and_preserves_existing_cards_and_presets(self):
        runtime = catalog.load_runtime_manifest(); card = runtime['cards'][421]; e = self.proof['effective']
        self.assertEqual((card['id'], card['official_id'], card['kind'], card['domains']), (421, 'ven-040-166', 'Spell', ['Calm']))
        self.assertEqual((card['energy'], card['power'], card['might']), (1, 0, 0))
        self.assertEqual(card['official_text'], e['rules_text'])
        self.assertEqual(card['official_text_html'], e['rules_html'])
        self.assertEqual(card['tags'], e['tags'])
        self.assertEqual(card['keywords'], ['Reaction'])
        self.assertEqual(card['art_url'], self.proof['runtime_image']['url'])
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(catalog.sha(review.canonical_bytes(runtime['cards'][:421])), pin['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(runtime['decks'][:5])), pin['presets_sha256'])

    def test_legal_single_printing_identity_has_no_conflict_or_erratum(self):
        data = catalog.make_catalog(self.origin, self.archives)
        card = next(c for c in data['cards'] if c['identity'] == 'ven-040-166')
        self.assertTrue(card['standard_legal'])
        self.assertEqual(card['official_ids'], self.proof['printing_cohort'])
        self.assertEqual(card['errata'], [])
        self.assertEqual(card['source_review']['unresolved_gameplay_fields'], [])
        self.assertTrue(card['implementation']['playable_in_client'])
