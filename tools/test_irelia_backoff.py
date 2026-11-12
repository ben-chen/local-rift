"""Back Off's full official face, legality and preserved runtime predecessors."""
import json
import struct
import unittest
import catalog_review as review
import import_catalog as catalog

class IreliaBackOffTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof=json.loads((catalog.ROOT/'assets/data/card_admissions/irelia_backoff/admissions.json').read_text())
        cls.origin,cls.archives=catalog.read_archives()
        cls.snapshot=catalog.make_catalog(cls.origin,cls.archives)

    def test_complete_official_faces_and_effective_facts_are_preserved(self):
        raw={p['official_id']:p for p in catalog.project_gallery(self.archives['gallery'])[0]}
        current={p['official_id']:p for p in self.snapshot['printings']}
        self.assertEqual([r['official_id'] for r in self.proof['records']],['unl-042-219'])
        for record in self.proof['records']:
            pid=record['official_id'];self.assertEqual(record['raw_printing'],raw[pid])
            self.assertEqual(record['raw_printing_sha256'],review.printing_sha256(raw[pid]))
            self.assertEqual(record['effective'],current[pid]['effective'])
            self.assertEqual(record['resolution_ids'],current[pid]['resolution_ids'])
            self.assertTrue(record['visual_review']['actual_full_face_viewed'])
            for key,dimensions in [('original_image',(744,1039)),('runtime_image',(280,391))]:
                image=record[key];path=catalog.ROOT/image['path'];self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
                data=path.read_bytes();self.assertEqual(catalog.sha(data),image['sha256'])
                self.assertEqual(struct.unpack('>II',data[16:24]),dimensions)
            self.assertEqual(record['visual_review']['original_image_sha256'],record['original_image']['sha256'])
        self.assertCountEqual(self.proof['records'][0]['resolution_ids'],[])

    def test_runtime_preserves_predecessor_cards_and_exact_new_fields(self):
        m=catalog.load_runtime_manifest();pin=self.proof['preserved_runtime_prefix']
        self.assertEqual(catalog.sha(review.canonical_bytes(m['cards'][:428])),pin['cards_sha256'])
        self.assertEqual(catalog.sha(review.canonical_bytes(m['decks'][:5])),pin['presets_sha256'])
        for record in self.proof['records']:
            card=m['cards'][record['runtime_id']];e=record['effective']
            self.assertEqual(card['official_id'],record['official_id'])
            self.assertEqual(card['kind'],'Spell');self.assertEqual(card['domains'],['Calm'])
            for key in ['energy','power','might']:self.assertEqual(card[key],e[key] or 0)
            self.assertEqual(card['official_text'],e['rules_text']);self.assertEqual(card['official_text_html'],e['rules_html'])
            self.assertEqual(card['art_url'],record['runtime_image']['url'])
            identity=next(c for c in self.snapshot['cards'] if c['identity']==card['official_id'])
            self.assertTrue(identity['standard_legal']);self.assertEqual(identity['official_ids'],record['printing_cohort'])
            self.assertEqual(identity['errata'],[]);self.assertTrue(identity['implementation']['playable_in_client'])
