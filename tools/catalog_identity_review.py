"""Validate complete, explicit variant reviews without rewriting printed facts.

The reviewer decides semantics. This module checks that the exact complete
identity, field corrections and evidence supporting that decision still apply.
It never strips reminders, chooses a preferred wording, or registers a card.
"""
from __future__ import annotations

import copy
from datetime import date
import hashlib
from pathlib import Path
import re

from catalog_review import (
    GAMEPLAY_FIELDS, HASH_ENCODING, PROJECTION_KEYS, PROJECTION_VERSION,
    _checked_bytes, _load_json, _official_url, _printing_map, _sha, _strings,
    _validate_facts, canonical_bytes, printing_sha256, require, typed_equal,
)

GALLERY_URL = "https://playriftbound.com/en-us/card-gallery/"
VARIANT_FIELDS = ("tags", "rules_text", "equipment_text")
SCALAR_FIELDS = ("types", "super_types", "domains", "energy", "power", "might", "might_bonus")
DISPOSITIONS = {"corrected_transcription", "reminder_equivalent", "rule_derived_equivalent"}


def _keys(value, required, label, optional=()):
    require(isinstance(value, dict), f"Expected object: {label}")
    require(set(required) <= set(value) <= set(required) | set(optional),
            f"Missing or unknown fields: {label}")


def _hash(value):
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def _raw(printing):
    return {key: printing[key] for key in PROJECTION_KEYS}


def _value(printing, field, effective=False):
    value = printing["effective"][field] if effective else printing[field]
    return sorted(value) if field in ("tags", "types", "super_types", "domains") else value


def _different(variants, field, effective=False):
    return len({canonical_bytes(_value(p, field, effective)) for p in variants}) > 1


def _rules(value, label):
    _strings(value, label)
    require(all(re.fullmatch(r"[0-9]{3}(?:\.[0-9a-z]+)*\.?", anchor) for anchor in value),
            f"Invalid individual Core rule anchor: {label}")


def _image_bytes(raw):
    return (raw.startswith(b"\x89PNG\r\n\x1a\n") or raw.startswith(b"\xff\xd8\xff")
            or (raw[:4] == b"RIFF" and raw[8:12] == b"WEBP"))


