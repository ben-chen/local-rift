"""Gust Monk's exact official face, runtime facts and unchanged predecessors."""
import copy
import json
import struct
import unittest
from unittest.mock import patch
import import_catalog as catalog
import catalog_review as review

class GustAdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin,cls.archives=catalog.read_archives()
        cls.proof=json.loads((catalog.ROOT/'assets/data/card_admissions/irelia_gust/admissions.json').read_text())
        cls.snapshot=catalog.make_catalog(cls.origin,cls.archives)

    def test_complete_face_and_explicit_runtime_match_official_facts(self):
        p=self.proof
        raw=next(x for x in catalog.project_gallery(self.archives['gallery'])[0] if x['official_id']=='ven-101-166')
        self.assertEqual(p['raw_printing'],raw)
        self.assertEqual(p['raw_printing_sha256'],review.printing_sha256(raw))
        self.assertEqual(p['printing_cohort'],['ven-101-166'])
        current=next(x for x in self.snapshot['printings'] if x['official_id']=='ven-101-166')
        self.assertEqual(current['effective'],p['effective']);self.assertEqual(p['resolution_ids'],[])
        for key,size in [('original_image',(744,1039)),('runtime_image',(280,391))]:
            image=p[key];data=(catalog.ROOT/image['path']).read_bytes()
            self.assertEqual(catalog.sha(data),image['sha256']);self.assertEqual(struct.unpack('>II',data[16:24]),size)
        self.assertTrue(p['original_image']['actual_full_face_viewed'])
        card=catalog.load_runtime_manifest()['cards'][431]
        self.assertEqual((card['name'],card['energy'],card['power'],card['might']),('Gust Monk',2,0,2))
        self.assertEqual(card['official_text'],raw['rules_text']);self.assertEqual(card['tags'],raw['tags'])
        self.assertEqual(card['keywords'],[], 'Assault is granted to a chosen unit; it is not intrinsic to Gust Monk')
        identity=next(x for x in self.snapshot['cards'] if x['identity']=='ven-101-166')
        self.assertTrue(identity['standard_legal']);self.assertEqual(identity['errata'],[])
        self.assertTrue(identity['implementation']['playable_in_client'])

    def test_all_predecessor_cards_presets_and_source_reviews_are_unchanged(self):
        p=self.proof['preserved_runtime_prefix'];runtime=catalog.load_runtime_manifest()
        self.assertEqual(catalog.sha(review.canonical_bytes(runtime['cards'][:p['card_count']])),p['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(runtime['decks'][:p['preset_count']])),p['presets_sha256'])
        for path,digest in self.proof['preserved_source_reviews'].items():self.assertEqual(catalog.sha((catalog.ROOT/path).read_bytes()),digest)

    def test_source_evidence_alone_does_not_register_playable_support(self):
        runtime=copy.deepcopy(catalog.load_runtime_manifest());runtime['cards']=runtime['cards'][:431]
        with patch.object(catalog,'load_runtime_manifest',return_value=runtime):snapshot=catalog.make_catalog(self.origin,self.archives)
        identity=next(x for x in snapshot['cards'] if x['identity']=='ven-101-166')
        self.assertFalse(identity['implementation']['playable_in_client'])
