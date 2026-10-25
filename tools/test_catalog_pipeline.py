"""End-to-end source admission, provenance preservation, and refresh failures."""
from contextlib import ExitStack, redirect_stdout
import copy
import gzip
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import import_catalog as catalog
from test_import_catalog import edit_page


class CatalogPipelineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.data = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.manifest, cls.data)

    def changed_gallery(self, change):
        def update(page):
            records = next(b['cards']['items'] for b in page['blades']
                           if isinstance(b.get('cards'), dict) and 'items' in b['cards'])
            change(records)
        return {**self.data, 'gallery': edit_page(self.data['gallery'], update)}

    def test_all_raw_printings_and_issue_history_match_published_c5(self):
        raw = [{k: v for k, v in p.items() if k not in
                ('effective', 'resolution_ids', 'unresolved_gameplay_fields')}
               for p in self.snapshot['printings']]
        # Pins the complete published C5 projection, not only the 38 corrected
        # records, so refactoring the parser cannot quietly rewrite raw facts.
        self.assertEqual(catalog.sha(catalog.canonical_json(raw).encode()),
                         'b6a40081e458580e7fe6fc3efc40e6001478565717a2ebf164bb7de8bd833e88')
        self.assertEqual(catalog.sha(catalog.canonical_json(self.snapshot['source_quality_issues'][:37]).encode()),
                         '923bd9fee267e4e4e2d46301ea30771438e6146d805ab3bb45f177095388afae')

    def test_resolved_history_is_separate_from_remaining_variant_reviews(self):
        counts = self.snapshot['counts']
        self.assertEqual((counts['reviewed_field_resolutions'], counts['resolved_source_issue_groups'],
                          counts['unresolved_source_issue_groups']), (122, 47, 0))
        self.assertEqual(counts['unresolved_gameplay_identities'], 0)
        self.assertEqual(counts['unreviewed_variant_identities'], 95)
        self.assertEqual(counts['unreviewed_variants_by_field'], {'tags': 33, 'rules_text': 84})
        self.assertEqual(counts['pinned_runtime_variant_identities'], 0)
        self.assertEqual(counts['standard_legal_by_kind'],
                         {'battlefield': 69, 'gear': 131, 'gear+unit': 1, 'legend': 58, 'rune': 6, 'spell': 268, 'unit': 546})
        self.assertTrue(all(r['status'] == 'resolved_by_review' for r in self.snapshot['source_issue_reviews']))
        self.assertEqual(counts['reviewed_variant_identities'], 29)
        self.assertEqual(counts['reviewed_variant_printings'], 90)
        self.assertEqual(counts['field_reviewed_printings'], 77)
        for card in self.snapshot['cards']:
            if card['implementation']['playable_in_client']:
                self.assertEqual(card['source_review']['unresolved_variant_fields'], [])
        self.assertEqual(counts['implemented_legal_identities'], 425)

    def test_required_fields_gate_every_mapping_kind_and_alternates(self):
        cases = [
            ('ogn-093-298', 'domain', {'values': []}, 'unresolved gameplay fields'),
            ('sfd-057a-221', 'might', {'value': {'id': 9}}, 'Unresolved effective scalar differences'),
            ('ven-197-166', 'domain', {'values': []}, 'Unresolved member gameplay facts'),
            ('sfd-218-221', 'cardType', {'type': [], 'superType': []}, 'unresolved official type'),
            ('ven-r04', 'domain', {'values': []}, 'Unresolved member gameplay facts'),
            ('sfd-057-221', 'might', {}, 'Unresolved member gameplay facts'),
        ]
        for official_id, field, value, error in cases:
            with self.subTest(official_id=official_id, field=field):
                def change(records):
                    next(p for p in records if p['id'] == official_id)[field] = value
                with self.assertRaisesRegex(ValueError, error):
                    catalog.make_catalog(self.manifest, self.changed_gallery(change))

    def test_new_alternate_difference_cannot_reuse_a_clean_runtime_mapping(self):
        def change(records):
            variant = copy.deepcopy(next(p for p in records if p['id'] == 'ogn-093-298'))
            variant.update(id='ogn-review-variant', publicCode='OGN-REVIEW/999')
            variant['text']['richText']['body'] = '<p>Draw 1.</p>'
            records.append(variant)
        with self.assertRaisesRegex(ValueError, 'Runtime mapping requires review'):
            catalog.make_catalog(self.manifest, self.changed_gallery(change))

    def test_new_main_mapping_with_variant_difference_requires_review(self):
        candidate = next(c for c in self.snapshot['cards'] if c['kind'] == 'unit'
                         and c['standard_legal'] and not c['implementation']['playable_in_client']
                         and c['source_review']['variant_diagnostics']
                         and not c['source_review']['unresolved_gameplay_fields'])
        runtime = copy.deepcopy(catalog.load_runtime_manifest())
        runtime['cards'].append({'id': len(runtime['cards']), 'official_id': candidate['preferred_printing'],
                                 'name': candidate['name']})
        with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
            with self.assertRaisesRegex(ValueError, 'Runtime mapping requires review'):
                catalog.make_catalog(self.manifest, self.data)

    def test_reviewed_printings_remain_pinned_when_differences_disappear(self):
        def change(records):
            base = next(p for p in records if p['id'] == 'sfd-057-221')
            for p in records:
                if p['name'] == base['name'] and p.get('subtitle') == base.get('subtitle'):
                    p['text']['richText']['body'] = '<p>Draw 4.</p>'
        with self.assertRaisesRegex(ValueError, 'Stale raw printing pin'):
            catalog.make_catalog(self.manifest, self.changed_gallery(change))

    def test_runtime_tags_and_both_html_panels_must_match_effective_sources(self):
        original = catalog.load_runtime_manifest()
        cases = [(0, 'tags', ['Piltover'], 'tags mismatch'),
                 (0, 'tags', original['cards'][0]['tags'] * 2, 'tags mismatch'),
                 (0, 'official_text_html', '<p>Draw 4.</p>', 'rules HTML mismatch'),
                 (121, 'equipment_effect_html', '<p>[Backline]</p>', 'equipment HTML mismatch')]
        for card_id, field, value, error in cases:
            with self.subTest(card_id=card_id, field=field):
                runtime = copy.deepcopy(original)
                runtime['cards'][card_id][field] = value
                with patch.object(catalog, 'load_runtime_manifest', return_value=runtime):
                    with self.assertRaisesRegex(ValueError, error):
                        catalog.make_catalog(self.manifest, self.data)

    def test_reviewed_reminder_variants_cannot_use_a_compatibility_fallback(self):
        identity = next(c for c in self.snapshot['cards'] if c['identity'] == 'sfd-057-221')
        variants = [p for p in self.snapshot['printings'] if p['official_id'] in identity['official_ids']]
        unreviewed = catalog.identity_source_review(variants)
        self.assertEqual(unreviewed['unresolved_variant_fields'], ['rules_text'])
        with self.assertRaisesRegex(ValueError, 'Runtime mapping requires review'):
            catalog.require_mapping_source_ready(unreviewed, variants, identity['implementation'])
        self.assertEqual(identity['source_review']['status'], 'reviewed_variants')
        self.assertEqual(len({p['effective']['rules_text'] for p in variants}), 2)

    def changed_hub_body(self, transform):
        def change(page):
            for blade in page['blades']:
                rich_text = blade.get('richText')
                if rich_text and rich_text.get('body'):
                    rich_text['body'] = transform(rich_text['body'])
        return edit_page(self.data['hub'], change)

    def test_current_core_link_change_requires_review_even_if_old_link_is_retained(self):
        old_url = catalog.parse_core_rules_url(self.data['hub'])
        new_url = 'https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/revised-core.pdf'
        hub = self.changed_hub_body(lambda text: text.replace(old_url, new_url) +
                                    f'<h3>Historical documents</h3><a href="{old_url}">Core Rules</a>')
        self.assertEqual(catalog.parse_core_rules_url(hub), new_url)
        with self.assertRaisesRegex(ValueError, 'Core.*(link|URL|authority)|core.*(link|URL|authority)'):
            catalog.make_catalog(self.manifest, {**self.data, 'hub': hub})

    def test_changed_current_rengar_erratum_requires_review(self):
        def update(page):
            for blade in page['blades']:
                rich_text = blade.get('richText')
                if rich_text:
                    rich_text['body'] = rich_text.get('body', '').replace(
                        'I can [Ambush] to a battlefield where there are enemy units,',
                        'I can [Ambush] to a battlefield where there are friendly units,')
        changed = edit_page(self.data['unleashed_errata'], update)
        self.assertNotEqual(changed, self.data['unleashed_errata'])
        with self.assertRaisesRegex(ValueError, '[Ee]rrata differ'):
            catalog.make_catalog(self.manifest, {**self.data, 'unleashed_errata': changed})

    def test_ambiguous_current_core_links_fail_closed(self):
        url = catalog.parse_core_rules_url(self.data['hub'])
        hub = self.changed_hub_body(lambda text: text.replace('<h3>Tournament Rules</h3>',
            f'<p><a href="{url}">Core Rules</a></p><h3>Tournament Rules</h3>'))
        with self.assertRaisesRegex(ValueError, 'exactly one current Core Rules link'):
            catalog.parse_core_rules_url(hub)

    def test_markup_only_live_check_preserves_origin_and_writes_nothing(self):
        changed = {**self.data, 'gallery': self.data['gallery'] + b'\n<!-- unrelated website build -->'}
        paths = (catalog.OUTPUT, catalog.REPORT, catalog.MANIFEST)
        before = {p: p.read_bytes() for p in paths}
        with patch('sys.argv', ['import_catalog.py', '--check-live']), \
             patch.object(catalog, 'fetch_sources', return_value=changed), \
             patch.object(catalog, 'archive_sources') as archive, redirect_stdout(io.StringIO()):
            self.assertEqual(catalog.main(), 0)
        archive.assert_not_called()
        self.assertEqual(before, {p: p.read_bytes() for p in paths})

    def test_live_check_requires_current_committed_outputs(self):
        with tempfile.TemporaryDirectory() as folder:
            stale = Path(folder) / 'catalog.json'
            stale.write_text('{}')
            with patch('sys.argv', ['import_catalog.py', '--check-live']), \
                 patch.object(catalog, 'OUTPUT', stale), patch.object(catalog, 'fetch_sources') as fetch:
                with self.assertRaisesRegex(ValueError, 'Committed inventory/report is stale'):
                    catalog.main()
            fetch.assert_not_called()

    def test_failed_candidate_or_report_cannot_replace_any_current_artifact(self):
        def stale_record(records):
            next(p for p in records if p['id'] == 'rad-sp4-005')['might']['value']['id'] = 5
        changed_core = self.changed_hub_body(lambda value: value.replace(
            catalog.parse_core_rules_url(self.data['hub']),
            'https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/revised-core.pdf'))
        for label, data, report_error in [('printing', self.changed_gallery(stale_record), False),
                                         ('core', {**self.data, 'hub': changed_core}, False),
                                         ('report', self.data, True)]:
            with self.subTest(failure=label), tempfile.TemporaryDirectory() as folder:
                output, report = Path(folder) / 'catalog.json', Path(folder) / 'report.md'
                output.write_bytes(b'old catalog'); report.write_bytes(b'old report')
                with ExitStack() as stack:
                    stack.enter_context(patch('sys.argv', ['import_catalog.py', '--refresh', '--as-of', '2026-10-24']))
                    stack.enter_context(patch.object(catalog, 'fetch_sources', return_value=data))
                    stack.enter_context(patch.object(catalog, 'OUTPUT', output))
                    stack.enter_context(patch.object(catalog, 'REPORT', report))
                    archive = stack.enter_context(patch.object(catalog, 'archive_sources'))
                    if report_error:
                        stack.enter_context(patch.object(catalog, 'report', side_effect=ValueError('report invalid')))
                    with self.assertRaises(ValueError):
                        catalog.main()
                archive.assert_not_called()
                self.assertEqual(output.read_bytes(), b'old catalog')
                self.assertEqual(report.read_bytes(), b'old report')

    def test_successful_markup_refresh_retains_reviewed_origin(self):
        real_root, real_make = catalog.ROOT, catalog.make_catalog
        changed = {**self.data, 'gallery': self.data['gallery'] + b'\n<!-- new website layout -->'}
        origins = self.snapshot['review_evidence']['origin_archives']
        origin_bytes = {spec['archive']: (real_root / spec['archive']).read_bytes() for spec in origins.values()}
        def validate_candidate(manifest, data):
            with patch.object(catalog, 'ROOT', real_root):
                return real_make(manifest, data)
        with tempfile.TemporaryDirectory() as folder, ExitStack() as stack:
            root = Path(folder)
            (root / 'assets/data').mkdir(parents=True); (root / 'docs').mkdir()
            for name, value in {'ROOT': root, 'ARCHIVE': root / 'assets/data/catalog_sources',
                                'MANIFEST': root / 'assets/data/catalog_sources.json',
                                'OUTPUT': root / 'assets/data/full_catalog.json',
                                'REPORT': root / 'docs/CARD_COVERAGE.md'}.items():
                stack.enter_context(patch.object(catalog, name, value))
            stack.enter_context(patch('sys.argv', ['import_catalog.py', '--refresh', '--as-of', '2026-10-25']))
            stack.enter_context(patch.object(catalog, 'fetch_sources', return_value=changed))
            stack.enter_context(patch.object(catalog, 'make_catalog', side_effect=validate_candidate))
            with redirect_stdout(io.StringIO()):
                self.assertEqual(catalog.main(), 0)
            refreshed = json.loads(catalog.OUTPUT.read_text())
            current = json.loads(catalog.MANIFEST.read_text())
            self.assertNotEqual(current['sources']['gallery']['sha256'], self.manifest['sources']['gallery']['sha256'])
            self.assertEqual(gzip.decompress((root / current['sources']['gallery']['archive']).read_bytes()), changed['gallery'])
            self.assertEqual(refreshed['review_evidence']['origin_archives'], origins)
            self.assertEqual(catalog.comparable(refreshed), catalog.comparable(self.snapshot))
        self.assertEqual(origin_bytes, {path: (real_root / path).read_bytes() for path in origin_bytes})


if __name__ == '__main__':
    unittest.main()
