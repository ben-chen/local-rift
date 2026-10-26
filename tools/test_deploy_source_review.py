"""Bounded source reconciliation; this review adds no executable Deploy cards."""
import copy
import hashlib
import json
from unittest.mock import patch
import unittest

import import_catalog as catalog
import catalog_review as review


def digest(value):
    return hashlib.sha256(review.canonical_bytes(value)).hexdigest()


class DeploySourceReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = catalog.ROOT
        cls.proof = json.loads((cls.root / 'assets/data/catalog_evidence/reviews/deploy-source.json').read_text())
        cls.manifest = json.loads((cls.root / 'assets/data/catalog_resolutions.json').read_text())
        cls.origin, cls.data = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.origin, cls.data)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}

    def test_complete_original_faces_and_raw_projection_hashes_are_pinned(self):
        self.assertEqual(len(self.proof['records']), 16)
        for record in self.proof['records']:
            p = record['raw_printing']; image = record['image']
            self.assertEqual(review.printing_sha256(p), record['raw_printing_sha256'])
            actual = (self.root / image['local_path']).read_bytes()
            self.assertEqual((len(actual), hashlib.sha256(actual).hexdigest()), (image['bytes'], image['sha256']))
            self.assertEqual(actual[:8], b'\x89PNG\r\n\x1a\n')
            self.assertEqual((int.from_bytes(actual[16:20], 'big'), int.from_bytes(actual[20:24], 'big')), tuple(image['dimensions']))
            self.assertEqual(image['url'], p['image_url'])
            cohort = sorted(v['official_id'] for v in self.snapshot['printings'] if catalog.normalize(v['name']) == catalog.normalize(p['name']))
            self.assertEqual(cohort, record['identity']['official_ids'])
            self.assertEqual(cohort, [p['official_id']])
            self.assertFalse(record['identity']['errata'])
            self.assertTrue(record['identity']['standard_legal'])

    def test_five_wrong_raw_domains_are_preserved_and_only_effective_values_change(self):
        expected = {'rad-024-167': ('chaos', 'calm'), 'rad-017-167': ('chaos', 'fury'),
                    'rad-062-167': ('calm', 'mind'), 'rad-073-167': ('calm', 'body'),
                    'rad-106-167': ('body', 'chaos')}
        for pid, (raw, effective) in expected.items():
            p = self.printings[pid]
            self.assertEqual(p['domains'], [raw]); self.assertEqual(p['effective']['domains'], [effective])
            r = next(r for r in self.manifest['resolutions'] if r['id'] == pid + ':domains')
            self.assertEqual(r['basis'], 'confirmed_artwork')
            issue = next(i for i in self.manifest['historical_issues'] if i['review_issue_id'] == pid + '-domains')
            self.assertEqual(issue['discovery_basis'], 'confirmed_artwork')
            self.assertEqual(issue['raw_issue']['values'], [{'official_id':pid, 'raw_value':[raw], 'printed_value':[effective]}])

    def test_ten_device_exclusions_are_rule_derived_and_keep_signature_tags(self):
        ids = [x for x in self.proof['proposed_resolution_ids'] if x.endswith(':tags')]
        self.assertEqual(len(ids), 10)
        for rid in ids:
            r = next(x for x in self.manifest['resolutions'] if x['id'] == rid)
            p = self.printings[r['official_id']]
            self.assertIn('Device', p['tags']); self.assertNotIn('Device', p['effective']['tags'])
            self.assertEqual(r['basis'], 'rule_derived')
            self.assertEqual(p['effective']['tags'], [t for t in p['tags'] if t != 'Device'])
            authority = next(x for x in r['evidence'] if x['kind'] == 'official_core_rules')
            self.assertEqual(authority['rules'], ['133.8', '763.1', '830'])
            self.assertEqual(hashlib.sha256((self.root / authority['local_path']).read_bytes()).hexdigest(), authority['sha256'])
            self.assertFalse(any(i.get('discovery_basis') == 'confirmed_artwork' and rid in i['resolution_ids'] for i in self.manifest['historical_issues']))
        self.assertEqual(self.printings['rad-156-167']['effective']['tags'], ['Jarvan IV'])
        self.assertEqual(self.printings['rad-146-167']['effective']['tags'], ['Orianna'])

    def test_previous_source_cohorts_remain_identical_prefixes(self):
        baseline = self.proof['preserved_source_prefixes']
        for key in ['resolutions', 'raw_printings', 'historical_issues']:
            pkey = 'field_resolutions' if key == 'resolutions' else key
            self.assertEqual(digest(self.manifest[key][:baseline[pkey]]), baseline[pkey + '_sha256'])
        images = json.loads((self.root / 'assets/data/catalog_evidence/images.json').read_text())
        self.assertEqual(digest(images[:baseline['images']]), baseline['images_sha256'])
        self.assertEqual(self.manifest['resolutions'][122:137], [r for r in self.manifest['resolutions'] if r['id'] in self.proof['proposed_resolution_ids']])
        self.assertTrue(all(not r['enables_runtime_support'] for r in self.manifest['resolutions'][122:137]))

    def test_source_review_does_not_register_any_of_the_sixteen_candidates(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest()); runtime['cards'] = runtime['cards'][:401]
        legends = copy.deepcopy(catalog.load_legend_manifest())[:10]
        fields = copy.deepcopy(catalog.load_battlefield_manifest())[:11]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime), \
             patch.object(catalog, 'load_legend_manifest', return_value=legends), \
             patch.object(catalog, 'load_battlefield_manifest', return_value=fields):
            source_only = catalog.make_catalog(self.origin, self.data)
        self.assertEqual(tuple(source_only['counts'][k] for k in ['implemented_main_cards','implemented_legal_identities','unimplemented_legal_identities']), (401,428,651))
        ids = {x['identity']['identity'] for x in self.proof['records']}
        for identity in source_only['cards']:
            if identity['identity'] in ids:
                self.assertEqual(identity['implementation'], {'status':'not_implemented','playable_in_client':False})
                self.assertFalse(identity['source_review']['unresolved_gameplay_fields'])
                self.assertFalse(identity['source_review']['unresolved_variant_fields'])

    def test_every_observed_gameplay_fact_matches_effective_source(self):
        for record in self.proof['records']:
            current = self.printings[record['raw_printing']['official_id']]
            for field, value in record['observed_facts'].items():
                self.assertEqual(current['effective'][field], value)
            for field in ['rules_html','rules_text','equipment_html','equipment_text']:
                self.assertEqual(current['effective'][field], record['previous_effective'][field])
        self.assertEqual(self.proof['source_only_check']['runtime_admissions'], 0)

    def test_changed_publisher_projection_requires_explicit_renewed_review(self):
        raw, _ = catalog.project_gallery(self.data['gallery'])
        for pid in ['rad-024-167','rad-017-167','rad-062-167','rad-073-167','rad-106-167']:
            changed = copy.deepcopy(raw); p = next(x for x in changed if x['official_id'] == pid)
            p['domains'] = self.printings[pid]['effective']['domains']
            with self.assertRaisesRegex(ValueError, 'Stale candidate printing hash'):
                review.apply_reviews(changed, root=self.root, project_gallery=lambda raw: catalog.project_gallery(raw)[0], plain=catalog.plain)


if __name__ == '__main__':
    unittest.main()
