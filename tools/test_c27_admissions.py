"""C27 source cohorts, Tusker's observed tag correction, and admission boundaries."""
import copy
import json
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog
import admission_history as history

BASE = 413
ORDER = ['ogn-076-298', 'rad-085-167', 'sfd-028-221', 'rad-078-167']
HOMES = [[0, 1], [1, 7, 8], [3, 4, 8], [1, 7, 8]]


def digest(value):
    return catalog.sha(review.canonical_bytes(value))


class C27AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(cls.archives['gallery'])[0]}
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c27/admissions.json').read_text())
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:417]; runtime['decks'] = runtime['decks'][:4]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime), patch.object(catalog, 'load_battlefield_manifest', return_value=history.legacy_battlefields()):
            cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}
        cls.identities = {p['identity']: p for p in cls.snapshot['cards']}

    def check_image(self, image, original=False):
        path = catalog.ROOT / image['path']
        self.assertFalse(image['path'].startswith('/'))
        self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
        data = path.read_bytes()
        self.assertEqual((len(data), catalog.sha(data)), (image['bytes'], image['sha256']))
        self.assertTrue(data.startswith(b'\x89PNG\r\n\x1a\n'))
        dimensions = struct.unpack('>II', data[16:24])
        self.assertEqual(dimensions, (744, 1039) if original else (280, 391))
        if not original:
            self.assertLess(len(data), 300_000)

    def check_face(self, face):
        pid = face['official_id']; raw = self.raw[pid]; effective = self.printings[pid]['effective']
        self.assertEqual(face['raw_printing'], raw)
        self.assertEqual(face['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(face['effective'], effective)
        self.assertEqual(face['effective_projection_sha256'], review.printing_sha256({**raw, **effective}))
        observed = face['visual_observation']
        self.assertTrue(observed['face_visually_inspected'])
        self.assertEqual(observed['uncertainties'], [])
        self.assertEqual(observed['independently_reinspected_on'], '2026-10-26')
        self.assertEqual((observed['official_id'], observed['name'], observed['collector_code']),
                         (pid, raw['name'], raw['public_code']))
        self.assertEqual(observed['original_image_sha256'], face['original_image']['sha256'])
        self.assertEqual(face['original_image']['url'], raw['image_url'])
        self.check_image(face['original_image'], original=True)
        for field, printed in [('types', 'printed_types'), ('super_types', 'printed_supertypes'),
                ('domains', 'printed_domains'), ('energy', 'printed_energy'), ('power', 'printed_power'),
                ('might', 'printed_might'), ('might_bonus', 'printed_might_bonus'),
                ('rules_text', 'printed_rules_icons_expanded'), ('equipment_text', 'printed_equipment_icons_expanded')]:
            self.assertEqual(observed[printed], effective[field])
            self.assertEqual(raw[field], effective[field])
        self.assertEqual(sorted(observed['printed_tags']), sorted(effective['tags']))
        self.assertEqual(face['existing_field_resolution_ids'], [pid + ':tags'] if pid == 'rad-078-167' else [])
        self.assertEqual(raw['tags'], [] if pid == 'rad-078-167' else effective['tags'])

    def test_all_four_complete_cohorts_preserve_six_original_faces(self):
        self.assertEqual([p['official_id'] for p in self.proof['records']], ORDER)
        seen = []
        for record in self.proof['records']:
            faces = [record, *record['alternate_printings']]
            ids = [face['official_id'] for face in faces]
            self.assertEqual(ids, record['printing_cohort'])
            self.assertEqual(ids, self.identities[record['official_id']]['official_ids'])
            for face in faces:
                self.check_face(face)
                self.assertEqual(face['effective'], record['effective'])
            seen.extend(ids)
        self.assertEqual(seen, ['ogn-076-298', 'ogn-076a-298', 'rad-085-167',
                                'sfd-028-221', 'sfd-028a-221', 'rad-078-167'])
        self.assertNotIn('/tmp/', json.dumps(self.proof))
        self.assertNotIn('/home/', json.dumps(self.proof))

    def test_runtime_stats_costs_tags_rules_and_bounded_art_match_reviewed_facts(self):
        runtime = catalog.load_runtime_manifest()['cards']
        for offset, record in enumerate(self.proof['records']):
            row = runtime[BASE + offset]; effective = record['effective']
            self.assertEqual((record['runtime_id'], row['id'], row['official_id']),
                             (BASE + offset, BASE + offset, ORDER[offset]))
            self.assertEqual(row['kind'].lower(), effective['types'][0])
            self.assertEqual([d.lower() for d in row['domains']], effective['domains'])
            for key in ['energy', 'power', 'might']:
                self.assertEqual(row[key], effective[key] or 0)
            self.assertEqual(row['tags'], effective['tags'])
            self.assertEqual(row['official_text'], effective['rules_text'])
            self.assertEqual(row['official_text_html'], effective['rules_html'])
            self.assertEqual(row['equipment_effect_html'], effective['equipment_html'])
            self.assertEqual(row['keywords'], self.identities[ORDER[offset]]['effective_keywords_observed'])
            self.assertEqual(row['art_url'], record['runtime_image']['url'])
            self.assertEqual(row['art_url'].split('?')[0], record['original_image']['url'].split('?')[0])
            self.check_image(record['runtime_image'])
        self.assertIsNone(self.proof['records'][2]['effective']['power'])
        self.assertEqual(runtime[415]['power'], 0)
        self.assertEqual([r['effective']['might'] for r in self.proof['records']], [6, 4, 2, 4])

    def test_tusker_correction_keeps_raw_absence_and_all_old_review_prefixes(self):
        root = catalog.ROOT
        correction = json.loads((root / 'assets/data/catalog_evidence/reviews/c27-tusker.json').read_text())
        manifest = json.loads((root / 'assets/data/catalog_resolutions.json').read_text())
        images = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        self.assertFalse(correction['enables_runtime_support'])
        for key, pin in correction['preserved_source_cohorts'].items():
            self.assertEqual(digest(manifest[key][:pin['count']]), pin['canonical_sha256'])
        pin = correction['preserved_field_image_index']
        self.assertEqual(digest(images[:pin['count']]), pin['canonical_sha256'])
        record = correction['records'][0]
        self.assertEqual(record['raw_printing'], self.raw['rad-078-167'])
        self.assertEqual(record['raw_printing']['tags'], [])
        self.assertEqual(record['effective_before']['tags'], [])
        self.assertEqual(record['effective_after'], {**record['effective_before'], 'tags': ['Noxus']})
        self.assertEqual(record['visual_observation']['printed_type_tag_line'], 'UNIT / NOXUS')
        self.assertEqual(record['resolution_ids'], ['rad-078-167:tags'])
        self.check_image(record['original_image'], original=True)
        resolution = next(r for r in manifest['resolutions'] if r['id'] == 'rad-078-167:tags')
        self.assertEqual((resolution['basis'], resolution['operation']), ('confirmed_artwork', 'override'))
        self.assertFalse(resolution['enables_runtime_support'])

    def test_existing_runtime_definitions_presets_legends_and_recipes_are_unchanged(self):
        root = catalog.ROOT; runtime = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(pin['card_count'], BASE)
        self.assertEqual(digest(runtime['cards'][:BASE]), pin['cards_sha256'])
        self.assertEqual(digest(runtime['decks'][:4]), pin['presets_sha256'])
        prefix = history.pre_registration_carddefs_source().split('        CardDef {\n            id: 413,', 1)[0]
        self.assertEqual(catalog.sha(prefix.encode()), pin['carddefs_source_prefix_sha256'])
        for path, key in [('assets/data/legends.json', 'legends_manifest_sha256'),
                          ('src/deck.rs', 'training_recipe_source_sha256')]:
            self.assertEqual(catalog.sha(history.pre_rengar_recipe_source_bytes() if path == 'src/deck.rs' else (root / path).read_bytes()), pin[key])
        for path, key in [('assets/data/catalog_identity_reviews.json', 'identity_review_manifest_sha256'),
                          ('assets/data/catalog_evidence/variant_images.json', 'variant_image_index_sha256')]:
            self.assertEqual(catalog.sha(history.legacy_variant_evidence_bytes(path)), self.proof['preserved_source_reviews'][key])

    def test_source_correction_does_not_admit_cards_and_main_deck_homes_are_exact(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest()); runtime['cards'] = runtime['cards'][:BASE]; runtime['decks'] = runtime['decks'][:4]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime), patch.object(catalog, 'load_battlefield_manifest', return_value=history.legacy_battlefields()):
            source_only = catalog.make_catalog(self.origin, self.archives)
        keys = ['implemented_main_cards', 'implemented_legal_identities', 'unimplemented_legal_identities']
        self.assertEqual(tuple(source_only['counts'][k] for k in keys), (413, 440, 639))
        self.assertEqual(tuple(self.snapshot['counts'][k] for k in keys), (417, 444, 635))
        for identity in source_only['cards']:
            if identity['identity'] in ORDER:
                self.assertFalse(identity['implementation']['playable_in_client'])
        legends = catalog.load_legend_manifest()
        for offset, record in enumerate(self.proof['records']):
            homes = [l['id'] for l in legends if set(record['effective']['domains']) <= {d.lower() for d in l['domains']}]
            self.assertEqual(homes, HOMES[offset])
            self.assertEqual(record['legal_main_deck_legend_ids'], homes)
            self.assertEqual(record['chosen_champion_legend_ids'], [])

    def test_all_errata_authorities_and_current_legality_are_pinned(self):
        names = {catalog.normalize(self.raw[pid]['name']) for pid in ORDER}
        self.assertEqual({s['key'] for s in self.proof['errata_sources']},
                         {k for k in catalog.SOURCES if k.endswith('_errata')})
        count = 0
        for source in self.proof['errata_sources']:
            data = self.archives[source['key']]; entries = catalog.parse_errata(data, source['key'])
            count += len(entries)
            self.assertEqual(source['uncompressed_sha256'], catalog.sha(data))
            self.assertEqual(source['parsed_entries_sha256'], digest(entries))
            self.assertEqual(source['live_parsed_entries_sha256'], digest(entries))
            self.assertFalse(any(catalog.normalize(e['article_heading']) in names for e in entries))
        self.assertEqual(count, 67)
        self.assertTrue(all(r['errata'] == [] for r in self.proof['records']))
        self.assertEqual(self.proof['live_authority_check']['checked_on'], '2026-10-26')
        self.assertTrue(all(self.identities[pid]['standard_legal'] for pid in ORDER))

    def test_corrupt_face_facts_and_image_hash_are_rejected(self):
        for offset, area, field, value in [(0, 'raw_printing', 'power', 0),
                (3, 'effective', 'tags', []), (3, 'visual_observation', 'printed_tags', []),
                (2, 'visual_observation', 'printed_power', 0),
                (1, 'original_image', 'sha256', '0' * 64)]:
            changed = copy.deepcopy(self.proof['records'][offset]); changed[area][field] = value
            with self.subTest(offset=offset, area=area, field=field), self.assertRaises(AssertionError):
                self.check_face(changed)

    def test_production_importer_rejects_stale_tusker_tag_or_wrong_attribute(self):
        runtime = catalog.load_runtime_manifest()
        for card, field, value in [(416, 'tags', []), (416, 'might', 7), (415, 'power', 1),
                                  (414, 'domains', ['Calm']), (413, 'kind', 'Gear')]:
            changed = copy.deepcopy(runtime); changed['cards'][card][field] = value
            with self.subTest(card=card, field=field), patch.object(catalog, 'load_runtime_manifest', return_value=changed):
                with self.assertRaisesRegex(ValueError, 'Runtime main-card .*mismatch'):
                    catalog.make_catalog(self.origin, self.archives)


if __name__ == '__main__':
    unittest.main()
