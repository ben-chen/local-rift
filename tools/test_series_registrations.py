"""Re-derive all admitted LA registrations and verify the two printed sideboard cards."""
import collections
import gzip
import json
import re
import struct
import unittest

import catalog_review as review
import import_catalog as catalog
from test_rengar_preset import ArticleLines


class SeriesRegistrations(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runtime = catalog.load_runtime_manifest()
        cls.full = json.loads((catalog.ROOT / 'assets/data/full_catalog.json').read_text())
        cls.cards = json.loads((catalog.ROOT / 'assets/data/card_admissions/series_sideboards/admissions.json').read_text())
        cls.regs = json.loads((catalog.ROOT / 'assets/data/preset_admissions/series_los_angeles/registrations.json').read_text())

    def test_card_faces_and_runtime_facts_match_reviewed_primary_printings(self):
        for entry in self.cards['cards']:
            identity = next(x for x in self.full['cards'] if x['identity'] == entry['identity'])
            printing = next(x for x in self.full['printings'] if x['official_id'] == entry['identity'])
            raw = {key: printing[key] for key in review.PROJECTION_KEYS}
            self.assertEqual(entry['raw_printing'], raw)
            self.assertEqual(entry['raw_printing_sha256'], review.printing_sha256(raw))
            self.assertEqual(entry['printing_cohort'], identity['official_ids'])
            self.assertEqual(entry['effective'], printing['effective'])
            self.assertEqual(entry['errata'], identity['errata'])
            self.assertFalse(printing['unresolved_gameplay_fields'])
            self.assertTrue(identity['standard_legal'])
            self.assertEqual(entry['gallery_raw_sha256'], self.full['sources']['gallery']['sha256'])
            runtime = self.runtime['cards'][entry['runtime_id']]
            for a, b in [('official_id', 'official_id'), ('name', 'name')]:
                self.assertEqual(runtime[a], raw[b])
            for key in ['energy', 'power', 'might']:
                self.assertEqual(runtime[key], printing['effective'][key] or 0)
            self.assertEqual(runtime['official_text'], printing['effective']['rules_text'])
            self.assertEqual(runtime['domains'], [d.title() for d in raw['domains']])
            self.assertEqual(runtime['keywords'], identity['effective_keywords_observed'])
            self.assertEqual(runtime['tags'], printing['effective']['tags'])
            self.assertTrue(entry['original_image']['actual_full_face_viewed'])
            for key in ['original_image', 'runtime_image']:
                image = entry[key]
                data = (catalog.ROOT / image['path']).read_bytes()
                self.assertEqual(catalog.sha(data), image['sha256'])
                self.assertEqual(list(struct.unpack('>II', data[16:24])), image['dimensions'])

    def test_preceding_cards_and_all_six_main_deck_recipes_are_preserved(self):
        old = self.cards['preserved_runtime_prefix']
        for key in ['cards', 'presets']:
            rows = self.runtime['cards' if key == 'cards' else 'decks']
            n = old['card_count' if key == 'cards' else 'preset_count']
            self.assertEqual(catalog.sha(review.canonical_bytes(rows[:n])), old[key + '_sha256'])

    def test_every_registration_is_exactly_the_primary_article_including_sideboard(self):
        raw = gzip.decompress((catalog.ROOT / self.regs['source_archive']).read_bytes())
        self.assertEqual(catalog.sha(raw), self.regs['source_raw_sha256'])
        parser = ArticleLines(); parser.feed(catalog.body(raw))
        lines = [x.strip() for x in ''.join(parser.parts).splitlines() if x.strip()]
        fields = catalog.load_battlefield_manifest()
        self.assertEqual([x['preset'] for x in self.regs['registrations']], [0, 2, 4])
        for reg in self.regs['registrations']:
            copies = []
            for start, line in enumerate(lines):
                if line != '@deck:' + reg['pilot']:
                    continue
                end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith(('@deck:', '@heading:'))), len(lines))
                source = lines[start + 1:end]
                self.assertEqual(source[1], 'Overall Ranking: #' + str(reg['placing']))
                sections = {}; section = None
                for line in source[2:]:
                    if line.endswith(':'):
                        section = line[:-1]; self.assertNotIn(section, sections); sections[section] = []
                    else:
                        row = re.fullmatch(r'(\d+)\s+(.+)', line); self.assertIsNotNone(row)
                        sections[section].append({'name': row[2], 'count': int(row[1])})
                copies.append(sections)
            self.assertEqual(len(copies), reg['source_copies'])
            for copy in copies:
                self.assertEqual(copy, reg['source_list'])
            source = copies[0]; preset = self.runtime['decks'][reg['preset']]
            def tally(rows):
                result = collections.Counter()
                for r in rows: result[r['name']] += r['count']
                return result
            self.assertEqual(tally(source['Main Deck'] + source['Champion']), tally(preset['cards']))
            self.assertEqual(source['Champion'], [{'name': preset['chosen'], 'count': 1}])
            self.assertEqual(source['Legend'], [{'name': preset['legend_name'], 'count': 1}])
            self.assertEqual(tally(source['Rune Pool']), collections.Counter({k + ' Rune': n for k, n in preset['runes'].items()}))
            self.assertEqual(source['Sideboard'], [{'name': self.runtime['cards'][i]['name'], 'count': n} for i, n in reg['sideboard']])
            self.assertEqual(source['Battlefields'], [{'name': fields[i]['name'], 'count': 1} for i in reg['battlefields']])
            pool = tally(preset['cards']) + tally(source['Sideboard'])
            self.assertTrue(all(n <= 3 for n in pool.values()))
            self.assertEqual(sum(tally(preset['cards']).values()), 40)
            self.assertEqual(sum(tally(source['Sideboard']).values()), 10)


if __name__ == '__main__':
    unittest.main()
