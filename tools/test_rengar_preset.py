"""The Rengar preset matches the archived winning list, without card substitutions."""
import collections
import gzip
import hashlib
import json
from html.parser import HTMLParser
import re
import unittest

import import_catalog as catalog


class ArticleLines(HTMLParser):
    """Read block/line boundaries without relying on the prior audit's parser."""
    def __init__(self):
        super().__init__()
        self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag in {"br", "p", "h1", "h2", "h3"}:
            self.parts.append("\n")
        if tag == 'h3':
            self.parts.append('@deck:')
        if tag in {'h1', 'h2'}:
            self.parts.append('@heading:')

    def handle_endtag(self, tag):
        if tag in {"p", "h1", "h2", "h3"}:
            self.parts.append("\n")

    def handle_data(self, text):
        self.parts.append(text)


class RengarPresetTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.folder = catalog.ROOT / 'assets/data/preset_admissions/rengar_2026_09'
        cls.proof = json.loads((cls.folder / 'admission.json').read_text())
        cls.runtime = catalog.load_runtime_manifest()
        cls.preset = cls.runtime['decks'][4]
        cls.original = gzip.decompress((catalog.ROOT / cls.proof['source_archive']).read_bytes())

    def test_exact_published_winning_list_and_source_hash(self):
        self.assertEqual(hashlib.sha256(self.original).hexdigest(), self.proof['source_raw_sha256'])
        parser = ArticleLines()
        parser.feed(catalog.body(self.original))
        lines = [line.strip() for line in ''.join(parser.parts).splitlines() if line.strip()]
        starts = [i for i, line in enumerate(lines) if line == '@deck:DSG Prismaticism']
        self.assertEqual(len(starts), 2)  # Best-of and Top 8 repeat the same list.
        copies = []
        for start in starts:
            end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith(('@deck:', '@heading:'))), len(lines))
            copies.append(lines[start + 1:end])
        self.assertEqual(copies[0], copies[1])
        source = copies[0]
        self.assertEqual(source[:2], ['Legend Rank: #1/92', 'Overall Ranking: #1'])
        sections = {}
        current = None
        for line in source[2:]:
            if line.endswith(':'):
                current = line[:-1]
                self.assertNotIn(current, sections)
                sections[current] = []
            else:
                row = re.fullmatch(r'(\d+)\s+(.+)', line)
                self.assertIsNotNone(row, line)
                sections[current].append((row[2], int(row[1])))
        self.assertEqual(set(sections), {'Legend', 'Champion', 'Main Deck', 'Battlefields', 'Rune Pool', 'Sideboard'})
        def tally(rows):
            result = collections.Counter()
            for name, count in rows:
                result[name] += count
            return result
        self.assertEqual(tally((r['name'], r['count']) for r in self.preset['cards']),
                         tally(sections['Champion'] + sections['Main Deck']))
        self.assertEqual(sum(r['count'] for r in self.preset['cards']), 40)
        self.assertEqual(sections['Champion'], [(self.preset['chosen'], 1)])
        self.assertEqual(sections['Legend'], [(self.preset['legend_name'], 1)])
        self.assertEqual(tally(sections['Rune Pool']),
                         tally((name + ' Rune', count) for name, count in self.preset['runes'].items()))
        self.assertEqual(sections['Battlefields'], [(name, 1) for name in self.preset['battlefields']])
        for section, rows in sections.items():
            self.assertEqual(rows, [(r['name'], r['count']) for r in self.proof['source_list'][section]])
        self.assertEqual((self.preset['pilot'], self.preset['placing']), ('DSG Prismaticism', 1))

    def test_preserved_four_presets_and_registered_card_identity(self):
        old = json.dumps(self.runtime['decks'][:4], sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
        self.assertEqual(hashlib.sha256(old).hexdigest(), self.proof['preserved_presets_sha256'])
        self.assertEqual(self.preset, json.loads((self.folder / 'preset.json').read_text()))
        self.assertEqual(len(self.runtime['decks']), 5)
        self.assertEqual(len({r['id'] for r in self.preset['cards']}), len(self.preset['cards']))
        for row in self.preset['cards']:
            card = self.runtime['cards'][row['id']]
            self.assertEqual(card['name'], row['name'])
            self.assertTrue(set(card['domains']) <= set(self.preset['domains']))
            self.assertIn(row['count'], range(1, 4))
        fields = catalog.load_battlefield_manifest()
        field = fields[self.proof['default_battlefield_id']]
        self.assertEqual(field['name'], 'Seat of Power')
        self.assertIn(field['name'], self.preset['battlefields'])

    def test_training_baseline_is_pinned_to_the_preceding_release(self):
        source = (catalog.ROOT / self.proof['predecessor_training_recipe_source']).read_bytes()
        self.assertEqual(hashlib.sha256(source).hexdigest(), self.proof['predecessor_training_recipe_sha256'])
        baseline = json.loads((self.folder / 'predecessor_training_decks.json').read_text())
        self.assertEqual(baseline['api_sha256'], 'a99d2d8db78f0fc97b623dfa1fd31c3132a3a41c3ef703699c72f19c5778b9a4')
        self.assertEqual(len(baseline['training_decks']), 10)
        self.assertEqual(baseline['training_decks'][8]['legend'], 8)
        self.assertNotEqual(baseline['training_decks'][8]['cards'],
                            [[r['id'], r['count']] for r in self.preset['cards']])


if __name__ == '__main__':
    unittest.main()
