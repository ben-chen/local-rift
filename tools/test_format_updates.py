"""Verify scheduled bans independently against the archived announcement."""
from datetime import date
import gzip
import hashlib
import json
from pathlib import Path
import re
import unittest

import import_catalog as catalog


class FormatUpdates(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads((catalog.ROOT / 'assets/data/format_updates.json').read_text())
        cls.catalog = json.loads(catalog.OUTPUT.read_text())
        cls.update, = cls.manifest['updates']
        cls.compressed = (catalog.ROOT / cls.update['archive']).read_bytes()
        cls.raw = gzip.decompress(cls.compressed)
        cls.article = catalog.body(cls.raw)

    def test_archive_and_dates_match_official_announcement(self):
        self.assertEqual(hashlib.sha256(self.compressed).hexdigest(), self.update['archive_sha256'])
        self.assertEqual(hashlib.sha256(self.raw).hexdigest(), self.update['content_sha256'])
        self.assertEqual(self.update['source_url'], 'https://playriftbound.com/en-us/news/announcements/radiance-ban-list-updates-effective-november-23-2026/')
        self.assertIn(b'2026-11-13T16:00:00.000Z', self.raw)
        self.assertIn('Effective: November 23, 2026', catalog.plain(self.article))
        self.assertEqual(self.update['announced_on'], '2026-11-13')
        self.assertEqual(self.update['effective_on'], '2026-11-23')
        self.assertLessEqual(date.fromisoformat(self.manifest['catalog_as_of']), date.fromisoformat(self.manifest['reviewed_on']))
        self.assertLess(date.fromisoformat(self.manifest['reviewed_on']), date.fromisoformat(self.update['effective_on']))

    def test_complete_ban_lists_and_every_printing_cohort_match_catalog(self):
        expected = {}
        for heading, count in [('Cards', 8), ('Battlefields', 2)]:
            match = re.search(r'<h4>' + heading + r'</h4><ul>(.*?)</ul>', self.article, re.S)
            self.assertIsNotNone(match)
            names = [catalog.plain(value) for value in re.findall(r'<li>(.*?)</li>', match[1], re.S)]
            self.assertEqual(len(names), count)
            expected.update({name: heading for name in names})
        records = self.update['banned']
        self.assertEqual(len(records), 10)
        self.assertEqual({row['name'] for row in records}, set(expected))
        by_id = {card['identity']: card for card in self.catalog['cards']}
        for row in records:
            card = by_id[row['identity']]
            self.assertEqual((row['name'], row['kind'], row['official_ids']),
                             (card['name'], card['kind'], card['official_ids']))
            self.assertEqual(row['kind'] == 'battlefield', expected[row['name']] == 'Battlefields')
            self.assertTrue(card['standard_legal'], 'Future bans must not rewrite the current accepted catalog.')

    def test_region_exceptions_are_explicit_and_not_assigned_the_global_date(self):
        text = catalog.plain(self.article)
        self.assertIn('Korea and Traditional Chinese regions until Radiance is released', text)
        self.assertEqual(self.manifest['scope'], 'global_standard_1v1')
        self.assertIn('does not give exact regional effective dates', self.update['regional_exceptions'])
        self.assertNotIn('regional_effective_on', self.update)


if __name__ == '__main__':
    unittest.main()
