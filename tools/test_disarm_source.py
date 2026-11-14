"""Disarm source facts, including Sentry's mechanically significant Mech tag."""
import copy
import json
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog


class DisarmSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/disarm/admissions.json').read_text())
        cls.origin, cls.archives = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def test_three_original_faces_and_effective_facts_match_review(self):
        raw = {p['official_id']: p for p in catalog.project_gallery(self.archives['gallery'])[0]}
        prints = {p['official_id']: p for p in self.snapshot['printings']}
        expected = {
            'rad-054-167': ('Trinket Peddler', ['Bandle City', 'Yordle']),
            'rad-055-167': ('Clanging Sentry', ['Piltover', 'Mech']),
            'rad-097-167': ('Siren of the Shallows', ['Bilgewater']),
        }
        self.assertEqual({r['official_id'] for r in self.proof['records']}, set(expected))
        for rec in self.proof['records']:
            pid = rec['official_id']
            with self.subTest(printing=pid):
                self.assertEqual(rec['raw_printing'], raw[pid])
                self.assertEqual(rec['raw_printing_sha256'], review.printing_sha256(raw[pid]))
                self.assertEqual(raw[pid]['name'], expected[pid][0])
                self.assertEqual(raw[pid]['tags'], [])
                self.assertEqual(prints[pid]['effective']['tags'], expected[pid][1])
                self.assertIn(pid + ':tags', prints[pid]['resolution_ids'])
                self.assertTrue(rec['visual_review']['actual_full_face_viewed'])
                for key, value in rec['visual_review']['printed_facts'].items():
                    effective = prints[pid]['effective'][key]
                    self.assertEqual(value, (effective or 0) if key == 'power' else effective)
                img = rec['original_image']
                self.assertEqual(img['url'], raw[pid]['image_url'])
                data = (catalog.ROOT / img['path']).read_bytes()
                self.assertEqual((len(data), catalog.sha(data)), (img['bytes'], img['sha256']))
                self.assertTrue(data.startswith(b'\x89PNG\r\n\x1a\n'))
                self.assertEqual(list(struct.unpack('>II', data[16:24])), [744, 1039])

    def test_source_corrections_preserve_all_predecessor_evidence_and_decks(self):
        manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        for key, pin in self.proof['preserved_source_prefix'].items():
            values = images if key == 'images' else manifest[key]
            self.assertEqual(catalog.sha(review.canonical_bytes(values[:pin['count']])), pin['sha256'], key)
        runtime = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[key][:pin[count]])), pin[digest], key)

    def test_reviewed_disarm_faces_are_not_playable_without_runtime_implementation(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:self.proof['preserved_runtime_prefix']['card_count']]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            snapshot = catalog.make_catalog(self.origin, self.archives)
        identities = {p['identity']: p for p in snapshot['cards']}
        for rec in self.proof['records']:
            identity = identities[rec['official_id']]
            self.assertFalse(identity['implementation']['playable_in_client'])
            self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])

    def test_runtime_registry_and_thumbnails_match_reviewed_faces(self):
        runtime = catalog.load_runtime_manifest()['cards']
        for i, rec in enumerate(self.proof['records'], 458):
            card = runtime[i]
            self.assertEqual((card['id'], card['official_id'], card['kind']), (i, rec['official_id'], 'Unit'))
            facts = rec['visual_review']['printed_facts']
            for key in ['energy', 'power', 'might', 'tags']:
                self.assertEqual(card[key], facts[key])
            self.assertEqual(card['official_text'], rec['raw_printing']['rules_text'])
            self.assertEqual(card['official_text_html'], rec['raw_printing']['rules_html'])
            image = rec['runtime_image']
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
            self.assertEqual(struct.unpack('>II', data[16:24]), (280, 391))


if __name__ == '__main__':
    unittest.main()
