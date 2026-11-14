"""Check the complete accepted review inventory without freezing its size.

Historical source tests pin the facts and runtime scope of their own cards.
New, unrelated reviews may append records; every current review must still
reach the generated catalog exactly once and retain its complete evidence.
"""
import json
import import_catalog as catalog


def assert_review_inventory(case, snapshot):
    manifest = json.loads((catalog.ROOT / 'assets/data/catalog_resolutions.json').read_text())
    case.assertEqual(snapshot['reviewed_field_resolutions'], manifest['resolutions'])
    expected = manifest['historical_issues']
    actual = snapshot['source_issue_reviews']
    case.assertEqual(len(actual), len(expected))
    for source, result in zip(expected, actual, strict=True):
        case.assertEqual(result['review_issue_id'], source['review_issue_id'])
        case.assertEqual(result['resolution_ids'], source['resolution_ids'])
        case.assertEqual(result['raw_issue'], source['raw_issue'])
        case.assertEqual(result['status'], 'resolved_by_review')
    case.assertEqual(snapshot['source_quality_issues'], [row['raw_issue'] for row in expected])
    counts = snapshot['counts']
    case.assertEqual(counts['reviewed_field_resolutions'], len(manifest['resolutions']))
    case.assertEqual(counts['field_reviewed_printings'], len({r['official_id'] for r in manifest['resolutions']}))
    case.assertEqual(counts['source_quality_issues'], len(expected))
    case.assertEqual(counts['resolved_source_issue_groups'], len(expected))
    case.assertEqual(counts['unresolved_source_issue_groups'], 0)
