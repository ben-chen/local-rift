"""Evidence/applicability tests, independent of runtime implementation admission."""
import copy
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import catalog_review as review


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def project(raw):
    return json.loads(raw)["printings"]


def plain(raw):
    return raw.replace("<p>", "").replace("</p>", "")


class ReviewValidationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.printing = {
            "official_id": "rad-test", "public_code": "RAD-001/167", "name": "Evidence Test",
            "set": "RAD", "collector_number": 1, "types": ["gear"],
            "type_source": "cardType.type", "structured_types": ["gear"],
            "accessibility_types": ["unit"], "type_labels_conflict": True,
            "super_types": [], "domains": [], "energy": 1, "power": None,
            "might": 3, "might_bonus": None, "rules_html": "<p>Draw 1.</p>",
            "rules_text": "Draw 1.", "equipment_html": "", "equipment_text": "",
            "tags": [], "rarity": "common",
            "image_url": "https://cmsassets.rgpub.io/sanity/images/test.png",
            "image_accessibility_text": "Riftbound Unit: Evidence Test. Draw 1.",
            "illustrators": ["Artist"],
        }
        self.origin = json.dumps({"build": "old", "printings": [self.printing]}).encode()
        self.origin_hash = sha(self.origin)
        self.archive_path = f"assets/data/catalog_evidence/pages/{self.origin_hash}.html.gz"
        self.write(self.archive_path, gzip.compress(self.origin, mtime=0))
        self.image = b"\x89PNG\r\n\x1a\nreviewed image bytes"
        self.image_path = "assets/data/catalog_evidence/images/rad-test.png"
        self.write(self.image_path, self.image)
        self.manifest = {
            "schema_version": 1, "printing_projection_version": 1, "status": "reviewed",
            "origin_archives": {self.origin_hash: {
                "url": "https://playriftbound.com/en-us/card-gallery/", "archive": self.archive_path,
                "sha256": self.origin_hash, "uncompressed_bytes": len(self.origin),
            }},
            "raw_printings": [{**copy.deepcopy(self.printing),
                               "review_draft_sha256": review.printing_sha256(self.printing)}],
            "resolutions": [], "historical_issues": [],
        }
        self.manifest["resolutions"] = [self.resolution("types", ["unit"]), self.resolution("domains", ["fury"])]

    def write(self, path, raw):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)

    def resolution(self, field, effective):
        source = "structured_types" if field == "types" else field
        return {
            "id": f"rad-test:{field}", "official_id": "rad-test", "name": self.printing["name"],
            "field": field, "reviewed_on": "2026-10-24", "basis": "confirmed_artwork",
            "operation": "confirm" if review.typed_equal(self.printing[field], effective) else "override",
            "expected_raw_value": copy.deepcopy(self.printing[field]), "raw_source_field": source,
            "expected_raw_source_value": copy.deepcopy(self.printing[source]), "effective_value": effective,
            "gallery_uncompressed_sha256": self.origin_hash,
            "expected_printing_sha256": review.printing_sha256(self.printing),
            "printing_hash_encoding": review.HASH_ENCODING, "reason": "Exact matching printed card face.",
            "evidence": [{"kind": "official_card_image", "url": self.printing["image_url"],
                          "sha256": sha(self.image), "bytes": len(self.image),
                          "local_path": self.image_path, "location": "Printed type/domain line"}],
            "enables_runtime_support": False,
        }

    def apply(self, *, printings=None, manifest=None):
        return review.apply_reviews(
            [self.printing] if printings is None else printings, root=self.root,
            project_gallery=project, plain=plain,
            manifest=self.manifest if manifest is None else manifest,
        )

    def test_effective_facts_are_complete_and_raw_inputs_never_mutate(self):
        before = copy.deepcopy((self.printing, self.manifest))
        result = self.apply()
        p = result["printings"][0]
        self.assertEqual(p["types"], ["gear"])
        self.assertEqual(p["domains"], [])
        self.assertEqual(p["effective"]["types"], ["unit"])
        self.assertEqual(p["effective"]["domains"], ["fury"])
        self.assertEqual(set(p["effective"]), set(review.GAMEPLAY_FIELDS))
        self.assertIsNone(p["effective"]["power"])
        self.assertEqual(p["unresolved_gameplay_fields"], [])
        self.assertEqual(before, (self.printing, self.manifest))
        p["effective"]["tags"].append("Invented")
        result["resolutions"][0]["effective_value"].append("Invented")
        self.assertEqual(before, (self.printing, self.manifest))
        self.assertNotIn("implementation", p)

    def test_unreviewed_missing_domains_and_types_remain_unresolved(self):
        empty = {"schema_version": 1, "printing_projection_version": 1, "resolutions": []}
        result = self.apply(manifest=empty)["printings"][0]
        self.assertEqual(result["unresolved_gameplay_fields"], ["domains", "types"])
        p = copy.deepcopy(self.printing)
        p.update(types=["unit"], structured_types=[], type_labels_conflict=False, domains=["colorless"])
        result = self.apply(manifest=empty, printings=[p])["printings"][0]
        self.assertEqual(result["unresolved_gameplay_fields"], ["types"])
        p["structured_types"] = ["unit"]
        self.assertEqual(self.apply(manifest=empty, printings=[p])["printings"][0]["unresolved_gameplay_fields"], [])

    def test_auxiliary_token_absence_is_not_invented_into_a_main_card(self):
        p = copy.deepcopy(self.printing)
        p.update(types=[], structured_types=[], accessibility_types=[], type_labels_conflict=False,
                 super_types=["token"], domains=[])
        empty = {"schema_version": 1, "printing_projection_version": 1, "resolutions": []}
        result = self.apply(manifest=empty, printings=[p])["printings"][0]
        self.assertEqual(result["unresolved_gameplay_fields"], [])
        self.assertEqual(result["effective"]["types"], [])

    def test_missing_unit_might_is_unresolved_but_explicit_zero_and_nonunits_are_valid(self):
        p = copy.deepcopy(self.printing)
        p.update(types=["unit"], structured_types=["unit"], type_labels_conflict=False,
                 domains=["fury"], might=None)
        empty = {"schema_version": 1, "printing_projection_version": 1, "resolutions": []}
        result = self.apply(manifest=empty, printings=[p])["printings"][0]
        self.assertEqual(result["unresolved_gameplay_fields"], ["might"])
        self.assertIsNone(result["effective"]["might"])
        p["might"] = 0
        self.assertEqual(self.apply(manifest=empty, printings=[p])["printings"][0]["unresolved_gameplay_fields"], [])
        p.update(types=["gear"], structured_types=["gear"], accessibility_types=["gear"], might=None)
        self.assertEqual(self.apply(manifest=empty, printings=[p])["printings"][0]["unresolved_gameplay_fields"], [])

    def test_candidate_website_markup_changes_do_not_invalidate_origin(self):
        candidate = json.dumps({"build": "new", "irrelevant": 3, "printings": [self.printing]}).encode()
        self.assertNotEqual(sha(candidate), self.origin_hash)
        self.assertEqual(self.apply(printings=project(candidate)), self.apply())

    def test_any_changed_printing_field_stales_the_complete_record_pin(self):
        for field, changed in [("rarity", "rare"), ("energy", 2), ("image_accessibility_text", "changed"),
                               ("image_url", "https://cmsassets.rgpub.io/sanity/images/new.png")]:
            with self.subTest(field=field):
                p = copy.deepcopy(self.printing)
                p[field] = changed
                with self.assertRaisesRegex(ValueError, "Stale candidate printing hash"):
                    self.apply(printings=[p])

    def test_a_publisher_fix_is_stale_not_automatic_override_retirement(self):
        p = copy.deepcopy(self.printing)
        p["domains"] = ["fury"]
        with self.assertRaisesRegex(ValueError, "Stale candidate printing hash"):
            self.apply(printings=[p])

    def test_typed_prerequisites_do_not_equate_boolean_and_integer(self):
        self.assertFalse(review.typed_equal({"a": [True]}, {"a": [1]}))
        self.assertFalse(review.typed_equal(1, 1.0))
        for field in ("expected_raw_value", "expected_raw_source_value"):
            with self.subTest(field=field):
                m = copy.deepcopy(self.manifest)
                r = self.resolution("energy", 1)
                r[field] = True
                m["resolutions"].append(r)
                with self.assertRaisesRegex(ValueError, "prerequisite mismatch"):
                    self.apply(manifest=m)

    def test_duplicate_keys_ids_and_resolution_fields_reject(self):
        for duplicate_id in (True, False):
            with self.subTest(duplicate_id=duplicate_id):
                m = copy.deepcopy(self.manifest)
                r = copy.deepcopy(m["resolutions"][0])
                if not duplicate_id:
                    r["id"] = "another-name-for-same-field"
                m["resolutions"].append(r)
                with self.assertRaisesRegex(ValueError, "Duplicate"):
                    self.apply(manifest=m)
        self.write("assets/data/catalog_resolutions.json", b'{"status":"reviewed","status":"reviewed"}')
        with self.assertRaisesRegex(ValueError, "Duplicate JSON key"):
            review.apply_reviews([self.printing], root=self.root, project_gallery=project, plain=plain)

    def test_unknown_version_field_or_wrong_projection_shape_rejects(self):
        for version in (None, True, 2):
            m = copy.deepcopy(self.manifest)
            m["printing_projection_version"] = version
            with self.assertRaisesRegex(ValueError, "projection version"):
                self.apply(manifest=m)
        m = copy.deepcopy(self.manifest)
        m["resolutions"][0]["field"] = "implementation"
        with self.assertRaisesRegex(ValueError, "Unknown gameplay resolution field"):
            self.apply(manifest=m)
        for change in (lambda p: p.pop("tags"), lambda p: p.update(extra=True)):
            p = copy.deepcopy(self.printing)
            change(p)
            with self.assertRaisesRegex(ValueError, "projection v1"):
                self.apply(printings=[p])

    def test_malformed_manifest_and_prerequisite_shapes_report_validation_errors(self):
        for manifest in ([], False, "reviewed"):
            with self.subTest(manifest=manifest):
                with self.assertRaisesRegex(ValueError, "Invalid source resolution manifest"):
                    self.apply(manifest=manifest)
        self.write("assets/data/catalog_resolutions.json", b"[]")
        with self.assertRaisesRegex(ValueError, "Invalid source resolution manifest"):
            review.apply_reviews([self.printing], root=self.root, project_gallery=project, plain=plain)
        for key, value, message in [("raw_source_field", [], "underlying source field"),
                                    ("reviewed_on", "20261024", "review date")]:
            m = copy.deepcopy(self.manifest)
            m["resolutions"][0][key] = value
            with self.assertRaisesRegex(ValueError, message):
                self.apply(manifest=m)

    def test_invalid_effective_types_domains_and_scalar_values_reject(self):
        for field, value in [("types", ["invented"]), ("domains", ["storm"]),
                             ("domains", ["colorless", "fury"]), ("energy", True),
                             ("might", -1), ("tags", ["Mech", "Mech"])]:
            with self.subTest(field=field, value=value):
                m = copy.deepcopy(self.manifest)
                r = self.resolution(field, value)
                m["resolutions"] = [x for x in m["resolutions"] if x["field"] != field] + [r]
                with self.assertRaises(ValueError):
                    self.apply(manifest=m)

    def test_runtime_enabling_operation_mismatch_and_source_field_spoof_reject(self):
        for key, value, message in [("enables_runtime_support", True, "enable runtime"),
                                    ("operation", "confirm", "operation/value"),
                                    ("raw_source_field", "name", "underlying source")]:
            m = copy.deepcopy(self.manifest)
            m["resolutions"][0][key] = value
            with self.assertRaisesRegex(ValueError, message):
                self.apply(manifest=m)

    def test_html_and_readable_text_are_validated_as_a_pair(self):
        self.manifest["resolutions"].append(self.resolution("rules_html", "<p>Draw 2.</p>"))
        with self.assertRaisesRegex(ValueError, "HTML/text mismatch"):
            self.apply()
        self.manifest["resolutions"].append(self.resolution("rules_text", "Draw 2."))
        self.assertEqual(self.apply()["printings"][0]["effective"]["rules_text"], "Draw 2.")
        self.assertEqual(self.printing["rules_text"], "Draw 1.")

    def test_corrupt_missing_origin_and_image_bytes_fail_offline(self):
        for path, replacement in [(self.archive_path, gzip.compress(b"altered")), (self.image_path, b"altered")]:
            with self.subTest(path=path):
                original = (self.root / path).read_bytes()
                self.write(path, replacement)
                with self.assertRaisesRegex(ValueError, "hash mismatch"):
                    self.apply()
                self.write(path, original)
        (self.root / self.image_path).unlink()
        with self.assertRaisesRegex(ValueError, "Cannot read evidence"):
            self.apply()

    def test_origin_reparsing_and_preserved_raw_records_are_verified(self):
        m = copy.deepcopy(self.manifest)
        m["raw_printings"][0]["might"] = 9
        with self.assertRaisesRegex(ValueError, "raw printing hash"):
            self.apply(manifest=m)
        def changed_project(raw):
            p = project(raw)
            p[0]["might"] = 9
            return p
        with self.assertRaisesRegex(ValueError, "Origin printing hash mismatch"):
            review.apply_reviews([self.printing], root=self.root, project_gallery=changed_project,
                                 plain=plain, manifest=self.manifest)

    def test_evidence_paths_cannot_point_at_mutable_archives_or_escape_root(self):
        for path in ("/tmp/image.png", "../image.png", "assets/data/catalog_sources/gallery.html.gz"):
            with self.subTest(path=path):
                m = copy.deepcopy(self.manifest)
                m["resolutions"][0]["evidence"][0]["local_path"] = path
                with self.assertRaisesRegex(ValueError, "repository-relative"):
                    self.apply(manifest=m)
        outside = self.root / "outside.png"
        outside.write_bytes(self.image)
        link = self.root / "assets/data/catalog_evidence/images/link.png"
        link.symlink_to(outside)
        m = copy.deepcopy(self.manifest)
        m["resolutions"][0]["evidence"][0]["local_path"] = str(link.relative_to(self.root))
        with self.assertRaisesRegex(ValueError, "escapes repository"):
            self.apply(manifest=m)

    def test_reviewed_image_url_and_byte_count_must_match(self):
        for key, value, message in [("url", "https://cmsassets.rgpub.io/wrong.png", "image URL mismatch"),
                                    ("bytes", len(self.image) + 1, "byte count mismatch")]:
            m = copy.deepcopy(self.manifest)
            m["resolutions"][0]["evidence"][0][key] = value
            with self.assertRaisesRegex(ValueError, message):
                self.apply(manifest=m)

    def test_rule_derived_exclusion_requires_pinned_core_bytes_and_anchors(self):
        r = self.resolution("tags", [])
        r["basis"] = "rule_derived"
        self.manifest["resolutions"].append(r)
        with self.assertRaisesRegex(ValueError, "requires Core"):
            self.apply()
        pdf = b"%PDF-1.7\nreviewed Core bytes"
        path = "assets/data/catalog_evidence/rules/core.pdf"
        self.write(path, pdf)
        r["evidence"].append({"kind": "official_core_rules", "url": "https://cmsassets.rgpub.io/core.pdf",
                              "local_path": path, "sha256": sha(pdf), "bytes": len(pdf), "rules": ["133.8"]})
        self.assertEqual(len(self.apply()["resolutions"]), 3)
        self.write(path, b"altered")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            self.apply()

    def test_image_index_hash_values_and_duplicate_records_are_checked(self):
        index = [{"official_id": "rad-test", "url": self.printing["image_url"],
                  "sha256": sha(self.image), "bytes": len(self.image), "path": self.image_path}]
        path = "assets/data/catalog_evidence/images.json"
        raw = json.dumps(index).encode()
        self.write(path, raw)
        self.manifest.update(evidence_manifest_path=path, evidence_manifest_sha256=sha(raw))
        self.apply()
        self.write(path, b"[]")
        with self.assertRaisesRegex(ValueError, "image-index hash"):
            self.apply()
        index[0]["url"] = "https://cmsassets.rgpub.io/other.png"
        raw = json.dumps(index).encode()
        self.write(path, raw)
        self.manifest["evidence_manifest_sha256"] = sha(raw)
        with self.assertRaisesRegex(ValueError, "Image-index mismatch"):
            self.apply()

    def test_historical_issue_references_remain_valid_and_unmodified(self):
        issue = {"review_issue_id": "legacy-01", "raw_issue": {"kind": "conflicting_type_labels"},
                 "printing_ids": ["rad-test"], "resolution_ids": ["rad-test:types"]}
        self.manifest["historical_issues"] = [issue]
        self.assertEqual(self.apply()["historical_issues"], [issue])
        issue["resolution_ids"] = ["unknown"]
        with self.assertRaisesRegex(ValueError, "Unknown historical resolution"):
            self.apply()


