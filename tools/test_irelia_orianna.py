"""Complete official Orianna printings, prior evidence, and explicit admission."""
import copy
import json
import struct
import unittest
from unittest.mock import patch
import catalog_review as review
import import_catalog as catalog
import admission_history as history


class OriannaAdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/irelia_orianna/admissions.json').read_text())
        cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def test_both_official_faces_and_runtime_fields_match_the_reviewed_identity(self):
        raws = {p['official_id']: p for p in catalog.project_gallery(self.archives['gallery'])[0]}
        current = {p['official_id']: p for p in self.snapshot['printings']}
        self.assertEqual(self.proof['printing_cohort'], ['rad-042-167', 'rad-042a-167'])
        for record in self.proof['records']:
            pid = record['official_id']
            self.assertEqual(record['raw_printing'], raws[pid])
            self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raws[pid]))
            self.assertEqual(record['effective'], current[pid]['effective'])
            self.assertEqual(record['resolution_ids'], [])
            self.assertTrue(record['actual_full_face_viewed'])
            image = record['original_image']
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual(catalog.sha(data), image['sha256'])
            self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039))
        runtime = catalog.load_runtime_manifest()['cards'][430]
        facts = current['rad-042-167']['effective']
        self.assertEqual((runtime['name'],runtime['kind'],runtime['domains']),('Orianna, Exquisite','Unit',['Calm']))
        self.assertEqual((runtime['energy'],runtime['power'],runtime['might']),(3,0,4))
        self.assertEqual(runtime['official_text'],facts['rules_text'])
        self.assertEqual(runtime['official_text_html'],facts['rules_html'])
        self.assertEqual(runtime['tags'],facts['tags'])
        image=self.proof['runtime_image'];data=(catalog.ROOT/image['path']).read_bytes()
        self.assertEqual(catalog.sha(data),image['sha256'])
        self.assertEqual(struct.unpack('>II',data[16:24]),(280,391))
        identity=next(c for c in self.snapshot['cards'] if c['identity']=='rad-042-167')
        self.assertTrue(identity['standard_legal']);self.assertEqual(identity['errata'],[])
        self.assertEqual(identity['source_review']['status'],'reviewed_variants')
        self.assertEqual(identity['source_review']['variant_diagnostics'][0]['disposition'],'reminder_equivalent')

    def test_predecessor_cards_presets_and_review_prefixes_are_unchanged(self):
        runtime=catalog.load_runtime_manifest();p=self.proof['preserved_runtime_prefix']
        self.assertEqual(catalog.sha(review.canonical_bytes(runtime['cards'][:p['card_count']])),p['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(runtime['decks'][:p['preset_count']])),p['presets_sha256'])
        p=self.proof['preserved_source_review_prefix']
        reviews=json.loads((catalog.ROOT/'assets/data/catalog_identity_reviews.json').read_text())['reviews']
        images=json.loads((catalog.ROOT/'assets/data/catalog_evidence/variant_images.json').read_text())
        self.assertEqual(catalog.sha(review.canonical_bytes(reviews[:p['review_count']])),p['reviews_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(images[:p['image_count']])),p['images_sha256'])
        self.assertEqual(catalog.sha(history.pre_riven_evidence_bytes('assets/data/catalog_resolutions.json')),p['field_manifest_sha256'])

    def test_variant_review_alone_does_not_admit_orianna(self):
        runtime=copy.deepcopy(catalog.load_runtime_manifest());runtime['cards']=runtime['cards'][:430]
        with history.pre_riven_runtime(runtime):
            snapshot=catalog.make_catalog(self.origin,self.archives)
        identity=next(c for c in snapshot['cards'] if c['identity']=='rad-042-167')
        self.assertEqual(identity['source_review']['status'],'reviewed_variants')
        self.assertFalse(identity['implementation']['playable_in_client'])
