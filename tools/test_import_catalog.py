"""Offline regression tests for completeness, legality, and honest support flags."""
import copy
import importlib.util
import json
from pathlib import Path
import re
import unittest

SPEC = importlib.util.spec_from_file_location("catalog", Path(__file__).with_name("import_catalog.py"))
catalog = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(catalog)


def edit_page(raw, change):
    pattern = rb'(<script id="__NEXT_DATA__" type="application/json">)(.*?)(</script>)'
    match = re.search(pattern, raw, re.S)
    document = json.loads(match[2])
    change(document["props"]["pageProps"]["page"])
    return raw[:match.start(2)] + json.dumps(document, ensure_ascii=False).encode() + raw[match.end(2):]


class OfficialInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.data = catalog.read_archives()
        cls.snapshot = catalog.make_catalog(cls.manifest, cls.data)

    def changed_gallery(self, change):
        data = self.data.copy()
        def update(page):
            records = next(b["cards"]["items"] for b in page["blades"] if isinstance(b.get("cards"), dict) and "items" in b["cards"])
            change(records)
        data["gallery"] = edit_page(data["gallery"], update)
        return data

    def test_exact_published_scope_and_runtime_gap(self):
        counts = self.snapshot["counts"]
        self.assertEqual((counts["gallery_printings"], counts["unique_gallery_identities"], counts["standard_legal_identities"]), (1408, 1103, 1079))
        self.assertEqual(counts["implemented_main_cards"], 230)
        self.assertEqual(counts["unimplemented_legal_identities"], 835)
        for card in self.snapshot["cards"]:
            if card["implementation"]["status"] == "not_implemented":
                self.assertFalse(card["implementation"]["playable_in_client"])

    def test_missing_base_slot_is_rejected(self):
        data = self.changed_gallery(lambda cards: cards.remove(next(c for c in cards if c["id"] == "rad-001-167")))
        with self.assertRaisesRegex(ValueError, "Missing/duplicated numbered base"):
            catalog.make_catalog(self.manifest, data)

    def test_duplicate_printing_is_rejected(self):
        data = self.changed_gallery(lambda cards: cards.append(copy.deepcopy(cards[0])))
        with self.assertRaisesRegex(ValueError, "Duplicate official printing IDs"):
            catalog.make_catalog(self.manifest, data)

    def test_unreleased_unknown_set_is_rejected(self):
        def add(cards):
            card = copy.deepcopy(cards[0])
            card["id"] = "future-001"
            card["set"]["value"]["id"] = "FUT"
            cards.append(card)
        with self.assertRaisesRegex(ValueError, "Gallery set scope changed"):
            catalog.make_catalog(self.manifest, self.changed_gallery(add))

    def test_unknown_ban_is_not_silently_ignored(self):
        data = self.data.copy()
        def update(page):
            for blade in page["blades"]:
                if "richText" in blade:
                    blade["richText"]["body"] = blade["richText"]["body"].replace("<li>Called Shot</li>", "<li>Unrecognized New Ban</li>")
        data["hub"] = edit_page(data["hub"], update)
        with self.assertRaisesRegex(ValueError, "Official name does not resolve"):
            catalog.make_catalog(self.manifest, data)

    def test_source_conflict_is_preserved_and_not_invented(self):
        card = next(c for c in self.snapshot["cards"] if c["name"] == "Kai'Sa, Rebel")
        self.assertEqual(card["conflicting_fields"], ["might"])
        values = {p["official_id"]: p["might"] for p in self.snapshot["printings"] if p["official_id"] in card["official_ids"]}
        self.assertEqual(values, {"rad-063-167": 5, "rad-sp4-005": 4})
        self.assertFalse(card["implementation"]["playable_in_client"])
        abandon = next(p for p in self.snapshot["printings"] if p["official_id"] == "unl-131-219")
        self.assertIsNone(abandon["power"])
        self.assertIsNone(abandon["might"])

    def test_conflicting_type_labels_are_preserved_and_counted_as_provisional(self):
        printing = next(p for p in self.snapshot["printings"] if p["official_id"] == "rad-059-167")
        self.assertEqual(printing["types"], ["unit"])
        self.assertEqual(printing["structured_types"], ["unit"])
        self.assertEqual(printing["accessibility_types"], ["spell"])
        self.assertEqual(printing["type_source"], "cardType.type")
        self.assertTrue(printing["type_labels_conflict"])
        identity = next(c for c in self.snapshot["cards"] if c["identity"] == "rad-059-167")
        self.assertIn("types", identity["conflicting_fields"])
        self.assertFalse(identity["implementation"]["playable_in_client"])
        issues = [i for i in self.snapshot["source_quality_issues"] if i["kind"] == "conflicting_type_labels"]
        self.assertEqual(len(issues), 23)
        self.assertTrue(all(i["official_ids"][0].startswith("rad-") for i in issues))
        self.assertEqual(self.snapshot["counts"]["source_quality_issues"], 37)
        self.assertTrue(self.snapshot["counts"]["standard_legal_by_kind_provisional"])
        self.assertEqual(self.snapshot["counts"]["unresolved_type_conflict_identities"], 23)
        self.assertIn("by-kind counts below are provisional", catalog.report(self.snapshot))

    def test_mapped_type_conflict_is_rejected(self):
        def conflict(cards):
            card = next(c for c in cards if c["id"] == "ogn-093-298")
            card["cardImage"]["accessibilityText"] = "Riftbound Unit: Smoke Screen."
        with self.assertRaisesRegex(ValueError, "Runtime mapping has unresolved official type conflict"):
            catalog.make_catalog(self.manifest, self.changed_gallery(conflict))

    def test_conflicting_alternate_printing_also_blocks_mapped_identity(self):
        def conflict(cards):
            base = next(c for c in cards if c["id"] == "sfd-057-221")
            alternate = next(c for c in cards if c["id"] != base["id"] and c["name"] == base["name"] and c.get("subtitle") == base.get("subtitle"))
            alternate["cardImage"]["accessibilityText"] = "Riftbound Spell: Irelia, Fervent."
        with self.assertRaisesRegex(ValueError, "Runtime mapping has unresolved official type conflict"):
            catalog.make_catalog(self.manifest, self.changed_gallery(conflict))

    def test_missing_type_requires_explicit_redundant_source(self):
        def remove_type(cards):
            card = next(c for c in cards if c["id"] == "rad-009-167")
            card["cardImage"]["accessibilityText"] = ""
        with self.assertRaisesRegex(ValueError, "Missing official card type"):
            catalog.make_catalog(self.manifest, self.changed_gallery(remove_type))

    def test_two_v_two_additional_ban_and_errata_aliases(self):
        yi = next(c for c in self.snapshot["cards"] if "ogs-019-024" in c["official_ids"])
        self.assertTrue(yi["standard_legal"])
        self.assertFalse(yi["two_v_two_legal"])
        for card_name in ["Dragon's Rage, Lee Sin", "Highlander, Yi", "Icathian Rain, Kai'Sa", "Deceiver"]:
            card = next(c for c in self.snapshot["cards"] if c["name"] == card_name)
            self.assertTrue(card["errata"], card_name)


if __name__ == "__main__":
    unittest.main()
