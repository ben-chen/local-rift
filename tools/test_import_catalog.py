"""Offline regression tests for completeness, legality, and honest support flags."""
import copy
import importlib.util
import json
from pathlib import Path
import re
import unittest
from unittest.mock import patch

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
        self.assertEqual(counts["implemented_main_cards"], 290)
        self.assertEqual(counts["implemented_legends"], 8)
        self.assertEqual(counts["implemented_battlefields"], 11)
        self.assertEqual(counts["implemented_legal_identities"], 315)
        self.assertEqual(counts["unimplemented_legal_identities"], 764)
        for card in self.snapshot["cards"]:
            if card["implementation"]["status"] == "not_implemented":
                self.assertFalse(card["implementation"]["playable_in_client"])

    def test_empowered_board_batch_and_ambessa_use_clean_official_sources(self):
        runtime = json.loads((catalog.ROOT / "assets/data/decks.json").read_text())["cards"]
        ids = ["ven-014-166", "ven-043-166", "ven-030-166", "ven-093-166", "ven-070-166", "ven-122-166", "ven-032-166", "ven-050-166", "ven-001-166", "ven-018-166", "ven-130-166", "ven-047-166", "ven-057-166", "ven-128-166", "ven-078-166", "ven-046-166", "ven-021-166", "ven-136-166"]
        for index, official_id in enumerate(ids, 272):
            identity = next(c for c in self.snapshot["cards"] if c["identity"] == official_id)
            printing = next(p for p in self.snapshot["printings"] if p["official_id"] == official_id)
            self.assertEqual(identity["implementation"]["runtime_id"], index)
            self.assertTrue(identity["standard_legal"])
            self.assertEqual(identity["conflicting_fields"], [])
            self.assertEqual(identity["errata"], [])
            self.assertEqual(runtime[index]["official_id"], official_id)
            self.assertEqual(runtime[index]["official_text"], printing["rules_text"])
            self.assertEqual(runtime[index]["official_text_html"], printing["rules_html"])
            self.assertEqual(runtime[index]["tags"], printing["tags"])
        legend = catalog.load_legend_manifest()[7]
        self.assertEqual((legend["official_id"], legend["default_champion"]), ("ven-153-166", 289))
        self.assertEqual(legend["champion"], "Ambessa")
        self.assertEqual(legend["domains"], ["Body", "Order"])
        self.assertEqual(self.snapshot["counts"]["source_quality_issues"], 37)
        self.assertEqual(self.snapshot["counts"]["unresolved_type_conflict_identities"], 0)

    def test_next_play_batch_maps_exact_source_ids_and_errata(self):
        runtime = json.loads((catalog.ROOT / "assets/data/decks.json").read_text())["cards"]
        ids = ["ogn-021-298", "ogn-032-298", "sfd-078-221", "ogn-031-298", "ven-044-166", "ogs-001-024", "ven-010-166", "unl-052-219", "sfd-004-221", "ogn-129-298", "ogn-011-298", "sfd-029-221", "unl-146-219", "ven-055-166", "ven-045-166", "ven-098-166", "sfd-146-221", "sfd-141-221"]
        for index, official_id in enumerate(ids, 254):
            identity = next(c for c in self.snapshot["cards"] if c["identity"] == official_id)
            self.assertEqual(identity["implementation"]["runtime_id"], index)
            self.assertTrue(identity["standard_legal"])
            self.assertEqual(identity["conflicting_fields"], [])
            self.assertEqual(runtime[index]["official_id"], official_id)
        self.assertIn("the next card you play this turn costs", runtime[258]["official_text"])
        self.assertNotIn("the next card you play this turn costs", runtime[258]["official_text_html"])
        self.assertEqual(runtime[258]["errata_source"], catalog.SOURCES["vendetta_errata"])
        self.assertEqual(runtime[255]["errata_source"], catalog.SOURCES["origins_errata"])

    def test_battlefield_registry_preserves_sources_and_rejects_changed_facts(self):
        fields = catalog.load_battlefield_manifest()
        self.assertEqual([v["id"] for v in fields], list(range(11)))
        for field in fields:
            identity = next(c for c in self.snapshot["cards"] if c["identity"] == field["source_id"])
            self.assertEqual(identity["implementation"]["runtime_id"], field["id"])
            self.assertEqual(identity["implementation"]["status"], "implemented_battlefield")
            self.assertEqual(identity["conflicting_fields"], [])
        for key, value, expected in [("id", 10, "Battlefield registry IDs"), ("name", "Invented", "Battlefield source name"), ("source_id", "ogn-021-298", "maps a non-battlefield"), ("official_html", "Invented", "Battlefield source text"), ("official_text", "Invented", "Battlefield readable source"), ("image_url", "https://invalid.example", "Battlefield source artwork")]:
            with self.subTest(key=key):
                changed = copy.deepcopy(fields)
                changed[4][key] = value
                with patch.object(catalog, "load_battlefield_manifest", return_value=changed):
                    with self.assertRaisesRegex(ValueError, expected):
                        catalog.make_catalog(self.manifest, self.data)

    def test_activation_batch_preserves_raw_sources_and_explicit_corrections(self):
        runtime = json.loads((catalog.ROOT / "assets/data/decks.json").read_text())["cards"]
        ids = ["sfd-046-221", "ogn-184-298", "rad-027-167", "unl-160-219", "ven-054-166", "ven-087-166", "ven-075-166", "ven-077-166"]
        for index, official_id in enumerate(ids, 246):
            identity = next(c for c in self.snapshot["cards"] if c["identity"] == official_id)
            self.assertEqual(identity["implementation"]["runtime_id"], index)
            self.assertTrue(identity["standard_legal"])
            self.assertEqual(identity["conflicting_fields"], [])
        syren = next(c for c in self.snapshot["cards"] if c["identity"] == ids[1])
        self.assertEqual(runtime[247]["errata_source"], syren["errata"][0]["source_url"])
        self.assertIn("its base", runtime[247]["official_text"])
        self.assertIn("your base", runtime[247]["official_text_html"])
        poro = next(p for p in self.snapshot["printings"] if p["official_id"] == ids[3])
        self.assertIn("[1 Energy] Might Bird", poro["rules_text"])
        self.assertEqual(runtime[249]["text_correction"]["raw_gallery_text"], poro["rules_text"])
        self.assertEqual(runtime[249]["text_correction"]["core_rule"], "377.2.b")
        self.assertIn("two 1 Might Bird", runtime[249]["official_text"])
        self.assertEqual(runtime[249]["official_text_html"], poro["rules_html"])

    def test_independent_legend_registry_keeps_original_ids_and_adds_no_presets(self):
        legends = catalog.load_legend_manifest()
        self.assertEqual([v["id"] for v in legends], list(range(8)))
        self.assertEqual([v["default_champion"] for v in legends], [0, 22, 33, 49, 183, 175, 182, 289])
        for entry in legends:
            identity = next(c for c in self.snapshot["cards"] if entry["official_id"] in c["official_ids"])
            self.assertEqual(identity["implementation"]["runtime_id"], entry["id"])
            self.assertEqual(identity["implementation"]["status"], "implemented_preset_legend" if entry["id"] < 4 else "implemented_legend")
            self.assertTrue(identity["standard_legal"])
        presets = json.loads((catalog.ROOT / "assets/data/decks.json").read_text())["decks"]
        self.assertEqual(len(presets), 4)

    def test_legend_manifest_rejects_changed_source_facts_or_invalid_champion(self):
        mutations = [
            ("text", "Invented effect", "Legend source text mismatch"),
            ("domains", ["Calm", "Body"], "Legend domain mismatch"),
            ("champion", "Kennen", "Legend champion tag mismatch"),
            ("art_url", "https://example.invalid/art.png", "Legend source artwork mismatch"),
            ("default_champion", 999999, "Legend default champion is not a runtime card"),
            ("default_champion_official_id", "ogn-117-298", "Legend default champion ID mismatch"),
        ]
        for field, value, message in mutations:
            with self.subTest(field=field):
                entries = copy.deepcopy(catalog.load_legend_manifest())
                entries[4][field] = value
                with patch.object(catalog, "load_legend_manifest", return_value=entries):
                    with self.assertRaisesRegex(ValueError, message):
                        catalog.make_catalog(self.manifest, self.data)

    def test_legend_registry_rejects_wrong_identity_and_nonchampion_unit(self):
        entries = copy.deepcopy(catalog.load_legend_manifest())
        entries[4]["official_id"] = "ogn-027-298"
        with patch.object(catalog, "load_legend_manifest", return_value=entries):
            with self.assertRaisesRegex(ValueError, "Legend registry maps a non-legend"):
                catalog.make_catalog(self.manifest, self.data)
        entries = copy.deepcopy(catalog.load_legend_manifest())
        entries[4]["default_champion"] = 81
        runtime = json.loads((catalog.ROOT / "assets/data/decks.json").read_text())
        entries[4]["default_champion_official_id"] = runtime["cards"][81]["official_id"]
        with patch.object(catalog, "load_legend_manifest", return_value=entries):
            with self.assertRaisesRegex(ValueError, "lacks matching champion identity"):
                catalog.make_catalog(self.manifest, self.data)

    def test_legend_registry_rejects_missing_original_or_duplicate_runtime_ids(self):
        entries = copy.deepcopy(catalog.load_legend_manifest())
        entries[6]["id"] = 5
        with patch.object(catalog, "load_legend_manifest", return_value=entries):
            with self.assertRaisesRegex(ValueError, "Legend registry IDs"):
                catalog.make_catalog(self.manifest, self.data)
        entries = catalog.load_legend_manifest()[1:]
        entries = [{**entry, "id": i} for i, entry in enumerate(entries)]
        with patch.object(catalog, "load_legend_manifest", return_value=entries):
            with self.assertRaisesRegex(ValueError, "Preset legend absent"):
                catalog.make_catalog(self.manifest, self.data)

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

    def test_conflicting_type_labels_are_preserved_with_reviewed_effective_types(self):
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
        self.assertFalse(self.snapshot["counts"]["standard_legal_by_kind_provisional"])
        self.assertEqual(self.snapshot["counts"]["unresolved_type_conflict_identities"], 0)
        self.assertEqual(printing["effective"]["types"], ["spell"])
        self.assertEqual(identity["source_review"]["unresolved_gameplay_fields"], [])
        self.assertEqual(self.snapshot["counts"]["resolved_source_issue_groups"], 37)
        self.assertIn("reviewed effective type", catalog.report(self.snapshot))

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
        with self.assertRaisesRegex(ValueError, "Unresolved member gameplay facts"):
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
