"""C20 exact faces, narrow printed-tag corrections, and append-only admission."""
import copy
import gzip
import json
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog

BASE = 389
ORDER = ['unl-032-219', 'unl-064-219', 'rad-129-167', 'sfd-041-221', 'rad-074-167', 'ven-033-166']
CORRECTED = {'rad-129-167': ['Noxus', 'Trifarian'], 'rad-074-167': ['Freljord']}


def digest(value):
    return catalog.sha(review.canonical_bytes(value))


class C20AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(cls.archives['gallery'])[0]}
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c20/admissions.json').read_text())
        cls.correction = json.loads((catalog.ROOT / 'assets/data/catalog_evidence/reviews/c20-tags.json').read_text())
        cls.manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:395]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}
        cls.identities = {p['identity']: p for p in cls.snapshot['cards']}
        runtime['cards'] = runtime['cards'][:BASE]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            cls.source_only = catalog.make_catalog(cls.origin, cls.archives)

    def apply(self, manifest=None, raw=None):
        return review.apply_reviews(list(self.raw.values()) if raw is None else raw,
            root=catalog.ROOT, project_gallery=lambda data: catalog.project_gallery(data)[0],
            plain=catalog.plain, manifest=self.manifest if manifest is None else manifest)

    def check_record(self, record):
        pid = record['official_id']; i = BASE + ORDER.index(pid)
        raw = self.raw[pid]; e = self.printings[pid]['effective']; identity = self.identities[pid]
        runtime = catalog.load_runtime_manifest()['cards'][i]
        self.assertEqual(record['runtime_id'], i)
        self.assertEqual(record['raw_printing'], raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(record['effective'], e)
        self.assertEqual(record['existing_field_resolution_ids'], [pid + ':tags'] if pid in CORRECTED else [])
        self.assertEqual(identity['official_ids'], [pid])
        self.assertTrue(identity['standard_legal'])
        self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
        self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
        self.assertEqual(record['errata'], identity['errata'])
        self.assertEqual(record['errata'], [])
        self.assertEqual(record['runtime_rules_text'], e['rules_text'])
        self.assertEqual(runtime['official_text'], e['rules_text'])
        self.assertEqual(runtime['official_text_html'], e['rules_html'])
        self.assertEqual(runtime['equipment_effect_html'], e['equipment_html'])
        self.assertEqual(runtime['official_id'], pid)
        for field in ['energy', 'power', 'might']:
            self.assertEqual(runtime[field], e[field] or 0)
        self.assertEqual(runtime['tags'], e['tags'])
        for key in ['original_image', 'runtime_image']:
            image = record[key]; path = catalog.ROOT / image['path']
            self.assertFalse(image['path'].startswith('/'))
            self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
            payload = path.read_bytes()
            self.assertEqual((len(payload), catalog.sha(payload)), (image['bytes'], image['sha256']))
            self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))
            w, h = struct.unpack('>II', payload[16:24])
            self.assertAlmostEqual(w / h, 744 / 1039, delta=.002)
            if key == 'runtime_image':
                self.assertLess(len(payload), 300_000)
                self.assertEqual((w, h), (image['width'], image['height']))
        self.assertEqual(record['original_image']['url'], raw['image_url'])
        self.assertEqual(record['runtime_image']['url'], runtime['art_url'])
        self.assertEqual(record['runtime_image']['url'].split('?')[0], raw['image_url'].split('?')[0])
        o = record['visual_observation']
        self.assertTrue(o['face_visually_inspected'])
        self.assertEqual(o['uncertainties'], [])
        self.assertEqual(o['independently_reinspected_on'], '2026-10-24')
        self.assertEqual((o['official_id'], o['name'], o['collector_code']), (pid, raw['name'], raw['public_code']))
        self.assertEqual(o['original_image_sha256'], record['original_image']['sha256'])
        for observed, source in [('printed_types', 'types'), ('printed_supertypes', 'super_types'),
                ('printed_domains', 'domains'), ('printed_tags', 'tags'), ('printed_energy', 'energy'),
                ('printed_power', 'power'), ('printed_might', 'might'), ('printed_might_bonus', 'might_bonus'),
                ('printed_rules_icons_expanded', 'rules_text'), ('printed_equipment_icons_expanded', 'equipment_text')]:
            self.assertEqual(o[observed], e[source])

    def test_six_complete_raw_effective_and_original_face_records(self):
        self.assertFalse(self.proof['draft_not_for_import'])
        self.assertEqual([r['official_id'] for r in self.proof['records']], ORDER)
        self.assertEqual(catalog.sha(self.archives['gallery']), self.proof['origin_gallery']['uncompressed_sha256'])
        for record in self.proof['records']:
            with self.subTest(card=record['name']):
                self.check_record(record)
        self.assertNotIn('/tmp/', json.dumps(self.proof))

    def test_two_corrections_preserve_raw_tags_and_cannot_admit_cards_alone(self):
        for record in self.correction['records']:
            pid = record['official_id']; p = self.printings[pid]
            self.assertEqual(p['tags'], [])
            self.assertEqual(p['effective']['tags'], CORRECTED[pid])
            self.assertEqual(record['effective_after'], p['effective'])
            self.assertEqual([k for k in record['effective_before'] if record['effective_before'][k] != record['effective_after'][k]], ['tags'])
            identity = next(x for x in self.source_only['cards'] if x['identity'] == pid)
            self.assertFalse(identity['implementation']['playable_in_client'])
        self.assertFalse(self.correction['enables_runtime_support'])
        counts = self.source_only['counts']
        self.assertEqual((counts['implemented_main_cards'], counts['implemented_legal_identities'], counts['unimplemented_legal_identities']), (389, 416, 663))
        self.assertEqual(self.correction['source_only_check']['field_resolutions'], 122)
        self.assertEqual(self.correction['source_only_check']['reviewed_printings'], 77)
        self.assertEqual(self.correction['source_only_check']['historical_issues'], 47)

    def test_correction_origin_full_raw_and_face_proof_agree(self):
        p = self.correction; origin = gzip.decompress((catalog.ROOT / p['origin_gallery']['archive']).read_bytes())
        self.assertEqual((len(origin), catalog.sha(origin)), (p['origin_gallery']['uncompressed_bytes'], p['origin_gallery']['sha256']))
        self.assertEqual({r['official_id']: r for r in catalog.project_gallery(origin)[0]}, self.raw)
        self.assertNotIn('/tmp/', json.dumps(p))
        for record in p['records']:
            pid = record['official_id']; raw = self.raw[pid]; o = record['visual_observation']; image = record['original_image']
            self.assertEqual(record['raw_printing'], raw)
            self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
            self.assertEqual(image['url'], raw['image_url'])
            payload = (catalog.ROOT / image['path']).read_bytes()
            self.assertEqual((len(payload), catalog.sha(payload)), (image['bytes'], image['sha256']))
            self.assertEqual(o['original_image_sha256'], image['sha256'])
            self.assertEqual(o['printed_tags'], CORRECTED[pid])
            admission = next(x for x in self.proof['records'] if x['official_id'] == pid)
            self.assertEqual(image['path'], admission['original_image']['path'])
        pin = self.proof['source_correction_proof']
        self.assertEqual(catalog.sha((catalog.ROOT / pin['path']).read_bytes()), pin['sha256'])

    def test_prior_runtime_recipes_and_entire_prior_source_cohorts_are_unchanged(self):
        runtime = catalog.load_runtime_manifest(); pin = self.proof['preserved_runtime_prefix']; root = catalog.ROOT
        self.assertEqual(pin['card_count'], BASE)
        self.assertEqual(digest(runtime['cards'][:BASE]), pin['cards_sha256'])
        self.assertEqual(digest(runtime['decks']), pin['presets_sha256'])
        prefix = (root / 'src/cards.rs').read_text().split('        CardDef {\n            id: 389,', 1)[0]
        self.assertEqual(catalog.sha(prefix.encode()), pin['carddefs_source_prefix_sha256'])
        self.assertEqual(catalog.sha((root / 'assets/data/legends.json').read_bytes()), pin['legends_manifest_sha256'])
        self.assertEqual(catalog.sha((root / 'src/deck.rs').read_bytes()), pin['training_recipe_source_sha256'])
        for key, old in self.correction['preserved_source_cohorts'].items():
            self.assertEqual(digest(self.manifest[key][:old['count']]), old['canonical_sha256'])
        index = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        old = self.correction['preserved_field_image_index']
        self.assertEqual(digest(index[:old['count']]), old['canonical_sha256'])
        v = self.correction['preserved_variant_review']
        self.assertEqual((v['identities'], v['printings']), (29, 90))
        for path, key in [('assets/data/catalog_identity_reviews.json', 'manifest_sha256'), ('assets/data/catalog_evidence/variant_images.json', 'image_index_sha256')]:
            self.assertEqual(catalog.sha((root / path).read_bytes()), v[key])

    def test_current_five_errata_authorities_have_no_applicable_entry(self):
        self.assertEqual({s['key'] for s in self.proof['errata_sources']}, {k for k in catalog.SOURCES if k.endswith('_errata')})
        for source in self.proof['errata_sources']:
            key = source['key']; parsed = catalog.parse_errata(self.archives[key], key)
            self.assertEqual(source['url'], catalog.SOURCES[key])
            self.assertEqual(source['archive'], self.origin['sources'][key]['archive'])
            self.assertEqual(source['uncompressed_sha256'], catalog.sha(self.archives[key]))
            self.assertEqual(source['parsed_entries_sha256'], digest(parsed))
            self.assertEqual(source['live_parsed_entries_sha256'], digest(parsed))
            self.assertEqual(source['live_checked_on'], '2026-10-24')
        self.assertTrue(all(not r['errata'] for r in self.proof['records']))

    def test_changed_raw_candidate_prerequisites_fail_closed(self):
        for pid in CORRECTED:
            for field, value in [('tags', CORRECTED[pid]), ('might', 99), ('domains', ['fury'])]:
                raw = copy.deepcopy(list(self.raw.values())); next(r for r in raw if r['official_id'] == pid)[field] = value
                with self.subTest(pid=pid, field=field), self.assertRaisesRegex(ValueError, 'Stale candidate printing hash'):
                    self.apply(raw=raw)

    def test_artwork_discovery_values_field_and_links_cannot_drift(self):
        for pid in CORRECTED:
            for mode in ['raw', 'printed', 'field', 'resolution', 'image', 'origin']:
                m = copy.deepcopy(self.manifest); issue = next(x for x in m['historical_issues'] if x['review_issue_id'] == pid + '-tags')
                resolution = next(x for x in m['resolutions'] if x['id'] == pid + ':tags')
                if mode == 'raw': issue['raw_issue']['values'][0]['raw_value'] = ['Invented']
                elif mode == 'printed': issue['raw_issue']['values'][0]['printed_value'] = ['Invented']
                elif mode == 'field': issue['raw_issue']['field'] = 'domains'
                elif mode == 'resolution': m['resolutions'].remove(resolution)
                elif mode == 'image': resolution['evidence'][0]['sha256'] = '0' * 64
                else: resolution['gallery_uncompressed_sha256'] = '0' * 64
                with self.subTest(pid=pid, mode=mode), self.assertRaises(ValueError):
                    self.apply(manifest=m)

    def test_admission_visual_image_and_effective_tampering_is_rejected(self):
        for section, field, value in [('raw_printing', 'energy', 9), ('effective', 'tags', ['Invented']),
                ('visual_observation', 'printed_rules_icons_expanded', 'Invented'),
                ('visual_observation', 'printed_tags', []), ('original_image', 'sha256', '0' * 64), ('runtime_image', 'bytes', 1)]:
            record = copy.deepcopy(self.proof['records'][2]); record[section][field] = value
            with self.subTest(section=section, field=field), self.assertRaises(AssertionError):
                self.check_record(record)

    def test_runtime_fact_drift_is_rejected(self):
        runtime = catalog.load_runtime_manifest()
        for field, value in [('kind', 'Unit'), ('energy', 9), ('power', 1), ('tags', ['Mech']), ('domains', ['Fury']), ('might', 1), ('official_text_html', '<p>Invented.</p>')]:
            changed = copy.deepcopy(runtime); changed['cards'][BASE][field] = value
            with self.subTest(field=field), patch.object(catalog, 'load_runtime_manifest', return_value=changed):
                with self.assertRaisesRegex(ValueError, 'Runtime main-card .*mismatch'):
                    catalog.make_catalog(self.origin, self.archives)

    def test_ten_existing_legend_homes_and_no_new_champion_eligibility(self):
        legends = catalog.load_legend_manifest(); self.assertEqual(len(legends), 10)
        for record in self.proof['records']:
            e = record['effective']; self.assertEqual(e['super_types'], [])
            self.assertTrue(any(set(e['domains']) <= {d.lower() for d in l['domains']} for l in legends))


if __name__ == '__main__':
    unittest.main()
