"""Riven-pool faces, source facts, engine admission and historical preservation."""
import copy
import json
import struct
import unittest
from unittest.mock import patch

import admission_history as history
import catalog_review as review
import import_catalog as catalog


class RivenAdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/riven/admissions.json').read_text())
        cls.origin, cls.archives = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def test_all_twelve_original_faces_match_archived_printings_and_reviewed_facts(self):
        raw = {p['official_id']: p for p in catalog.project_gallery(self.archives['gallery'])[0]}
        prints = {p['official_id']: p for p in self.snapshot['printings']}
        identities = {p['identity']: p for p in self.snapshot['cards']}
        self.assertEqual(self.proof['gallery'], self.snapshot['sources']['gallery'])
        self.assertEqual([r['runtime_id'] for r in self.proof['records']], [435, 436, 437, 438, 439, 10, 440, 1000])
        face_count = 0
        for record in self.proof['records']:
            with self.subTest(identity=record['identity']):
                identity = identities[record['identity']]
                self.assertEqual(record['printing_cohort'], identity['official_ids'])
                self.assertEqual(record['errata'], identity['errata'])
                self.assertEqual(record['standard_legal'], identity['standard_legal'])
                self.assertEqual(record['auxiliary'], identity['auxiliary'])
                self.assertEqual(record['runtime_id'], identity['implementation']['runtime_id'])
                self.assertEqual(record['implementation_status'], identity['implementation']['status'])
                self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
                self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
                for printing in record['printings']:
                    pid = printing['official_id']
                    self.assertEqual(printing['raw_printing'], raw[pid])
                    self.assertEqual(printing['raw_printing_sha256'], review.printing_sha256(raw[pid]))
                    self.assertEqual(printing['effective'], prints[pid]['effective'])
                    self.assertEqual(printing['resolution_ids'], prints[pid]['resolution_ids'])
                    effective = printing['effective']
                    for key, value in record['printed_facts'].items():
                        if key == 'observation':
                            continue
                        if key == 'might' and record['identity'] == 'rad-150-167':
                            # Gallery's normalized gear Might repeats its equipment bonus.
                            self.assertIsNone(value)
                            self.assertEqual(effective[key], record['printed_facts']['might_bonus'])
                        elif key == 'domains' and record['auxiliary']:
                            self.assertEqual(value, [])
                            self.assertEqual(effective[key], ['colorless'])
                        elif isinstance(value, list):
                            self.assertEqual(set(value), set(effective[key]))
                        elif key == 'power' and value == 0:
                            self.assertEqual(effective[key] or 0, 0)
                        else:
                            self.assertEqual(value, effective[key], (pid, key))
                    image = printing['original_image']
                    self.assertTrue(image['actual_full_face_viewed'])
                    self.assertEqual(image['url'], raw[pid]['image_url'])
                    self.check_image(image)
                    face_count += 1
                self.check_image(record['runtime_image'])
        self.assertEqual(face_count, 12)

    def check_image(self, image):
        path = catalog.ROOT / image['path']
        self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
        data = path.read_bytes()
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertTrue(data.startswith(b'\x89PNG\r\n\x1a\n'))
        self.assertEqual(list(struct.unpack('>II', data[16:24])), image['dimensions'])

    def test_new_reviews_do_not_enable_cards_without_explicit_runtime_registration(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:435]
        with history.pre_riven_runtime(runtime), patch.object(catalog, 'load_battlefield_token_manifest', return_value=[]):
            before = catalog.make_catalog(self.origin, self.archives)
        records = {c['identity']: c for c in before['cards']}
        for entry in self.proof['records']:
            self.assertFalse(records[entry['identity']]['implementation']['playable_in_client'])
        self.assertEqual(before['counts']['implemented_legal_identities'], 468)
        runtime = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'), ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[key][:pin[count]])), pin[digest])


if __name__ == '__main__':
    unittest.main()
