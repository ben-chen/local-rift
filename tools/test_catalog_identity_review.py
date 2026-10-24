"""Adversarial complete-identity applicability/evidence tests."""
import copy
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import catalog_identity_review as identity_review
import catalog_review as review


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def project(raw):
    return json.loads(raw)["printings"]


def parse_errata(raw, url):
    return json.loads(raw)["entries"]


class IdentityReviewTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.core_url = "https://cmsassets.rgpub.io/sanity/files/test/core.pdf"
        self.raw = []
        for index, rules in enumerate(("[Assault] (+1 Might while attacking.)", "[Assault]")):
            self.raw.append({
                "official_id": f"test-{index}", "public_code": f"RAD-00{index + 1}/167",
                "name": "Review Poro", "set": "RAD", "collector_number": index + 1,
                "types": ["unit"], "type_source": "cardType.type", "structured_types": ["unit"],
                "accessibility_types": ["unit"], "type_labels_conflict": False,
                "super_types": [], "domains": ["fury"], "energy": 2, "power": 0,
                "might": 2, "might_bonus": None, "rules_html": f"<p>{rules}</p>", "rules_text": rules,
                "equipment_html": "", "equipment_text": "", "tags": ["Poro"], "rarity": "common",
                "image_url": f"https://cmsassets.rgpub.io/sanity/images/test-{index}.png",
                "image_accessibility_text": "Poro", "illustrators": ["Artist"],
            })
        self.groups = {"test-0": [self.annotate(p) for p in self.raw]}
        self.resolutions = []
        self.errata = {}
        origin = json.dumps({"website_build": "old", "printings": self.raw}).encode()
        digest = sha(origin)
        origin_path = f"assets/data/catalog_evidence/pages/{digest}.html.gz"
        self.write(origin_path, gzip.compress(origin, mtime=0))
        self.images = []
        for p in self.raw:
            raw = b"\x89PNG\r\n\x1a\n" + p["official_id"].encode()
            path = f"assets/data/catalog_evidence/identity_images/{p['official_id']}.png"
            self.write(path, raw)
            self.images.append({"official_id": p["official_id"], "url": p["image_url"], "local_path": path,
                                "sha256": sha(raw), "bytes": len(raw)})
        core = b"%PDF-1.7\nreviewed exact Core document"
        core_path = "assets/data/catalog_evidence/rules/core.pdf"
        self.write(core_path, core)
        self.manifest = {
            "schema_version": 1, "status": "reviewed", "printing_projection_version": 1,
            "printing_hash_encoding": review.HASH_ENCODING,
            "origin_archives": {digest: {"url": "https://playriftbound.com/en-us/card-gallery/",
                                         "archive": origin_path, "sha256": digest, "uncompressed_bytes": len(origin)}},
            "authorities": {"core": {"kind": "official_core_rules", "url": self.core_url, "local_path": core_path,
                                     "sha256": sha(core), "bytes": len(core)}},
            "reviews": [{"id": "review-poro", "identity": "test-0", "name": "Review Poro",
                         "reviewed_on": "2026-10-24", "status": "complete", "enables_runtime_support": False,
                         "core_authority_id": "core", "members": [], "fields": [{
                             "field": "rules_text", "disposition": "reminder_equivalent",
                             "reason": "These exact two inspected faces differ only in italic reminder text.",
                             "rules": ["135.2.c.2", "135.2.d.3"], "resolution_ids": []}], "errata": []}],
        }
        for p in self.groups["test-0"]:
            self.entry["members"].append(self.member(p, digest))
        self.sync_images()

    @property
    def entry(self):
        return self.manifest["reviews"][0]

    def annotate(self, raw):
        return {**copy.deepcopy(raw), "effective": {k: copy.deepcopy(raw[k]) for k in review.GAMEPLAY_FIELDS},
                "resolution_ids": [], "unresolved_gameplay_fields": []}

    def member(self, p, digest):
        raw = {k: p[k] for k in review.PROJECTION_KEYS}
        return {"official_id": p["official_id"], "raw_sha256": review.printing_sha256(raw),
                "effective_sha256": review.printing_sha256(raw | p["effective"]),
                "gallery_uncompressed_sha256": digest, "resolution_sha256": {}, "image_id": p["official_id"]}

    def write(self, relative, raw):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)

    def sync_images(self):
        raw = review.canonical_bytes(self.images)
        path = "assets/data/catalog_evidence/identity_images.json"
        self.write(path, raw)
        self.manifest["image_index"] = {"local_path": path, "sha256": sha(raw), "bytes": len(raw)}

    def repin_origin(self):
        raw = [{k: p[k] for k in review.PROJECTION_KEYS} for p in self.groups["test-0"]]
        origin = review.canonical_bytes({"printings": raw})
        digest = sha(origin)
        path = f"assets/data/catalog_evidence/pages/{digest}.html.gz"
        self.write(path, gzip.compress(origin, mtime=0))
        self.manifest["origin_archives"] = {digest: {
            "url": "https://playriftbound.com/en-us/card-gallery/", "archive": path,
            "sha256": digest, "uncompressed_bytes": len(origin)}}
        for member, p in zip(self.entry["members"], self.groups["test-0"]):
            member.update(self.member(p, digest))
        return digest

    def add_resolution(self, index, field, value, *, reason="Image correction"):
        p = self.groups["test-0"][index]
        rid = p["official_id"] + ":" + field
        resolution = {"id": rid, "official_id": p["official_id"], "field": field, "operation": "confirm" if review.typed_equal(p[field], value) else "override",
                      "effective_value": copy.deepcopy(value), "reason": reason,
                      "evidence": [{"kind": "official_card_image", **self.images[index]}]}
        self.resolutions.append(resolution)
        p["effective"][field] = copy.deepcopy(value)
        p["resolution_ids"].append(rid)
        member = self.entry["members"][index]
        member["resolution_sha256"][rid] = sha(review.canonical_bytes(resolution))
        member["effective_sha256"] = review.printing_sha256({k: p[k] for k in review.PROJECTION_KEYS} | p["effective"])
        return rid

    def add_erratum(self):
        url = "https://playriftbound.com/en-us/news/rules-and-releases/test-errata/"
        record = {"article_heading": "Review Poro", "text": "Updated current operative text.", "source_url": url}
        raw = review.canonical_bytes({"entries": [record]})
        path = "assets/data/catalog_evidence/pages/errata.html.gz"
        self.write(path, gzip.compress(raw, mtime=0))
        self.manifest["authorities"]["errata"] = {
            "kind": "official_errata", "url": url, "archive": path,
            "sha256": sha(raw), "uncompressed_bytes": len(raw)}
        self.errata["test-0"] = [record]
        self.entry["errata"] = [{"authority_id": "errata", "record": copy.deepcopy(record),
                                 "record_sha256": sha(review.canonical_bytes(record)), "location": "Review Poro NEW TEXT"}]
        return record

    def apply(self, **overrides):
        args = {"root": self.root, "project_gallery": project, "normalize": str.casefold,
                "resolutions": self.resolutions, "current_core_rules_url": self.core_url,
                "errata_by_identity": self.errata, "parse_errata": parse_errata, "manifest": self.manifest}
        args.update(overrides)
        return identity_review.apply_identity_reviews(self.groups, **args)

    def test_complete_review_preserves_distinct_text_and_all_inputs(self):
        before = copy.deepcopy((self.groups, self.manifest, self.resolutions))
        result = self.apply()
        summary = result["by_identity"]["test-0"]
        self.assertEqual(summary["reviewed_variant_fields"], ["rules_text"])
        self.assertEqual(summary["member_ids"], ["test-0", "test-1"])
        self.assertEqual(result["evidence_summary"]["reviewed_printings"], 2)
        self.assertNotEqual(self.groups["test-0"][0]["effective"]["rules_text"], self.groups["test-0"][1]["effective"]["rules_text"])
        result["reviews"][0]["field_reviews"][0]["reason"] = "mutated result"
        self.assertEqual(before, (self.groups, self.manifest, self.resolutions))
        self.assertFalse(result["evidence_summary"]["live_image_bytes_checked"])

    def test_exact_rule_derived_case_does_not_need_equal_printed_text(self):
        self.entry["fields"][0].update(disposition="rule_derived_equivalent", rules=["808.1.b", "808.1.c"])
        self.assertEqual(self.apply()["by_identity"]["test-0"]["field_reviews"][0]["disposition"], "rule_derived_equivalent")

    def test_corrected_tags_remain_in_union_after_effective_difference_disappears(self):
        self.groups["test-0"][1]["tags"] = []
        self.groups["test-0"][1]["effective"]["tags"] = []
        self.repin_origin()
        rid = self.add_resolution(1, "tags", ["Poro"])
        self.entry["fields"].append({"field": "tags", "disposition": "corrected_transcription", "reason": "Printed tag observed.", "rules": [], "resolution_ids": [rid]})
        result = self.apply()["by_identity"]["test-0"]
        self.assertEqual(result["raw_variant_fields"], ["tags", "rules_text"])
        self.assertEqual(result["effective_variant_fields"], ["rules_text"])
        self.entry["fields"].pop()
        with self.assertRaisesRegex(ValueError, "all raw/effective"):
            self.apply()

    def test_semantic_review_can_retain_separate_punctuation_corrections(self):
        first = self.add_resolution(1, "rules_text", "[Assault].")
        second = self.add_resolution(1, "rules_html", "<p>[Assault].</p>")
        self.entry["fields"][0]["resolution_ids"] = [first, second]
        self.apply()
        self.entry["fields"][0]["resolution_ids"].pop()
        with self.assertRaisesRegex(ValueError, "resolution membership"):
            self.apply()

    def test_every_raw_field_is_pinned_even_when_differences_disappear(self):
        for field, value in (("rarity", "rare"), ("rules_text", self.raw[0]["rules_text"]),
                             ("image_accessibility_text", "changed title")):
            with self.subTest(field=field):
                old = self.groups["test-0"][1][field]
                self.groups["test-0"][1][field] = value
                with self.assertRaisesRegex(ValueError, "Stale raw"):
                    self.apply()
                self.groups["test-0"][1][field] = old

    def test_effective_text_and_resolution_evidence_pin_changes_reject(self):
        rid = self.add_resolution(1, "tags", ["Poro"])
        self.apply()  # Same value can be confirmed by an already validated field layer.
        self.resolutions[0]["reason"] = "Changed review basis but same result"
        with self.assertRaisesRegex(ValueError, "field-resolution evidence"):
            self.apply()
        self.resolutions[0]["reason"] = "Image correction"
        self.groups["test-0"][1]["effective"]["rules_text"] = self.raw[0]["rules_text"]
        with self.assertRaisesRegex(ValueError, "Stale effective"):
            self.apply()
        self.assertIn(rid, self.entry["members"][1]["resolution_sha256"])

    def test_missing_or_new_member_is_not_partial_approval(self):
        removed = self.entry["members"].pop()
        with self.assertRaisesRegex(ValueError, "Incomplete identity"):
            self.apply()
        self.entry["members"].append(removed)
        p = copy.deepcopy(self.groups["test-0"][0])
        p["official_id"] = "test-new"
        self.groups["test-0"].append(p)
        with self.assertRaisesRegex(ValueError, "Origin identity membership"):
            self.apply()

    def test_independently_reparsed_origin_must_match_even_after_current_pin_changes(self):
        p = self.groups["test-0"][1]
        p["rarity"] = "rare"
        self.entry["members"][1]["raw_sha256"] = review.printing_sha256({k: p[k] for k in review.PROJECTION_KEYS})
        self.entry["members"][1]["effective_sha256"] = review.printing_sha256({k: p[k] for k in review.PROJECTION_KEYS} | p["effective"])
        with self.assertRaisesRegex(ValueError, "Origin printing differs"):
            self.apply()

    def test_gallery_origin_cannot_be_attributed_to_an_unrelated_official_page(self):
        origin = next(iter(self.manifest["origin_archives"].values()))
        origin["url"] = "https://playriftbound.com/en-us/rules-hub/"
        with self.assertRaisesRegex(ValueError, "approved gallery URL"):
            self.apply()

    def test_website_markup_is_not_a_candidate_applicability_pin(self):
        candidate = json.dumps({"irrelevant_website_build": 999, "printings": self.raw}).encode()
        self.groups["test-0"] = [self.annotate(p) for p in project(candidate)]
        self.assertEqual(self.apply()["evidence_summary"]["reviewed_identities"], 1)

    def test_duplicate_reviews_members_fields_and_ids_reject(self):
        for key in ("reviews", "members", "fields"):
            with self.subTest(key=key):
                target = self.manifest[key] if key == "reviews" else self.entry[key]
                target.append(copy.deepcopy(target[0]))
                with self.assertRaisesRegex(ValueError, "[Dd]uplicate"):
                    self.apply()
                target.pop()

    def test_unknown_disposition_no_rules_or_blank_reason_reject(self):
        for key, value in (("disposition", "strip_parentheses"), ("rules", []), ("reason", " "),
                           ("rules", ["135.2.d.1–3"]), ("rules", ["805.1"])):
            with self.subTest(key=key, value=value):
                old = self.entry["fields"][0][key]
                self.entry["fields"][0][key] = value
                with self.assertRaises(ValueError):
                    self.apply()
                self.entry["fields"][0][key] = old

    def test_cannot_dispose_real_tag_disagreement_as_reminder(self):
        self.groups["test-0"][1]["tags"] = ["Invented"]
        self.groups["test-0"][1]["effective"]["tags"] = ["Invented"]
        self.repin_origin()
        self.entry["fields"].append({"field": "tags", "disposition": "reminder_equivalent", "reason": "Not authorized.",
                                     "rules": ["135.2.d.3"], "resolution_ids": []})
        with self.assertRaisesRegex(ValueError, "differing effective text"):
            self.apply()

    def test_current_core_url_and_local_core_bytes_are_both_pinned(self):
        with self.assertRaisesRegex(ValueError, "Current Core URL"):
            self.apply(current_core_rules_url=self.core_url + "?revised")
        self.write(self.manifest["authorities"]["core"]["local_path"], b"%PDF-changed")
        with self.assertRaisesRegex(ValueError, "Evidence hash mismatch"):
            self.apply()

    def test_non_pdf_core_is_rejected_even_with_matching_hash(self):
        raw = b"not a PDF"
        spec = self.manifest["authorities"]["core"]
        self.write(spec["local_path"], raw)
        spec.update(sha256=sha(raw), bytes=len(raw))
        with self.assertRaisesRegex(ValueError, "not a PDF"):
            self.apply()

    def test_image_index_exact_membership_and_matching_original_urls(self):
        self.images[0]["url"] = self.images[1]["url"]
        self.sync_images()
        with self.assertRaisesRegex(ValueError, "image URL mismatch"):
            self.apply()
        self.images[0]["url"] = self.raw[0]["image_url"]
        self.images.append({**self.images[0], "official_id": "unreviewed-extra"})
        self.sync_images()
        with self.assertRaisesRegex(ValueError, "exactly cover"):
            self.apply()

    def test_corrupt_image_and_index_bytes_reject(self):
        path = self.images[0]["local_path"]
        raw = (self.root / path).read_bytes()
        self.write(path, raw + b"changed")
        with self.assertRaisesRegex(ValueError, "Evidence hash mismatch"):
            self.apply()
        self.write(path, raw)
        self.write(self.manifest["image_index"]["local_path"], b"[]")
        with self.assertRaisesRegex(ValueError, "Evidence hash mismatch"):
            self.apply()

    def test_field_and_identity_reviews_cannot_use_contradictory_image_bytes(self):
        self.add_resolution(1, "tags", ["Poro"])
        self.apply()
        image = self.images[1]
        new_bytes = b"\x89PNG\r\n\x1a\nchanged face at same official URL"
        self.write(image["local_path"], new_bytes)
        image.update(sha256=sha(new_bytes), bytes=len(new_bytes))
        self.sync_images()  # Each layer is internally pinned, but they disagree.
        with self.assertRaisesRegex(ValueError, "field-resolution image evidence differ"):
            self.apply()

    def test_evidence_cannot_escape_root_or_use_nonofficial_url(self):
        original = self.images[0]["local_path"]
        for bad in ("/tmp/review.png", "assets/data/catalog_evidence/../secret.png"):
            self.images[0]["local_path"] = bad
            self.sync_images()
            with self.assertRaisesRegex(ValueError, "repository-relative"):
                self.apply()
        self.images[0]["local_path"] = original
        self.images[0]["url"] = "https://example.com/card.png"
        self.sync_images()
        with self.assertRaisesRegex(ValueError, "Unexpected official URL"):
            self.apply()

    def test_complete_errata_pin_does_not_replace_printed_text(self):
        self.add_erratum()
        self.apply()
        self.assertEqual(self.groups["test-0"][0]["effective"]["rules_text"], self.raw[0]["rules_text"])

    def test_new_changed_or_removed_current_errata_stale_review(self):
        record = self.add_erratum()
        for records in ([], [{**record, "text": "Publisher correction"}], [record, {**record, "article_heading": "Second entry"}]):
            with self.subTest(records=records):
                with self.assertRaisesRegex(ValueError, "applicable errata differ"):
                    self.apply(errata_by_identity={"test-0": records})

    def test_erratum_must_independently_exist_in_its_pinned_document(self):
        record = self.add_erratum()
        changed = {**record, "text": "Invented current text"}
        self.errata["test-0"] = [changed]
        self.entry["errata"][0].update(record=changed, record_sha256=sha(review.canonical_bytes(changed)))
        with self.assertRaisesRegex(ValueError, "pinned source"):
            self.apply()

    def test_missing_applicable_erratum_is_not_an_approval(self):
        self.add_erratum()
        self.entry["errata"] = []
        with self.assertRaisesRegex(ValueError, "applicable errata differ"):
            self.apply()

    def test_unresolved_or_conflicting_effective_facts_cannot_be_approved(self):
        self.groups["test-0"][0]["unresolved_gameplay_fields"] = ["domains"]
        with self.assertRaisesRegex(ValueError, "Unresolved member"):
            self.apply()
        self.groups["test-0"][0]["unresolved_gameplay_fields"] = []
        self.groups["test-0"][0]["effective"]["energy"] = 9
        with self.assertRaisesRegex(ValueError, "Unresolved effective scalar"):
            self.apply()

    def test_unknown_fields_versions_incomplete_status_and_runtime_enable_reject(self):
        for key, value in (("schema_version", True), ("status", "draft"), ("new_semantic_rule", "ignore all changes")):
            with self.subTest(key=key):
                m = copy.deepcopy(self.manifest)
                m[key] = value
                with self.assertRaises(ValueError):
                    self.apply(manifest=m)
        for key, value in (("status", "partial"), ("enables_runtime_support", True)):
            old = self.entry[key]
            self.entry[key] = value
            with self.assertRaises(ValueError):
                self.apply()
            self.entry[key] = old

    def test_equipment_text_is_not_an_unchecked_second_panel(self):
        p = self.groups["test-0"][1]
        p.update(equipment_text="Give +1 Might.", equipment_html="<p>Give +1 Might.</p>")
        p["effective"].update(equipment_text=p["equipment_text"], equipment_html=p["equipment_html"])
        self.repin_origin()
        with self.assertRaisesRegex(ValueError, "all raw/effective differing fields"):
            self.apply()

    def test_disappearance_does_not_skip_even_a_fully_repinned_review(self):
        p = self.groups["test-0"][1]
        p.update(rules_text=self.raw[0]["rules_text"], rules_html=self.raw[0]["rules_html"])
        p["effective"].update(rules_text=p["rules_text"], rules_html=p["rules_html"])
        self.repin_origin()
        with self.assertRaisesRegex(ValueError, "differing effective text"):
            self.apply()

    def test_empty_fixture_manifest_is_explicit_and_does_not_approve_groups(self):
        m = {"schema_version": 1, "status": "reviewed", "printing_projection_version": 1,
             "printing_hash_encoding": review.HASH_ENCODING, "origin_archives": {}, "authorities": {}, "reviews": []}
        result = self.apply(manifest=m)
        self.assertEqual(result["by_identity"], {})
        self.assertEqual(result["reviews"], [])


if __name__ == "__main__":
    unittest.main()
