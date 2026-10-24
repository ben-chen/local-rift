#!/usr/bin/env python3
"""Rebuild/check the official card inventory with Python's standard library only.

No card in this inventory becomes playable by being imported. Runtime mappings
are read from the separately reviewed card, legend, and battlefield manifests.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from datetime import date
import gzip
import hashlib
import html
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import sys
import unicodedata
import urllib.request

from catalog_review import apply_reviews
from catalog_identity_review import apply_identity_reviews

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / "assets/data/catalog_sources"
MANIFEST = ROOT / "assets/data/catalog_sources.json"
OUTPUT = ROOT / "assets/data/full_catalog.json"
REPORT = ROOT / "docs/CARD_COVERAGE.md"
SOURCES = {
    "gallery": "https://playriftbound.com/en-us/card-gallery/",
    "hub": "https://playriftbound.com/en-us/rules-hub/",
    "radiance": "https://playriftbound.com/en-us/news/announcements/the-radiance-overview/",
    "origins_errata": "https://playriftbound.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/",
    "spiritforged_errata": "https://playriftbound.com/en-us/news/rules-and-releases/riftbound-spiritforged-errata/",
    "unleashed_errata": "https://playriftbound.com/en-us/news/rules-and-releases/unleashed-errata-updates/",
    "vendetta_errata": "https://playriftbound.com/en-us/news/announcements/vendetta-errata-updates/",
    "radiance_errata": "https://playriftbound.com/en-us/news/announcements/radiance-errata-updates/",
}
# Explicit reviewed release boundary. A future preview set must not silently
# become legal just because it appears in the gallery.
RELEASED_SETS = {"OGN": 298, "OGS": 24, "SFD": 221, "UNL": 219, "VEN": 166, "RAD": 167}
SET_ORDER = {s: i for i, s in enumerate(RELEASED_SETS)}
BAN_ALIASES = {"dreamingtree": "thedreamingtree", "masteryiwujubladesman": "wujubladesmanstarter"}
ERRATA_ALIASES = {
    "dragonsrage": "dragonsrageleesin",
    "highlander": "highlanderyi",
    "icathianrain": "icathianrainkaisa",
    "leblancdeceiver": "deceiver",
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def normalize(value):
    return re.sub(r"[^a-z0-9]", "", unicodedata.normalize("NFKD", value).lower())


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=False) + "\n").encode()


def load_legend_manifest():
    """Reviewed runtime identities, independent of tournament-list provenance."""
    return json.loads((ROOT / "assets/data/legends.json").read_text())["legends"]


def load_runtime_manifest():
    return json.loads((ROOT / "assets/data/decks.json").read_text())


def load_battlefield_manifest():
    """Reviewed field identities, independent of tournament defaults."""
    return json.loads((ROOT / "assets/data/battlefields.json").read_text())["battlefields"]


def sha(value):
    return hashlib.sha256(value).hexdigest()


def next_data(raw):
    match = re.search(rb'<script id="__NEXT_DATA__" type="application/json">(.*?)</script>', raw, re.S)
    require(match, "Official page no longer exposes expected __NEXT_DATA__; importer review required")
    return json.loads(match[1])["props"]["pageProps"]["page"]


def body(raw):
    return "\n".join(b.get("richText", {}).get("body", "") for b in next_data(raw)["blades"])


def plain(value):
    value = re.sub(r"<br\s*/?>|</(?:p|h[1-6]|li)>", "\n", value, flags=re.I)
    value = html.unescape(re.sub(r"<[^>]+>", "", value))
    value = re.sub(r":rb_energy_(\d+):", r"[\1 Energy]", value)
    value = re.sub(r":rb_rune_(\w+):", lambda m: "[Any Power]" if m[1] == "rainbow" else f"[{m[1].title()} Power]", value)
    value = value.replace(":rb_exhaust:", "[Exhaust]").replace(":rb_might:", "Might")
    return "\n".join(line.strip() for line in value.splitlines() if line.strip())


def gallery_cards(raw):
    candidates = [b["cards"]["items"] for b in next_data(raw)["blades"] if isinstance(b.get("cards"), dict) and "items" in b["cards"]]
    require(len(candidates) == 1, "Expected exactly one complete gallery card collection")
    cards = candidates[0]
    require(cards and all("id" in c and "publicCode" in c for c in cards), "Malformed gallery card collection")
    require(len({c["id"] for c in cards}) == len(cards), "Duplicate official printing IDs")
    return cards


def section(value, start, end):
    require(start in value and end in value, f"Rules Hub section missing: {start}")
    return value.split(start, 1)[1].split(end, 1)[0]


def parse_legality(raw):
    value = body(raw)
    standard = section(value, "Constructed Format Legality", "2v2 Constructed Legality")
    teams = section(value, "2v2 Constructed Legality", ">Core Rules<")
    require("All sets currently produced" in plain(standard), "Released-set legality policy changed; human review required")
    def names(text):
        values = [plain(s) for s in re.findall(r"<li\b[^>]*>(.*?)</li>", text, re.S)]
        require(values and len(values) == len(set(values)), "Empty or duplicate official ban list")
        return values
    updated = re.search(r"Last updated:\s*([^<]+)", standard)
    require(updated, "Missing legality update date")
    return {"source_url": SOURCES["hub"], "last_updated": html.unescape(updated[1]), "standard_banned_names": names(standard), "two_v_two_banned_names": names(teams)}


def parse_core_rules_url(raw):
    """Read the current named link, not an older PDF mentioned elsewhere."""
    class CoreLinks(HTMLParser):
        def __init__(self):
            super().__init__()
            self.heading = None
            self.active = False
            self.anchor = None
            self.links = []

        def handle_starttag(self, tag, attrs):
            if re.fullmatch(r"h[1-6]", tag):
                self.active = False
                self.heading = []
            if tag == "a" and self.active:
                self.anchor = (dict(attrs).get("href"), [])

        def handle_data(self, value):
            if self.heading is not None:
                self.heading.append(value)
            if self.anchor is not None:
                self.anchor[1].append(value)

        def handle_endtag(self, tag):
            if re.fullmatch(r"h[1-6]", tag) and self.heading is not None:
                self.active = normalize("".join(self.heading)) == "corerules"
                self.heading = None
            if tag == "a" and self.anchor is not None:
                url, label = self.anchor
                if normalize("".join(label)) == "corerules":
                    self.links.append(url)
                self.anchor = None

    parser = CoreLinks()
    parser.feed(body(raw))
    require(len(parser.links) == 1 and isinstance(parser.links[0], str),
            "Expected exactly one current Core Rules link in the Rules Hub")
    return parser.links[0]


def parse_errata(raw, key):
    pieces = re.split(r"<h[12]\b[^>]*>(.*?)</h[12]>", body(raw), flags=re.S)
    result = []
    for heading, content in zip(pieces[1::2], pieces[2::2]):
        content_plain = plain(content)
        if "[NEW TEXT]" not in content_plain:
            continue
        require("[OLD TEXT]" in content_plain, f"Missing old/new errata delimiter: {key}, {plain(heading)}")
        new = content_plain.split("[NEW TEXT]", 1)[1].split("[OLD TEXT]", 1)[0].replace("▲", "").strip()
        require(new, f"Empty erratum: {plain(heading)}")
        result.append({"article_heading": plain(heading), "text": new, "source_url": SOURCES[key]})
    require(result, f"No errata parsed from {key}")
    return result


def read_archives():
    manifest = json.loads(MANIFEST.read_text())
    require(set(manifest["sources"]) == set(SOURCES), "Source manifest does not cover approved sources")
    data = {}
    for key, spec in manifest["sources"].items():
        require(spec["url"] == SOURCES[key], f"Unexpected source URL: {key}")
        raw = gzip.decompress((ROOT / spec["archive"]).read_bytes())
        require(sha(raw) == spec["sha256"], f"Source archive hash mismatch: {key}")
        data[key] = raw
    return manifest, data


def fetch_sources():
    def fetch(item):
        key, url = item
        request = urllib.request.Request(url, headers={"User-Agent": "LocalRiftCatalog/1.0 (personal noncommercial card inventory)"})
        with urllib.request.urlopen(request, timeout=60) as response:
            return key, response.read()
    return dict(ThreadPoolExecutor(max_workers=4).map(fetch, SOURCES.items()))


def source_manifest(data, as_of):
    require(re.fullmatch(r"\d{4}-\d{2}-\d{2}", as_of), "--as-of must be YYYY-MM-DD")
    date.fromisoformat(as_of)
    require(set(data) == set(SOURCES), "Candidate source set differs from reviewed sources")
    manifest = {"schema_version": 1, "as_of": as_of, "released_sets_reviewed": RELEASED_SETS, "sources": {}}
    for key in SOURCES:
        require(isinstance(data[key], bytes), f"Candidate source must be bytes: {key}")
        path = ARCHIVE / f"{key}.html.gz"
        manifest["sources"][key] = {"url": SOURCES[key], "archive": str(path.relative_to(ROOT)), "sha256": sha(data[key]), "uncompressed_bytes": len(data[key])}
    return manifest


def archive_sources(data, as_of):
    manifest = source_manifest(data, as_of)
    payloads = {key: gzip.compress(data[key], mtime=0) for key in SOURCES}
    ARCHIVE.mkdir(parents=True, exist_ok=True)
    for key, payload in payloads.items():
        (ARCHIVE / f"{key}.html.gz").write_bytes(payload)
    MANIFEST.write_bytes(json_bytes(manifest))
    return manifest


def project_gallery(raw):
    """Version-1 normalized printing projection; raw evidence stays unchanged."""
    raw_cards = gallery_cards(raw)
    quality = []
    printings = []
    for raw in sorted(raw_cards, key=lambda c: c["id"]):
        name = raw["name"] + (", " + raw["subtitle"] if raw.get("subtitle") else "")
        types = [t["id"] for t in raw["cardType"].get("type", [])]
        super_types = [t["id"] for t in raw["cardType"].get("superType", [])]
        type_source = "cardType.type"
        structured_types = types.copy()
        label = re.match(r"Riftbound (Unit|Spell|Gear|Battlefield|Legend|Rune):", raw["cardImage"].get("accessibilityText", ""))
        accessibility_types = [label[1].lower()] if label else []
        type_labels_conflict = bool(types and label and label[1].lower() not in types)
        if type_labels_conflict:
            quality.append({"kind": "conflicting_type_labels", "official_ids": [raw["id"]], "name": name, "structured_types": structured_types, "accessibility_types": accessibility_types, "detail": "Official structured type contradicts the same record's accessibility label. Both are preserved; runtime mapping requires source review."})
        if not types and "token" not in super_types:
            require(label, f"Missing official card type without redundant official label: {raw['id']}")
            types = accessibility_types.copy()
            type_source = "cardImage.accessibilityText"
            quality.append({"kind": "missing_structured_type", "official_ids": [raw["id"]], "detail": "Type retained from the same official record's image accessibility label; structured type is empty."})
        rules_html = raw.get("text", {}).get("richText", {}).get("body", "")
        equip_html = raw.get("effect", {}).get("richText", {}).get("body", "")
        def stat(key):
            return raw.get(key, {}).get("value", {}).get("id")
        value = {"official_id": raw["id"], "public_code": raw["publicCode"], "name": name, "set": raw["set"]["value"]["id"], "collector_number": raw["collectorNumber"], "types": types, "type_source": type_source, "structured_types": structured_types, "accessibility_types": accessibility_types, "type_labels_conflict": type_labels_conflict, "super_types": super_types, "domains": [d["id"] for d in raw["domain"].get("values", [])], "energy": stat("energy"), "power": stat("power"), "might": stat("might"), "might_bonus": stat("mightBonus"), "rules_html": rules_html, "rules_text": plain(rules_html), "equipment_html": equip_html, "equipment_text": plain(equip_html), "tags": raw.get("tags", {}).get("tags", []), "rarity": raw["rarity"]["value"]["id"], "image_url": raw["cardImage"]["url"], "image_accessibility_text": raw["cardImage"].get("accessibilityText", ""), "illustrators": [a["label"] for a in raw["illustrator"]["values"]]}
        printings.append(value)
    return printings, quality


def preferred_printing(p):
    match = re.fullmatch(r"[A-Z]+-(\d+)/(\d+)", p["public_code"])
    is_base = match and int(match[1]) <= int(match[2])
    return (not bool(is_base), SET_ORDER[p["set"]], p["collector_number"], p["official_id"])


def canonical_json(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def raw_identity_conflicts(groups, quality):
    """Keep the original issue objects and ordering as historical source data."""
    result = {}
    for key, variants in sorted(groups.items(), key=lambda pair: pair[1][0]["name"].casefold()):
        variants.sort(key=preferred_printing)
        conflicts = ["types"] if any(p["type_labels_conflict"] for p in variants) else []
        if len({canonical_json(p["types"]) for p in variants}) > 1 and "types" not in conflicts:
            conflicts.append("types")
        for attr in ["energy", "power", "might", "might_bonus", "domains"]:
            if len({canonical_json(p[attr]) for p in variants}) > 1:
                conflicts.append(attr)
                quality.append({"kind": "conflicting_printing_stat", "name": variants[0]["name"], "field": attr, "values": [{"official_id": p["official_id"], "value": p[attr]} for p in variants], "detail": "Every source value is preserved. Preferred base printing is an inventory representative, not adjudication of the conflict."})
        result[key] = conflicts
    return result


def identity_source_review(variants, identity_review=None):
    unresolved = {field for p in variants for field in p["unresolved_gameplay_fields"]}
    for field in ("types", "super_types", "domains", "energy", "power", "might", "might_bonus"):
        values = [p["effective"][field] for p in variants]
        if field in ("types", "super_types", "domains"):
            values = [sorted(value) for value in values]
        if len({canonical_json(value) for value in values}) > 1:
            unresolved.add(field)
    diagnostics = []
    for field in ("tags", "rules_text", "equipment_text"):
        values = [sorted(set(p["effective"][field])) if field == "tags" else p["effective"][field] for p in variants]
        if len({canonical_json(value) for value in values}) > 1:
            diagnostics.append({"field": field, "status": "unreviewed_variant_difference", "values": [{"official_id": p["official_id"], "value": value} for p, value in zip(variants, values)]})
    resolutions = sorted({r for p in variants for r in p["resolution_ids"]})
    reviewed_fields = []
    if identity_review is not None:
        reviewed_fields = [entry["field"] for entry in identity_review["field_reviews"]]
        dispositions = {entry["field"]: entry["disposition"] for entry in identity_review["field_reviews"]}
        for diagnostic in diagnostics:
            diagnostic["status"] = "reviewed_variant_difference"
            diagnostic["disposition"] = dispositions[diagnostic["field"]]
    unreviewed = [] if identity_review is not None else [d["field"] for d in diagnostics]
    status = "unresolved_gameplay_fields" if unresolved else "unreviewed_variants" if unreviewed else "reviewed_variants" if identity_review else "reviewed_corrections" if resolutions else "no_detected_conflicts"
    result = {"status": status, "resolution_ids": resolutions, "unresolved_gameplay_fields": sorted(unresolved), "variant_diagnostics": diagnostics, "reviewed_variant_fields": reviewed_fields, "unresolved_variant_fields": unreviewed}
    if identity_review is not None:
        result["identity_review_id"] = identity_review["id"]
    return result


def require_mapping_source_ready(review, variants, mapping):
    source_id = mapping.get("official_id", variants[0]["official_id"])
    unresolved = review["unresolved_gameplay_fields"]
    if "types" in unresolved:
        raise ValueError(f"Runtime mapping has unresolved official type conflict: {source_id}")
    require(not unresolved, f"Runtime mapping has unresolved gameplay fields: {source_id}: {', '.join(unresolved)}")
    require(not review["unresolved_variant_fields"],
            f"Runtime mapping requires review of alternate-printing differences: {source_id}")


def make_catalog(manifest, data, *, review_manifest=None, identity_review_manifest=None):
    require(manifest["released_sets_reviewed"] == RELEASED_SETS, "Reviewed set boundary differs from importer")
    raw_cards = gallery_cards(data["gallery"])
    raw_by_id = {c["id"]: c for c in raw_cards}
    observed_sets = {c["set"]["value"]["id"] for c in raw_cards}
    require(observed_sets == set(RELEASED_SETS), f"Gallery set scope changed: {sorted(observed_sets)}; release/legality review required")
    overview = plain(body(data["radiance"]))
    require("October 23, 2026" in overview, "Radiance release date must be reviewed")
    require(manifest["as_of"] >= "2026-10-23", "Radiance is not released on requested as-of date")
    quality = []
    printings = []
    groups = defaultdict(list)
    set_counts = {}
    for set_id, total in RELEASED_SETS.items():
        members = [c for c in raw_cards if c["set"]["value"]["id"] == set_id]
        numbered = []
        for c in members:
            match = re.fullmatch(r"([A-Z]+)-(\d+)/(\d+)", c["publicCode"])
            if match and int(match[2]) <= int(match[3]):
                require(match[1] == set_id and int(match[3]) == total, f"Unexpected set denominator: {c['publicCode']}")
                numbered.append(int(match[2]))
        require(sorted(numbered) == list(range(1, total + 1)), f"Missing/duplicated numbered base cards for {set_id}")
        set_counts[set_id] = {"name": members[0]["set"]["value"]["label"], "numbered_base_slots": total, "numbered_base_slots_present": len(numbered), "gallery_printings": len(members)}
    printings, quality = project_gallery(data["gallery"])
    for printing in printings:
        groups[normalize(printing["name"])].append(printing)
    raw_conflicts = raw_identity_conflicts(groups, quality)
    reviewed = apply_reviews(printings, root=ROOT, project_gallery=lambda raw: project_gallery(raw)[0], plain=plain, manifest=review_manifest)
    printings = reviewed["printings"]
    groups = defaultdict(list)
    for printing in printings:
        groups[normalize(printing["name"])].append(printing)
    for variants in groups.values():
        variants.sort(key=preferred_printing)
    def resolve_name(name, aliases=BAN_ALIASES):
        key = aliases.get(normalize(name), normalize(name))
        require(key in groups, f"Official name does not resolve uniquely in gallery: {name}")
        return key
    legality = parse_legality(data["hub"])
    legality["core_rules_url"] = parse_core_rules_url(data["hub"])
    banned = {resolve_name(n) for n in legality["standard_banned_names"]}
    banned_teams = {resolve_name(n) for n in legality["two_v_two_banned_names"]}
    require(banned <= banned_teams, "Unexpected 2v2 ban relationship; review format policy")
    errata = defaultdict(list)
    for key in SOURCES:
        if key.endswith("_errata"):
            for entry in parse_errata(data[key], key):
                errata[resolve_name(entry["article_heading"], ERRATA_ALIASES)].append(entry)
    def parse_review_errata(raw, url):
        keys = [key for key, source in SOURCES.items() if key.endswith("_errata") and source == url]
        require(len(keys) == 1, f"Unapproved review errata source: {url}")
        return parse_errata(raw, keys[0])
    identities = apply_identity_reviews(
        {variants[0]["official_id"]: variants for variants in groups.values()},
        root=ROOT, project_gallery=lambda raw: project_gallery(raw)[0], normalize=normalize,
        resolutions=reviewed["resolutions"], current_core_rules_url=legality["core_rules_url"],
        errata_by_identity={variants[0]["official_id"]: errata.get(key, []) for key, variants in groups.items()},
        parse_errata=parse_review_errata, manifest=identity_review_manifest,
    )
    source_reviews = {key: identity_source_review(variants, identities["by_identity"].get(variants[0]["official_id"]))
                      for key, variants in groups.items()}
    runtime = load_runtime_manifest()
    fields = load_battlefield_manifest()
    mappings = {}
    def add_mapping(official_id, kind, runtime_id, expected_name=None):
        require(official_id in raw_by_id, f"Runtime card absent from official gallery: {official_id}")
        raw = raw_by_id[official_id]
        name = raw["name"] + (", " + raw["subtitle"] if raw.get("subtitle") else "")
        key = normalize(name)
        require(key not in banned, f"Runtime includes Standard banned card: {name}")
        if expected_name:
            require(normalize(expected_name) == key, f"Runtime official ID/name mismatch: {expected_name}")
        require(key not in mappings, f"Duplicate runtime mapping: {official_id}")
        mapping = {"status": kind, "runtime_id": runtime_id, "official_id": official_id, "playable_in_client": True}
        require_mapping_source_ready(source_reviews[key], groups[key], mapping)
        mappings[key] = mapping
    printing_by_id = {p["official_id"]: p for p in printings}
    for card in runtime["cards"]:
        add_mapping(card["official_id"], "implemented_main_card", card["id"], card["name"])
        effective = printing_by_id[card["official_id"]]["effective"]
        require(effective["types"] == [card["kind"].lower()], f"Runtime main-card kind mismatch: {card['official_id']}")
        for stat in ("energy", "power"):
            require((card.get(stat) or 0) == (effective[stat] or 0), f"Runtime main-card {stat} mismatch: {card['official_id']}")
        # Gear's runtime Might value is its attached-unit bonus, not a unit's
        # base stat. Some gallery records duplicate that bonus in both fields.
        might_field = "might" if card["kind"] == "Unit" else "might_bonus" if card["kind"] == "Gear" else None
        expected_might = (effective[might_field] or 0) if might_field else 0
        require((card.get("might") or 0) == expected_might, f"Runtime main-card Might/attachment bonus mismatch: {card['official_id']}")
        require({domain.lower() for domain in card["domains"]} == set(effective["domains"]), f"Runtime main-card domains mismatch: {card['official_id']}")
        tags = card.get("tags")
        require(isinstance(tags, list) and all(type(tag) is str for tag in tags)
                and len(tags) == len(set(tags)) and set(tags) == set(effective["tags"]),
                f"Runtime main-card tags mismatch: {card['official_id']}")
        # The readable runtime text also includes equipment panels and reviewed
        # errata/formatting. Compare its original HTML sources separately.
        require(card.get("official_text_html") == effective["rules_html"],
                f"Runtime main-card rules HTML mismatch: {card['official_id']}")
        require(card.get("equipment_effect_html") == effective["equipment_html"],
                f"Runtime main-card equipment HTML mismatch: {card['official_id']}")
    legends = load_legend_manifest()
    require([v["id"] for v in legends] == list(range(len(legends))), "Legend registry IDs must be unique, ordered, and contiguous")
    preset_legend_ids = {deck["legend_id"] for deck in runtime["decks"]}
    require(preset_legend_ids <= {v["official_id"] for v in legends}, "Preset legend absent from independent registry")
    for entry in legends:
        official_id = entry["official_id"]
        require(official_id in printing_by_id, f"Legend absent from official gallery: {official_id}")
        printing = printing_by_id[official_id]
        require(printing["effective"]["types"] == ["legend"], f"Legend registry maps a non-legend: {official_id}")
        require(entry["name"] == printing["name"], f"Legend source name mismatch: {official_id}")
        require(entry["text"] == printing["effective"]["rules_text"] and entry["rules_html"] == printing["effective"]["rules_html"], f"Legend source text mismatch: {official_id}")
        require(entry["art_url"] == printing["image_url"], f"Legend source artwork mismatch: {official_id}")
        require(entry["champion"] in printing["effective"]["tags"], f"Legend champion tag mismatch: {official_id}")
        require(len(entry["domains"]) == 2 and {d.lower() for d in entry["domains"]} == set(printing["effective"]["domains"]), f"Legend domain mismatch: {official_id}")
        chosen_id = entry["default_champion"]
        require(isinstance(chosen_id, int) and not isinstance(chosen_id, bool) and 0 <= chosen_id < len(runtime["cards"]), f"Legend default champion is not a runtime card: {official_id}")
        chosen = runtime["cards"][chosen_id]
        require(chosen["id"] == chosen_id and chosen["official_id"] == entry["default_champion_official_id"], f"Legend default champion ID mismatch: {official_id}")
        chosen_printing = printing_by_id[chosen["official_id"]]
        require(chosen_printing["effective"]["types"] == ["unit"] and "champion" in chosen_printing["effective"]["super_types"] and entry["champion"] in chosen_printing["effective"]["tags"], f"Legend default champion lacks matching champion identity: {official_id}")
        require(set(chosen_printing["effective"]["domains"]) <= set(printing["effective"]["domains"]), f"Legend default champion has incompatible domains: {official_id}")
        for deck in runtime["decks"]:
            if deck["legend_id"] == official_id:
                require(deck["chosen_champion"] == chosen_id and deck["domains"] == entry["domains"], f"Preset legend/default champion mismatch: {official_id}")
        status = "implemented_preset_legend" if official_id in preset_legend_ids else "implemented_legend"
        add_mapping(official_id, status, entry["id"], entry["name"])
    require([v["id"] for v in fields] == list(range(len(fields))), "Battlefield registry IDs must be unique, ordered, and contiguous")
    for field in fields:
        official_id = field["source_id"]
        require(official_id in printing_by_id, f"Battlefield absent from official gallery: {official_id}")
        printing = printing_by_id[official_id]
        require(printing["effective"]["types"] == ["battlefield"], f"Battlefield registry maps a non-battlefield: {official_id}")
        require(field["name"] == printing["name"], f"Battlefield source name mismatch: {official_id}")
        require(field["official_html"] == printing["effective"]["rules_html"], f"Battlefield source text mismatch: {official_id}")
        if "official_text" in field:
            require(field["official_text"] == printing["effective"]["rules_text"], f"Battlefield readable source text mismatch: {official_id}")
        require(field["image_url"] == printing["image_url"], f"Battlefield source artwork mismatch: {official_id}")
        add_mapping(official_id, "implemented_battlefield", field["id"], field["name"])
    cards = []
    for key, variants in sorted(groups.items(), key=lambda pair: pair[1][0]["name"].casefold()):
        canonical = variants[0]
        auxiliary = any("token" in p["effective"]["super_types"] for p in variants)
        require(all(("token" in p["effective"]["super_types"]) == auxiliary for p in variants), f"Token/card identity collision: {key}")
        source_kind = canonical["types"][0] if canonical["types"] else "tracker"
        kind = canonical["effective"]["types"][0] if canonical["effective"]["types"] else "tracker"
        implementation = mappings.get(key, {"status": "not_implemented", "playable_in_client": False})
        if kind == "rune" and not auxiliary:
            implementation = {"status": "implemented_resource_model", "playable_in_client": True, "domain": canonical["effective"]["domains"][0]}
            require_mapping_source_ready(source_reviews[key], variants, implementation)
        def keywords(effective):
            values = [p["effective"] if effective else p for p in variants]
            combined = "\n".join(p["rules_text"] + "\n" + p["equipment_text"] for p in values)
            return sorted(set(re.findall(r"\[([A-Za-z][A-Za-z -]*)\]", combined)) - {"Exhaust", "Any Power", "Body Power", "Calm Power", "Chaos Power", "Fury Power", "Mind Power", "Order Power"})
        cards.append({"identity": canonical["official_id"], "name": canonical["name"], "kind": kind, "source_kind": source_kind, "auxiliary": auxiliary, "preferred_printing": canonical["official_id"], "official_ids": [p["official_id"] for p in variants], "standard_legal": not auxiliary and key not in banned, "two_v_two_legal": not auxiliary and key not in banned_teams, "exclusion": "auxiliary_token_or_tracker" if auxiliary else "banned_in_constructed" if key in banned else None, "keywords_observed": keywords(False), "effective_keywords_observed": keywords(True), "errata": errata.get(key, []), "rules_text_status": "official_gallery_plus_separate_errata" if key in errata else "official_gallery_printing_text", "conflicting_fields": raw_conflicts[key], "source_review": source_reviews[key], "implementation": implementation})
    history = {canonical_json(item["raw_issue"]): item for item in reviewed["historical_issues"]}
    require(len(history) == len(reviewed["historical_issues"]), "Duplicate historical issue review")
    require(set(history) <= {canonical_json(issue) for issue in quality}, "Historical review issue no longer matches current raw source")
    resolved_ids = {r["id"] for r in reviewed["resolutions"]}
    issue_reviews = []
    for issue in quality:
        historical = history.get(canonical_json(issue))
        ids = issue.get("official_ids", [v["official_id"] for v in issue.get("values", [])])
        issue_field = issue.get("field", "types")
        resolutions = historical["resolution_ids"] if historical else []
        resolved = bool(historical and resolutions and set(resolutions) <= resolved_ids and all(issue_field not in source_reviews[normalize(printing_by_id[i]["name"])]["unresolved_gameplay_fields"] for i in ids))
        issue_reviews.append({"review_issue_id": historical["review_issue_id"] if historical else "source-" + sha(canonical_json(issue).encode())[:16], "official_ids": ids, "resolution_ids": resolutions, "status": "resolved_by_review" if resolved else "unresolved", "raw_issue": issue})
    by_kind = Counter(c["kind"] for c in cards if c["standard_legal"])
    implemented_kind = Counter(c["kind"] for c in cards if c["standard_legal"] and c["implementation"]["playable_in_client"])
    counts = {"gallery_printings": len(printings), "unique_gallery_identities": len(cards), "additional_gallery_printings": len(printings) - len(cards), "numbered_base_printings": sum(RELEASED_SETS.values()), "auxiliary_identities": sum(c["auxiliary"] for c in cards), "standard_banned_identities": len(banned), "standard_legal_identities": sum(c["standard_legal"] for c in cards), "two_v_two_legal_identities": sum(c["two_v_two_legal"] for c in cards), "standard_legal_by_kind": dict(sorted(by_kind.items())), "standard_legal_by_kind_provisional": any("types" in c["source_review"]["unresolved_gameplay_fields"] for c in cards), "unresolved_type_conflict_identities": sum("types" in c["source_review"]["unresolved_gameplay_fields"] for c in cards), "implemented_legal_by_kind": dict(sorted(implemented_kind.items())), "implemented_main_cards": len(runtime["cards"]), "implemented_legends": len(legends), "implemented_battlefields": len(fields), "implemented_legal_identities": sum(implemented_kind.values()), "unimplemented_legal_identities": sum(by_kind.values()) - sum(implemented_kind.values()), "errata_entries": sum(len(v) for v in errata.values()), "identities_with_errata": len(errata), "source_quality_issues": len(quality), "reviewed_field_resolutions": len(reviewed["resolutions"]), "resolved_source_issue_groups": sum(r["status"] == "resolved_by_review" for r in issue_reviews), "unresolved_source_issue_groups": sum(r["status"] == "unresolved" for r in issue_reviews), "unresolved_gameplay_identities": sum(bool(c["source_review"]["unresolved_gameplay_fields"]) for c in cards), "unreviewed_variant_identities": sum(bool(c["source_review"]["unresolved_variant_fields"]) for c in cards), "unreviewed_variants_by_field": dict(sorted(Counter(field for c in cards for field in c["source_review"]["unresolved_variant_fields"]).items())), "pinned_runtime_variant_identities": 0, "reviewed_variant_identities": len(identities["reviews"]), "reviewed_variant_printings": sum(len(review["member_ids"]) for review in identities["reviews"]), "field_reviewed_printings": len({r["official_id"] for r in reviewed["resolutions"]})}
    gap_keywords = Counter(keyword for c in cards if c["standard_legal"] and not c["implementation"]["playable_in_client"] for keyword in c["effective_keywords_observed"])
    return {"schema_version": 2, "as_of": manifest["as_of"], "scope": "English official gallery; released sets through Radiance; Standard/1v1 Constructed legality. Raw evidence and reviewed effective facts are separate; inventory data does not enable executable card support.", "sources": manifest["sources"], "legality": legality, "set_coverage": set_counts, "counts": counts, "unimplemented_by_observed_keyword": dict(sorted(gap_keywords.items())), "cards": cards, "printings": printings, "source_quality_issues": quality, "source_issue_reviews": issue_reviews, "reviewed_field_resolutions": reviewed["resolutions"], "review_evidence": reviewed["evidence_summary"], "reviewed_identity_reviews": identities["reviews"], "identity_review_evidence": identities["evidence_summary"]}


def report(catalog):
    c = catalog["counts"]
    lines = ["# Official card inventory and implementation coverage", "", f"Snapshot checked **{catalog['as_of']}**. The official gallery contains **{c['gallery_printings']:,} printing records** grouped into **{c['unique_gallery_identities']:,} distinct full-name identities**. After excluding **{c['auxiliary_identities']} auxiliary token/tracker identities** and **{c['standard_banned_identities']} banned identities**, **{c['standard_legal_identities']:,} identities are legal in Standard/1v1 Constructed**. The separate 2v2 list leaves **{c['two_v_two_legal_identities']:,}**.", "", f"**This client does not yet implement every legal card.** It currently maps **{c['implemented_main_cards']} main-deck cards, {c['implemented_legends']} independently registered legends, {c['implemented_battlefields']} battlefields, and 6 rune resource types** ({c['implemented_legal_identities']} legal identities). **{c['unimplemented_legal_identities']:,} legal identities remain unimplemented and are not playable.** Importing this snapshot does not register cards with the engine. Full inventory coverage is not full rules support.", "", "## Scope and authority", "", f"The [official Rules Hub]({SOURCES['hub']}) says all currently produced sets are legal; its Constructed lists were updated **{catalog['legality']['last_updated']}**. Its Core and Tournament Rules links are dated October 9, 2026. The [Radiance overview]({SOURCES['radiance']}) confirms its October 23, 2026 release. The reviewed release boundary is Origins, Proving Grounds, Spiritforged, Unleashed, Vendetta, and Radiance. Future gallery previews fail the importer until their release/legality is reviewed. This is the globally legal card pool; deck identity, signature, copy-limit, and other deckbuilding requirements still apply. The client supports 1v1, not the separately reported 2v2 format.", "", f"The inventory is extracted from the [official English card gallery]({SOURCES['gallery']}). A gameplay identity here means one full gallery name including subtitle; alternate art, signed, showcase, and cross-set reprints with the same name are grouped. All {c['additional_gallery_printings']} additional gallery printings remain individually available. This proves completeness against this published gallery and all numbered base-set positions; it does not claim an inventory of every physical foil/promo/language variant.", "", "| Set | Numbered base positions present | Gallery printings |", "| --- | ---: | ---: |"]
    for key, v in catalog["set_coverage"].items():
        lines.append(f"| {v['name']} ({key}) | {v['numbered_base_slots_present']}/{v['numbered_base_slots']} | {v['gallery_printings']} |")
    lines += ["", "Numbered positions include token slots where the publisher numbers them within a set (including Origins). Special, alternate, signed, rune, and token codes remain in the printing inventory even when they are outside these numbered ranges.", "", "## Runtime gap by card kind", "", "| Kind | Standard legal | Implemented mapping | Not implemented |", "| --- | ---: | ---: | ---: |"]
    if c["standard_legal_by_kind_provisional"]:
        lines[-2:-2] = [f"**The by-kind counts below are provisional:** {c['unresolved_type_conflict_identities']} identities retain unresolved type findings after the evidence review. The table uses the preferred effective type provisionally; total identities and legality counts are unaffected. None of these unresolved identities is mapped to the runtime.", ""]
    for kind, count in c["standard_legal_by_kind"].items():
        supported = c["implemented_legal_by_kind"].get(kind, 0)
        lines.append(f"| {kind.title()} | {count} | {supported} | {count-supported} |")
    lines += ["", "Implemented mapping means the existing engine/preset/field/resource implementation, not a claim that every interaction has been formally proven. See `docs/RULES.md` for engine limits. Every unimplemented identity has `implementation.status: not_implemented` and `playable_in_client: false`. Printed keywords are recorded per card and counted under `unimplemented_by_observed_keyword` as discovery hints only; shared keywords do not imply shared effect semantics or completed implementation. Each unimplemented card still requires its specific effect, targets, timing, costs, continuous/replacement effects, and interaction tests to be reviewed and implemented.", "", "## Ban lists", "", "Standard/1v1 bans from the Rules Hub:", ""]
    for name in catalog["legality"]["standard_banned_names"]:
        lines.append(f"- {name}")
    lines += ["", "2v2 additionally bans Master Yi, Wuju Bladesman (gallery identity **Wuju Bladesman, Starter**, `ogs-019-024`). The Rules Hub calls one banned field “Dreaming Tree”; the gallery calls it **The Dreaming Tree**. These two explicit name aliases are audited in the importer. Punctuation/case normalization handles the 2v2 list’s “Draven Vanquisher”. Bans apply to all printings of the identity.", "", "## Text provenance and source issues", "", f"The snapshot preserves printed rules HTML, readable rules, equipment text, exact official IDs, costs/stats, domains, tags, and source image URLs for every printing. Missing structured values stay JSON `null`; they are not invented. **{c['errata_entries']} errata entries affecting {c['identities_with_errata']} identities** are separately linked to their official article. Errata text preserves Riot’s `[C]`, `[A]`, `[M]`, and exhaustion notation. These patches are not automatically interpreted as executable effects or silently merged into printed text. Explicit errata aliases connect Dragon's Rage to its Lee Sin signature printing, Highlander to its Yi signature printing, Icathian Rain to its Kai'Sa signature printing, and Leblanc, Deceiver to the Deceiver legend.", ""]
    for key in SOURCES:
        if key.endswith("_errata"):
            lines.append(f"- [{key.replace('_', ' ').title()}]({SOURCES[key]})")
    issues = catalog["source_quality_issues"]
    missing = [v for v in issues if v["kind"] == "missing_structured_type"]
    type_conflicts = [v for v in issues if v["kind"] == "conflicting_type_labels"]
    printing_by_id = {p["official_id"]: p for p in catalog["printings"]}
    lines += ["", f"**{c['source_quality_issues']} historical issue groups are preserved; {c['resolved_source_issue_groups']} have evidence-backed resolutions and {c['unresolved_source_issue_groups']} remain unresolved.** The reviewed manifest contains {c['reviewed_field_resolutions']} field corrections or confirmations across {c['field_reviewed_printings']} printings. It preserves original gallery values, both type labels, original HTML, and the original issue records. The generated `effective` object applies only explicitly reviewed fields. This is a targeted review, not a complete visual audit of the gallery.", "", f"**{len(missing)} Radiance records have an empty structured type field.** Their raw normalized type still records the accessibility-label fallback. Matching card images corroborate the effective types; no structured source value was overwritten.", "", f"**{len(type_conflicts)} records have contradictory structured and accessibility type labels.** The table below preserves both claims beside the reviewed effective type. Corrections come from the matching printed artwork, not a blanket preference for accessibility labels. The by-kind coverage table uses effective types. Any still-unresolved gameplay field, including an alternate printing's costs, Might, domain or type, prevents runtime admission.", "", "| Official ID | Card | Structured type | Accessibility type | Effective type |", "| --- | --- | --- | --- | --- |"]
    for issue in type_conflicts:
        official_id = issue['official_ids'][0]
        effective_type = ', '.join(printing_by_id[official_id]['effective']['types'])
        lines.append(f"| `{official_id}` | {issue['name']} | {', '.join(issue['structured_types'])} | {', '.join(issue['accessibility_types'])} | {effective_type} |")
    for issue in issues:
        if issue["kind"] == "conflicting_printing_stat":
            values = "; ".join(f"`{v['official_id']}` = {v['value']}" for v in issue['values'])
            effective = "; ".join(f"`{v['official_id']}` = {printing_by_id[v['official_id']]['effective'][issue['field']]}" for v in issue['values'])
            lines += ["", f"**Historical official stat conflict:** {issue['name']}, `{issue['field']}`: {values}. The source values remain unchanged. Reviewed effective values: {effective}."]
    lines += ["", "The same reviewed evidence corrects missing/wrong domains, printed tags, Runeblade Anvil's activation power glyph, and Rousing Display's trailing editorial text. Four Device-tag exclusions are explicitly rule-derived and retain their Core Rules evidence. Every resolution names exact raw prerequisites, its immutable origin gallery, complete printing hash, and local official image evidence. A changed printing or corrupted evidence fails validation; publisher fixes require a deliberate review update. See [the review contract and evidence](CATALOG_REVIEW.md).", "", f"**Alternate-printing review remains incomplete:** {c['unreviewed_variant_identities']} identities have unreviewed differences ({c['unreviewed_variants_by_field'].get('tags', 0)} tag sets and {c['unreviewed_variants_by_field'].get('rules_text', 0)} rules-text variants, with overlap). Reminder omissions and possible gameplay differences are recorded without automatic normalization. The {c['reviewed_variant_identities']} already-playable affected identities now have complete individual reviews of all {c['reviewed_variant_printings']} printings, replacing their old compatibility exceptions. The [variant review](VARIANT_REVIEW.md) preserves genuine printed differences and identifies exact transcription corrections, reminder equivalence, and two narrowly justified wording/separator interpretations. Any mapping with an unreviewed variant difference is rejected. Changes to reviewed printings, corrections, membership, local evidence, current Core Rules link, or applicable errata require renewed review, even if a difference disappears.", "", "The gallery's auxiliary entries are incomplete as a complete token catalog: official rules/card text mentions additional tokens such as Tentacle, Mech, and Bomb. Tokens cannot be included in legal constructed decks and are excluded from the 1,079 denominator; a full engine still needs every rules-created token and marker. These source corrections enable no new playable cards.", "", "## Reproduce and validate", "", "No extra Python packages are needed:", "", "```sh", "python3 tools/import_catalog.py --check", "python3 -m unittest discover -s tools -p 'test_*.py'", "cargo test --test catalog_coverage --test catalog_review", "```", "", "The four tournament lists remain separate from the legend registry. The offline check verifies current source archives, immutable origin evidence, every reviewed image and rules PDF, printing prerequisites, all numbered base positions, legality/errata, and runtime source readiness. It regenerates raw/effective inventory and this report deterministically. No unsupported identity becomes playable through a resolution entry.", "", "```sh", "# Compare live official page data without changing the committed snapshot.", "python3 tools/import_catalog.py --check-live", "# Refresh after reviewing newly released sets/legality; adjust RELEASED_SETS when needed.", "python3 tools/import_catalog.py --refresh --as-of YYYY-MM-DD", "# Regenerate after an intentional reviewed mapping/source change.", "python3 tools/import_catalog.py --write", "```", "", "Current pages are archived under `assets/data/catalog_sources/`; immutable review evidence is retained separately under `assets/data/catalog_evidence/`. Schema 2 keeps every original printing field and adds `effective`, `resolution_ids`, and `unresolved_gameplay_fields`. `source_quality_issues` and `conflicting_fields` retain raw history; `source_issue_reviews` and `source_review` describe the current reviewed status. `--check-live` compares parsed page facts, bans, errata, and the current named Core Rules link, ignoring unrelated website markup. It verifies local evidence but does not re-download remote image or PDF bytes, so it is not a fresh visual review. Failed candidate validation replaces no committed sources or outputs.", "", "## Implemented main-card ID mapping", "", "| Runtime ID | Card | Official printing ID |", "| ---: | --- | --- |"]
    mapped = sorted((card for card in catalog["cards"] if card["implementation"]["status"] == "implemented_main_card"), key=lambda card: card["implementation"]["runtime_id"])
    for card in mapped:
        m = card["implementation"]
        lines.append(f"| {m['runtime_id']} | {card['name']} | `{m['official_id']}` |")
    lines += ["", "## Implemented legend ID mapping", "", "Registry IDs are independent of the four sourced tournament presets. The original four retain their existing implementation status for compatibility; additional legends use `implemented_legend`. These mappings identify reviewed runtime identities, not a proof of all interactions.", "", "| Legend ID | Legend | Official printing ID |", "| ---: | --- | --- |"]
    mapped_legends = sorted((card for card in catalog["cards"] if card["implementation"]["status"] in ("implemented_preset_legend", "implemented_legend")), key=lambda card: card["implementation"]["runtime_id"])
    for card in mapped_legends:
        m = card["implementation"]
        lines.append(f"| {m['runtime_id']} | {card['name']} | `{m['official_id']}` |")
    lines += ["", "Card names, rules, and images are Riot Games intellectual property. This noncommercial, unofficial project includes source references for verification and does not claim Riot affiliation.", ""]
    return "\n".join(lines)


def comparable(catalog):
    return {k: v for k, v in catalog.items() if k not in ("sources", "as_of")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--check", action="store_true", help="validate committed sources, snapshot and coverage report (default)")
    action.add_argument("--write", action="store_true", help="regenerate snapshot/report from archived sources")
    action.add_argument("--refresh", action="store_true", help="download official sources, validate, then archive and regenerate")
    action.add_argument("--check-live", action="store_true", help="compare current official inventory without modifying files")
    parser.add_argument("--as-of", help="required ISO retrieval date for --refresh")
    args = parser.parse_args()
    if args.refresh:
        require(args.as_of, "--refresh requires an explicitly reviewed --as-of date")
        data = fetch_sources()
        manifest = source_manifest(data, args.as_of)
    else:
        manifest, data = read_archives()
    catalog = make_catalog(manifest, data)
    # Validate and render every candidate artifact before changing any current
    # archive or output. Immutable origin evidence is never replaced here.
    catalog_payload = json_bytes(catalog)
    report_payload = report(catalog)
    if args.check_live:
        require(OUTPUT.read_bytes() == catalog_payload and REPORT.read_text() == report_payload,
                "Committed inventory/report is stale; validate or regenerate before checking live sources")
        live = make_catalog(manifest, fetch_sources())
        require(comparable(live) == comparable(catalog), "Official inventory/legality/errata changed. Review differences, then refresh the snapshot.")
    elif args.write or args.refresh:
        if args.refresh:
            archive_sources(data, args.as_of)
        OUTPUT.write_bytes(catalog_payload)
        REPORT.write_text(report_payload)
    else:
        require(OUTPUT.read_bytes() == catalog_payload, "full_catalog.json differs from validated official sources/runtime mappings; run --write after reviewing the change")
        require(REPORT.read_text() == report_payload, "CARD_COVERAGE.md differs from generated coverage; run --write")
    print(json.dumps(catalog["counts"], indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as error:
        print(f"catalog validation failed: {error}", file=sys.stderr)
        sys.exit(1)
