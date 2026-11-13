"""Two complete printed equipment faces, independent source and prefix checks."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog


class ConditionalEquipmentAdmissions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/azir_equipment/admissions.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(archives['gallery'])[0]}
        cls.snapshot = catalog.make_catalog(origin, archives)

    def test_original_faces_and_effect_panels_match_the_official_gallery(self):
        printings = {p['official_id']: p for p in self.snapshot['printings']}
        identities = {p['identity']: p for p in self.snapshot['cards']}
        self.assertEqual([r['official_id'] for r in self.proof['records']], ['sfd-042-221', 'ven-027-166'])
        for record in self.proof['records']:
            pid = record['official_id']
            self.assertEqual(record['raw_printing'], self.raw[pid])
            self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(self.raw[pid]))
            self.assertEqual(record['effective'], printings[pid]['effective'])
            self.assertEqual(record['printing_cohort'], identities[pid]['official_ids'])
            self.assertEqual(record['errata'], identities[pid]['errata'])
            self.assertEqual(record['errata'], [])
            self.assertTrue(identities[pid]['standard_legal'])
            self.assertTrue(record['visual_observation']['actual_full_face_viewed'])
            effective = record['effective']
            self.assertEqual(effective['types'], ['gear'])
            self.assertEqual(effective['tags'], ['Equipment'])
            self.assertEqual(effective['domains'], ['calm'])
            self.assertEqual(effective['energy'], 2)
            self.assertEqual(effective['power'] or 0, 0)
            self.assertEqual(effective['might_bonus'], 1)
            self.assertIn('[Equip] [Calm Power]', effective['rules_text'])
            self.assertIn('+2 Might', effective['equipment_text'])
            for key, dimensions in [('original_image', (744, 1039)), ('runtime_image', (280, 391))]:
                image = record[key]
                path = catalog.ROOT / image['path']
                self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
                data = path.read_bytes()
                self.assertEqual(catalog.sha(data), image['sha256'])
                self.assertEqual(struct.unpack('>II', data[16:24]), dimensions)

    def test_runtime_append_preserves_previous_cards_and_sourced_presets(self):
        manifest = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(pin['card_count'], 444)
        self.assertEqual(pin['preset_count'], 7)
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['cards'][:444])), pin['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(manifest['decks'])), pin['presets_sha256'])
        for runtime_id, record in enumerate(self.proof['records'], start=444):
            card = manifest['cards'][runtime_id]
            effective = record['effective']
            self.assertEqual(card['official_id'], record['official_id'])
            self.assertEqual(card['kind'], 'Gear')
            self.assertEqual(card['domains'], ['Calm'])
            self.assertEqual(card['keywords'], ['Equip'])
            self.assertEqual(card['tags'], ['Equipment'])
            self.assertEqual(card['might_bonus'], 1)
            self.assertEqual(card['official_text_html'], effective['rules_html'])
            self.assertEqual(card['equipment_effect'], effective['equipment_text'])
            self.assertEqual(card['equipment_effect_html'], effective['equipment_html'])
            self.assertEqual(card['official_text'], effective['rules_text'] + '\nWhile attached: ' + effective['equipment_text'])
            self.assertEqual(card['art_url'], record['runtime_image']['url'])
