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
import json
from pathlib import Path
import re
import sys
import unicodedata
import urllib.request

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


def archive_sources(data, as_of):
    require(re.fullmatch(r"\d{4}-\d{2}-\d{2}", as_of), "--as-of must be YYYY-MM-DD")
    date.fromisoformat(as_of)
    ARCHIVE.mkdir(parents=True, exist_ok=True)
    manifest = {"schema_version": 1, "as_of": as_of, "released_sets_reviewed": RELEASED_SETS, "sources": {}}
    for key in SOURCES:
        path = ARCHIVE / f"{key}.html.gz"
        path.write_bytes(gzip.compress(data[key], mtime=0))
        manifest["sources"][key] = {"url": SOURCES[key], "archive": str(path.relative_to(ROOT)), "sha256": sha(data[key]), "uncompressed_bytes": len(data[key])}
    MANIFEST.write_bytes(json_bytes(manifest))
    return manifest


def make_catalog(manifest, data):
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
        groups[normalize(name)].append(value)
    def resolve_name(name, aliases=BAN_ALIASES):
        key = aliases.get(normalize(name), normalize(name))
        require(key in groups, f"Official name does not resolve uniquely in gallery: {name}")
        return key
    legality = parse_legality(data["hub"])
    banned = {resolve_name(n) for n in legality["standard_banned_names"]}
    banned_teams = {resolve_name(n) for n in legality["two_v_two_banned_names"]}
    require(banned <= banned_teams, "Unexpected 2v2 ban relationship; review format policy")
    errata = defaultdict(list)
    for key in SOURCES:
        if key.endswith("_errata"):
            for entry in parse_errata(data[key], key):
                errata[resolve_name(entry["article_heading"], ERRATA_ALIASES)].append(entry)
    runtime = json.loads((ROOT / "assets/data/decks.json").read_text())
    fields = json.loads((ROOT / "assets/data/battlefields.json").read_text())["battlefields"]
    mappings = {}
    def add_mapping(official_id, kind, runtime_id, expected_name=None):
        require(official_id in raw_by_id, f"Runtime card absent from official gallery: {official_id}")
        raw = raw_by_id[official_id]
        name = raw["name"] + (", " + raw["subtitle"] if raw.get("subtitle") else "")
        key = normalize(name)
        require(key not in banned, f"Runtime includes Standard banned card: {name}")
        require(not any(p["type_labels_conflict"] for p in groups[key]), f"Runtime mapping has unresolved official type conflict: {official_id}")
        if expected_name:
            require(normalize(expected_name) == key, f"Runtime official ID/name mismatch: {expected_name}")
        require(key not in mappings, f"Duplicate runtime mapping: {official_id}")
        mappings[key] = {"status": kind, "runtime_id": runtime_id, "official_id": official_id, "playable_in_client": True}
    for card in runtime["cards"]:
        add_mapping(card["official_id"], "implemented_main_card", card["id"], card["name"])
    legends = load_legend_manifest()
    require([v["id"] for v in legends] == list(range(len(legends))), "Legend registry IDs must be unique, ordered, and contiguous")
    printing_by_id = {p["official_id"]: p for p in printings}
    preset_legend_ids = {deck["legend_id"] for deck in runtime["decks"]}
    require(preset_legend_ids <= {v["official_id"] for v in legends}, "Preset legend absent from independent registry")
    for entry in legends:
        official_id = entry["official_id"]
        require(official_id in printing_by_id, f"Legend absent from official gallery: {official_id}")
        printing = printing_by_id[official_id]
        require(printing["types"] == ["legend"], f"Legend registry maps a non-legend: {official_id}")
        require(entry["name"] == printing["name"], f"Legend source name mismatch: {official_id}")
        require(entry["text"] == printing["rules_text"] and entry["rules_html"] == printing["rules_html"], f"Legend source text mismatch: {official_id}")
        require(entry["art_url"] == printing["image_url"], f"Legend source artwork mismatch: {official_id}")
        require(entry["champion"] in printing["tags"], f"Legend champion tag mismatch: {official_id}")
        require(len(entry["domains"]) == 2 and {d.lower() for d in entry["domains"]} == set(printing["domains"]), f"Legend domain mismatch: {official_id}")
        chosen_id = entry["default_champion"]
        require(isinstance(chosen_id, int) and not isinstance(chosen_id, bool) and 0 <= chosen_id < len(runtime["cards"]), f"Legend default champion is not a runtime card: {official_id}")
        chosen = runtime["cards"][chosen_id]
        require(chosen["id"] == chosen_id and chosen["official_id"] == entry["default_champion_official_id"], f"Legend default champion ID mismatch: {official_id}")
        chosen_printing = printing_by_id[chosen["official_id"]]
        require(chosen_printing["types"] == ["unit"] and "champion" in chosen_printing["super_types"] and entry["champion"] in chosen_printing["tags"], f"Legend default champion lacks matching champion identity: {official_id}")
        require(set(chosen_printing["domains"]) <= set(printing["domains"]), f"Legend default champion has incompatible domains: {official_id}")
        for deck in runtime["decks"]:
            if deck["legend_id"] == official_id:
                require(deck["chosen_champion"] == chosen_id and deck["domains"] == entry["domains"], f"Preset legend/default champion mismatch: {official_id}")
        status = "implemented_preset_legend" if official_id in preset_legend_ids else "implemented_legend"
        add_mapping(official_id, status, entry["id"], entry["name"])
    for i, field in enumerate(fields):
        add_mapping(field["source_id"], "implemented_battlefield", i)
    cards = []
    for key, variants in sorted(groups.items(), key=lambda pair: pair[1][0]["name"].casefold()):
        def pref(p):
            match = re.fullmatch(r"[A-Z]+-(\d+)/(\d+)", p["public_code"])
            is_base = match and int(match[1]) <= int(match[2])
            return (not bool(is_base), SET_ORDER[p["set"]], p["collector_number"], p["official_id"])
        variants.sort(key=pref)
        canonical = variants[0]
        auxiliary = any("token" in p["super_types"] for p in variants)
        require(all(("token" in p["super_types"]) == auxiliary for p in variants), f"Token/card identity collision: {key}")
        type_set = {tuple(p["types"]) for p in variants}
        require(len(type_set) == 1, f"Conflicting card types require manual review: {key}")
        kind = canonical["types"][0] if canonical["types"] else "tracker"
        implementation = mappings.get(key, {"status": "not_implemented", "playable_in_client": False})
        if kind == "rune" and not auxiliary:
            require(not any(p["type_labels_conflict"] for p in variants), f"Runtime resource mapping has unresolved official type conflict: {key}")
            implementation = {"status": "implemented_resource_model", "playable_in_client": True, "domain": canonical["domains"][0]}
        conflicts = ["types"] if any(p["type_labels_conflict"] for p in variants) else []
        for attr in ["energy", "power", "might", "might_bonus", "domains"]:
            seen = {json.dumps(p[attr], sort_keys=True) for p in variants}
            if len(seen) > 1:
                conflicts.append(attr)
                quality.append({"kind": "conflicting_printing_stat", "name": canonical["name"], "field": attr, "values": [{"official_id": p["official_id"], "value": p[attr]} for p in variants], "detail": "Every source value is preserved. Preferred base printing is an inventory representative, not adjudication of the conflict."})
        combined_text = "\n".join(p["rules_text"] + "\n" + p["equipment_text"] for p in variants)
        keyword_names = sorted(set(re.findall(r"\[([A-Za-z][A-Za-z -]*)\]", combined_text)) - {"Exhaust", "Any Power", "Body Power", "Calm Power", "Chaos Power", "Fury Power", "Mind Power", "Order Power"})
        cards.append({"identity": canonical["official_id"], "name": canonical["name"], "kind": kind, "auxiliary": auxiliary, "preferred_printing": canonical["official_id"], "official_ids": [p["official_id"] for p in variants], "standard_legal": not auxiliary and key not in banned, "two_v_two_legal": not auxiliary and key not in banned_teams, "exclusion": "auxiliary_token_or_tracker" if auxiliary else "banned_in_constructed" if key in banned else None, "keywords_observed": keyword_names, "errata": errata.get(key, []), "rules_text_status": "official_gallery_plus_separate_errata" if key in errata else "official_gallery_printing_text", "conflicting_fields": conflicts, "implementation": implementation})
    by_kind = Counter(c["kind"] for c in cards if c["standard_legal"])
    implemented_kind = Counter(c["kind"] for c in cards if c["standard_legal"] and c["implementation"]["playable_in_client"])
    counts = {"gallery_printings": len(printings), "unique_gallery_identities": len(cards), "additional_gallery_printings": len(printings) - len(cards), "numbered_base_printings": sum(RELEASED_SETS.values()), "auxiliary_identities": sum(c["auxiliary"] for c in cards), "standard_banned_identities": len(banned), "standard_legal_identities": sum(c["standard_legal"] for c in cards), "two_v_two_legal_identities": sum(c["two_v_two_legal"] for c in cards), "standard_legal_by_kind": dict(sorted(by_kind.items())), "standard_legal_by_kind_provisional": any("types" in c["conflicting_fields"] for c in cards), "unresolved_type_conflict_identities": sum("types" in c["conflicting_fields"] for c in cards), "implemented_legal_by_kind": dict(sorted(implemented_kind.items())), "implemented_main_cards": len(runtime["cards"]), "implemented_legends": len(legends), "implemented_legal_identities": sum(implemented_kind.values()), "unimplemented_legal_identities": sum(by_kind.values()) - sum(implemented_kind.values()), "errata_entries": sum(len(v) for v in errata.values()), "identities_with_errata": len(errata), "source_quality_issues": len(quality)}
    gap_keywords = Counter(keyword for c in cards if c["standard_legal"] and not c["implementation"]["playable_in_client"] for keyword in c["keywords_observed"])
    return {"schema_version": 1, "as_of": manifest["as_of"], "scope": "English official gallery; released sets through Radiance; Standard/1v1 Constructed legality. This is inventory data, not executable card support.", "sources": manifest["sources"], "legality": legality, "set_coverage": set_counts, "counts": counts, "unimplemented_by_observed_keyword": dict(sorted(gap_keywords.items())), "cards": cards, "printings": printings, "source_quality_issues": quality}


