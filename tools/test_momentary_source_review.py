"""Momentary's printed tags are a bounded source correction, never an admission."""
import copy
import gzip
import json
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog
import admission_history as history


class MomentarySourceReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.archive_manifest, cls.data = catalog.read_archives()
        cls.manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        cls.proof = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/reviews/momentary-companion.json').read_text())
        cls.raw = catalog.project_gallery(cls.data['gallery'])[0]
        cls.historical_runtime = copy.deepcopy(catalog.load_runtime_manifest())
        cls.historical_runtime["cards"] = cls.historical_runtime["cards"][:349]
        with patch.object(catalog, "load_runtime_manifest", return_value=cls.historical_runtime), patch.object(catalog, "load_legend_manifest", return_value=history.legacy_legends()):
            cls.snapshot = catalog.make_catalog(cls.archive_manifest, cls.data)

    def apply(self, manifest=None, raw=None):
        return review.apply_reviews(
            self.raw if raw is None else raw, root=catalog.ROOT,
            project_gallery=lambda b: catalog.project_gallery(b)[0], plain=catalog.plain,
            manifest=self.manifest if manifest is None else manifest,
        )

    @staticmethod
    def resolution(manifest):
        return next(r for r in manifest['resolutions'] if r['id'] == 'rad-095-167:tags')

    @staticmethod
    def observation(manifest):
        return next(i for i in manifest['historical_issues']
                    if i['review_issue_id'] == 'momentary-rad-095-167-tags')

    def test_only_effective_tags_change_and_card_stays_unavailable(self):
        pid = 'rad-095-167'
        printing = next(p for p in self.snapshot['printings'] if p['official_id'] == pid)
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == pid)
        self.assertEqual(printing['tags'], [])
        self.assertEqual(printing['effective']['tags'], ['Shadow Isles', 'Demon'])
        self.assertEqual(printing['effective'], self.proof['effective_after'])
        before, after = self.proof['effective_before'], self.proof['effective_after']
        self.assertEqual([key for key in before if before[key] != after[key]], ['tags'])
        self.assertEqual(printing['resolution_ids'], ['rad-095-167:tags'])
        self.assertEqual(identity['official_ids'], [pid])
        self.assertEqual(identity['errata'], [])
        self.assertTrue(identity['standard_legal'])
        self.assertFalse(identity['implementation']['playable_in_client'])
        self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
        self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
        counts = self.snapshot['counts']
        self.assertEqual((counts['implemented_main_cards'], counts['implemented_legal_identities'],
                          counts['unimplemented_legal_identities']), (349, 374, 705))
        self.assertEqual((counts['reviewed_field_resolutions'], counts['field_reviewed_printings'],
                          counts['source_quality_issues'], counts['resolved_source_issue_groups']),
                         (137, 87, 52, 52))
        self.assertFalse(self.proof['enables_runtime_support'])
        self.assertFalse(self.resolution(self.manifest)['enables_runtime_support'])
        self.assertNotIn(pid, {c['official_id'] for c in self.historical_runtime['cards']})

    def test_original_face_and_origin_match_complete_raw_facts(self):
        raw = next(p for p in self.raw if p['official_id'] == 'rad-095-167')
        self.assertEqual(raw, self.proof['raw_printing'])
        self.assertEqual(review.printing_sha256(raw), self.proof['raw_printing_sha256'])
        origin = self.proof['origin_gallery']
        archive = (catalog.ROOT / origin['archive']).read_bytes()
        uncompressed = gzip.decompress(archive)
        self.assertEqual(catalog.sha(uncompressed), origin['sha256'])
        origin_raw = next(p for p in catalog.project_gallery(uncompressed)[0]
                          if p['official_id'] == 'rad-095-167')
        self.assertEqual(origin_raw, raw)
        image = self.proof['original_image']
        payload = (catalog.ROOT / image['path']).read_bytes()
        self.assertEqual((len(payload), catalog.sha(payload)), (759_278, image['sha256']))
        self.assertEqual(image['bytes'], len(payload))
        self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))
        self.assertEqual(image['url'], raw['image_url'])
        observation = self.proof['visual_observation']
        self.assertEqual(observation['original_image_sha256'], image['sha256'])
        self.assertEqual(observation['collector_code'], raw['public_code'])
        self.assertEqual(observation['name'], raw['name'])
        self.assertTrue(observation['face_visually_inspected'])
        self.assertEqual(observation['uncertainties'], [])
        for face, effective in [
            ('printed_types', 'types'), ('printed_supertypes', 'super_types'),
            ('printed_domains', 'domains'), ('printed_tags', 'tags'),
            ('printed_energy', 'energy'), ('printed_power', 'power'),
            ('printed_might', 'might'), ('printed_might_bonus', 'might_bonus'),
            ('printed_rules_icons_expanded', 'rules_text'),
            ('printed_equipment_icons_expanded', 'equipment_text'),
        ]:
            self.assertEqual(observation[face], self.proof['effective_after'][effective], face)

    def test_previous_source_cohorts_and_all_variant_evidence_are_unchanged(self):
        digest = lambda value: catalog.sha(review.canonical_bytes(value))
        for key, preserved in self.proof['preserved_source_cohorts'].items():
            self.assertGreaterEqual(len(self.manifest[key]), preserved['count'] + 1)
            self.assertEqual(digest(self.manifest[key][:preserved['count']]),
                             preserved['canonical_sha256'])
        images = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/images.json').read_text())
        preserved = self.proof['preserved_field_image_index']
        self.assertGreaterEqual(len(images), preserved['count'] + 1)
        self.assertEqual(digest(images[:preserved['count']]), preserved['canonical_sha256'])
        variant = self.proof['preserved_variant_review']
        for path, key in [
            ('assets/data/catalog_identity_reviews.json', 'manifest_sha256'),
            ('assets/data/catalog_evidence/variant_images.json', 'image_index_sha256'),
        ]:
            self.assertEqual(catalog.sha((catalog.ROOT / path).read_bytes()), variant[key])
        self.assertEqual((variant['identities'], variant['printings']), (29, 90))
        self.assertEqual(self.snapshot['counts']['reviewed_variant_identities'], 29)
        self.assertEqual(self.snapshot['counts']['reviewed_variant_printings'], 90)

    def test_superseded_observation_remains_explicit_and_proof_is_portable(self):
        superseded = self.proof['superseded_visual_observation']
        self.assertEqual(superseded['status'], 'superseded')
        self.assertTrue(superseded['reason'])
        old, new = superseded['observation'], self.proof['visual_observation']
        self.assertEqual(old['printed_tags'], [])
        self.assertEqual(new['printed_tags'], ['Shadow Isles', 'Demon'])
        self.assertEqual(old['original_image_sha256'], new['original_image_sha256'])
        self.assertEqual(old['official_id'], new['official_id'])
        self.assertNotIn('/tmp/', json.dumps(self.proof))
        self.assertFalse(self.proof['original_image']['path'].startswith('/'))

    def test_tampered_visual_values_fail_closed(self):
        for key, value in [('raw_value', ['Demon']), ('printed_value', ['Shadow Isles'])]:
            with self.subTest(key=key):
                changed = copy.deepcopy(self.manifest)
                self.observation(changed)['raw_issue']['values'][0][key] = value
                with self.assertRaisesRegex(ValueError, 'Artwork discovery values differ'):
                    self.apply(changed)

    def test_wrong_linked_field_or_printing_fails_closed(self):
        for mode in ['field', 'printing', 'resolution']:
            with self.subTest(mode=mode):
                changed = copy.deepcopy(self.manifest)
                issue = self.observation(changed)
                if mode == 'field':
                    issue['raw_issue']['field'] = 'domains'
                elif mode == 'printing':
                    issue['raw_issue']['official_ids'] = ['rad-003-167']
                else:
                    issue['resolution_ids'] = ['rad-003-167:tags']
                with self.assertRaises(ValueError):
                    self.apply(changed)

    def test_artwork_discovery_requires_its_override(self):
        changed = copy.deepcopy(self.manifest)
        changed['resolutions'] = [r for r in changed['resolutions'] if r['id'] != 'rad-095-167:tags']
        with self.assertRaises(ValueError):
            self.apply(changed)

    def test_changed_raw_prerequisites_require_a_new_review(self):
        for field, value in [('tags', ['Demon']), ('domains', ['body']), ('might', 5),
                             ('image_url', 'https://cmsassets.rgpub.io/changed.png')]:
            with self.subTest(field=field):
                raw = copy.deepcopy(self.raw)
                next(p for p in raw if p['official_id'] == 'rad-095-167')[field] = value
                with self.assertRaisesRegex(ValueError, 'Stale candidate printing hash'):
                    self.apply(raw=raw)

    def test_original_image_and_archive_proof_cannot_be_substituted(self):
        for mode in ['image_hash', 'image_size', 'image_file', 'archive']:
            with self.subTest(mode=mode):
                changed = copy.deepcopy(self.manifest)
                evidence = self.resolution(changed)['evidence'][0]
                if mode == 'image_hash':
                    evidence['sha256'] = '0' * 64
                elif mode == 'image_size':
                    evidence['bytes'] += 1
                elif mode == 'image_file':
                    evidence['local_path'] = 'assets/data/catalog_evidence/images/rad-003-167.png'
                else:
                    next(iter(changed['origin_archives'].values()))['sha256'] = '0' * 64
                with self.assertRaises(ValueError):
                    self.apply(changed)


if __name__ == '__main__':
    unittest.main()
