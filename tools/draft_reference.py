#!/usr/bin/env python3
"""Separately pinned offline Draft06B reference; GPU invocation belongs to root."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import stat
import struct
import sys


MODEL = "Qwen/Qwen3-0.6B"
REVISION = "c1899de289a04d12100db370d81485cdf75e47ca"
MODEL_ID = "351fc121a569f0a53e9bb5c98caaeff80d6f8d94737eecf5e179cfa54d9cf998"
IMAGE = "vllm/vllm-openai-rocm@sha256:e0a3b2bd3fe7ec563916c3a5d949898d133458c18d6b2f460c906885cfb32032"
IMAGE_ID = "sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba"
PROMPT = "The capital of France is"
VOCABULARY = 151_936
PAYLOAD_BYTES = 1_503_264_768
MAX_DOCUMENT = 1_048_576
FILES = {
    "config.json": (726, "660db3b73d788119c04535e48cf9be5f55bc3100841a718637ae695b442f27dd"),
    "model.safetensors": (1_503_300_328, "f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b"),
    "tokenizer.json": (11_422_654, "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4"),
    "tokenizer_config.json": (9_732, "d5d09f07b48c3086c508b30d1c9114bd1189145b74e982a265350c923acd8101"),
}
VERSIONS = {"torch": "2.12.0+git6bbd260", "transformers": "5.15.1", "tokenizers": "0.22.2", "safetensors": "0.8.0"}
SOURCES = {
    "models/qwen3/modeling_qwen3.py": "cbb7f2dc274c2f5592746c0dc6985ca50353efa07376f92cc922b77680a74f69",
    "models/qwen3/configuration_qwen3.py": "49d9ccd0f29ccd977b93dba2004f84b867891b3d0509b71fd8ba3ed01054e64d",
    "utils/loading_report.py": "5e201aa79b46d7831e760984cf178bacaf33d6cedfe507972d2491537eb01fde",
}
POLICY = {
    "dtype": "bfloat16", "head_dtype": "bfloat16", "attention": "sdpa",
    "schedule": "six-token-at-a-time-cached-forwards", "position_ids": [0, 1, 2, 3, 4, 5],
    "prompt_tokens": 5, "output_tokens": 2, "repetitions": 2,
    "greedy": "lowest-id-exact-maximum", "eos_stopping": False,
    "logits_processors": False, "chat_template": False, "add_special_tokens": False,
    "decode_skip_special_tokens": True, "local_files_only": True, "trust_remote_code": False,
    "loading_info_input_format": "transformers5-empty-sets-and-error-list",
}
RAW_SCHEMA = "FerricIndependentDraftTorchObservationV1"
IDENTITY_SCHEMA = "FerricDraftReferenceIdentityV1"
ADAPTER_SCHEMA = "FerricDraftCanaryReferenceV1"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha(value):
    require(type(value) is str and len(value) == 64 and all(c in "0123456789abcdef" for c in value), "noncanonical SHA-256")
    return value


def exact(value, keys, name):
    require(type(value) is dict and set(value) == set(keys), f"{name} field set")


def integer(value, low, high, name):
    require(type(value) is int and low <= value <= high, f"{name} integer bound")


def finite(value):
    require(type(value) in (int, float) and math.isfinite(value), "nonfinite or nonnumeric logit")
    return float(value)


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON field")
        result[key] = value
    return result


def reject_constant(value):
    raise ValueError(f"nonfinite JSON constant {value}")


def parse(data):
    require(0 < len(data) <= MAX_DOCUMENT, "document byte bound")
    return json.loads(data, object_pairs_hook=unique, parse_constant=reject_constant)


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode("ascii")


def read_bound(path, limit=MAX_DOCUMENT):
    with Path(path).open("rb") as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit, "input file bound")
        data = source.read(limit + 1)
        after = os.fstat(source.fileno())
        require(before.st_size == after.st_size == len(data) <= limit, "input changed length")
        return data


def read_pinned(path, expected):
    data = read_bound(path)
    require(digest(data) == sha(expected), "external document hash mismatch")
    return parse(data)


def file_digest(path, expected_bytes=None, offset=0):
    with Path(path).open("rb") as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode), "nonregular source")
        if expected_bytes is not None:
            require(before.st_size == expected_bytes, "source file byte length")
        require(0 <= offset <= before.st_size, "payload offset")
        source.seek(offset)
        state = hashlib.sha256()
        count = 0
        while block := source.read(1_048_576):
            state.update(block)
            count += len(block)
        after = os.fstat(source.fileno())
        require(before.st_size == after.st_size == offset + count, "source changed length")
        return state.hexdigest()


def checkpoint(source):
    result = {}
    for name, (size, expected) in FILES.items():
        observed = file_digest(source / name, size)
        require(observed == expected, f"canonical checkpoint mismatch: {name}")
        result[name] = {"bytes": size, "sha256": observed}
    with (source / "model.safetensors").open("rb") as weight:
        header_size = struct.unpack("<Q", weight.read(8))[0]
    total = FILES["model.safetensors"][0]
    require(header_size + 8 + PAYLOAD_BYTES == total, "canonical tensor payload boundary")
    return result, file_digest(source / "model.safetensors", total, header_size + 8)


def versions_and_sources():
    require(sys.version_info[:2] == (3, 12), "reference Python must be 3.12")
    observed = {name: importlib.metadata.version(name) for name in VERSIONS}
    require(observed == VERSIONS, "cached package versions differ")
    dist = importlib.metadata.distribution("transformers")
    sources = {name: file_digest(dist.locate_file("transformers/" + name)) for name in SOURCES}
    require(sources == SOURCES, "Qwen3 implementation source changed")
    return observed, sources


def normalize_loading_info(value):
    keys = ("missing_keys", "unexpected_keys", "mismatched_keys", "error_msgs")
    exact(value, keys, "loading info")
    for key in keys[:3]:
        require(type(value[key]) is set and not value[key], f"nonempty or changed loading-info set: {key}")
    require(type(value["error_msgs"]) is list and not value["error_msgs"], "loading error messages")
    # Pinned Transformers5 loading_report raises conversion errors before return.
    return {key: [] for key in keys}


def token_list(values, length):
    require(type(values) is list and len(values) == length, "token array length")
    for token in values:
        integer(token, 0, VOCABULARY - 1, "token")


def byte_list(values):
    require(type(values) is list and len(values) <= 16_384, "decoded byte bound")
    for value in values:
        integer(value, 0, 255, "decoded byte")


def decode_generated(tokenizer, tokens):
    token_list(tokens, 2)
    # Match EngineeringQwenModelV1::decode; generated IDs are never filtered.
    return list(tokenizer.decode(tokens, skip_special_tokens=True).encode("utf-8"))


def top_two(values):
    require(len(values) == VOCABULARY, "full-vocabulary logit extent")
    best = runner = None
    for index, value in enumerate(values):
        value = finite(value)
        candidate = (value, -index)
        if best is None or candidate > best[0]:
            runner, best = best, (candidate, index, value)
        elif runner is None or candidate > runner[0]:
            runner = (candidate, index, value)
    assert best is not None and runner is not None
    return {"ids": [best[1], runner[1]], "values": [best[2], runner[2]], "gap": best[2] - runner[2]}


def validate_pass(value, prompt):
    exact(value, ("steps", "generated_tokens", "generated_utf8_bytes"), "pass")
    require(type(value["steps"]) is list and len(value["steps"]) == 6, "six exact forwards required")
    choices = []
    for position, step in enumerate(value["steps"]):
        exact(step, ("position", "input_token", "choice", "cache_tokens", "logits_dtype", "finite_count", "top2"), "step")
        integer(step["position"], position, position, "position")
        integer(step["cache_tokens"], position + 1, position + 1, "cache length")
        integer(step["finite_count"], VOCABULARY, VOCABULARY, "finite count")
        integer(step["input_token"], 0, VOCABULARY - 1, "input token")
        integer(step["choice"], 0, VOCABULARY - 1, "choice")
        require(step["input_token"] == (prompt[position] if position < 5 else choices[4]), "cached input schedule")
        require(step["logits_dtype"] == "torch.bfloat16", "unexpected head dtype")
        top = step["top2"]
        exact(top, ("ids", "values", "gap"), "top2")
        token_list(top["ids"], 2)
        require(top["ids"][0] != top["ids"][1], "top2 duplicate IDs")
        require(type(top["values"]) is list and len(top["values"]) == 2, "top2 values")
        first, second = map(finite, top["values"])
        require(first >= second and finite(top["gap"]) == first - second, "top2 gap")
        require(first != second or top["ids"][0] < top["ids"][1], "tie ordering")
        require(step["choice"] == top["ids"][0], "argmax/top2 mismatch")
        choices.append(step["choice"])
    token_list(value["generated_tokens"], 2)
    byte_list(value["generated_utf8_bytes"])
    require(value["generated_tokens"] == choices[4:6], "published output choices")


def validate_raw(value, producer_sha):
    exact(value, ("schema", "authority", "performance_qualified", "model", "model_revision", "source", "checkpoint", "draft_payload_sha256", "producer_sha256", "externally_pinned_image", "externally_pinned_image_id", "versions", "implementation_sources", "policy", "prompt", "prompt_tokens", "device", "loading_info", "tied_weights", "passes", "repeated_exact"), "raw observation")
    require(value["schema"] == RAW_SCHEMA and value["authority"] == "independent-draft-reference-only" and value["performance_qualified"] is False, "raw authority")
    require(value["model"] == MODEL and value["model_revision"] == REVISION, "model identity")
    require(type(value["source"]) is str and value["source"].startswith("/"), "source path")
    require(value["checkpoint"] == {name: {"bytes": size, "sha256": expected} for name, (size, expected) in FILES.items()}, "checkpoint identities")
    for name, (size, _) in FILES.items():
        integer(value["checkpoint"][name]["bytes"], size, size, "checkpoint size")
    sha(value["draft_payload_sha256"])
    require(value["producer_sha256"] == sha(producer_sha), "producer source identity")
    require(value["externally_pinned_image"] == IMAGE and value["externally_pinned_image_id"] == IMAGE_ID, "cached image identity")
    require(value["versions"] == VERSIONS and value["implementation_sources"] == SOURCES, "producer dependency identities")
    require(value["policy"] == POLICY and encoded(value["policy"]) == encoded(POLICY), "reference policy")
    require(value["prompt"] == PROMPT, "prompt changed")
    token_list(value["prompt_tokens"], 5)
    exact(value["device"], ("index", "name", "gcn_arch", "torch_hip"), "device")
    integer(value["device"]["index"], 0, 0, "device index")
    for key in ("name", "gcn_arch", "torch_hip"):
        require(type(value["device"][key]) is str and 0 < len(value["device"][key]) <= 256, f"device metadata {key}")
    require(value["device"]["gcn_arch"].split(":")[0] == "gfx950", "reference target")
    require(value["loading_info"] == {name: [] for name in ("missing_keys", "unexpected_keys", "mismatched_keys", "error_msgs")}, "checkpoint loading gaps")
    require(value["tied_weights"] is True and value["repeated_exact"] is True, "tie/repeated result")
    require(type(value["passes"]) is list and len(value["passes"]) == 2, "two reference passes")
    for observed in value["passes"]:
        validate_pass(observed, value["prompt_tokens"])
    require(value["passes"][0] == value["passes"][1], "repeated complete observations differ")


def ensure_fresh_output(path):
    path = Path(path)
    parent = path.parent.stat()
    require(stat.S_ISDIR(parent.st_mode) and stat.S_IMODE(parent.st_mode) == 0o700 and parent.st_uid == os.getuid(), "output parent must be owned mode0700")
    require(not path.exists() and not path.is_symlink(), "output already exists")


def publish(path, value):
    ensure_fresh_output(path)
    data = encoded(value)
    require(len(data) <= MAX_DOCUMENT, "output byte bound")
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        directory = os.open(Path(path).parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    except BaseException:
        # Wrapper success is required even if a final fsync leaves bytes present.
        raise


def device_metadata(properties, hip_version):
    values = {"name": properties.name, "gcn_arch": properties.gcnArchName, "torch_hip": hip_version}
    for name, value in values.items():
        require(isinstance(value, str) and 0 < len(value) <= 256,
                f"runtime device metadata {name}: {type(value).__name__} {str(value)[:256]!r}")
    require(values["gcn_arch"].split(":")[0] == "gfx950", "only root-selected gfx950 supported")
    # PyTorch version metadata can be a str subclass; JSON receipts use plain strings.
    return {"index": 0, **{name: str(value) for name, value in values.items()}}


def produce(options, producer_sha):
    require(options.image == IMAGE and options.image_id == IMAGE_ID, "external image pin")
    ensure_fresh_output(options.output)
    source = options.source.resolve(strict=True)
    files, payload_sha = checkpoint(source)
    versions, sources = versions_and_sources()
    for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE"):
        require(os.environ.get(name) == "1", "offline environment must be explicit")
    import torch
    from tokenizers import Tokenizer
    from transformers import Qwen3ForCausalLM

    require(torch.__version__ == VERSIONS["torch"], "imported torch version")
    require(torch.cuda.is_available() and torch.cuda.device_count() == 1, "exactly one visible GPU required")
    properties = torch.cuda.get_device_properties(0)
    device = device_metadata(properties, torch.version.hip)
    torch.set_num_threads(2)
    torch.backends.cuda.matmul.allow_tf32 = False
    tokenizer = Tokenizer.from_file(str(source / "tokenizer.json"))
    prompt = tokenizer.encode(PROMPT, add_special_tokens=False).ids
    token_list(prompt, 5)
    model, loading = Qwen3ForCausalLM.from_pretrained(str(source), dtype=torch.bfloat16, attn_implementation="sdpa", local_files_only=True, trust_remote_code=False, output_loading_info=True)
    loading = normalize_loading_info(loading)
    config = model.config
    for name, expected in {"hidden_size": 1024, "intermediate_size": 3072, "num_hidden_layers": 28, "num_attention_heads": 16, "num_key_value_heads": 8, "head_dim": 128, "vocab_size": VOCABULARY, "tie_word_embeddings": True}.items():
        require(getattr(config, name) == expected, f"model geometry {name}")
    require(config._attn_implementation == "sdpa", "attention backend drift")
    require(all(p.dtype == torch.bfloat16 for p in model.parameters()), "model parameter dtype")
    embedding, head = model.get_input_embeddings().weight, model.get_output_embeddings().weight
    require(tuple(embedding.shape) == (VOCABULARY, 1024) and tuple(head.shape) == (VOCABULARY, 1024), "tied weight geometry")
    require(embedding.data_ptr() == head.data_ptr() and torch.equal(embedding, head), "tied embedding/head not preserved")
    model.eval().to("cuda:0")
    passes = []
    with torch.inference_mode():
        for _ in range(2):
            cache = None
            choices, steps = [], []
            for position in range(6):
                token = prompt[position] if position < 5 else choices[4]
                result = model(input_ids=torch.tensor([[token]], dtype=torch.long, device="cuda:0"), position_ids=torch.tensor([[position]], dtype=torch.long, device="cuda:0"), past_key_values=cache, use_cache=True, return_dict=True)
                cache = result.past_key_values
                require(cache.get_seq_length() == position + 1, "runtime cache length")
                require(tuple(result.logits.shape) == (1, 1, VOCABULARY) and result.logits.dtype == torch.bfloat16, "runtime full logits extent/dtype")
                values = result.logits[0, 0].float().cpu().tolist()
                top = top_two(values)
                choice = top["ids"][0]
                require(int(torch.argmax(result.logits[0, 0]).item()) == choice, "device versus CPU exact argmax")
                choices.append(choice)
                steps.append({"position": position, "input_token": token, "choice": choice, "cache_tokens": cache.get_seq_length(), "logits_dtype": str(result.logits.dtype), "finite_count": len(values), "top2": top})
            generated = choices[4:6]
            passes.append({"steps": steps, "generated_tokens": generated, "generated_utf8_bytes": decode_generated(tokenizer, generated)})
        torch.cuda.synchronize()
    require(checkpoint(source) == (files, payload_sha), "checkpoint changed during model execution")
    raw = {"schema": RAW_SCHEMA, "authority": "independent-draft-reference-only", "performance_qualified": False, "model": MODEL, "model_revision": REVISION, "source": str(source), "checkpoint": files, "draft_payload_sha256": payload_sha, "producer_sha256": producer_sha, "externally_pinned_image": IMAGE, "externally_pinned_image_id": IMAGE_ID, "versions": versions, "implementation_sources": sources, "policy": POLICY, "prompt": PROMPT, "prompt_tokens": prompt, "device": device, "loading_info": loading, "tied_weights": True, "passes": passes, "repeated_exact": passes[0] == passes[1]}
    validate_raw(raw, producer_sha)
    publish(options.output, raw)


def adapt(raw, identity, producer_sha, raw_sha):
    validate_raw(raw, producer_sha)
    exact(identity, ("schema", "checkpoint", "identity"), "identity admission")
    require(identity["schema"] == IDENTITY_SCHEMA and identity["checkpoint"] == raw["checkpoint"], "identity checkpoint binding")
    bound = identity["identity"]
    exact(bound, ("model_bundle_id", "draft_model_id", "draft_config_id", "draft_weights_sha256"), "Ferric identity")
    for value in bound.values():
        sha(value)
    require(bound["draft_model_id"] == MODEL_ID and bound["draft_weights_sha256"] == raw["draft_payload_sha256"], "draft role/payload identity")
    observed = raw["passes"][0]
    return {"schema": ADAPTER_SCHEMA, "model": MODEL, "model_revision": REVISION, "identity": bound, "prompt_tokens": raw["prompt_tokens"], "generated_tokens": observed["generated_tokens"], "generated_utf8_bytes": observed["generated_utf8_bytes"], "producer": f"independent Draft06B torch BF16/SDPA six-step reference; producer_sha256={producer_sha}; raw_sha256={sha(raw_sha)}; no performance qualification"}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--producer-sha256", required=True)
    actions = parser.add_subparsers(dest="action", required=True)
    run = actions.add_parser("produce")
    run.add_argument("--source", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True)
    run.add_argument("--image", required=True)
    run.add_argument("--image-id", required=True)
    adapter = actions.add_parser("adapt")
    for name in ("raw", "identity", "output"):
        adapter.add_argument("--" + name, type=Path, required=True)
    adapter.add_argument("--raw-sha256", required=True)
    adapter.add_argument("--identity-sha256", required=True)
    options = parser.parse_args(argv)
    producer_sha = file_digest(Path(__file__))
    require(producer_sha == sha(options.producer_sha256), "producer bytes differ from external pin")
    if options.action == "produce":
        produce(options, producer_sha)
    else:
        raw = read_pinned(options.raw, options.raw_sha256)
        identity = read_pinned(options.identity, options.identity_sha256)
        result = adapt(raw, identity, producer_sha, options.raw_sha256)
        publish(options.output, result)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, TypeError, AttributeError) as error:
        print(f"draft reference rejected: {error}", file=sys.stderr)
        sys.exit(1)
