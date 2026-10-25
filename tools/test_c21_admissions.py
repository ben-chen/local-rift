"""C21 complete printing cohorts, original faces, and append-only admission."""
import copy
import json
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog

BASE = 395
ORDER = ['sfd-058-221', 'ven-056-166', 'unl-136-219']
HOMES = [[0, 1], [3, 5, 6, 9], [0, 2, 9]]


def digest(value):
    return catalog.sha(review.canonical_bytes(value))


class C21AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(cls.archives['gallery'])[0]}
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c21/admissions.json').read_text())
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:398]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}
        cls.identities = {p['identity']: p for p in cls.snapshot['cards']}

    def check_image(self, image, original=False):
        self.assertFalse(image['path'].startswith('/'))
        path = catalog.ROOT / image['path']
        self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
        payload = path.read_bytes()
        self.assertEqual((len(payload), catalog.sha(payload)), (image['bytes'], image['sha256']))
        self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))
        width, height = struct.unpack('>II', payload[16:24])
        if original:
            self.assertEqual((width, height), (744, 1039))
        else:
            self.assertLess(len(payload), 300_000)
            self.assertEqual((width, height), (image['width'], image['height']))
        self.assertAlmostEqual(width / height, 744 / 1039, delta=.002)

    def check_face(self, face):
        pid = face['official_id']; raw = self.raw[pid]; e = self.printings[pid]['effective']
        self.assertEqual(face['name'], raw['name'])
        self.assertEqual(face['raw_printing'], raw)
        self.assertEqual(face['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(face['effective'], e)
        self.assertEqual(face['effective_projection_sha256'], review.printing_sha256({**raw, **e}))
        self.assertEqual({k: raw[k] for k in e}, e)
        self.assertEqual(face['existing_field_resolution_ids'], [])
        self.assertEqual(face['original_image']['url'], raw['image_url'])
        self.check_image(face['original_image'], original=True)
        o = face['visual_observation']
        self.assertTrue(o['face_visually_inspected'])
        self.assertEqual(o['uncertainties'], [])
        self.assertEqual(o['independently_reinspected_on'], '2026-10-24')
        self.assertEqual((o['official_id'], o['name'], o['collector_code']), (pid, raw['name'], raw['public_code']))
        self.assertEqual(o['original_image_sha256'], face['original_image']['sha256'])
        for observed, source in [('printed_types', 'types'), ('printed_supertypes', 'super_types'),
                ('printed_domains', 'domains'), ('printed_tags', 'tags'), ('printed_energy', 'energy'),
                ('printed_power', 'power'), ('printed_might', 'might'), ('printed_might_bonus', 'might_bonus'),
                ('printed_rules_icons_expanded', 'rules_text'), ('printed_equipment_icons_expanded', 'equipment_text')]:
            if source in ('rules_text', 'equipment_text'):
                # Physical line wrapping and the feed's doubled Bloom space
                # are retained separately; no operative word is normalized.
                self.assertEqual(' '.join(o[observed].split()), ' '.join(e[source].split()))
            else:
                self.assertEqual(o[observed], e[source])

    def test_three_identities_have_all_four_original_faces(self):
        self.assertFalse(self.proof['draft_not_for_import'])
        self.assertEqual([r['official_id'] for r in self.proof['records']], ORDER)
        self.assertEqual(catalog.sha(self.archives['gallery']), self.proof['origin_gallery']['uncompressed_sha256'])
        all_ids = []
        for record in self.proof['records']:
            faces = [record, *record['alternate_printings']]
            ids = [face['official_id'] for face in faces]
            self.assertEqual(ids, self.identities[record['official_id']]['official_ids'])
            self.assertEqual(ids, record['printing_cohort'])
            for face in faces:
                self.check_face(face)
            all_ids.extend(ids)
        self.assertEqual(all_ids, ['sfd-058-221', 'sfd-058a-221', 'ven-056-166', 'unl-136-219'])
        self.assertNotIn('/tmp/', json.dumps(self.proof))

    def test_runtime_records_match_effective_facts_and_bounded_full_face_art(self):
        runtime = catalog.load_runtime_manifest()['cards']
        for offset, record in enumerate(self.proof['records']):
            row = runtime[BASE + offset]; e = record['effective']
            self.assertEqual(record['runtime_id'], BASE + offset)
            self.assertEqual(row['official_id'], ORDER[offset])
            self.assertEqual(row['kind'].lower(), e['types'][0])
            self.assertEqual([d.lower() for d in row['domains']], e['domains'])
            for field in ['energy', 'power', 'might']:
                self.assertEqual(row[field], e[field] or 0)
            self.assertEqual(row['tags'], e['tags'])
            self.assertEqual(row['official_text'], e['rules_text'])
            self.assertEqual(record['runtime_rules_text'], e['rules_text'])
            self.assertEqual(row['official_text_html'], e['rules_html'])
            self.assertEqual(row['equipment_effect_html'], e['equipment_html'])
            self.assertEqual(row['keywords'], self.identities[ORDER[offset]]['effective_keywords_observed'])
            self.assertEqual(row['art_url'], record['runtime_image']['url'])
            self.assertEqual(row['art_url'].split('?')[0], record['original_image']['url'].split('?')[0])
            self.check_image(record['runtime_image'])
        self.assertIsNone(self.proof['records'][1]['raw_printing']['power'])
        self.assertIsNone(self.proof['records'][2]['raw_printing']['might'])

    def test_ornn_complete_cohort_agrees_without_a_variant_override(self):
        base = self.proof['records'][0]; alt = base['alternate_printings'][0]
        self.assertEqual(base['effective'], alt['effective'])
        self.assertNotEqual(base['original_image']['sha256'], alt['original_image']['sha256'])
        self.assertEqual(base['effective']['super_types'], ['champion'])
        self.assertEqual(base['effective']['tags'], ['Ornn', 'Freljord'])
        for record in self.proof['records']:
            identity = self.identities[record['official_id']]
            self.assertTrue(identity['standard_legal'])
            for field in ['resolution_ids', 'unresolved_gameplay_fields', 'variant_diagnostics', 'unresolved_variant_fields']:
                self.assertEqual(identity['source_review'][field], [])

    def test_all_five_errata_authorities_are_pinned_and_no_candidate_is_changed(self):
        authorities = self.proof['errata_sources']
        self.assertEqual({s['key'] for s in authorities}, {k for k in catalog.SOURCES if k.endswith('_errata')})
        names = {catalog.normalize(self.raw[pid]['name']) for pid in ORDER}
        count = 0
        for source in authorities:
            data = self.archives[source['key']]
            self.assertEqual(source['url'], catalog.SOURCES[source['key']])
            self.assertEqual(source['archive'], self.origin['sources'][source['key']]['archive'])
            self.assertEqual(source['uncompressed_sha256'], catalog.sha(data))
            parsed = catalog.parse_errata(data, source['key']); count += len(parsed)
            self.assertEqual(source['parsed_entries_sha256'], digest(parsed))
            self.assertEqual(source['live_parsed_entries_sha256'], digest(parsed))
            self.assertFalse(any(catalog.normalize(e['article_heading']) in names for e in parsed))
        self.assertEqual(count, 67)
        self.assertTrue(all(r['errata'] == [] for r in self.proof['records']))

    def test_existing_395_cards_presets_legends_recipes_and_review_prefixes_remain_unchanged(self):
        root = catalog.ROOT; runtime = catalog.load_runtime_manifest(); pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(pin['card_count'], BASE)
        self.assertEqual(digest(runtime['cards'][:BASE]), pin['cards_sha256'])
        self.assertEqual(digest(runtime['decks']), pin['presets_sha256'])
        prefix = (root / 'src/cards.rs').read_text().split('        CardDef {\n            id: 395,', 1)[0]
        self.assertEqual(catalog.sha(prefix.encode()), pin['carddefs_source_prefix_sha256'])
        self.assertEqual(catalog.sha((root / 'assets/data/legends.json').read_bytes()), pin['legends_manifest_sha256'])
        self.assertEqual(catalog.sha((root / 'src/deck.rs').read_bytes()), pin['training_recipe_source_sha256'])
        reviewed = json.loads((root / 'assets/data/catalog_resolutions.json').read_text())
        images = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        source = self.proof['preserved_source_reviews']
        for values, count, key in [(reviewed['resolutions'], 122, 'field_resolutions_sha256'),
                (reviewed['raw_printings'], 77, 'raw_printings_sha256'),
                (reviewed['historical_issues'], 47, 'historical_issues_sha256'),
                (images, 77, 'field_image_index_sha256')]:
            self.assertGreaterEqual(len(values), count)
            self.assertEqual(digest(values[:count]), source[key])
        for path, key in [('assets/data/catalog_identity_reviews.json', 'identity_review_manifest_sha256'),
                ('assets/data/catalog_evidence/variant_images.json', 'variant_image_index_sha256')]:
            self.assertEqual(catalog.sha((root / path).read_bytes()), source[key])

    def test_source_review_does_not_admit_cards_or_make_ornn_a_current_chosen_champion(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest()); runtime['cards'] = runtime['cards'][:BASE]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            original = catalog.make_catalog(self.origin, self.archives)
        self.assertEqual(self.proof['pre_admission_counts'], {'main_cards':395, 'mapped_legal':422, 'remaining_legal':657})
        for card in original['cards']:
            if card['identity'] in ORDER:
                self.assertFalse(card['implementation']['playable_in_client'])
        legends = catalog.load_legend_manifest()
        for offset, record in enumerate(self.proof['records']):
            homes = [l['id'] for l in legends if set(record['effective']['domains']) <= {d.lower() for d in l['domains']}]
            self.assertEqual(homes, HOMES[offset])
            self.assertEqual(record['legal_main_deck_legend_ids'], homes)
            self.assertEqual(record['chosen_champion_legend_ids'], [])
        self.assertFalse(any(l['champion'] == 'Ornn' for l in legends))

    def test_tampered_raw_effective_visual_or_original_hash_is_rejected(self):
        original = self.proof['records'][0]
        cases = [('raw_printing', 'power', 0), ('effective', 'tags', []),
                 ('visual_observation', 'printed_supertypes', []),
                 ('visual_observation', 'printed_rules_icons_expanded', 'Draw 4.'),
                 ('original_image', 'sha256', '0' * 64)]
        for area, field, value in cases:
            changed = copy.deepcopy(original); changed[area][field] = value
            with self.subTest(area=area, field=field), self.assertRaises(AssertionError):
                self.check_face(changed)
        changed = copy.deepcopy(original['alternate_printings'][0])
        changed['raw_printing'] = copy.deepcopy(original['raw_printing'])
        with self.assertRaises(AssertionError):
            self.check_face(changed)

    def test_runtime_fact_mismatch_is_rejected_by_production_importer(self):
        runtime = catalog.load_runtime_manifest()
        for field, value in [('kind', 'Gear'), ('energy', 9), ('power', 0), ('tags', []),
                             ('domains', ['Fury']), ('might', 0), ('official_text_html', '<p>Draw 4.</p>')]:
            changed = copy.deepcopy(runtime); changed['cards'][BASE][field] = value
            with self.subTest(field=field), patch.object(catalog, 'load_runtime_manifest', return_value=changed):
                with self.assertRaisesRegex(ValueError, 'Runtime main-card .*mismatch'):
                    catalog.make_catalog(self.origin, self.archives)


if __name__ == '__main__':
    unittest.main()
