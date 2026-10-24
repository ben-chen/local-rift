"""Offline, evidence-backed source corrections; never executable card support.

The complete normalized printing projection is the applicability boundary. The
origin page hash proves provenance, not equality with a future website build.
"""
from __future__ import annotations

import copy
from datetime import date
import gzip
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import urlparse


PROJECTION_VERSION = 1
PROJECTION_KEYS = frozenset({
    "official_id", "public_code", "name", "set", "collector_number", "types",
    "type_source", "structured_types", "accessibility_types", "type_labels_conflict",
    "super_types", "domains", "energy", "power", "might", "might_bonus",
    "rules_html", "rules_text", "equipment_html", "equipment_text", "tags",
    "rarity", "image_url", "image_accessibility_text", "illustrators",
})
GAMEPLAY_FIELDS = (
    "types", "super_types", "domains", "energy", "power", "might", "might_bonus",
    "tags", "rules_html", "rules_text", "equipment_html", "equipment_text",
)
HASH_ENCODING = "UTF-8 JSON sorted keys; ensure_ascii=false; separators=(',', ':')"
TYPES = {"unit", "spell", "gear", "legend", "battlefield", "rune"}
SUPER_TYPES = {"basic", "champion", "signature", "token"}
DOMAINS = {"fury", "calm", "mind", "body", "chaos", "order", "colorless"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def typed_equal(left, right):
    """JSON equality which does not equate bool with integer or float."""
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(typed_equal(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(typed_equal(a, b) for a, b in zip(left, right))
    return left == right


def canonical_bytes(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode()


def printing_sha256(printing):
    require(isinstance(printing, dict) and set(printing) == PROJECTION_KEYS,
            "Printing projection v1 has missing or unknown fields")
    return hashlib.sha256(canonical_bytes(printing)).hexdigest()


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def _load_json(raw):
    def invalid_constant(value):
        raise ValueError(f"Invalid JSON constant: {value}")
    return json.loads(raw, object_pairs_hook=_unique_object, parse_constant=invalid_constant)


def _sha(value, label):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value), f"Invalid SHA-256: {label}")
    return value


def _official_url(value, label, *, image=False):
    require(isinstance(value, str), f"Missing official URL: {label}")
    parsed = urlparse(value)
    allowed = {"cmsassets.rgpub.io"} if image else {"playriftbound.com", "cmsassets.rgpub.io"}
    require(parsed.scheme == "https" and parsed.hostname in allowed
            and parsed.username is None and parsed.password is None and parsed.port in (None, 443),
            f"Unexpected official URL: {label}")
    return value


def _path(root, relative, label):
    require(isinstance(relative, str) and relative, f"Missing evidence path: {label}")
    path = Path(relative)
    require(not path.is_absolute() and ".." not in path.parts
            and path.parts[:3] == ("assets", "data", "catalog_evidence"),
            f"Evidence path must be repository-relative under catalog_evidence: {label}")
    resolved = (root / path).resolve()
    require(resolved.is_relative_to((root / "assets/data/catalog_evidence").resolve())
            and resolved.is_relative_to(root.resolve()), f"Evidence path escapes repository: {label}")
    return resolved


def _checked_bytes(root, spec, label, *, origin=False):
    path_key, size_key = ("archive", "uncompressed_bytes") if origin else ("local_path", "bytes")
    path = _path(root, spec.get(path_key), label)
    try:
        raw = path.read_bytes()
        if origin:
            require(path.suffix == ".gz", f"Origin archive must be gzip: {label}")
            raw = gzip.decompress(raw)
    except (OSError, EOFError) as error:
        raise ValueError(f"Cannot read evidence {label}: {error}") from error
    expected = _sha(spec.get("sha256"), label)
    require(hashlib.sha256(raw).hexdigest() == expected, f"Evidence hash mismatch: {label}")
    require(type(spec.get(size_key)) is int and spec[size_key] >= 0 and len(raw) == spec[size_key],
            f"Evidence byte count mismatch: {label}")
    return raw


def _strings(value, label, allowed=None):
    require(isinstance(value, list) and all(type(v) is str and v for v in value), f"Invalid string array: {label}")
    require(len(value) == len(set(value)), f"Duplicate values: {label}")
    if allowed is not None:
        require(set(value) <= allowed, f"Unknown value: {label}")


def _validate_facts(facts, label):
    for field, allowed in (("types", TYPES), ("super_types", SUPER_TYPES), ("domains", DOMAINS), ("tags", None)):
        _strings(facts[field], f"{label}:{field}", allowed)
    require(len(facts["types"]) <= 1, f"Multiple card types: {label}")
    require("colorless" not in facts["domains"] or facts["domains"] == ["colorless"],
            f"Colorless mixed with colored domains: {label}")
    for field in ("energy", "power", "might", "might_bonus"):
        value = facts[field]
        require(value is None or (type(value) is int and value >= 0), f"Invalid statistic: {label}:{field}")
    for field in ("rules_html", "rules_text", "equipment_html", "equipment_text"):
        require(type(facts[field]) is str, f"Invalid text: {label}:{field}")


def _printing_map(printings, label):
    require(isinstance(printings, list), f"Expected printing list: {label}")
    by_id = {}
    for p in printings:
        printing_sha256(p)
        official_id = p["official_id"]
        require(type(official_id) is str and official_id, f"Missing printing ID: {label}")
        require(official_id not in by_id, f"Duplicate printing ID: {label}:{official_id}")
        _validate_facts(p, official_id)
        _strings(p["structured_types"], f"{official_id}:structured_types", TYPES)
        _strings(p["accessibility_types"], f"{official_id}:accessibility_types", TYPES)
        require(type(p["type_labels_conflict"]) is bool, f"Invalid conflict flag: {official_id}")
        by_id[official_id] = p
    return by_id


def apply_reviews(printings, *, root: Path, project_gallery, plain, manifest=None):
    """Return annotated copies after verifying every supplied resolution.

    An explicit empty manifest is supported for isolated parser tests. Normal
    use loads the committed reviewed manifest and verifies all evidence offline.
    """
    root = Path(root)
    from_disk = manifest is None
    if manifest is None:
        manifest = _load_json((root / "assets/data/catalog_resolutions.json").read_bytes())
    else:
        manifest = copy.deepcopy(manifest)
    require(isinstance(manifest, dict), "Invalid source resolution manifest")
    if from_disk:
        require(manifest.get("status") == "reviewed", "Source resolution manifest is not reviewed")
    require(type(manifest.get("schema_version")) is int and manifest["schema_version"] == 1,
            "Unknown source resolution schema version")
    require(type(manifest.get("printing_projection_version")) is int
            and manifest["printing_projection_version"] == PROJECTION_VERSION,
            "Unknown or missing printing projection version")
    resolutions = manifest.get("resolutions")
    require(isinstance(resolutions, list), "Missing resolutions list")
    if resolutions:
        require(manifest.get("status") == "reviewed", "Source resolution manifest is not reviewed")
    candidates = _printing_map(printings, "candidate")
    origins = {}
    origin_specs = manifest.get("origin_archives", {})
    require(isinstance(origin_specs, dict), "Invalid origin archive registry")
    for digest, spec in origin_specs.items():
        require(isinstance(spec, dict), f"Invalid origin archive: {digest}")
        require(_sha(digest, "origin key") == spec.get("sha256"), f"Origin archive key/hash mismatch: {digest}")
        _official_url(spec.get("url"), digest)
        raw = _checked_bytes(root, spec, digest, origin=True)
        origins[digest] = _printing_map(project_gallery(raw), f"origin {digest}")

    # Preserve and verify the reviewed raw records, including their old values.
    raw_printings = manifest.get("raw_printings", [])
    require(isinstance(raw_printings, list), "Invalid raw-printing evidence")
    preserved = {}
    for entry in raw_printings:
        require(isinstance(entry, dict), "Invalid raw-printing entry")
        p = {key: value for key, value in entry.items() if key != "review_draft_sha256"}
        digest = printing_sha256(p)
        require(digest == entry.get("review_draft_sha256"), f"Preserved raw printing hash mismatch: {p['official_id']}")
        require(p["official_id"] not in preserved, f"Duplicate preserved printing: {p['official_id']}")
        preserved[p["official_id"]] = p

    # An optional image index is provenance too; its values cannot contradict
    # the per-resolution evidence used below.
    image_index = None
    if "evidence_manifest_path" in manifest:
        index_path = _path(root, manifest["evidence_manifest_path"], "image index")
        raw = index_path.read_bytes()
        require(hashlib.sha256(raw).hexdigest() == _sha(manifest.get("evidence_manifest_sha256"), "image index"),
                "Evidence image-index hash mismatch")
        index = _load_json(raw)
        require(isinstance(index, list), "Invalid evidence image index")
        image_index = {}
        for entry in index:
            require(isinstance(entry, dict) and type(entry.get("official_id")) is str,
                    "Invalid evidence image-index entry")
            require(entry["official_id"] not in image_index, "Duplicate image-index printing")
            image_index[entry["official_id"]] = entry

    reviewed = {}
    seen_ids = set()
    checked_evidence = {}
    for r in resolutions:
        require(isinstance(r, dict), "Invalid source resolution entry")
        rid, pid, field = r.get("id"), r.get("official_id"), r.get("field")
        require(type(rid) is str and rid and rid not in seen_ids, f"Duplicate or invalid resolution ID: {rid}")
        seen_ids.add(rid)
        require(field in GAMEPLAY_FIELDS, f"Unknown gameplay resolution field: {rid}")
        require(type(pid) is str and pid in candidates, f"Resolution printing is absent: {rid}")
        require((pid, field) not in reviewed, f"Duplicate printing/field resolution: {rid}")
        require(r.get("enables_runtime_support") is False, f"Resolution may not enable runtime support: {rid}")
        require(r.get("basis") in ("confirmed_artwork", "rule_derived"), f"Unknown evidence basis: {rid}")
        require(r.get("operation") in ("override", "confirm"), f"Unknown resolution operation: {rid}")
        require(r.get("printing_hash_encoding") == HASH_ENCODING, f"Unknown printing hash encoding: {rid}")
        require(type(r.get("reviewed_on")) is str
                and re.fullmatch(r"\d{4}-\d{2}-\d{2}", r["reviewed_on"]), f"Invalid review date: {rid}")
        try:
            date.fromisoformat(r["reviewed_on"])
        except ValueError as error:
            raise ValueError(f"Invalid review date: {rid}") from error
        require(type(r.get("reason")) is str and r["reason"].strip(), f"Missing review reason: {rid}")
        origin_hash = _sha(r.get("gallery_uncompressed_sha256"), rid)
        require(origin_hash in origins, f"Missing immutable origin archive: {rid}")
        origin = origins[origin_hash].get(pid)
        require(origin is not None, f"Printing absent from reviewed origin: {rid}")
        candidate = candidates[pid]
        expected_hash = _sha(r.get("expected_printing_sha256"), rid)
        require(printing_sha256(origin) == expected_hash, f"Origin printing hash mismatch: {rid}")
        require(printing_sha256(candidate) == expected_hash, f"Stale candidate printing hash: {rid}")
        require(pid in preserved and typed_equal(preserved[pid], origin), f"Preserved raw printing differs from origin: {rid}")
        require(r.get("name") == origin["name"], f"Resolution name mismatch: {rid}")
        source_field = r.get("raw_source_field")
        allowed_source = {field, "structured_types"} if field == "types" else {field}
        require(type(source_field) is str and source_field in allowed_source, f"Invalid underlying source field: {rid}")
        require("expected_raw_value" in r and typed_equal(candidate[field], r["expected_raw_value"]),
                f"Raw field prerequisite mismatch: {rid}")
        require("expected_raw_source_value" in r
                and typed_equal(candidate[source_field], r["expected_raw_source_value"]),
                f"Underlying source prerequisite mismatch: {rid}")
        require("effective_value" in r, f"Missing effective value: {rid}")
        unchanged = typed_equal(r["expected_raw_value"], r["effective_value"])
        require(unchanged == (r["operation"] == "confirm"), f"Resolution operation/value mismatch: {rid}")
        facts = {key: copy.deepcopy(candidate[key]) for key in GAMEPLAY_FIELDS}
        facts[field] = r["effective_value"]
        _validate_facts(facts, rid)
        evidence = r.get("evidence")
        require(isinstance(evidence, list) and evidence, f"Missing evidence: {rid}")
        image_seen = core_seen = False
        for spec in evidence:
            require(isinstance(spec, dict), f"Invalid evidence entry: {rid}")
            kind = spec.get("kind")
            require(kind in ("official_card_image", "official_core_rules"), f"Unknown evidence kind: {rid}")
            _official_url(spec.get("url"), rid, image=kind == "official_card_image")
            cache_key = canonical_bytes(spec)
            if cache_key not in checked_evidence:
                checked_evidence[cache_key] = _checked_bytes(root, spec, rid)
            if kind == "official_card_image":
                require(spec["url"] == candidate["image_url"] == origin["image_url"], f"Reviewed image URL mismatch: {rid}")
                require(type(spec.get("location")) is str and spec["location"].strip(), f"Missing visual evidence location: {rid}")
                image_seen = True
                if image_index is not None:
                    indexed = image_index.get(pid)
                    require(indexed is not None, f"Missing image-index printing: {rid}")
                    for key in ("url", "sha256", "bytes"):
                        require(typed_equal(indexed.get(key), spec[key]), f"Image-index mismatch: {rid}:{key}")
                    require(indexed.get("path", indexed.get("local_path")) == spec["local_path"], f"Image-index path mismatch: {rid}")
            else:
                require(checked_evidence[cache_key].startswith(b"%PDF-"), f"Core evidence is not a PDF: {rid}")
                _strings(spec.get("rules"), f"{rid}:rule anchors")
                require(spec["rules"], f"Missing Core rule anchors: {rid}")
                core_seen = True
        require(image_seen, f"Missing matching official image evidence: {rid}")
        require(r["basis"] != "rule_derived" or core_seen, f"Rule-derived resolution requires Core evidence: {rid}")
        reviewed[pid, field] = r

    require(set(preserved) == {pid for pid, _ in reviewed}, "Preserved printing evidence does not match resolutions")
    if image_index is not None:
        require(set(image_index) == set(preserved), "Image-index evidence does not match reviewed printings")
    history = manifest.get("historical_issues", [])
    require(isinstance(history, list), "Invalid historical source issues")
    history_ids = set()
    resolution_printings = {r["id"]: r["official_id"] for r in resolutions}
    for issue in history:
        require(isinstance(issue, dict), "Invalid historical issue entry")
        issue_id = issue.get("review_issue_id")
        require(type(issue_id) is str and issue_id and issue_id not in history_ids, "Duplicate or invalid historical issue ID")
        history_ids.add(issue_id)
        _strings(issue.get("printing_ids"), f"{issue_id}:printings")
        _strings(issue.get("resolution_ids"), f"{issue_id}:resolutions")
        require(set(issue["printing_ids"]) <= set(preserved), f"Unknown historical printing: {issue_id}")
        require(set(issue["resolution_ids"]) <= seen_ids, f"Unknown historical resolution: {issue_id}")
        require(all(resolution_printings[rid] in issue["printing_ids"] for rid in issue["resolution_ids"]),
                f"Historical resolution belongs to another printing: {issue_id}")
        require(isinstance(issue.get("raw_issue"), dict), f"Missing historical raw issue: {issue_id}")

    result = []
    for p in printings:
        value = copy.deepcopy(p)
        pid = p["official_id"]
        effective = {field: copy.deepcopy(p[field]) for field in GAMEPLAY_FIELDS}
        applied = [reviewed[pid, field] for field in GAMEPLAY_FIELDS if (pid, field) in reviewed]
        for r in applied:
            effective[r["field"]] = copy.deepcopy(r["effective_value"])
        _validate_facts(effective, pid)
        for prefix in ("rules", "equipment"):
            require(plain(effective[f"{prefix}_html"]) == effective[f"{prefix}_text"],
                    f"Effective HTML/text mismatch: {pid}:{prefix}")
        unresolved = set()
        auxiliary = "token" in effective["super_types"]
        if not auxiliary:
            if not effective["types"] or ((p["type_labels_conflict"] or not p["structured_types"])
                                           and (pid, "types") not in reviewed):
                unresolved.add("types")
            if set(effective["types"]) & {"unit", "spell", "gear", "legend", "rune"} and not effective["domains"]:
                unresolved.add("domains")
            if effective["types"] == ["unit"] and effective["might"] is None:
                unresolved.add("might")
        value.update(effective=effective, resolution_ids=[r["id"] for r in applied],
                     unresolved_gameplay_fields=sorted(unresolved))
        result.append(value)
    return {
        "printings": result,
        "resolutions": copy.deepcopy(resolutions),
        "historical_issues": copy.deepcopy(history),
        "evidence_summary": {
            "printing_projection_version": PROJECTION_VERSION,
            "origin_archives": copy.deepcopy(origin_specs),
            "reviewed_printings": len(preserved),
            "field_resolutions": len(resolutions),
            "historical_issue_groups": len(history),
            "image_evidence": len(preserved),
            "live_image_bytes_checked": False,
        },
    }
