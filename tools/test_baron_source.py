"""Baron's runtime facts and auxiliary Pit are checked against archived Riot facts."""
import copy
import json
import unittest
from unittest.mock import patch
import import_catalog as catalog


class BaronSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.snapshot = json.loads(catalog.OUTPUT.read_text())
        cls.manifest, cls.data = catalog.read_archives()

    def test_reviewed_baron_and_token_are_distinct_from_legal_starting_choices(self):
        records = {c["identity"]: c for c in self.snapshot["cards"]}
        baron = records["unl-147-219"]
        self.assertEqual(baron["implementation"]["runtime_id"], 440)
        self.assertEqual(baron["implementation"]["status"], "implemented_main_card")
        self.assertEqual(set(baron["official_ids"]), {"unl-147-219", "unl-147a-219", "unl-238-219"})
        self.assertEqual(baron["source_review"]["status"], "reviewed_variants")
        pit = records["unl-t01"]
        self.assertTrue(pit["auxiliary"])
        self.assertFalse(pit["standard_legal"])
        self.assertEqual(pit["implementation"]["status"], "implemented_battlefield_token")
        self.assertEqual(pit["implementation"]["runtime_id"], 1000)
        self.assertTrue(pit["implementation"]["playable_in_client"])
        self.assertNotIn("unl-t01", {f["source_id"] for f in catalog.load_battlefield_manifest()})
        self.assertEqual(self.snapshot["counts"]["implemented_battlefields"], 17)
        self.assertEqual(self.snapshot["counts"]["implemented_legal_identities"], 484)

    def test_auxiliary_registry_rejects_invented_text_and_normal_fields(self):
        fields = catalog.load_battlefield_token_manifest()
        for key, value, error in [
            ("id", 17, "auxiliary range"),
            ("source_id", "ven-163-166", "token/starting-choice mismatch"),
            ("official_text", "Invented movement rule", "readable source text"),
            ("official_html", "Invented", "source text"),
            ("image_url", "https://invalid.example", "source artwork"),
        ]:
            with self.subTest(key=key):
                changed = copy.deepcopy(fields)
                changed[0][key] = value
                with patch.object(catalog, "load_battlefield_token_manifest", return_value=changed):
                    with self.assertRaisesRegex(ValueError, error):
                        catalog.make_catalog(self.manifest, self.data)


if __name__ == "__main__":
    unittest.main()
