"""Printed gear-review corrections preserve evidence and cannot admit gameplay."""
import copy
import gzip
import json
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog
import admission_history as history


class GearSourceReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.archive_manifest, cls.data = catalog.read_archives()
        cls.manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        cls.proof = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/reviews/gear-source-corrections.json').read_text())
        cls.raw = catalog.project_gallery(cls.data['gallery'])[0]
        # Reconstruct the runtime at the source-only review; later admission is separate.
        cls.historical_runtime = copy.deepcopy(catalog.load_runtime_manifest())
        cls.historical_runtime['cards'] = cls.historical_runtime['cards'][:362]
        with patch.object(catalog, 'load_runtime_manifest', return_value=cls.historical_runtime), patch.object(catalog, "load_legend_manifest", return_value=history.legacy_legends()):
            cls.snapshot = catalog.make_catalog(cls.archive_manifest, cls.data)

    def apply(self, manifest=None, raw=None):
        return review.apply_reviews(
            self.raw if raw is None else raw, root=catalog.ROOT,
            project_gallery=lambda data: catalog.project_gallery(data)[0], plain=catalog.plain,
            manifest=self.manifest if manifest is None else manifest,
        )

    def test_only_two_reviewed_effective_fields_change_and_neither_card_is_admitted(self):
        printings = {p['official_id']: p for p in self.snapshot['printings']}
        identities = {c['identity']: c for c in self.snapshot['cards']}
        expected = {'rad-083-167': ('tags', [], ['Mount Targon']),
                    'ven-058-166': ('types', ['unit'], ['unit', 'gear'])}
        self.assertEqual(set(expected), {r['official_id'] for r in self.proof['records']})
        for record in self.proof['records']:
            pid = record['official_id']
            field, raw, corrected = expected[pid]
            printing, identity = printings[pid], identities[pid]
            self.assertEqual(printing[field], raw)
            self.assertEqual(printing['effective'][field], corrected)
            self.assertEqual(printing['effective'], record['effective_after'])
            self.assertEqual([key for key in record['effective_before']
                              if record['effective_before'][key] != record['effective_after'][key]], [field])
            self.assertEqual(printing['resolution_ids'], [f'{pid}:{field}'])
            self.assertTrue(identity['standard_legal'])
            self.assertEqual(identity['official_ids'], [pid])
            self.assertEqual(identity['errata'], [])
            self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
            self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
            self.assertFalse(identity['implementation']['playable_in_client'])
        c = self.snapshot['counts']
        self.assertEqual((c['implemented_main_cards'], c['implemented_legal_identities'],
                          c['unimplemented_legal_identities']), (362, 387, 692))
        self.assertEqual((c['reviewed_field_resolutions'], c['field_reviewed_printings'],
                          c['source_quality_issues'], c['resolved_source_issue_groups']), (118, 74, 43, 43))
        self.assertFalse(self.proof['enables_runtime_support'])

    def test_compound_type_is_one_inventory_identity_with_both_types(self):
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'ven-058-166')
        self.assertEqual(identity['source_kind'], 'unit')
        self.assertEqual(identity['kind'], 'gear+unit')
        self.assertEqual(catalog.inventory_kind(['unit', 'gear']), catalog.inventory_kind(['gear', 'unit']))
        self.assertEqual(catalog.inventory_kind([]), 'tracker')
        counts = self.snapshot['counts']
        self.assertEqual(counts['standard_legal_by_kind']['gear+unit'], 1)
        self.assertEqual(counts['standard_legal_by_kind']['unit'], 546)
        self.assertEqual(sum(counts['standard_legal_by_kind'].values()), 1079)
        self.assertNotIn('gear+unit', counts['implemented_legal_by_kind'])
        printing = next(p for p in self.snapshot['printings'] if p['official_id'] == 'ven-058-166')
        # Projection v1 and the full original label are preserved, not reparsed in place.
        self.assertEqual(printing['accessibility_types'], [])
        self.assertTrue(printing['image_accessibility_text'].startswith('Riftbound Unit, Gear:'))
        self.assertEqual(printing['structured_types'], ['unit'])
        self.assertEqual(printing['effective']['equipment_text'], '')

    def test_original_faces_and_complete_raw_projections_match_immutable_origin(self):
        raw = {p['official_id']: p for p in self.raw}
        origin = self.proof['origin_gallery']
        payload = gzip.decompress((catalog.ROOT / origin['archive']).read_bytes())
        self.assertEqual((len(payload), catalog.sha(payload)), (origin['uncompressed_bytes'], origin['sha256']))
        self.assertEqual(catalog.project_gallery(payload)[0], self.raw)
        for record in self.proof['records']:
            pid, image, observed = record['official_id'], record['original_image'], record['visual_observation']
            self.assertEqual(raw[pid], record['raw_printing'])
            self.assertEqual(review.printing_sha256(raw[pid]), record['raw_printing_sha256'])
            data = (catalog.ROOT / image['path']).read_bytes()
            self.assertTrue(data.startswith(b'\x89PNG\r\n\x1a\n'))
            self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
            self.assertEqual(image['url'], raw[pid]['image_url'])
            self.assertTrue(observed['face_visually_inspected'])
            self.assertEqual(observed['original_image_sha256'], image['sha256'])
            self.assertEqual(observed['collector_code'], raw[pid]['public_code'])
            self.assertEqual(observed['name'], raw[pid]['name'])
            self.assertEqual(observed['uncertainties'], [])
            for face, field in [('types', 'types'), ('supertypes', 'super_types'), ('domains', 'domains'),
                                ('tags', 'tags'), ('energy', 'energy'), ('power', 'power'), ('might', 'might'),
                                ('might_bonus', 'might_bonus'), ('rules_icons_expanded', 'rules_text'),
                                ('equipment_icons_expanded', 'equipment_text')]:
                self.assertEqual(observed[f'printed_{face}'], record['effective_after'][field])
        self.assertEqual(sum(r['original_image']['bytes'] for r in self.proof['records']), 1_551_992)
        semantics = self.proof['type_semantics']
        self.assertEqual(catalog.sha((catalog.ROOT / semantics['core_rules']).read_bytes()), semantics['core_rules_sha256'])
        self.assertNotIn('/tmp/', json.dumps(self.proof))

    def test_all_previous_reviews_runtime_lists_and_variant_evidence_are_unchanged(self):
        digest = lambda value: catalog.sha(review.canonical_bytes(value))
        for key, saved in self.proof['preserved_source_cohorts'].items():
            # Subsequent reviews append evidence; the original cohort stays pinned.
            self.assertGreaterEqual(len(self.manifest[key]), saved['count'] + 2)
            self.assertEqual(digest(self.manifest[key][:saved['count']]), saved['canonical_sha256'])
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        saved = self.proof['preserved_field_image_index']
        self.assertGreaterEqual(len(images), saved['count'] + 2)
        self.assertEqual(digest(images[:saved['count']]), saved['canonical_sha256'])
        for path, sha in self.proof['preserved_files'].items():
            payload = (catalog.ROOT / path).read_bytes()
            if path == 'assets/data/decks.json':
                payload = (json.dumps(self.historical_runtime, ensure_ascii=False, indent=2) + '\n').encode()
            elif path == 'assets/data/legends.json':
                payload = history.legacy_legend_manifest_bytes()
            elif path == 'src/deck.rs':
                payload = history.legacy_recipe_source_bytes()
            self.assertEqual(catalog.sha(payload), sha, path)

    def test_porobot_cannot_be_registered_as_an_ordinary_unit_or_gear(self):
        record = next(r for r in self.proof['records'] if r['official_id'] == 'ven-058-166')
        facts = record['effective_after']
        for kind in ['Unit', 'Gear']:
            with self.subTest(kind=kind):
                runtime = copy.deepcopy(catalog.load_runtime_manifest())
                candidate = copy.deepcopy(runtime['cards'][0])
                candidate.update(id=len(runtime['cards']), official_id=record['official_id'],
                                 name=record['name'], kind=kind, energy=facts['energy'], power=0,
                                 might=facts['might'], domains=['Mind'], tags=facts['tags'],
                                 official_text=facts['rules_text'], official_text_html=facts['rules_html'],
                                 equipment_effect_html='', art_url=record['original_image']['url'])
                runtime['cards'].append(candidate)
                with patch.object(catalog, 'load_runtime_manifest', return_value=runtime), patch.object(catalog, "load_legend_manifest", return_value=history.legacy_legends()):
                    with self.assertRaisesRegex(ValueError, 'Runtime main-card kind mismatch: ven-058-166'):
                        catalog.make_catalog(self.archive_manifest, self.data)

    def test_reviewed_compound_unit_requires_might_and_valid_distinct_types(self):
        printing = copy.deepcopy(next(p for p in self.raw if p['official_id'] == 'ven-058-166'))
        empty = {'schema_version': 1, 'printing_projection_version': 1, 'resolutions': []}
        printing.update(types=['unit', 'gear'], structured_types=['unit', 'gear'], might=None)
        result = self.apply(manifest=empty, raw=[printing])['printings'][0]
        self.assertEqual(result['unresolved_gameplay_fields'], ['might'])
        printing['might'] = 0
        self.assertEqual(self.apply(manifest=empty, raw=[printing])['printings'][0]['unresolved_gameplay_fields'], [])
        for types in [['unit', 'unit'], ['unit', 'invented']]:
            with self.subTest(types=types):
                printing['types'] = types
                with self.assertRaises(ValueError):
                    self.apply(manifest=empty, raw=[printing])

    def test_new_observations_and_complete_source_prerequisites_fail_closed(self):
        for record in self.proof['records']:
            pid = record['official_id']
            with self.subTest(pid=pid, change='observed value'):
                manifest = copy.deepcopy(self.manifest)
                issue = next(i for i in manifest['historical_issues'] if i['printing_ids'] == [pid])
                issue['raw_issue']['values'][0]['printed_value'] = ['invented']
                with self.assertRaisesRegex(ValueError, 'Artwork discovery values differ'):
                    self.apply(manifest=manifest)
            with self.subTest(pid=pid, change='publisher source'):
                raw = copy.deepcopy(self.raw)
                next(p for p in raw if p['official_id'] == pid)['energy'] += 1
                with self.assertRaisesRegex(ValueError, 'Stale candidate printing hash'):
                    self.apply(raw=raw)


if __name__ == '__main__':
    unittest.main()
