"""C19 original faces, independent errata authority, and append-only source admission."""
import copy
import json
import re
import struct
import unittest
from unittest.mock import patch

import catalog_review as review
import import_catalog as catalog

BASE = 385
ORDER = ['ogn-242-298', 'ogn-160-298', 'ogn-062-298', 'ven-089-166']
PROOF = 'assets/data/card_admissions/c19/admissions.json'


def digest(value):
    return catalog.sha(review.canonical_bytes(value))


def old_erratum_text(raw, heading):
    parts = re.split(r'<h[12]\b[^>]*>(.*?)</h[12]>', catalog.body(raw), flags=re.S)
    for title, body in zip(parts[1::2], parts[2::2]):
        if catalog.plain(title) == heading:
            return catalog.plain(body).split('[OLD TEXT]', 1)[1].strip()
    raise AssertionError('Missing old erratum section')


def readable_icons(text, domain):
    return text.replace('[1]', '[1 Energy]').replace('[5]', '[5 Energy]') \
        .replace('[C]', f'[{domain.title()} Power]').replace('[E]', '[Exhaust]')


class C19AdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.data = catalog.read_archives()
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'] = runtime['cards'][:389]
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            cls.snapshot = catalog.make_catalog(cls.manifest, cls.data)
        cls.proof = json.loads((catalog.ROOT / PROOF).read_text())
        cls.raw = {p['official_id']: p for p in catalog.project_gallery(cls.data['gallery'])[0]}
        cls.printings = {p['official_id']: p for p in cls.snapshot['printings']}
        cls.identities = {p['identity']: p for p in cls.snapshot['cards']}

    def check_record(self, record):
        pid = record['official_id']
        index = BASE + ORDER.index(pid)
        raw = self.raw[pid]
        effective = self.printings[pid]['effective']
        identity = self.identities[pid]
        runtime = catalog.load_runtime_manifest()['cards'][index]
        self.assertEqual(record['runtime_id'], index)
        self.assertEqual(record['raw_printing'], raw)
        self.assertEqual(record['raw_printing_sha256'], review.printing_sha256(raw))
        self.assertEqual(record['effective'], effective)
        self.assertEqual(record['existing_field_resolution_ids'], [])
        self.assertEqual(self.printings[pid]['resolution_ids'], [])
        self.assertEqual(identity['official_ids'], [pid])
        self.assertTrue(identity['standard_legal'])
        for field in ['unresolved_variant_fields', 'unresolved_gameplay_fields']:
            self.assertEqual(identity['source_review'][field], [])
        self.assertEqual(record['errata'], identity['errata'])
        expected_text = identity['errata'][0]['text'] if identity['errata'] else effective['rules_text']
        self.assertEqual(record['runtime_rules_text'], expected_text)
        self.assertEqual(runtime['official_text'], expected_text)
        self.assertEqual(record['runtime_text_authority'], 'official_errata' if identity['errata'] else 'official_gallery_and_original_face')
        self.assertEqual(runtime['official_text_html'], effective['rules_html'])
        self.assertEqual(runtime['equipment_effect_html'], effective['equipment_html'])
        self.assertEqual(runtime['official_id'], pid)
        for key in ['energy', 'power']:
            self.assertEqual(runtime[key], effective[key] or 0)
        self.assertEqual(runtime['tags'], effective['tags'])
        for key in ['original_image', 'runtime_image']:
            image = record[key]
            path = catalog.ROOT / image['path']
            self.assertFalse(image['path'].startswith('/'))
            self.assertTrue(path.resolve().is_relative_to(catalog.ROOT.resolve()))
            payload = path.read_bytes()
            self.assertEqual((len(payload), catalog.sha(payload)), (image['bytes'], image['sha256']))
            self.assertTrue(payload.startswith(b'\x89PNG\r\n\x1a\n'))
            width, height = struct.unpack('>II', payload[16:24])
            self.assertAlmostEqual(width / height, 744 / 1039, delta=.002)
            if key == 'runtime_image':
                self.assertLess(len(payload), 300_000)
                self.assertEqual((width, height), (image['width'], image['height']))
        self.assertEqual(record['original_image']['url'], raw['image_url'])
        self.assertEqual(record['runtime_image']['url'], runtime['art_url'])
        self.assertEqual(record['runtime_image']['url'].split('?')[0], raw['image_url'].split('?')[0])
        observation = record['visual_observation']
        self.assertTrue(observation['face_visually_inspected'])
        self.assertEqual(observation['uncertainties'], [])
        self.assertEqual(observation['independently_reinspected_on'], '2026-10-24')
        self.assertEqual((observation['official_id'], observation['name'], observation['collector_code']),
                         (pid, raw['name'], raw['public_code']))
        self.assertEqual(observation['original_image_sha256'], record['original_image']['sha256'])
        for physical, field in [('printed_types', 'types'), ('printed_supertypes', 'super_types'),
                                ('printed_domains', 'domains'), ('printed_tags', 'tags'),
                                ('printed_energy', 'energy'), ('printed_power', 'power'),
                                ('printed_might', 'might'), ('printed_might_bonus', 'might_bonus'),
                                ('printed_equipment_icons_expanded', 'equipment_text')]:
            self.assertEqual(observation[physical], effective[field])
        if identity['errata']:
            erratum = identity['errata'][0]
            key = next(k for k, url in catalog.SOURCES.items() if url == erratum['source_url'])
            physical_text = readable_icons(old_erratum_text(self.data[key], erratum['article_heading']), effective['domains'][0])
        else:
            physical_text = effective['rules_text']
        self.assertEqual(observation['printed_rules_icons_expanded'], physical_text)

    def test_complete_raw_effective_errata_and_original_face_facts(self):
        self.assertFalse(self.proof['draft_not_for_import'])
        self.assertEqual([r['official_id'] for r in self.proof['records']], ORDER)
        self.assertEqual(catalog.sha(self.data['gallery']), self.proof['origin_gallery']['uncompressed_sha256'])
        for record in self.proof['records']:
            with self.subTest(card=record['name']):
                self.check_record(record)

    def test_errata_authority_pins_all_five_current_official_articles(self):
        expected = {key for key in catalog.SOURCES if key.endswith('_errata')}
        self.assertEqual({s['key'] for s in self.proof['errata_sources']}, expected)
        for source in self.proof['errata_sources']:
            key = source['key']
            self.assertEqual(source['url'], catalog.SOURCES[key])
            self.assertEqual(source['archive'], self.manifest['sources'][key]['archive'])
            self.assertEqual(source['uncompressed_sha256'], catalog.sha(self.data[key]))
            parsed_hash = digest(catalog.parse_errata(self.data[key], key))
            self.assertEqual(source['parsed_entries_sha256'], parsed_hash)
            self.assertEqual(source['live_parsed_entries_sha256'], parsed_hash)
            self.assertEqual(source['live_checked_on'], '2026-10-24')
        self.assertEqual([len(r['errata']) for r in self.proof['records']], [1, 1, 1, 0])

    def test_aurora_stale_gallery_is_preserved_but_runtime_uses_authoritative_banish(self):
        record = self.proof['records'][1]
        self.assertNotIn('banish', record['raw_printing']['rules_text'])
        self.assertNotIn('banish', record['effective']['rules_text'])
        self.assertNotIn('banish', record['visual_observation']['printed_rules_icons_expanded'])
        self.assertIn('and banish it', record['runtime_rules_text'])
        self.assertEqual(record['existing_field_resolution_ids'], [])

    def test_old_runtime_recipes_and_source_review_cohorts_are_unchanged(self):
        root = catalog.ROOT
        runtime = catalog.load_runtime_manifest()
        pin = self.proof['preserved_runtime_prefix']
        self.assertEqual(pin['card_count'], BASE)
        self.assertEqual(digest(runtime['cards'][:BASE]), pin['cards_sha256'])
        self.assertEqual(digest(runtime['decks']), pin['presets_sha256'])
        rust_prefix = (root / 'src/cards.rs').read_text().split('        CardDef {\n            id: 385,', 1)[0]
        self.assertEqual(catalog.sha(rust_prefix.encode()), pin['carddefs_source_prefix_sha256'])
        self.assertEqual(catalog.sha((root / 'assets/data/legends.json').read_bytes()), pin['legends_manifest_sha256'])
        self.assertEqual(catalog.sha((root / 'src/deck.rs').read_bytes()), pin['training_recipe_source_sha256'])
        reviewed = json.loads((root / 'assets/data/catalog_resolutions.json').read_text())
        identities = json.loads((root / 'assets/data/catalog_identity_reviews.json').read_text())
        images = json.loads((root / 'assets/data/catalog_evidence/images.json').read_text())
        for values, count, key in [
            (reviewed['resolutions'], 120, 'field_resolutions_sha256'),
            (reviewed['raw_printings'], 75, 'raw_printings_sha256'),
            (reviewed['historical_issues'], 45, 'historical_issues_sha256'),
            (images, 75, 'field_image_index_sha256'),
            (identities['reviews'], 29, 'identity_reviews_sha256'),
        ]:
            self.assertGreaterEqual(len(values), count)
            self.assertEqual(digest(values[:count]), self.proof['preserved_source_reviews'][key])
        self.assertEqual(self.proof['pre_admission_counts'], {'main_cards': 385, 'mapped_legal': 412, 'remaining_legal': 667})

    def test_face_raw_effective_image_and_errata_tampering_is_detected(self):
        cases = [
            ('raw_printing', 'energy', 99), ('effective', 'domains', ['fury']),
            ('visual_observation', 'printed_power', 1),
            ('visual_observation', 'printed_rules_icons_expanded', 'Invented wording.'),
            ('original_image', 'sha256', '0' * 64), ('runtime_image', 'bytes', 1),
        ]
        for section, field, value in cases:
            changed = copy.deepcopy(self.proof['records'][0])
            changed[section][field] = value
            with self.subTest(section=section, field=field), self.assertRaises(AssertionError):
                self.check_record(changed)
        changed = copy.deepcopy(self.proof['records'][1])
        changed['errata'][0]['text'] = changed['effective']['rules_text']
        with self.assertRaises(AssertionError):
            self.check_record(changed)

    def test_runtime_source_drift_is_rejected(self):
        runtime = catalog.load_runtime_manifest()
        for key, value in [('kind', 'Unit'), ('energy', 9), ('power', 1), ('tags', ['Mech']),
                           ('domains', ['Fury']), ('might', 1),
                           ('equipment_effect_html', '<p>Invented.</p>'),
                           ('official_text_html', '<p>Invented.</p>')]:
            changed = copy.deepcopy(runtime)
            changed['cards'][BASE][key] = value
            with self.subTest(field=key), patch.object(catalog, 'load_runtime_manifest', return_value=changed):
                with self.assertRaisesRegex(ValueError, 'Runtime main-card .*mismatch'):
                    catalog.make_catalog(self.manifest, self.data)

    def test_existing_legend_homes_without_new_champions(self):
        legends = catalog.load_legend_manifest()
        self.assertEqual(len(legends), 10)
        for record in self.proof['records']:
            facts = record['effective']
            self.assertEqual(facts['super_types'], [])
            self.assertTrue(any(set(facts['domains']) <= {d.lower() for d in legend['domains']} for legend in legends))


if __name__ == '__main__':
    unittest.main()
