"""C28 complete faces, distinct errata, and source-only admission boundaries."""
import copy
import json
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog
import admission_history as history

UNITS = ['unl-112-219', 'unl-097-219', 'unl-114-219', 'unl-028-219']
FIELDS = ['sfd-207-221', 'unl-215-219', 'unl-205-219', 'ogn-289-298',
          'ven-165-166', 'ogn-298-298']


def digest(value):
    return catalog.sha(review.canonical_bytes(value))


class C28AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.origin, cls.archives = catalog.read_archives()
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(cls.archives['gallery'])[0]}
        cls.proof = json.loads((catalog.ROOT / 'assets/data/card_admissions/c28/admissions.json').read_text())
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:421]
        runtime['decks'] = runtime['decks'][:5]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            cls.snapshot = catalog.make_catalog(cls.origin, cls.archives)
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}
        cls.identities = {p['identity']: p for p in cls.snapshot['cards']}
        cls.review_manifest = json.loads((catalog.ROOT / 'assets/data/catalog_identity_reviews.json').read_text())

    def check_image(self, image, field=False, original=False):
        path = catalog.ROOT / image['path']
        self.assertFalse(image['path'].startswith('/'))
        self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
        data = path.read_bytes()
        self.assertEqual((catalog.sha(data), len(data)), (image['sha256'], image['bytes']))
        self.assertTrue(data.startswith(b'\x89PNG\r\n\x1a\n'))
        width, height = struct.unpack('>II', data[16:24])
        if original and field:
            self.assertIn((width, height), [(1038, 744), (1039, 744)])
        else:
            self.assertEqual((width, height), (744, 1039) if original else
                             (360, 258) if field else (280, 391))
        if not original:
            self.assertLess(len(data), 300_000)
            self.assertEqual((image['width'], image['height']), (width, height))

    def check_face(self, face, field=False):
        pid = face['official_id']; raw = self.raw[pid]
        self.assertEqual(face['raw_printing'], raw)
        self.assertEqual(face['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(face['effective'], self.printings[pid]['effective'])
        self.assertEqual(face['effective'], {k: raw[k] for k in review.GAMEPLAY_FIELDS})
        self.assertEqual(face['effective_projection_sha256'], review.printing_sha256(raw))
        self.assertEqual(face['existing_field_resolution_ids'], [])
        self.assertEqual(face['original_image']['url'], raw['image_url'])
        observation = face['visual_observation']
        self.assertEqual(observation['official_id'], pid)
        self.assertTrue(observation['actual_full_face_viewed'])
        self.assertEqual(observation['sha256'], face['original_image']['sha256'])
        self.check_image(face['original_image'], field=field, original=True)

    def test_all_ten_identities_preserve_all_eleven_complete_faces(self):
        self.assertEqual([r['official_id'] for r in self.proof['records']], UNITS)
        self.assertEqual([r['official_id'] for r in self.proof['battlefield_records']], FIELDS)
        seen = []
        for field, records in [(False, self.proof['records']), (True, self.proof['battlefield_records'])]:
            for record in records:
                faces = [record, *record['alternate_printings']]
                ids = [face['official_id'] for face in faces]
                self.assertEqual(ids, record['printing_cohort'])
                self.assertEqual(ids, self.identities[record['official_id']]['official_ids'])
                for face in faces:
                    self.check_face(face, field=field)
                    seen.append(face['official_id'])
        self.assertEqual(seen, [*UNITS, 'unl-028a-219', *FIELDS])
        provenance = self.proof['visual_review_provenance']
        data = (catalog.ROOT / provenance['path']).read_bytes()
        self.assertEqual(catalog.sha(data), provenance['sha256'])
        actual = json.loads(data)
        self.assertEqual(provenance['reviewed_by'], actual['reviewed_by'])
        self.assertEqual(actual['faces'], [face['visual_observation'] for record in
            self.proof['records'] + self.proof['battlefield_records']
            for face in [record, *record['alternate_printings']]])
        self.assertNotIn('/tmp/', json.dumps(self.proof))
        self.assertNotIn('/home/', json.dumps(self.proof))

    def test_runtime_unit_facts_and_main_deck_homes_match_sources(self):
        cards = catalog.load_runtime_manifest()['cards']
        for offset, record in enumerate(self.proof['records']):
            row = cards[417 + offset]; effective = record['effective']
            self.assertEqual((record['runtime_id'], row['id'], row['official_id']),
                             (417 + offset, 417 + offset, UNITS[offset]))
            self.assertEqual(row['kind'], 'Unit')
            self.assertEqual([d.lower() for d in row['domains']], effective['domains'])
            for key in ['energy', 'power', 'might']:
                self.assertEqual(row[key], effective[key] or 0)
            for key, source in [('tags', 'tags'), ('official_text', 'rules_text'),
                                ('official_text_html', 'rules_html'),
                                ('equipment_effect_html', 'equipment_html')]:
                self.assertEqual(row[key], effective[source])
            self.assertEqual(row['keywords'], self.identities[UNITS[offset]]['effective_keywords_observed'])
            self.assertEqual(row['art_url'], record['runtime_image']['url'])
            self.check_image(record['runtime_image'])
            homes = [l['id'] for l in catalog.load_legend_manifest()
                     if set(effective['domains']) <= {d.lower() for d in l['domains']}]
            self.assertEqual(homes, [1, 7, 8] if offset < 3 else [3, 4, 8])
            self.assertEqual(record['legal_main_deck_legend_ids'], homes)
            self.assertEqual(record['chosen_champion_legend_ids'], [])
        self.assertEqual([r['effective']['power'] for r in self.proof['records']], [None, None, 1, None])

    def test_dais_and_peak_keep_original_printings_separate_from_operative_errata(self):
        fields = catalog.load_battlefield_manifest()
        for offset, record in enumerate(self.proof['battlefield_records']):
            row = fields[11 + offset]
            self.assertEqual((record['runtime_id'], row['id'], row['source_id']),
                             (11 + offset, 11 + offset, FIELDS[offset]))
            self.assertEqual(row['official_text'], record['effective']['rules_text'])
            self.assertEqual(row['official_html'], record['effective']['rules_html'])
            self.assertEqual(row['image_url'], record['original_image']['url'])
            self.assertEqual(record['errata'], self.identities[FIELDS[offset]]['errata'])
            if offset in [0, 3]:
                self.assertEqual(len(record['errata']), 1)
                entry = record['errata'][0]
                expected = entry['text'].replace('[1]', '[1 Energy]').replace('[M]', 'Might')
                self.assertEqual((row['corrected_text'], row['errata_url']), (expected, entry['source_url']))
                self.assertNotEqual(row['corrected_text'], row['official_text'])
                self.assertEqual(record['runtime_rules_text'], expected)
            else:
                self.assertEqual(record['errata'], [])
                self.assertNotIn('corrected_text', row)
                self.assertEqual(record['runtime_rules_text'], row['official_text'])
            self.check_image(record['runtime_image'], field=True)
        self.assertIn("hand to play a 2 Might", fields[11]['corrected_text'])
        self.assertIn('ready up to 2 runes at the end', fields[14]['corrected_text'])

    def test_pyke_reminder_equivalence_is_explicit_without_rewriting_raw_strings(self):
        entry = next(row for row in self.review_manifest['reviews'] if row['id'] == 'c28-unl-028-219')
        self.assertEqual((entry['id'], entry['identity']), ('c28-unl-028-219', UNITS[-1]))
        self.assertFalse(entry['enables_runtime_support'])
        self.assertEqual([m['official_id'] for m in entry['members']], ['unl-028-219', 'unl-028a-219'])
        self.assertEqual([(f['field'], f['disposition']) for f in entry['fields']],
                         [('rules_text', 'reminder_equivalent')])
        self.assertEqual(entry['fields'][0]['rules'], ['135.2.d.1', '135.2.d.2', '135.2.d.3', '810', '811'])
        first, second = self.raw['unl-028-219'], self.raw['unl-028a-219']
        self.assertNotEqual(first['rules_text'], second['rules_text'])
        self.assertIn('(Hide now', first['rules_text'])
        self.assertNotIn('(Hide now', second['rules_text'])
        for text in [first['rules_text'], second['rules_text']]:
            self.assertIn('You may pay [Fury Power] as an additional cost to play me.', text)
            self.assertIn('if you paid the additional cost, ready me and give me +2 Might this turn.', text)
        changed = copy.deepcopy(self.review_manifest); changed['reviews'].remove(entry)
        with self.assertRaisesRegex(ValueError, 'variant|review'):
            catalog.make_catalog(self.origin, self.archives, identity_review_manifest=changed)

    def test_source_only_review_does_not_admit_any_of_the_ten_candidates(self):
        runtime = copy.deepcopy(catalog.load_runtime_manifest()); runtime['cards'] = runtime['cards'][:417]; runtime['decks'] = runtime['decks'][:4]
        fields = catalog.load_battlefield_manifest()[:11]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime), \
                patch.object(catalog, 'load_battlefield_manifest', return_value=fields):
            source_only = catalog.make_catalog(self.origin, self.archives)
        keys = ['implemented_main_cards', 'implemented_battlefields',
                'implemented_legal_identities', 'unimplemented_legal_identities']
        self.assertEqual(tuple(source_only['counts'][k] for k in keys), (417, 11, 444, 635))
        self.assertEqual(tuple(self.snapshot['counts'][k] for k in keys), (421, 17, 454, 625))
        self.assertEqual(source_only['printings'], self.snapshot['printings'])
        for card in source_only['cards']:
            if card['identity'] in UNITS + FIELDS:
                self.assertFalse(card['implementation']['playable_in_client'])
        self.assertEqual(tuple(source_only['counts'][k] for k in [
            'reviewed_variant_identities', 'reviewed_variant_printings', 'unreviewed_variant_identities']),
            (31, 94, 93))

    def test_all_previous_runtime_records_and_review_prefixes_are_unchanged(self):
        root = catalog.ROOT; runtime = catalog.load_runtime_manifest(); pin = self.proof['preserved_runtime_prefix']
        self.assertEqual((pin['card_count'], pin['battlefield_count'], pin['preset_count']), (417, 11, 4))
        self.assertEqual(digest(runtime['cards'][:417]), pin['cards_sha256'])
        self.assertEqual(digest(catalog.load_battlefield_manifest()[:11]), pin['battlefields_sha256'])
        self.assertEqual(digest(runtime['decks'][:4]), pin['presets_sha256'])
        self.assertEqual(len(runtime['decks'][:pin['preset_count']]), 4)
        prefix = (root / 'src/cards.rs').read_text().split('        CardDef {\n            id: 417,', 1)[0]
        self.assertEqual(catalog.sha(prefix.encode()), pin['carddefs_source_prefix_sha256'])
        for path, key in [('assets/data/legends.json', 'legends_manifest_sha256'),
                          ('src/deck.rs', 'training_recipe_source_sha256')]:
            self.assertEqual(catalog.sha(history.pre_rengar_recipe_source_bytes() if path == 'src/deck.rs' else (root / path).read_bytes()), pin[key])
        source = self.proof['preserved_source_reviews']
        # Later field reviews append records. Preserve this checkpoint's exact
        # bytes and require every old record to remain an unchanged prefix.
        import gzip
        previous = gzip.decompress((root / 'assets/data/catalog_evidence/reviews/pre-reckless-resolutions.json.gz').read_bytes())
        self.assertEqual(catalog.sha(previous), source['field_manifest_sha256'])
        prior = json.loads(previous)
        current = json.loads((root / 'assets/data/catalog_resolutions.json').read_text())
        for field in ['resolutions', 'raw_printings', 'historical_issues']:
            self.assertEqual(current[field][:len(prior[field])], prior[field])
        field_images = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        prior_images = (json.dumps(field_images[:88], ensure_ascii=False, indent=2) + '\n').encode()
        self.assertEqual(catalog.sha(prior_images), source['field_image_index_sha256'])
        images = json.loads((root / 'assets/data/catalog_evidence/variant_images.json').read_text())
        self.assertEqual(digest(images[:90]), source['variant_image_prefix_sha256'])
        self.assertEqual(digest(self.review_manifest['reviews'][:29]), source['identity_review_prefix_sha256'])
        index = self.review_manifest['image_index']; data = (root / index['local_path']).read_bytes()
        self.assertEqual((catalog.sha(data), len(data)), (index['sha256'], index['bytes']))

    def test_current_legality_and_all_five_errata_authorities_are_pinned(self):
        self.assertEqual({s['key'] for s in self.proof['errata_sources']},
                         {k for k in catalog.SOURCES if k.endswith('_errata')})
        count = 0
        for source in self.proof['errata_sources']:
            data = self.archives[source['key']]; parsed = catalog.parse_errata(data, source['key'])
            count += len(parsed)
            self.assertEqual(catalog.sha(data), source['uncompressed_sha256'])
            self.assertEqual(digest(parsed), source['parsed_entries_sha256'])
            self.assertEqual(digest(parsed), source['live_parsed_entries_sha256'])
        self.assertEqual(count, 67)
        self.assertTrue(all(self.identities[pid]['standard_legal'] for pid in UNITS + FIELDS))
        self.assertTrue(self.proof['live_authority_check']['normalized_catalog_matches_after_source_only_review'])

    def test_corrupt_fact_image_and_index_pin_are_rejected(self):
        for index, area, field, value in [(0, 'raw_printing', 'power', 0),
                (2, 'effective', 'might', 9), (3, 'original_image', 'sha256', '0' * 64)]:
            changed = copy.deepcopy(self.proof['records'][index]); changed[area][field] = value
            with self.subTest(area=area, field=field), self.assertRaises(AssertionError):
                self.check_face(changed)
        changed = copy.deepcopy(self.review_manifest); changed['image_index']['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'hash|SHA'):
            catalog.make_catalog(self.origin, self.archives, identity_review_manifest=changed)


if __name__ == '__main__':
    unittest.main()
