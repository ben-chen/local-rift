"""Every original and alternate face for the Rengar/Ezreal legend admissions."""
import copy
import json
import unittest
from unittest.mock import patch

import admission_history as history
import import_catalog as catalog


class C16LegendTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.archives = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.manifest, cls.archives)
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c16/legends.json').read_text())

    def test_complete_printing_membership_and_independent_face_facts(self):
        printings = {p['official_id']: p for p in self.snapshot['printings']}
        identities = {c['identity']: c for c in self.snapshot['cards']}
        self.assertEqual([r['legend_id'] for r in self.proof['records']], [8, 9])
        self.assertEqual([len(r['members']) for r in self.proof['records']], [3, 2])
        for record in self.proof['records']:
            identity = identities[record['identity']['identity']]
            self.assertEqual([m['printing']['official_id'] for m in record['members']], identity['official_ids'])
            self.assertTrue(identity['standard_legal'])
            self.assertEqual(identity['errata'], [])
            self.assertEqual(identity['implementation']['runtime_id'], record['legend_id'])
            self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
            self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
            for m in record['members']:
                printing = printings[m['printing']['official_id']]
                self.assertEqual(m['printing'], printing)
                o = m['visual_observation']
                self.assertTrue(o['face_visually_inspected'])
                self.assertFalse(m['visual_review_pending'])
                self.assertEqual(o['uncertainties'], [])
                self.assertEqual(o['name'], printing['name'])
                self.assertEqual(o['collector_code'], printing['public_code'])
                for observed, effective in [('printed_types','types'),('printed_supertypes','super_types'),('printed_domains','domains'),('printed_tags','tags'),('printed_energy','energy'),('printed_power','power'),('printed_might','might'),('printed_might_bonus','might_bonus'),('printed_rules_icons_expanded','rules_text'),('printed_equipment_icons_expanded','equipment_text')]:
                    self.assertEqual(o[observed], printing['effective'][effective])
                image = m['original_image']; payload = (catalog.ROOT / image['path']).read_bytes()
                self.assertFalse(image['path'].startswith('/'))
                self.assertEqual((len(payload), catalog.sha(payload)), (image['bytes'], image['sha256']))
                self.assertEqual(image['sha256'], o['original_image_sha256'])
                self.assertEqual(image['url'], printing['image_url'])
                self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))

    def test_runtime_art_and_registry_are_the_preferred_official_face(self):
        legends = catalog.load_legend_manifest()
        for record in self.proof['records']:
            row = legends[record['legend_id']]; printing = record['members'][0]['printing']
            self.assertEqual(row['official_id'], printing['official_id'])
            self.assertEqual(row['text'], printing['effective']['rules_text'])
            self.assertEqual(row['tags'], printing['effective']['tags'])
            self.assertEqual({d.lower() for d in row['domains']},set(printing['effective']['domains']))
            self.assertEqual(row['art_path'],record['runtime_image']['path'])
            payload=(catalog.ROOT/row['art_path']).read_bytes()
            self.assertEqual(payload,(catalog.ROOT/record['members'][0]['original_image']['path']).read_bytes())
            self.assertEqual(catalog.sha(payload),record['runtime_image']['sha256'])

    def test_old_cohort_evidence_remains_exact_and_old_registry_entries_survive(self):
        self.assertEqual(len(history.legacy_legends()),8)
        for entry in self.proof['preserved'].values():
            self.assertEqual(catalog.sha((catalog.ROOT/entry['path']).read_bytes()),entry['sha256'])

    def test_new_legends_reject_altered_rules_domains_and_champion_identity(self):
        for index,key,value in [(8,'text','Optional trigger'),(9,'domains',['Fury','Body']),(8,'default_champion',35)]:
            with self.subTest(index=index,key=key):
                legends=copy.deepcopy(catalog.load_legend_manifest());legends[index][key]=value
                with patch.object(catalog,'load_legend_manifest',return_value=legends):
                    with self.assertRaises(ValueError):catalog.make_catalog(self.manifest,self.archives)


if __name__=='__main__':unittest.main()