def apply_identity_reviews(groups, *, root: Path, project_gallery, normalize,
                           resolutions, current_core_rules_url, errata_by_identity,
                           parse_errata, manifest=None):
    """Return validated summaries keyed by canonical official ID.

    ``groups`` contains the already field-reviewed printings, in the importer's
    canonical order. All review records are verified, even if their differences
    have disappeared. Unreviewed groups are left to the importer's admission
    gate. An explicit empty manifest supports isolated parser fixtures.
    """
    root = Path(root)
    if manifest is None:
        manifest = _load_json((root / "assets/data/catalog_identity_reviews.json").read_bytes())
    else:
        manifest = copy.deepcopy(manifest)
    _keys(manifest, {"schema_version", "status", "printing_projection_version", "printing_hash_encoding",
                     "origin_archives", "authorities", "reviews"}, "identity review manifest", {"image_index"})
    require(type(manifest["schema_version"]) is int and manifest["schema_version"] == 1,
            "Unknown identity review schema")
    require(manifest["status"] == "reviewed", "Identity review manifest is not reviewed")
    require(type(manifest["printing_projection_version"]) is int
            and manifest["printing_projection_version"] == PROJECTION_VERSION,
            "Unknown identity printing projection")
    require(manifest["printing_hash_encoding"] == HASH_ENCODING, "Unknown identity hash encoding")
    require(isinstance(manifest["reviews"], list), "Missing identity review list")
    require(isinstance(groups, dict), "Invalid identity groups")
    require(isinstance(errata_by_identity, dict), "Invalid applicable errata map")
    require(isinstance(resolutions, list), "Invalid validated field resolutions")

    resolution_map = {}
    for resolution in resolutions:
        require(isinstance(resolution, dict), "Invalid field resolution")
        rid = resolution.get("id")
        require(type(rid) is str and rid and rid not in resolution_map, "Duplicate or invalid field resolution ID")
        resolution_map[rid] = resolution
    all_printings, group_names = {}, {}
    for identity, variants in groups.items():
        require(type(identity) is str and isinstance(variants, list) and variants,
                "Invalid or empty identity group")
        require(variants[0].get("official_id") == identity, f"Canonical identity does not match first printing: {identity}")
        group_names[identity] = normalize(variants[0]["name"])
        for p in variants:
            raw = _raw(p)
            printing_sha256(raw)
            pid = p["official_id"]
            require(pid not in all_printings, f"Printing belongs to multiple groups: {pid}")
            require(normalize(p["name"]) == group_names[identity], f"Mixed identity names: {identity}")
            require(isinstance(p.get("effective"), dict) and set(p["effective"]) == set(GAMEPLAY_FIELDS),
                    f"Incomplete effective printing: {pid}")
            _validate_facts(raw, pid)
            _validate_facts(p["effective"], pid)
            _strings(p.get("resolution_ids"), f"{pid}:resolution IDs")
            _strings(p.get("unresolved_gameplay_fields"), f"{pid}:unresolved fields")
            all_printings[pid] = p
    require(len(set(group_names.values())) == len(group_names), "Same normalized identity split into multiple groups")

    origins = {}
    origin_specs = manifest["origin_archives"]
    require(isinstance(origin_specs, dict), "Invalid identity origin registry")
    for digest, spec in origin_specs.items():
        _keys(spec, {"url", "archive", "sha256", "uncompressed_bytes"}, f"origin {digest}")
        require(_sha(digest, "origin key") == spec["sha256"], "Origin key/hash mismatch")
        _official_url(spec["url"], digest)
        require(spec["url"] == GALLERY_URL, f"Origin is not the approved gallery URL: {digest}")
        raw = _checked_bytes(root, spec, digest, origin=True)
        origins[digest] = _printing_map(project_gallery(raw), f"identity origin {digest}")

    authorities, authority_errata = manifest["authorities"], {}
    require(isinstance(authorities, dict), "Invalid identity authority registry")
    authority_urls = set()
    for aid, authority in authorities.items():
        require(type(aid) is str and aid, "Invalid authority ID")
        require(isinstance(authority, dict), f"Invalid authority: {aid}")
        kind = authority.get("kind")
        require(kind in {"official_core_rules", "official_errata"}, f"Unknown authority kind: {aid}")
        archive = kind == "official_errata"
        _keys(authority, {"kind", "url", "archive", "sha256", "uncompressed_bytes"} if archive
              else {"kind", "url", "local_path", "sha256", "bytes"}, f"authority {aid}")
        _official_url(authority["url"], aid)
        require(authority["url"] not in authority_urls, f"Duplicate authority URL: {aid}")
        authority_urls.add(authority["url"])
        raw = _checked_bytes(root, authority, aid, origin=archive)
        if archive:
            entries = parse_errata(raw, authority["url"])
            require(isinstance(entries, list) and entries, f"Empty errata authority: {aid}")
            require(all(isinstance(e, dict) and e.get("source_url") == authority["url"] for e in entries),
                    f"Errata authority URL/record mismatch: {aid}")
            hashes = [_hash(e) for e in entries]
            require(len(hashes) == len(set(hashes)), f"Duplicate errata authority records: {aid}")
            authority_errata[aid] = set(hashes)
        else:
            require(raw.startswith(b"%PDF-"), f"Core authority is not a PDF: {aid}")
            require(authority["url"] == current_core_rules_url, f"Current Core URL differs from reviewed authority: {aid}")

    images = {}
    if "image_index" in manifest:
        _keys(manifest["image_index"], {"local_path", "sha256", "bytes"}, "identity image index")
        entries = _load_json(_checked_bytes(root, manifest["image_index"], "identity image index"))
        require(isinstance(entries, list), "Invalid identity image index")
        for entry in entries:
            require(isinstance(entry, dict), "Invalid identity image entry")
            pid = entry.get("official_id")
            require(type(pid) is str and pid and pid not in images, "Duplicate or invalid identity image ID")
            _official_url(entry.get("url"), pid, image=True)
            raw = _checked_bytes(root, entry, pid)
            require(_image_bytes(raw), f"Evidence is not an original supported image: {pid}")
            images[pid] = entry
    require(not manifest["reviews"] or images, "Identity reviews require image evidence")

    summaries, seen_reviews, seen_members = {}, set(), set()
    used_origins, used_authorities = set(), set()
    for review in manifest["reviews"]:
        _keys(review, {"id", "identity", "name", "reviewed_on", "status", "enables_runtime_support",
                       "core_authority_id", "members", "fields", "errata"}, "identity review")
        rid, identity = review["id"], review["identity"]
        require(type(rid) is str and rid and rid not in seen_reviews, "Duplicate or invalid identity review ID")
        seen_reviews.add(rid)
        require(type(identity) is str and identity in groups and identity not in summaries,
                f"Missing or duplicate reviewed identity: {rid}")
        variants = groups[identity]
        require(review["name"] == variants[0]["name"], f"Reviewed identity name mismatch: {rid}")
        require(review["status"] == "complete", f"Identity review is not complete: {rid}")
        require(review["enables_runtime_support"] is False, f"Identity review cannot enable runtime support: {rid}")
        require(type(review["reviewed_on"]) is str
                and re.fullmatch(r"\d{4}-\d{2}-\d{2}", review["reviewed_on"]), f"Invalid review date: {rid}")
        try:
            date.fromisoformat(review["reviewed_on"])
        except ValueError as error:
            raise ValueError(f"Invalid review date: {rid}") from error
        core_id = review["core_authority_id"]
        require(type(core_id) is str and core_id in authorities
                and authorities[core_id]["kind"] == "official_core_rules", f"Missing Core authority: {rid}")
        used_authorities.add(core_id)
        require(all(not p["unresolved_gameplay_fields"] for p in variants), f"Unresolved member gameplay facts: {rid}")
        require(all(not _different(variants, field, True) for field in SCALAR_FIELDS),
                f"Unresolved effective scalar differences: {rid}")
        require(isinstance(review["members"], list), f"Invalid reviewed members: {rid}")
        members = {}
        for member in review["members"]:
            _keys(member, {"official_id", "raw_sha256", "effective_sha256", "gallery_uncompressed_sha256",
                           "resolution_sha256", "image_id"}, f"{rid}:member")
            pid = member["official_id"]
            require(type(pid) is str and pid in all_printings and pid not in members and pid not in seen_members,
                    f"Missing or duplicate reviewed member: {rid}:{pid}")
            p = all_printings[pid]
            require(normalize(p["name"]) == group_names[identity], f"Foreign identity member: {rid}:{pid}")
            raw = _raw(p)
            require(printing_sha256(raw) == _sha(member["raw_sha256"], pid), f"Stale raw printing pin: {pid}")
            effective = raw | p["effective"]
            require(printing_sha256(effective) == _sha(member["effective_sha256"], pid), f"Stale effective printing pin: {pid}")
            origin_hash = _sha(member["gallery_uncompressed_sha256"], pid)
            require(origin_hash in origins, f"Missing identity origin: {pid}")
            origin = origins[origin_hash]
            require(pid in origin and typed_equal(origin[pid], raw), f"Origin printing differs: {pid}")
            origin_members = {opid for opid, op in origin.items() if normalize(op["name"]) == group_names[identity]}
            require(origin_members == {p["official_id"] for p in variants}, f"Origin identity membership differs: {rid}")
            used_origins.add(origin_hash)
            pins = member["resolution_sha256"]
            require(isinstance(pins, dict) and set(pins) == set(p["resolution_ids"]), f"Stale field-resolution membership: {pid}")
            for fid, digest in pins.items():
                require(fid in resolution_map and resolution_map[fid].get("official_id") == pid,
                        f"Unknown or foreign field resolution: {pid}:{fid}")
                require(_hash(resolution_map[fid]) == _sha(digest, fid), f"Stale field-resolution evidence: {fid}")
            require(member["image_id"] == pid and pid in images, f"Missing matching identity image: {pid}")
            require(images[pid]["url"] == p["image_url"] == origin[pid]["image_url"], f"Identity image URL mismatch: {pid}")
            for fid in p["resolution_ids"]:
                evidence = resolution_map[fid].get("evidence", [])
                matching_images = [spec for spec in evidence if spec.get("kind") == "official_card_image"]
                require(matching_images, f"Field resolution lacks matching image evidence: {fid}")
                for spec in matching_images:
                    require(all(typed_equal(spec.get(key), images[pid][key]) for key in ("url", "sha256", "bytes")),
                            f"Identity and field-resolution image evidence differ: {pid}:{fid}")
            members[pid] = member
            seen_members.add(pid)
        require(set(members) == {p["official_id"] for p in variants}, f"Incomplete identity membership: {rid}")

        raw_fields = [f for f in VARIANT_FIELDS if _different(variants, f)]
        effective_fields = [f for f in VARIANT_FIELDS if _different(variants, f, True)]
        required_fields = set(raw_fields) | set(effective_fields)
        require(isinstance(review["fields"], list), f"Invalid field reviews: {rid}")
        fields = {}
        for field_review in review["fields"]:
            _keys(field_review, {"field", "disposition", "reason", "rules", "resolution_ids"}, f"{rid}:field review")
            field, disposition = field_review["field"], field_review["disposition"]
            require(field in VARIANT_FIELDS and field not in fields, f"Unknown or duplicate reviewed field: {rid}")
            require(disposition in DISPOSITIONS, f"Unknown semantic disposition: {rid}:{field}")
            require(type(field_review["reason"]) is str and field_review["reason"].strip(), f"Missing semantic reasoning: {rid}:{field}")
            _rules(field_review["rules"], f"{rid}:{field}")
            _strings(field_review["resolution_ids"], f"{rid}:{field}:resolution IDs")
            related = {field} if field == "tags" else {field, field.replace("_text", "_html")}
            expected = {fid for p in variants for fid in p["resolution_ids"] if resolution_map[fid]["field"] in related}
            require(set(field_review["resolution_ids"]) == expected, f"Field disposition resolution membership differs: {rid}:{field}")
            if disposition == "corrected_transcription":
                require(field in raw_fields and field not in effective_fields, f"Transcription disposition does not resolve variants: {rid}:{field}")
                require(any(resolution_map[fid]["operation"] == "override" for fid in expected),
                        f"Transcription disposition lacks actual field correction: {rid}:{field}")
            else:
                require(field != "tags" and field in effective_fields, f"Semantic equivalence requires differing effective text: {rid}:{field}")
                require(field_review["rules"], f"Semantic equivalence requires Core anchors: {rid}:{field}")
                if disposition == "reminder_equivalent":
                    require(any(rule.rstrip(".") == "135.2.d.3" for rule in field_review["rules"]),
                            f"Reminder equivalence lacks explicit reminder authority: {rid}:{field}")
            fields[field] = field_review
        require(set(fields) == required_fields, f"Review must cover all raw/effective differing fields: {rid}")

        require(isinstance(review["errata"], list), f"Invalid reviewed errata: {rid}")
        actual_errata = errata_by_identity.get(identity, [])
        require(isinstance(actual_errata, list), f"Invalid current errata list: {rid}")
        current_hashes = [_hash(record) for record in actual_errata]
        require(len(current_hashes) == len(set(current_hashes)), f"Duplicate current errata: {rid}")
        reviewed_hashes = set()
        for entry in review["errata"]:
            _keys(entry, {"authority_id", "record", "record_sha256", "location"}, f"{rid}:erratum")
            aid = entry["authority_id"]
            require(type(aid) is str and aid in authority_errata, f"Missing errata authority: {rid}")
            record = entry["record"]
            require(isinstance(record, dict) and record.get("source_url") == authorities[aid]["url"],
                    f"Erratum source URL mismatch: {rid}")
            digest = _sha(entry["record_sha256"], rid)
            require(_hash(record) == digest and digest in authority_errata[aid], f"Erratum does not match pinned source: {rid}")
            require(digest not in reviewed_hashes, f"Duplicate reviewed erratum: {rid}")
            require(type(entry["location"]) is str and entry["location"].strip(), f"Missing erratum location: {rid}")
            reviewed_hashes.add(digest)
            used_authorities.add(aid)
        require(reviewed_hashes == set(current_hashes), f"Current applicable errata differ from review: {rid}")
        summaries[identity] = {"id": rid, "identity": identity, "name": review["name"], "status": "complete",
                               "reviewed_variant_fields": [f for f in VARIANT_FIELDS if f in fields],
                               "raw_variant_fields": raw_fields, "effective_variant_fields": effective_fields,
                               "member_ids": sorted(members), "field_reviews": copy.deepcopy(review["fields"]),
                               "errata": copy.deepcopy(review["errata"])}

    require(set(images) == seen_members, "Identity image index does not exactly cover reviewed members")
    require(set(origins) == used_origins, "Unused identity origin archive")
    require(set(authorities) == used_authorities, "Unused identity authority")
    result = [summaries[identity] for identity in sorted(summaries)]
    return {"reviews": result, "by_identity": summaries,
            "evidence_summary": {"printing_projection_version": PROJECTION_VERSION,
                                 "reviewed_identities": len(result), "reviewed_printings": len(seen_members),
                                 "reviewed_fields": sum(len(r["field_reviews"]) for r in result),
                                 "image_evidence": len(images), "origin_archives": copy.deepcopy(origin_specs),
                                 "authorities": copy.deepcopy(authorities), "live_image_bytes_checked": False}}
