"""Complete Deploy source cohorts and append-only card admission; no runtime proof."""
import copy
import json
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog

BASE = 401
ORDER = ['rad-002-167', 'rad-024-167', 'rad-116-167', 'rad-044-167',
         'rad-073-167', 'rad-087-167', 'rad-052-167', 'rad-106-167',
         'rad-072-167', 'rad-033-167']
HOMES = [[3, 4, 8], [0, 1], [2, 4, 5, 6, 7], [0, 1], [1, 7, 8],
         [1, 7, 8], [3, 5, 6, 9], [0, 2, 9], [1, 7, 8], [0, 1]]


def digest(value):
    return catalog.sha(review.canonical_bytes(value))


class C24AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = catalog.ROOT
        cls.origin, cls.archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(cls.archives['gallery'])[0]}
        cls.proof = json.loads((cls.root / 'assets/data/card_admissions/c24/admissions.json').read_text())
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:411]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}
        cls.identities = {p['identity']: p for p in cls.snapshot['cards']}

    def check_image(self, image, original=False):
        path = self.root / image['path']
        self.assertFalse(image['path'].startswith('/'))
        self.assertTrue(path.resolve().is_relative_to(self.root.resolve()))
        data = path.read_bytes()
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
        self.assertEqual(struct.unpack('>II', data[16:24]), (744, 1039) if original else (280, 391))
        if not original:
            self.assertLess(len(data), 300_000)
            self.assertEqual((image['width'], image['height']), (280, 391))

    def check_face(self, face):
        pid = face['official_id']; raw = self.raw[pid]; printing = self.printings[pid]
        self.assertEqual(face['name'], raw['name'])
        self.assertEqual(face['raw_printing'], raw)
        self.assertEqual(face['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(face['effective'], printing['effective'])
        self.assertEqual(face['effective_projection_sha256'], review.printing_sha256({**raw, **printing['effective']}))
        self.assertEqual(face['existing_field_resolution_ids'], printing['resolution_ids'])
        self.assertEqual(face['original_image']['url'], raw['image_url'])
        self.check_image(face['original_image'], original=True)
        o = face['visual_observation']
        self.assertTrue(o['face_visually_inspected'])
        self.assertEqual(o['uncertainties'], [])
        self.assertEqual(o['independently_reinspected_on'], '2026-10-25')
        self.assertEqual((o['official_id'], o['name'], o['collector_code']), (pid, raw['name'], raw['public_code']))
        self.assertEqual(o['original_image_sha256'], face['original_image']['sha256'])
        for observed, field in [('printed_types', 'types'), ('printed_supertypes', 'super_types'),
                ('printed_domains', 'domains'), ('printed_tags', 'tags'), ('printed_energy', 'energy'),
                ('printed_power', 'power'), ('printed_might', 'might'), ('printed_might_bonus', 'might_bonus'),
                ('printed_rules_icons_expanded', 'rules_text'), ('printed_equipment_icons_expanded', 'equipment_text')]:
            expected = printing['effective'][field]
            if field == 'tags':
                self.assertEqual(sorted(o[observed]), sorted(expected))
            elif field in ('rules_text', 'equipment_text'):
                self.assertEqual(' '.join(o[observed].split()), ' '.join(expected.split()))
            else:
                self.assertEqual(o[observed], expected)

    def test_ten_complete_original_single_printing_cohorts(self):
        self.assertFalse(self.proof['draft_not_for_import'])
        self.assertEqual([r['official_id'] for r in self.proof['records']], ORDER)
        self.assertEqual(catalog.sha(self.archives['gallery']), self.proof['origin_gallery']['uncompressed_sha256'])
        for record in self.proof['records']:
            pid = record['official_id']
            self.assertEqual(self.identities[pid]['official_ids'], [pid])
            self.assertEqual(record['printing_cohort'], [pid])
            self.assertEqual(record['alternate_printings'], [])
            self.check_face(record)
        self.assertNotIn('/tmp/', json.dumps(self.proof))

    def test_runtime_stats_domains_types_and_official_art_match_reviewed_facts(self):
        runtime = catalog.load_runtime_manifest()['cards']
        expected = [('fury', 1, 1, 0), ('calm', 2, 0, 0), ('order', 1, 0, 0),
                    ('calm', 4, 2, 0), ('body', 1, 0, 0), ('body', 2, 0, 0),
                    ('mind', 1, 0, 0), ('chaos', 3, 1, 0), ('body', 2, 0, 2), ('calm', 1, 1, 0)]
        for offset, record in enumerate(self.proof['records']):
            row = runtime[BASE + offset]; e = record['effective']
            self.assertEqual((record['runtime_id'], row['id']), (BASE + offset, BASE + offset))
            self.assertEqual(row['official_id'], ORDER[offset])
            self.assertEqual((row['domains'][0].lower(), row['energy'], row['power'], row['might']), expected[offset])
            self.assertEqual(row['kind'].lower(), 'gear' if offset < 8 else 'unit' if offset == 8 else 'spell')
            self.assertEqual(row['tags'], e['tags'])
            self.assertEqual(row['official_text'], e['rules_text'])
            self.assertEqual(row['official_text_html'], e['rules_html'])
            self.assertEqual(row['equipment_effect'], '')
            self.assertEqual(row['keywords'], self.identities[ORDER[offset]]['effective_keywords_observed'])
            self.assertEqual(row['art_url'], record['runtime_image']['url'])
            self.assertEqual(row['art_url'].split('?')[0], record['original_image']['url'].split('?')[0])
            self.check_image(record['runtime_image'])

    def test_raw_domain_errors_and_rule_derived_tag_exclusions_are_not_erased(self):
        records = {r['official_id']: r for r in self.proof['records']}
        for pid, old, new in [('rad-024-167', 'chaos', 'calm'), ('rad-073-167', 'calm', 'body'), ('rad-106-167', 'body', 'chaos')]:
            record = records[pid]
            self.assertEqual(record['raw_printing']['domains'], [old])
            self.assertEqual(record['effective']['domains'], [new])
            self.assertIn(pid + ':domains', record['existing_field_resolution_ids'])
        resolutions = json.loads((self.root / 'assets/data/catalog_resolutions.json').read_text())['resolutions']
        by_id = {r['id']: r for r in resolutions}
        for record in self.proof['records'][:8]:
            self.assertEqual(record['effective']['tags'], [])
            self.assertEqual(by_id[record['official_id'] + ':tags']['basis'], 'rule_derived')
        traveler, command = self.proof['records'][8:]
        self.assertEqual(traveler['visual_observation']['printed_tags'], ['Bandle City', 'Yordle'])
        self.assertNotIn('[Deploy]', traveler['effective']['rules_text'].splitlines()[0][:8])
        self.assertEqual(command['effective']['types'], ['spell'])
        self.assertEqual(command['effective']['super_types'], [])
        self.assertNotIn('Action', command['effective']['rules_text'])
        self.assertNotIn('Reaction', command['effective']['rules_text'])

    def test_all_five_errata_authorities_and_fresh_matching_core_are_pinned(self):
        authorities = self.proof['errata_sources']
        self.assertEqual({s['key'] for s in authorities}, {k for k in catalog.SOURCES if k.endswith('_errata')})
        names = {catalog.normalize(self.raw[pid]['name']) for pid in ORDER}; count = 0
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
        live = self.proof['live_authority_check']
        self.assertTrue(live['core_pdf_fresh_download_matches_bundled'])
        self.assertEqual(live['core_pdf_sha256'], '94d41fee86f88b61c4157c0389c0595d990b98e5b42555c99d9f649ee64d1fec')
        self.assertTrue(all(r['errata'] == [] for r in self.proof['records']))

    def test_401_card_prefix_recipes_and_all_prior_review_prefixes_are_preserved(self):
        runtime = catalog.load_runtime_manifest(); pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(pin['card_count'], BASE)
        self.assertEqual(digest(runtime['cards'][:BASE]), pin['cards_sha256'])
        self.assertEqual(digest(runtime['decks']), pin['presets_sha256'])
        prefix = (self.root / 'src/cards.rs').read_text().split('        CardDef {\n            id: 401,', 1)[0]
        self.assertEqual(catalog.sha(prefix.encode()), pin['carddefs_source_prefix_sha256'])
        for path, key in [('assets/data/legends.json', 'legends_manifest_sha256'), ('src/deck.rs', 'training_recipe_source_sha256')]:
            self.assertEqual(catalog.sha((self.root / path).read_bytes()), pin[key])
        reviewed = json.loads((self.root / 'assets/data/catalog_resolutions.json').read_text())
        images = json.loads((self.root / 'assets/data/catalog_evidence/images.json').read_text())
        source = self.proof['preserved_source_reviews']
        for values, count, key in [(reviewed['resolutions'], 137, 'field_resolutions_sha256'),
                (reviewed['raw_printings'], 87, 'raw_printings_sha256'), (reviewed['historical_issues'], 52, 'historical_issues_sha256'),
                (images, 87, 'field_image_index_sha256')]:
            self.assertGreaterEqual(len(values), count)
            self.assertEqual(digest(values[:count]), source[key])
        for path, key in [('assets/data/catalog_identity_reviews.json', 'identity_review_manifest_sha256'), ('assets/data/catalog_evidence/variant_images.json', 'variant_image_index_sha256')]:
            self.assertEqual(catalog.sha((self.root / path).read_bytes()), source[key])

    def test_source_only_review_never_admits_candidates_or_signature_follow_ons(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest()); runtime['cards'] = runtime['cards'][:BASE]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            previous = catalog.make_catalog(self.origin, self.archives)
        self.assertEqual(self.proof['pre_admission_counts'], {'main_cards':401, 'mapped_legal':428, 'remaining_legal':651})
        keys = ['implemented_main_cards', 'implemented_legal_identities', 'unimplemented_legal_identities']
        self.assertEqual(tuple(previous['counts'][k] for k in keys), (401, 428, 651))
        self.assertEqual(tuple(self.snapshot['counts'][k] for k in keys), (411, 438, 641))
        for card in previous['cards']:
            if card['identity'] in ORDER:
                self.assertFalse(card['implementation']['playable_in_client'])
        for pid in ['rad-108-167', 'rad-156-167', 'rad-017-167', 'rad-136-167', 'rad-062-167', 'rad-146-167']:
            self.assertFalse(self.identities[pid]['implementation']['playable_in_client'])

    def test_actual_supported_legend_homes_and_no_chosen_champion_eligibility(self):
        legends = catalog.load_legend_manifest()
        for offset, record in enumerate(self.proof['records']):
            homes = [l['id'] for l in legends if set(record['effective']['domains']) <= {d.lower() for d in l['domains']}]
            self.assertEqual(homes, HOMES[offset])
            self.assertEqual(record['legal_main_deck_legend_ids'], homes)
            self.assertEqual(record['chosen_champion_legend_ids'], [])
            self.assertEqual(record['effective']['super_types'], [])
            identity = self.identities[record['official_id']]
            self.assertTrue(identity['standard_legal'])
            self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])
            self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])

    def test_tampered_raw_effective_visual_resolution_and_original_proof_are_rejected(self):
        original = self.proof['records'][1]
        for area, field, value in [('raw_printing', 'power', 0), ('effective', 'domains', ['chaos']),
                ('visual_observation', 'printed_tags', ['Device']), ('visual_observation', 'printed_rules_icons_expanded', 'Draw 4.'),
                ('original_image', 'sha256', '0' * 64)]:
            changed = copy.deepcopy(original); changed[area][field] = value
            with self.subTest(area=area, field=field), self.assertRaises(AssertionError):
                self.check_face(changed)
        changed = copy.deepcopy(original); changed['existing_field_resolution_ids'] = []
        with self.assertRaises(AssertionError):
            self.check_face(changed)
