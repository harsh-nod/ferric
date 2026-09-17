"""Narrow observation identity join, not EngineeringTpArtifact admission."""
import json

from common import CHECK, digest, fields, hash_value, integer, require

OBSERVATION_ORDER = "schema namespace authority artifact crate_name target code_object_version compiler_handoff tools providers options execution hsaco grants".split()


def content_identity(value, label):
    fields(value, {"sha256", "byte_len"}, label)
    hash_value(value["sha256"], label + " hash")
    integer(value["byte_len"], 1, 64 * 1024 * 1024, label + " length")
    return value


def handoff_binding(raw_manifest, hsaco, retained=None):
    value = CHECK.json_value(raw_manifest)
    fields(value, set(OBSERVATION_ORDER), "engineering observation")
    require(list(value) == OBSERVATION_ORDER, "canonical engineering observation field order")
    encoded = json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(",", ":")).encode() + b"\n"
    require(encoded == raw_manifest, "canonical engineering observation encoding")
    constants = {"schema": "EngineeringHsacoObservationV1", "namespace": "fe2o3-engineering-v1",
        "authority": "none", "artifact": "observation.hsaco",
        "crate_name": "ferric_qwen3_draft_batch32_kernels_device_v10", "target": "gfx950:xnack-",
        "code_object_version": 6}
    for key, expected in constants.items():
        require(type(value[key]) is type(expected) and value[key] == expected, "engineering profile: " + key)
    fields(value["grants"], {"publication", "load", "launch"}, "engineering grants")
    require(all(item is False for item in value["grants"].values()), "engineering observation grants authority")
    fields(value["hsaco"], {"identity", "canonical_descriptor_sha256", "kernel_names"}, "engineering image identity")
    image = content_identity(value["hsaco"]["identity"], "engineering image")
    require(image == {"sha256": digest(hsaco), "byte_len": len(hsaco)}, "manifest/HSACO byte identity")
    hash_value(value["hsaco"]["canonical_descriptor_sha256"], "recorded descriptor identity")
    handoff = content_identity(value["compiler_handoff"], "compiler handoff")
    if retained is not None:
        require(handoff == {"sha256": digest(retained), "byte_len": len(retained)}, "retained handoff/manifest identity")
    return {"sha256": handoff["sha256"], "byte_len": handoff["byte_len"],
            "mode": "retained-bytes" if retained is not None else "manifest-identity",
            "raw_handoff_bytes_retained_and_revalidated": retained is not None,
            "observation_manifest_sha256": digest(raw_manifest),
            "scope": "Identity consistency only; open_draft32 retains descriptor/roster/admission checks. Not attestation."}
