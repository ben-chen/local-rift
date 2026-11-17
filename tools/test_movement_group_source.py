"""Reviewed MovementGroup source facts."""
import json
import struct
import unittest

import catalog_review as review
import import_catalog as catalog
from catalog_test_support import assert_review_inventory


class MovementGroupSources(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/movement_groups/source_review.json').read_text())
        origin, archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(archives['gallery'])[0]}
        cls.snapshot = catalog.make_catalog(origin, archives)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}

    def test_full_faces_and_exact_effective_costs(self):
        self.assertEqual([r['official_id'] for r in self.proof['records']],
                         ['sfd-043-221', 'unl-054-219', 'unl-124-219'])
        for record, energy, power in zip(self.proof['records'], [2, 4, 2], [None, 1, None], strict=True):
            with self.subTest(card=record['name']):
                ident = record['official_id']
                self.assertEqual(record['raw_printing'], self.raw[ident])
                self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(self.raw[ident]))
                effective = self.printings[ident]['effective']
                self.assertEqual(record['effective'], effective)
                self.assertEqual((effective['energy'], effective['power']), (energy, power))
                self.assertEqual(effective['types'], ['spell'])
                self.assertEqual(effective['domains'], ['chaos'] if ident == 'unl-124-219' else ['calm'])
                self.assertEqual(effective['tags'], [])
                self.assertEqual(record['visual_review']['printed_facts'], effective)
                self.assertTrue(record['visual_review']['actual_full_face_viewed'])
                image = record['original_image']
                data = (catalog.ROOT / image['path']).read_bytes()
                self.assertEqual(image['url'], self.raw[ident]['image_url'])
                self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
                self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039))

    def test_exact_runtime_admission_without_source_corrections(self):
        for record in self.proof['records']:
            self.assertEqual(self.printings[record['official_id']]['resolution_ids'], [])
        for runtime_id, record in enumerate(self.proof['records'], start=488):
            identity = next(c for c in self.snapshot['cards'] if c['identity'] == record['official_id'])
            self.assertEqual(identity['errata'], [])
            self.assertTrue(identity['standard_legal'])
            self.assertTrue(identity['implementation']['playable_in_client'])
            self.assertEqual(identity['implementation']['runtime_id'], runtime_id)
            self.assertEqual(record['runtime_id'], runtime_id)
            self.assertEqual((catalog.ROOT / f'assets/art/card_{runtime_id}.png').read_bytes(),
                             (catalog.ROOT / record['original_image']['path']).read_bytes())
        assert_review_inventory(self, self.snapshot)

    def test_prior_evidence_and_runtime_remain_unchanged(self):
        manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        for key, pin in self.proof['preserved_source_prefix'].items():
            values = images if key == 'images' else manifest[key]
            self.assertEqual(catalog.sha(review.canonical_bytes(values[:pin['count']])), pin['sha256'])
        runtime = catalog.load_runtime_manifest()
        for key, pin in self.proof['preserved_runtime_prefix'].items():
            self.assertEqual(catalog.sha(review.canonical_bytes(runtime[key][:pin['count']])), pin['sha256'])


if __name__ == '__main__':
    unittest.main()
