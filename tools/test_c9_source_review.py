"""The Tracker review preserves raw facts and cannot silently admit a card."""
import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog


class ArtworkDiscoveryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.data = catalog.read_archives()
        cls.reviews = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        cls.raw = catalog.project_gallery(cls.data['gallery'])[0]

    def apply(self, manifest=None, raw=None):
        return review.apply_reviews(
            self.raw if raw is None else raw, root=catalog.ROOT,
            project_gallery=lambda b: catalog.project_gallery(b)[0], plain=catalog.plain,
            manifest=self.reviews if manifest is None else manifest,
        )

    @staticmethod
    def observation(manifest, field='domains'):
        return next(i for i in manifest['historical_issues']
                    if i['review_issue_id'] == f'c9-rad-003-167-{field}')

    def test_source_only_correction_does_not_add_runtime_support(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:308]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            result = catalog.make_catalog(self.manifest, self.data)
        counts = result['counts']
        self.assertEqual((counts['implemented_main_cards'], counts['implemented_legal_identities'],
                          counts['unimplemented_legal_identities']), (308, 333, 746))
        tracker = next(c for c in result['cards'] if c['identity'] == 'rad-003-167')
        self.assertFalse(tracker['implementation']['playable_in_client'])
        printing = next(p for p in result['printings'] if p['official_id'] == 'rad-003-167')
        self.assertEqual((printing['domains'], printing['tags']), (['body'], []))
        self.assertEqual((printing['effective']['domains'], printing['effective']['tags']),
                         (['fury'], ['Bilgewater']))
        self.assertEqual(printing['resolution_ids'], ['rad-003-167:domains', 'rad-003-167:tags'])
        self.assertEqual((counts['reviewed_field_resolutions'], counts['field_reviewed_printings'],
                          counts['resolved_source_issue_groups']), (118, 74, 43))
        old = result['source_quality_issues'][:37]
        self.assertEqual(hashlib.sha256(catalog.canonical_json(old).encode()).hexdigest(),
                         '923bd9fee267e4e4e2d46301ea30771438e6146d805ab3bb45f177095388afae')
        self.assertEqual([i['kind'] for i in result['source_quality_issues'][37:39]],
                         ['official_artwork_field_mismatch'] * 2)

    def test_artwork_observation_cannot_invent_raw_or_printed_values(self):
        for key, value in [('raw_value', []), ('printed_value', ['chaos'])]:
            with self.subTest(key=key):
                changed = copy.deepcopy(self.reviews)
                self.observation(changed)['raw_issue']['values'][0][key] = value
                with self.assertRaisesRegex(ValueError, 'Artwork discovery values differ'):
                    self.apply(changed)

    def test_artwork_observation_cannot_refer_to_another_field_or_printing(self):
        for field, value in [('field', 'tags'), ('official_ids', ['rad-009-167'])]:
            with self.subTest(field=field):
                changed = copy.deepcopy(self.reviews)
                self.observation(changed)['raw_issue'][field] = value
                with self.assertRaisesRegex(ValueError, 'Artwork discovery'):
                    self.apply(changed)

    def test_artwork_observation_requires_its_specific_override(self):
        for field in ['domains', 'tags']:
            with self.subTest(field=field):
                changed = copy.deepcopy(self.reviews)
                changed['resolutions'] = [r for r in changed['resolutions']
                                         if r['id'] != f'rad-003-167:{field}']
                with self.assertRaisesRegex(ValueError, 'Unknown historical resolution'):
                    self.apply(changed)

    def test_artwork_discovery_basis_and_shape_are_not_arbitrary(self):
        for mode in ['basis', 'missing_basis', 'duplicate', 'kind', 'extra', 'date', 'linked_field']:
            with self.subTest(mode=mode):
                changed = copy.deepcopy(self.reviews)
                issue = self.observation(changed)
                if mode == 'basis':
                    issue['discovery_basis'] = 'guess'
                elif mode == 'missing_basis':
                    del issue['discovery_basis']
                elif mode == 'duplicate':
                    duplicate = copy.deepcopy(issue)
                    duplicate['review_issue_id'] += '-duplicate'
                    changed['historical_issues'].append(duplicate)
                elif mode == 'kind':
                    issue['raw_issue']['kind'] = 'missing_structured_type'
                elif mode == 'extra':
                    issue['raw_issue']['inferred'] = True
                elif mode == 'date':
                    issue['discovered_on'] = '2026-01-01'
                else:
                    issue['resolution_ids'] = ['rad-003-167:tags']
                with self.assertRaises(ValueError):
                    self.apply(changed)

    def test_any_changed_raw_tracker_fact_requires_new_review(self):
        for field, value in [('domains', ['fury']), ('tags', ['Bilgewater']), ('energy', 4),
                             ('image_url', 'https://cmsassets.rgpub.io/changed.png')]:
            with self.subTest(field=field):
                raw = copy.deepcopy(self.raw)
                next(p for p in raw if p['official_id'] == 'rad-003-167')[field] = value
                with self.assertRaisesRegex(ValueError, 'Stale candidate printing hash'):
                    self.apply(raw=raw)

    def test_tracker_image_and_origin_proof_are_mandatory(self):
        for mode in ['image', 'origin']:
            with self.subTest(mode=mode):
                changed = copy.deepcopy(self.reviews)
                if mode == 'image':
                    next(r for r in changed['resolutions'] if r['id'] == 'rad-003-167:domains')[
                        'evidence'][0]['sha256'] = '0' * 64
                else:
                    digest, spec = next(iter(changed['origin_archives'].items()))
                    spec['sha256'] = '0' * 64
                    changed['origin_archives'] = {'0' * 64: spec}
                    for r in changed['resolutions']:
                        if r['gallery_uncompressed_sha256'] == digest:
                            r['gallery_uncompressed_sha256'] = '0' * 64
                with self.assertRaisesRegex(ValueError, 'Evidence hash mismatch'):
                    self.apply(changed)


if __name__ == '__main__':
    unittest.main()
