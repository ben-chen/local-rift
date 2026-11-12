"""Reckless Innovator's printed Piltover tag, without implicit runtime admission."""
import copy
import gzip
import json
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog


class RecklessSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.proof = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/reviews/reckless-innovator.json').read_text())
        cls.manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def test_complete_face_preserves_raw_record_and_corrects_only_the_printed_tag(self):
        proof = self.proof
        raw = next(p for p in catalog.project_gallery(self.archives['gallery'])[0] if p['official_id'] == 'rad-105-167')
        self.assertEqual(proof['raw_printing'], raw)
        self.assertEqual(proof['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(raw['tags'], [])
        printing = next(p for p in self.snapshot['printings'] if p['official_id'] == raw['official_id'])
        for field in review.GAMEPLAY_FIELDS:
            self.assertEqual(printing['effective'][field], ['Piltover'] if field == 'tags' else raw[field])
        image = proof['original_image']
        path = catalog.ROOT / image['path']
        self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
        data = path.read_bytes()
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039))
        self.assertEqual(image['url'], raw['image_url'])
        observed = proof['visual_observation']
        self.assertTrue(observed['actually_viewed'])
        self.assertEqual(observed['printed_type_tag_line'], 'UNIT / PILTOVER')
        self.assertEqual(observed['uncertainties'], [])
        self.assertEqual(printing['resolution_ids'], ['rad-105-167:tags'])

    def test_prior_review_cohorts_remain_exact_prefixes(self):
        archive = self.proof['preserved_source_cohorts']['field_manifest_archive']
        data = (catalog.ROOT / archive['path']).read_bytes()
        self.assertEqual(catalog.sha(data), archive['sha256'])
        raw = gzip.decompress(data)
        self.assertEqual((len(raw), catalog.sha(raw)), (archive['uncompressed_bytes'], archive['uncompressed_sha256']))
        previous = json.loads(raw)
        for name in ['resolutions', 'raw_printings', 'historical_issues']:
            pin = self.proof['preserved_source_cohorts'][name]
            self.assertEqual(catalog.sha(review.canonical_bytes(self.manifest[name][:pin['count']])), pin['canonical_sha256'])
            self.assertEqual(self.manifest[name][:pin['count']], previous[name])
        pin = self.proof['preserved_source_cohorts']['image_index']
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        self.assertEqual(catalog.sha(review.canonical_bytes(images[:pin['count']])), pin['canonical_sha256'])

    def test_source_correction_alone_does_not_register_a_card(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:self.proof['runtime_base_count']]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            snapshot = catalog.make_catalog(self.origin, self.archives)
        identity = next(c for c in snapshot['cards'] if c['identity'] == 'rad-105-167')
        self.assertTrue(identity['standard_legal'])
        self.assertEqual(identity['official_ids'], ['rad-105-167'])
        self.assertEqual(identity['errata'], [])
        self.assertFalse(identity['implementation']['playable_in_client'])
        self.assertFalse(self.proof['enables_runtime_support'])
