"""The sixth preset and Edge of Night are checked against reviewed source facts."""
import collections
import copy
import json
import struct
import unittest
from unittest.mock import patch
import import_catalog as catalog
import catalog_review as review


class AritoPresetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runtime = catalog.load_runtime_manifest()
        cls.edge = json.loads((catalog.ROOT/'assets/data/card_admissions/irelia_edge/admissions.json').read_text())
        cls.folder = catalog.ROOT/'assets/data/preset_admissions/irelia_arito_2026_10'
        cls.proof = json.loads((cls.folder/'admission.json').read_text())
        cls.origin, cls.archives = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)

    def test_edge_face_erratum_stats_and_runtime_equipment_are_explicit(self):
        p = self.edge
        raw = next(x for x in catalog.project_gallery(self.archives['gallery'])[0] if x['official_id']=='sfd-139-221')
        self.assertEqual(raw,p['raw_printing'])
        self.assertEqual(review.printing_sha256(raw),p['raw_printing_sha256'])
        self.assertEqual(p['printing_cohort'],['sfd-139-221'])
        identity = next(x for x in self.snapshot['cards'] if x['identity']=='sfd-139-221')
        self.assertEqual(identity['errata'],p['errata'])
        self.assertEqual(len(p['errata']),1)
        self.assertIn('unit you control (here)',p['errata'][0]['text'])
        self.assertNotIn('(here)',raw['rules_text'])
        card=self.runtime['cards'][432]
        self.assertEqual((card['energy'],card['power'],card['might'],card['might_bonus']),(3,0,2,2))
        self.assertEqual((card['kind'],card['tags'],card['domains']),('Gear',['Equipment'],['Chaos']))
        self.assertEqual(card['official_text'],p['runtime_rules_text'])
        self.assertEqual(card['official_text_html'],raw['rules_html'])
        self.assertEqual(card['keywords'],['Hidden','Equip'])
        for key,size in [('original_image',(744,1039)),('runtime_image',(280,391))]:
            image=p[key]; data=(catalog.ROOT/image['path']).read_bytes()
            self.assertEqual(catalog.sha(data),image['sha256'])
            self.assertEqual(struct.unpack('>II',data[16:24]),size)
        self.assertTrue(p['original_image']['actual_full_face_viewed'])
        self.assertTrue(identity['standard_legal'])
        self.assertTrue(identity['implementation']['playable_in_client'])

    def test_exact_original_main_sideboard_runes_fields_and_chosen_champion(self):
        p=self.runtime['decks'][5]; source=self.proof['source']
        self.assertEqual(p,json.loads((self.folder/'preset.json').read_text()))
        self.assertEqual((p['pilot'],p['placing'],p['date']),('Arito',2,'2026-10-31'))
        self.assertEqual(p['source_url'],'https://topdeck.gg/deck/convergence-4/3Mb0hLsUTwgBYrpAauGOaRvDhnq1')
        main=collections.Counter({r['name']:r['count'] for r in p['cards']})
        expected=collections.Counter(source['mainboard_excluding_chosen']);expected['Irelia, Fervent']+=1
        self.assertEqual(main,expected);self.assertEqual(sum(main.values()),40)
        self.assertEqual(main['Edge of Night'],2);self.assertEqual(main['Orianna, Exquisite'],3);self.assertEqual(main['Gust Monk'],1)
        side=collections.Counter({r['name']:r['count'] for r in p['sideboard']})
        self.assertEqual(side,source['sideboard']);self.assertEqual(sum(side.values()),10)
        self.assertTrue(all(n<=3 for n in (main+side).values()))
        self.assertEqual(p['chosen_champion'],0);self.assertEqual(p['chosen'],source['chosen_champion'])
        self.assertEqual(p['legend_name'],source['legend'])
        self.assertEqual({name+' Rune':count for name,count in p['runes'].items()},source['runes'])
        self.assertEqual(p['battlefields'],source['battlefields'])
        fields=catalog.load_battlefield_manifest()
        self.assertEqual([fields[i]['name'] for i in self.proof['battlefield_ids']],source['battlefields'])
        for row in p['cards']+p['sideboard']:
            card=self.runtime['cards'][row['id']]
            self.assertEqual(row['name'],card['name'])
            self.assertTrue(set(card['domains'])<=set(p['domains']))
            identity=next(x for x in self.snapshot['cards'] if card['official_id'] in x['official_ids'])
            self.assertTrue(identity['standard_legal'] and identity['implementation']['playable_in_client'])

    def test_predecessor_cards_presets_and_source_reviews_are_unchanged(self):
        for proof in [self.edge['preserved_runtime_prefix'],{
            'card_count':self.proof['preserved_card_count'],'cards_sha256':self.proof['preserved_cards_sha256'],
            'preset_count':self.proof['preserved_preset_count'],'presets_sha256':self.proof['preserved_presets_sha256']}]:
            for field,count in [('cards','card_count'),('decks','preset_count')]:
                digest_key='cards_sha256' if field=='cards' else 'presets_sha256'
                self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[field][:proof[count]])),proof[digest_key])
        for path,digest in self.edge['preserved_source_reviews'].items():
            self.assertEqual(catalog.sha((catalog.ROOT/path).read_bytes()),digest)

    def test_source_review_alone_does_not_enable_edge_or_the_new_preset(self):
        runtime=copy.deepcopy(self.runtime);runtime['cards']=runtime['cards'][:432];runtime['decks']=runtime['decks'][:5]
        with patch.object(catalog,'load_runtime_manifest',return_value=runtime):
            snapshot=catalog.make_catalog(self.origin,self.archives)
        edge=next(x for x in snapshot['cards'] if x['identity']=='sfd-139-221')
        self.assertFalse(edge['implementation']['playable_in_client'])