def report(catalog):
    c = catalog["counts"]
    lines = ["# Official card inventory and implementation coverage", "", f"Snapshot checked **{catalog['as_of']}**. The official gallery contains **{c['gallery_printings']:,} printing records** grouped into **{c['unique_gallery_identities']:,} distinct full-name identities**. After excluding **{c['auxiliary_identities']} auxiliary token/tracker identities** and **{c['standard_banned_identities']} banned identities**, **{c['standard_legal_identities']:,} identities are legal in Standard/1v1 Constructed**. The separate 2v2 list leaves **{c['two_v_two_legal_identities']:,}**.", "", f"**This client does not yet implement every legal card.** It currently maps **{c['implemented_main_cards']} main-deck cards, {c['implemented_legends']} independently registered legends, 4 battlefields, and 6 rune resource types** ({c['implemented_legal_identities']} legal identities). **{c['unimplemented_legal_identities']:,} legal identities remain unimplemented and are not playable.** Importing this snapshot does not register cards with the engine. Full inventory coverage is not full rules support.", "", "## Scope and authority", "", f"The [official Rules Hub]({SOURCES['hub']}) says all currently produced sets are legal; its Constructed lists were updated **{catalog['legality']['last_updated']}**. Its Core and Tournament Rules links are dated October 9, 2026. The [Radiance overview]({SOURCES['radiance']}) confirms its October 23, 2026 release. The reviewed release boundary is Origins, Proving Grounds, Spiritforged, Unleashed, Vendetta, and Radiance. Future gallery previews fail the importer until their release/legality is reviewed. This is the globally legal card pool; deck identity, signature, copy-limit, and other deckbuilding requirements still apply. The client supports 1v1, not the separately reported 2v2 format.", "", f"The inventory is extracted from the [official English card gallery]({SOURCES['gallery']}). A gameplay identity here means one full gallery name including subtitle; alternate art, signed, showcase, and cross-set reprints with the same name are grouped. All {c['additional_gallery_printings']} additional gallery printings remain individually available. This proves completeness against this published gallery and all numbered base-set positions; it does not claim an inventory of every physical foil/promo/language variant.", "", "| Set | Numbered base positions present | Gallery printings |", "| --- | ---: | ---: |"]
    for key, v in catalog["set_coverage"].items():
        lines.append(f"| {v['name']} ({key}) | {v['numbered_base_slots_present']}/{v['numbered_base_slots']} | {v['gallery_printings']} |")
    lines += ["", "Numbered positions include token slots where the publisher numbers them within a set (including Origins). Special, alternate, signed, rune, and token codes remain in the printing inventory even when they are outside these numbered ranges.", "", "## Runtime gap by card kind", "", "| Kind | Standard legal | Implemented mapping | Not implemented |", "| --- | ---: | ---: | ---: |"]
    if c["standard_legal_by_kind_provisional"]:
        lines[-2:-2] = [f"**The by-kind counts below are provisional:** {c['unresolved_type_conflict_identities']} identities have contradictory official structured types and accessibility labels. The table retains structured types without adjudicating them; total identities and legality counts are unaffected. None of these unresolved identities is mapped to the runtime.", ""]
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
    lines += ["", f"**{len(missing)} Radiance records have an empty structured type field.** Their type is taken from the same record’s explicit official image accessibility label and marked `type_source: cardImage.accessibilityText`. The archived record preserves the missing field."]
    type_conflicts = [v for v in issues if v["kind"] == "conflicting_type_labels"]
    lines += ["", f"**{len(type_conflicts)} records have contradictory structured and accessibility type labels.** `structured_types`, `accessibility_types`, and `type_labels_conflict` preserve both claims. Each affected identity lists `types` under `conflicting_fields`; the importer rejects any runtime mapping to such an identity, even when another printing is selected. No source label is silently preferred as the correct answer.", "", "| Official ID | Card | Structured type | Accessibility type |", "| --- | --- | --- | --- |"]
    for issue in type_conflicts:
        lines.append(f"| `{issue['official_ids'][0]}` | {issue['name']} | {', '.join(issue['structured_types'])} | {', '.join(issue['accessibility_types'])} |")
    for issue in issues:
        if issue["kind"] == "conflicting_printing_stat":
            values = "; ".join(f"`{v['official_id']}` = {v['value']}" for v in issue["values"])
            lines += ["", f"**Unresolved official stat conflict:** {issue['name']}, `{issue['field']}`: {values}. Both values are retained. The numbered base card is the inventory representative, not a ruling that resolves this discrepancy."]
    lines += ["", "Gallery text may contain editorial errors (for example Rousing Display ends with `instead.disa`); source text is preserved rather than silently repaired. The gallery’s auxiliary entries are also incomplete as a complete token catalog: official rules/card text mentions additional tokens such as Tentacle, Mech, and Bomb. Tokens cannot be included in legal constructed decks and are excluded from the 1,079 denominator; a future full engine still needs every rules-created token and marker.", "", "## Reproduce and validate", "", "No extra Python packages are needed:", "", "```sh", "python3 tools/import_catalog.py --check", "python3 -m unittest discover -s tools -p 'test_*.py'", "cargo test --test catalog_coverage", "```", "", "The four sourced tournament lists remain separate from the legend registry; new legend availability does not create a new tournament preset. The offline check verifies every archived source SHA-256, reparses gallery and legality, checks every official printing ID and all numbered base positions, rejects unknown sets/unresolved bans or errata, remaps the runtime manifests by official ID, verifies each independent legend’s source text/domains/artwork and matching chosen champion, and requires the generated snapshot and report to match byte for byte. It fails on truncated sources, missing records, stale implementation mappings, mapped identities with unresolved type conflicts, or unreviewed set changes.", "", "```sh", "# Compare live official source data without changing the committed snapshot.", "python3 tools/import_catalog.py --check-live", "# Refresh after reviewing newly released sets/legality; adjust RELEASED_SETS when needed.", "python3 tools/import_catalog.py --refresh --as-of YYYY-MM-DD", "# Regenerate after an intentional runtime mapping change, without a network request.", "python3 tools/import_catalog.py --write", "```", "", "Source pages are archived as deterministic gzip files under `assets/data/catalog_sources/`; `assets/data/catalog_sources.json` records their official URLs, retrieval date, SHA-256 hashes, and reviewed release boundary. `--check-live` compares gameplay inventory, ban lists, and errata rather than unrelated website build markup. The snapshot is dated evidence; it cannot assert that a future ban list remains unchanged.", "", "## Implemented main-card ID mapping", "", "| Runtime ID | Card | Official printing ID |", "| ---: | --- | --- |"]
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
        proposed = {"as_of": args.as_of, "released_sets_reviewed": RELEASED_SETS, "sources": {k: {"url": url} for k, url in SOURCES.items()}}
        make_catalog(proposed, data)  # Validate before replacing any committed sources.
        manifest = archive_sources(data, args.as_of)
    else:
        manifest, data = read_archives()
    catalog = make_catalog(manifest, data)
    if args.check_live:
        live = make_catalog(manifest, fetch_sources())
        require(comparable(live) == comparable(catalog), "Official inventory/legality/errata changed. Review differences, then refresh the snapshot.")
    elif args.write or args.refresh:
        OUTPUT.write_bytes(json_bytes(catalog))
        REPORT.write_text(report(catalog))
    else:
        require(OUTPUT.read_bytes() == json_bytes(catalog), "full_catalog.json differs from validated official sources/runtime mappings; run --write after reviewing the change")
        require(REPORT.read_text() == report(catalog), "CARD_COVERAGE.md differs from generated coverage; run --write")
    print(json.dumps(catalog["counts"], indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as error:
        print(f"catalog validation failed: {error}", file=sys.stderr)
        sys.exit(1)