class CommittedReviewIntegrationTests(unittest.TestCase):
    def test_all_reviewed_records_apply_against_real_parser_and_archived_evidence(self):
        import import_catalog as catalog
        _, data = catalog.read_archives()
        printings, _ = catalog.project_gallery(data["gallery"])
        before = copy.deepcopy(printings)
        result = review.apply_reviews(printings, root=catalog.ROOT,
                                      project_gallery=lambda raw: catalog.project_gallery(raw)[0], plain=catalog.plain)
        self.assertEqual(printings, before)
        self.assertEqual(len(result["resolutions"]), 115)
        self.assertEqual(len(result["historical_issues"]), 40)
        self.assertEqual(result["evidence_summary"]["reviewed_printings"], 71)
        self.assertEqual(len(result["printings"]), 1408)
        self.assertFalse(any(p["unresolved_gameplay_fields"] for p in result["printings"]))
        by_id = {p["official_id"]: p for p in result["printings"]}
        for p in result["printings"]:
            self.assertEqual({key: p[key] for key in review.PROJECTION_KEYS},
                             next(raw for raw in before if raw["official_id"] == p["official_id"]))
            self.assertNotIn("playable_in_client", p)
        self.assertEqual(by_id["rad-sp4-005"]["might"], 4)
        self.assertEqual(by_id["rad-sp4-005"]["effective"]["might"], 5)
        self.assertEqual(by_id["rad-063-167"]["effective"]["might"], 5)
        self.assertEqual(by_id["rad-012-167"]["effective"]["domains"], ["fury"])
        self.assertIn("[Fury Power]", by_id["rad-012-167"]["effective"]["rules_text"])
        self.assertIn("instead.disa", by_id["rad-127-167"]["rules_text"])
        self.assertNotIn("instead.disa", by_id["rad-127-167"]["effective"]["rules_text"])
        self.assertEqual(by_id["rad-123-167"]["effective"]["tags"], ["Equipment"])
        self.assertEqual(by_id["rad-009-167"]["effective"]["tags"], ["Bandle City", "Yordle", "Mech"])
        self.assertEqual(by_id["rad-120-167"]["effective"]["tags"], ["Piltover"])
        for pid in ("rad-044-167", "rad-087-167", "rad-108-167", "rad-136-167"):
            self.assertEqual(by_id[pid]["tags"], ["Device"])
            self.assertEqual(by_id[pid]["effective"]["tags"], [])
        self.assertFalse(result["evidence_summary"]["live_image_bytes_checked"])


if __name__ == "__main__":
    unittest.main()
