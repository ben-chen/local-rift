"""The Stuttgart preset is the complete primary-source registration."""
import collections
import gzip
import json
import re
import unittest

import catalog_review as review
import import_catalog as catalog
from test_rengar_preset import ArticleLines


class TableArticleLines(ArticleLines):
    def handle_endtag(self, tag):
        super().handle_endtag(tag)
        if tag in {'td', 'th', 'tr'}:
            self.parts.append('\n')


class SoneconPresetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.folder = catalog.ROOT / 'assets/data/preset_admissions/riven_stuttgart_2026_11'
        cls.proof = json.loads((cls.folder / 'admission.json').read_text())
        cls.runtime = catalog.load_runtime_manifest()
        cls.preset = cls.runtime['decks'][6]

    def test_complete_registration_matches_both_published_copies(self):
        proof = self.proof
        raw = gzip.decompress((catalog.ROOT / proof['source_archive']).read_bytes())
        self.assertEqual(catalog.sha(raw), proof['source_raw_sha256'])
        parser = TableArticleLines()
        parser.feed(catalog.body(raw))
        lines = [re.sub(r'\s+', ' ', line).strip()
                 for line in ''.join(parser.parts).splitlines() if line.strip()]
        copies = []
        for start, line in enumerate(lines):
            if line != '@deck:Sonecon':
                continue
            end = next((i for i in range(start + 1, len(lines))
                        if lines[i].startswith(('@deck:', '@heading:'))), len(lines))
            self.assertEqual(lines[start + 2], 'Overall Ranking: #2')
            sections = {}
            section = None
            for line in lines[start + 3:end]:
                if line.endswith(':'):
                    section = line[:-1]
                    self.assertNotIn(section, sections)
                    sections[section] = []
                else:
                    row = re.fullmatch(r'(\d+)\s+(.+)', line)
                    self.assertIsNotNone(row, line)
                    sections[section].append({'name': row[2], 'count': int(row[1])})
            copies.append(sections)
        self.assertEqual(len(copies), 2)
        self.assertTrue(all(copy == proof['source_list'] for copy in copies))
        source = copies[0]
        p = self.preset
        self.assertEqual(p, json.loads((self.folder / 'preset.json').read_text()))
        tally = lambda rows: collections.Counter({catalog.normalize(r['name']): r['count'] for r in rows})
        self.assertEqual(tally(source['Main Deck'] + source['Champion']), tally(p['cards']))
        self.assertEqual(tally(source['Sideboard']), tally(p['sideboard']))
        self.assertEqual(source['Champion'], [{'name': p['chosen'], 'count': 1}])
        self.assertEqual(source['Legend'], [{'name': p['legend_name'], 'count': 1}])
        self.assertEqual(source['Rune Pool'], [{'name': k + ' Rune', 'count': v} for k, v in p['runes'].items()])
        self.assertEqual((p['pilot'], p['placing'], p['chosen_champion'], p['legend_id']),
                         ('Sonecon', 2, 439, 'rad-149-167'))
        fields = catalog.load_battlefield_manifest()
        expected_names = []
        for row in source['Battlefields']:
            self.assertEqual(row['count'], 1)
            name = row['name']
            if name in proof['source_name_equivalences']:
                equivalent = proof['source_name_equivalences'][name]
                field = fields[equivalent['battlefield_id']]
                self.assertEqual(field['source_id'], equivalent['official_id'])
                self.assertEqual(field['name'], equivalent['runtime_name'])
                name = field['name']
            expected_names.append(name)
        self.assertEqual(expected_names, p['battlefields'])
        self.assertEqual(expected_names, [fields[i]['name'] for i in [11, 7, 12]])
        self.assertEqual(sum(tally(p['cards']).values()), 40)
        self.assertEqual(sum(tally(p['sideboard']).values()), 10)
        self.assertTrue(all(n <= 3 for n in (tally(p['cards']) + tally(p['sideboard'])).values()))
        full = json.loads(catalog.OUTPUT.read_text())
        identities = {pid: c for c in full['cards'] for pid in c['official_ids']}
        for row in p['cards'] + p['sideboard']:
            card = self.runtime['cards'][row['id']]
            self.assertEqual(card['name'], row['name'])
            self.assertTrue(set(card['domains']) <= set(p['domains']))
            identity = identities[card['official_id']]
            self.assertTrue(identity['standard_legal'])
            self.assertTrue(identity['implementation']['playable_in_client'])
            self.assertEqual(identity['source_review']['unresolved_variant_fields'], [])
            self.assertEqual(identity['source_review']['unresolved_gameplay_fields'], [])

    def test_published_irelia_and_other_recipes_are_unchanged(self):
        pin = self.proof['preserved_runtime_prefix']
        for key, count, digest in [('cards', 'card_count', 'cards_sha256'),
                                   ('decks', 'preset_count', 'presets_sha256')]:
            self.assertEqual(catalog.sha(review.canonical_bytes(self.runtime[key][:pin[count]])), pin[digest])
        self.assertEqual((pin['card_count'], pin['preset_count']), (435, 6))


if __name__ == '__main__':
    unittest.main()
