//! Every accepted review must reach the generated catalog with its evidence.
//! Individual admission tests separately preserve their historical card facts.
use serde_json::Value;
use std::collections::BTreeSet;

pub fn assert_review_inventory(catalog: &Value) {
    let manifest: Value =
        serde_json::from_str(include_str!("../../assets/data/catalog_resolutions.json")).unwrap();
    let resolutions = manifest["resolutions"].as_array().unwrap();
    assert_eq!(
        catalog["reviewed_field_resolutions"],
        manifest["resolutions"]
    );
    let printings: BTreeSet<_> = resolutions
        .iter()
        .map(|row| row["official_id"].as_str().unwrap())
        .collect();
    let raw_printings: BTreeSet<_> = manifest["raw_printings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["official_id"].as_str().unwrap())
        .collect();
    assert_eq!(printings, raw_printings);
    let expected = manifest["historical_issues"].as_array().unwrap();
    let actual = catalog["source_issue_reviews"].as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (source, result) in expected.iter().zip(actual) {
        for field in ["review_issue_id", "resolution_ids", "raw_issue"] {
            assert_eq!(result[field], source[field], "review {field}");
        }
        assert_eq!(result["status"], "resolved_by_review");
    }
    assert_eq!(
        catalog["source_quality_issues"].as_array().unwrap(),
        &expected
            .iter()
            .map(|row| row["raw_issue"].clone())
            .collect::<Vec<_>>()
    );
    let counts = &catalog["counts"];
    assert_eq!(counts["reviewed_field_resolutions"], resolutions.len());
    assert_eq!(counts["field_reviewed_printings"], printings.len());
    assert_eq!(counts["source_quality_issues"], expected.len());
    assert_eq!(counts["resolved_source_issue_groups"], expected.len());
    assert_eq!(counts["unresolved_source_issue_groups"], 0);
}
