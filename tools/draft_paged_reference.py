#!/usr/bin/env python3
"""Independent paged Draft06B FP32-head oracle. GPU execution is root-owned.

The frozen BF16 helper is imported only after exact byte verification. Its policy,
producer and adapters are never modified or used to qualify this separate profile.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import os
from pathlib import Path
import stat
import struct
import sys


HELPER_SHA = "e491b4f243855dd57ce5b7cd6b2cd7b78811b5605622018827bb3932d41cb96e"
RAW_SCHEMA = "FerricIndependentDraftPagedFp32ObservationV10"
REFERENCE_SCHEMA = "FerricDraftPagedCanaryReferenceV10"
VOCABULARY = 151_936
LOGIT_BYTES = VOCABULARY * 4
POLICY = {
    "body_dtype": "bfloat16", "head_dtype": "float32", "attention": "sdpa",
    "head": "functional-linear-fp32-final-hidden-and-tied-weight",
    "schedules": {"full": [[0, 1, 2, 3, 4], [5]], "tokenwise": [[i] for i in range(6)]},
    "prompt_tokens": 5, "output_tokens": 2, "repetitions_per_schedule": 2,
    "full_logits": "every-forward-little-endian-float32", "head_selection": "last-input-row",
    "greedy": "lowest-id-exact-maximum", "eos_stopping": False,
    "logits_processors": False, "chat_template": False, "add_special_tokens": False,
    "decode_skip_special_tokens": True, "local_files_only": True, "trust_remote_code": False,
    "fresh_cache_per_repetition": True, "tf32": False,
    "loading_info_input_format": "transformers5-empty-sets-and-error-list",
}


def load_helper(path, expected):
    data = Path(path).read_bytes()
    if expected != HELPER_SHA or hashlib.sha256(data).hexdigest() != HELPER_SHA:
        raise ValueError("frozen helper source drift")
    # Execute precisely the verified bytes, not a pathname reopened after hashing.
    spec = importlib.util.spec_from_loader("ferric_frozen_draft_reference", loader=None)
    module = importlib.util.module_from_spec(spec)
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def plan(helper, mode, prompt, ordinal, previous):
    helper.require(mode in POLICY["schedules"], "unsupported prefill schedule")
    helper.token_list(prompt, 5)
    schedule = POLICY["schedules"][mode]
    helper.integer(ordinal, 0, len(schedule) - 1, "forward ordinal")
    positions = schedule[ordinal]
    if positions == [5]:
        helper.integer(previous, 0, VOCABULARY - 1, "completed decode input")
        inputs = [previous]
    else:
        inputs = [prompt[position] for position in positions]
    return {"inputs": inputs, "positions": positions, "selected_row": len(inputs) - 1}


def payload_name(mode, repetition, ordinal):
    return f"logits-{mode}-{repetition}-{ordinal}.f32le"


def read_regular(helper, path, size):
    path = Path(path)
    info = path.lstat()
    helper.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size == size,
                   "payload must be a single-link regular file with exact extent")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, "rb") as source:
        before = os.fstat(source.fileno())
        data = source.read(size + 1)
        after = os.fstat(source.fileno())
    helper.require((info.st_dev, info.st_ino) == (before.st_dev, before.st_ino), "payload replaced")
    stable = lambda info: (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns, info.st_nlink)
    helper.require(stable(before) == stable(after) and len(data) == size, "payload changed during read")
    return data


def write_payload(helper, path, values):
    helper.top_two(values)
    helper.ensure_fresh_output(path)
    data = struct.pack(f"<{VOCABULARY}f", *values)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())
    return {"file": Path(path).name, "bytes": len(data), "sha256": helper.digest(data)}


def validate_pass(helper, observed, prompt, mode, repetition, read_payload):
    helper.exact(observed, ("steps", "generated_tokens", "generated_utf8_bytes"), "pass")
    helper.require(type(observed["steps"]) is list and len(observed["steps"]) == len(POLICY["schedules"][mode]), "forward count")
    previous = None
    generated = []
    for ordinal, step in enumerate(observed["steps"]):
        helper.exact(step, ("inputs", "positions", "selected_row", "choice", "cache_tokens", "logits_dtype", "finite_count", "top2", "logits"), "forward")
        expected = plan(helper, mode, prompt, ordinal, previous)
        helper.token_list(step["inputs"], len(expected["inputs"]))
        helper.require(type(step["positions"]) is list and len(step["positions"]) == len(expected["positions"]), "position extent")
        for actual, position in zip(step["positions"], expected["positions"]):
            helper.integer(actual, position, position, "position")
        helper.integer(step["selected_row"], expected["selected_row"], expected["selected_row"], "selected row")
        helper.require({key: step[key] for key in expected} == expected, "autoregressive input schedule")
        cursor = expected["positions"][-1] + 1
        helper.integer(step["cache_tokens"], cursor, cursor, "cache cursor")
        helper.integer(step["finite_count"], VOCABULARY, VOCABULARY, "finite extent")
        helper.integer(step["choice"], 0, VOCABULARY - 1, "choice")
        helper.require(step["logits_dtype"] == "torch.float32", "head precision")
        descriptor = step["logits"]
        helper.exact(descriptor, ("file", "bytes", "sha256"), "logit descriptor")
        helper.require(descriptor["file"] == payload_name(mode, repetition, ordinal), "payload filename binding")
        helper.integer(descriptor["bytes"], LOGIT_BYTES, LOGIT_BYTES, "payload bytes")
        data = read_payload(descriptor["file"])
        helper.require(len(data) == LOGIT_BYTES and helper.digest(data) == helper.sha(descriptor["sha256"]), "payload hash/extent")
        top = helper.top_two(struct.unpack(f"<{VOCABULARY}f", data))
        helper.require(helper.encoded(step["top2"]) == helper.encoded(top), "full-logit top2 mismatch")
        helper.require(step["choice"] == top["ids"][0], "full-logit exact argmax mismatch")
        previous = step["choice"]
        if cursor >= 5:
            generated.append(previous)
    helper.token_list(observed["generated_tokens"], 2)
    helper.byte_list(observed["generated_utf8_bytes"])
    bytes(observed["generated_utf8_bytes"]).decode("utf-8", errors="strict")
    helper.require(observed["generated_tokens"] == generated, "published choices differ")


def repetition_body(value):
    result = copy.deepcopy(value)
    for step in result["steps"]:
        del step["logits"]["file"]
    return result


def validate_raw(helper, raw, producer_sha, read_payload):
    helper.exact(raw, ("schema", "authority", "performance_qualified", "model", "model_revision", "source", "checkpoint", "draft_payload_sha256", "producer_sha256", "legacy_helper_sha256", "externally_pinned_image", "externally_pinned_image_id", "versions", "implementation_sources", "policy", "prompt", "prompt_tokens", "device", "loading_info", "tied_weights", "schedules", "repeated_exact", "logit_payload_bytes"), "raw observation")
    helper.require(raw["schema"] == RAW_SCHEMA and raw["authority"] == "independent-draft-reference-only" and raw["performance_qualified"] is False, "raw profile/authority")
    helper.require(raw["model"] == helper.MODEL and raw["model_revision"] == helper.REVISION, "model identity")
    helper.require(type(raw["source"]) is str and raw["source"].startswith("/"), "absolute checkpoint path")
    expected_files = {name: {"bytes": size, "sha256": sha} for name, (size, sha) in helper.FILES.items()}
    helper.require(helper.encoded(raw["checkpoint"]) == helper.encoded(expected_files), "checkpoint identities")
    helper.sha(raw["draft_payload_sha256"])
    helper.require(raw["producer_sha256"] == helper.sha(producer_sha) and raw["legacy_helper_sha256"] == HELPER_SHA, "source identities")
    helper.require(raw["externally_pinned_image"] == helper.IMAGE and raw["externally_pinned_image_id"] == helper.IMAGE_ID, "image identities")
    helper.require(raw["versions"] == helper.VERSIONS and raw["implementation_sources"] == helper.SOURCES, "dependency identities")
    helper.require(helper.encoded(raw["policy"]) == helper.encoded(POLICY), "frozen FP32 policy")
    helper.require(raw["prompt"] == helper.PROMPT, "prompt literal")
    helper.token_list(raw["prompt_tokens"], 5)
    helper.exact(raw["device"], ("index", "name", "gcn_arch", "torch_hip"), "device")
    helper.integer(raw["device"]["index"], 0, 0, "device index")
    for name in ("name", "gcn_arch", "torch_hip"):
        helper.require(type(raw["device"][name]) is str and 0 < len(raw["device"][name]) <= 256, "device metadata")
    helper.require(raw["device"]["gcn_arch"].split(":")[0] == "gfx950", "GPU architecture")
    helper.require(raw["loading_info"] == {name: [] for name in ("missing_keys", "unexpected_keys", "mismatched_keys", "error_msgs")}, "model loading gaps")
    helper.require(raw["tied_weights"] is True and raw["repeated_exact"] is True, "tied/repeated result")
    helper.integer(raw["logit_payload_bytes"], 16 * LOGIT_BYTES, 16 * LOGIT_BYTES, "total logit bytes")
    helper.exact(raw["schedules"], POLICY["schedules"], "two predeclared schedules")
    for mode, passes in raw["schedules"].items():
        helper.require(type(passes) is list and len(passes) == 2, "two repetitions per schedule")
        for repetition, observed in enumerate(passes):
            validate_pass(helper, observed, raw["prompt_tokens"], mode, repetition, read_payload)
        helper.require(repetition_body(passes[0]) == repetition_body(passes[1]), "repeated complete logits/observations differ")


def load_raw(helper, directory, expected_sha, producer_sha):
    directory = Path(directory)
    helper.require(directory.is_dir() and not directory.is_symlink(), "raw directory")
    raw_path = directory / "raw.json"
    size = raw_path.lstat().st_size
    helper.integer(size, 1, helper.MAX_DOCUMENT, "raw document size")
    data = read_regular(helper, raw_path, size)
    helper.require(helper.digest(data) == helper.sha(expected_sha), "external raw hash")
    raw = helper.parse(data)
    names = {"raw.json"} | {payload_name(mode, repetition, ordinal)
            for mode, schedule in POLICY["schedules"].items()
            for repetition in range(2) for ordinal in range(len(schedule))}
    helper.require({entry.name for entry in directory.iterdir()} == names, "raw directory exact file roster")
    validate_raw(helper, raw, producer_sha, lambda name: read_regular(helper, directory / name, LOGIT_BYTES))
    return raw


def adapt(helper, raw, identity, producer_sha, raw_sha, mode):
    helper.require(mode in POLICY["schedules"], "adapter schedule")
    helper.exact(identity, ("schema", "checkpoint", "identity"), "identity admission")
    helper.require(identity["schema"] == helper.IDENTITY_SCHEMA and identity["checkpoint"] == raw["checkpoint"], "identity checkpoint binding")
    bound = identity["identity"]
    helper.exact(bound, ("model_bundle_id", "draft_model_id", "draft_config_id", "draft_weights_sha256"), "Ferric identity")
    for value in bound.values():
        helper.sha(value)
    helper.require(bound["draft_model_id"] == helper.MODEL_ID and bound["draft_weights_sha256"] == raw["draft_payload_sha256"], "draft role/payload identity")
    observed = raw["schedules"][mode][0]
    return {"schema": REFERENCE_SCHEMA, "model": helper.MODEL, "model_revision": helper.REVISION,
            "identity": bound, "head_precision": "fp32-v10", "prefill": mode,
            "prompt_tokens": raw["prompt_tokens"],
            "steps": [{key: step[key] for key in ("inputs", "positions", "selected_row", "choice", "cache_tokens")} for step in observed["steps"]],
            "generated_tokens": observed["generated_tokens"], "generated_utf8_bytes": observed["generated_utf8_bytes"],
            "producer": f"independent Draft06B BF16-body/SDPA FP32-head; producer_sha256={helper.sha(producer_sha)}; legacy_helper_sha256={HELPER_SHA}; raw_sha256={helper.sha(raw_sha)}; schedule={mode}; no performance qualification"}


def produce(helper, options, producer_sha):
    helper.require(options.image == helper.IMAGE and options.image_id == helper.IMAGE_ID, "external image pin")
    helper.ensure_fresh_output(options.output)
    source = options.source.resolve(strict=True)
    files, payload_sha = helper.checkpoint(source)
    versions, sources = helper.versions_and_sources()
    for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE"):
        helper.require(os.environ.get(name) == "1", "explicit offline environment")
    import torch
    from tokenizers import Tokenizer
    from transformers import Qwen3ForCausalLM

    helper.require(torch.__version__ == helper.VERSIONS["torch"], "imported torch version")
    helper.require(torch.cuda.is_available() and torch.cuda.device_count() == 1, "exactly one visible GPU")
    device = helper.device_metadata(torch.cuda.get_device_properties(0), torch.version.hip)
    torch.set_num_threads(2)
    torch.backends.cuda.matmul.allow_tf32 = False
    tokenizer = Tokenizer.from_file(str(source / "tokenizer.json"))
    prompt = tokenizer.encode(helper.PROMPT, add_special_tokens=False).ids
    helper.token_list(prompt, 5)
    model, loading = Qwen3ForCausalLM.from_pretrained(str(source), dtype=torch.bfloat16, attn_implementation="sdpa", local_files_only=True, trust_remote_code=False, output_loading_info=True)
    loading = helper.normalize_loading_info(loading)
    for name, expected in {"hidden_size": 1024, "intermediate_size": 3072, "num_hidden_layers": 28, "num_attention_heads": 16, "num_key_value_heads": 8, "head_dim": 128, "vocab_size": VOCABULARY, "tie_word_embeddings": True}.items():
        helper.require(getattr(model.config, name) == expected, f"model geometry {name}")
    helper.require(model.config._attn_implementation == "sdpa", "attention backend drift")
    helper.require(all(parameter.dtype == torch.bfloat16 for parameter in model.parameters()), "body/weight precision")
    embedding, head = model.get_input_embeddings().weight, model.get_output_embeddings().weight
    helper.require(tuple(embedding.shape) == tuple(head.shape) == (VOCABULARY, 1024), "tied weight geometry")
    helper.require(embedding.data_ptr() == head.data_ptr() and torch.equal(embedding, head), "tied embedding/head")
    model.eval().to("cuda:0")
    options.output.mkdir(mode=0o700)
    schedules = {}
    with torch.inference_mode():
        head32 = model.get_output_embeddings().weight.float()
        for mode, schedule in POLICY["schedules"].items():
            passes = []
            for repetition in range(2):
                cache, previous = None, None
                steps, generated = [], []
                for ordinal in range(len(schedule)):
                    expected = plan(helper, mode, prompt, ordinal, previous)
                    result = model.model(input_ids=torch.tensor([expected["inputs"]], dtype=torch.long, device="cuda:0"), position_ids=torch.tensor([expected["positions"]], dtype=torch.long, device="cuda:0"), past_key_values=cache, use_cache=True, return_dict=True)
                    cache = result.past_key_values
                    cursor = expected["positions"][-1] + 1
                    helper.require(cache.get_seq_length() == cursor, "runtime cache cursor")
                    hidden = result.last_hidden_state
                    helper.require(tuple(hidden.shape) == (1, len(expected["inputs"]), 1024) and hidden.dtype == torch.bfloat16, "final hidden geometry/dtype")
                    logits = torch.nn.functional.linear(hidden[0, expected["selected_row"]].float(), head32)
                    helper.require(tuple(logits.shape) == (VOCABULARY,) and logits.dtype == torch.float32, "full FP32 logit geometry")
                    values = logits.cpu().tolist()
                    top = helper.top_two(values)
                    previous = top["ids"][0]
                    helper.require(int(torch.argmax(logits).item()) == previous, "device/CPU exact argmax")
                    descriptor = write_payload(helper, options.output / payload_name(mode, repetition, ordinal), values)
                    steps.append({**expected, "choice": previous, "cache_tokens": cursor, "logits_dtype": "torch.float32", "finite_count": len(values), "top2": top, "logits": descriptor})
                    if cursor >= 5:
                        generated.append(previous)
                passes.append({"steps": steps, "generated_tokens": generated, "generated_utf8_bytes": helper.decode_generated(tokenizer, generated)})
            schedules[mode] = passes
        torch.cuda.synchronize()
    helper.require(helper.checkpoint(source) == (files, payload_sha), "checkpoint changed during execution")
    helper.require(helper.versions_and_sources() == (versions, sources), "implementation changed during execution")
    raw = {"schema": RAW_SCHEMA, "authority": "independent-draft-reference-only", "performance_qualified": False,
           "model": helper.MODEL, "model_revision": helper.REVISION, "source": str(source), "checkpoint": files,
           "draft_payload_sha256": payload_sha, "producer_sha256": producer_sha, "legacy_helper_sha256": HELPER_SHA,
           "externally_pinned_image": helper.IMAGE, "externally_pinned_image_id": helper.IMAGE_ID,
           "versions": versions, "implementation_sources": sources, "policy": POLICY, "prompt": helper.PROMPT,
           "prompt_tokens": prompt, "device": device, "loading_info": loading, "tied_weights": True,
           "schedules": schedules, "repeated_exact": all(repetition_body(value[0]) == repetition_body(value[1]) for value in schedules.values()),
           "logit_payload_bytes": 16 * LOGIT_BYTES}
    validate_raw(helper, raw, producer_sha, lambda name: read_regular(helper, options.output / name, LOGIT_BYTES))
    helper.publish(options.output / "raw.json", raw)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--producer-sha256", required=True)
    parser.add_argument("--legacy-helper", type=Path, required=True)
    parser.add_argument("--legacy-helper-sha256", required=True)
    actions = parser.add_subparsers(dest="action", required=True)
    run = actions.add_parser("produce")
    run.add_argument("--source", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True)
    run.add_argument("--image", required=True)
    run.add_argument("--image-id", required=True)
    adapter = actions.add_parser("adapt")
    for name in ("raw-dir", "identity", "output"):
        adapter.add_argument("--" + name, type=Path, required=True)
    adapter.add_argument("--raw-sha256", required=True)
    adapter.add_argument("--identity-sha256", required=True)
    adapter.add_argument("--prefill", choices=tuple(POLICY["schedules"]), required=True)
    options = parser.parse_args(argv)
    helper = load_helper(options.legacy_helper, options.legacy_helper_sha256)
    producer_sha = helper.file_digest(Path(__file__))
    helper.require(producer_sha == helper.sha(options.producer_sha256), "producer source drift")
    if options.action == "produce":
        produce(helper, options, producer_sha)
    else:
        raw = load_raw(helper, options.raw_dir, options.raw_sha256, producer_sha)
        identity = helper.read_pinned(options.identity, options.identity_sha256)
        helper.publish(options.output, adapt(helper, raw, identity, producer_sha, options.raw_sha256, options.prefill))
    helper.require(helper.file_digest(Path(__file__)) == producer_sha, "producer source changed during operation")
    helper.require(helper.file_digest(options.legacy_helper) == HELPER_SHA, "helper source changed during operation")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, TypeError, AttributeError) as error:
        print(f"paged FP32 reference rejected: {error}", file=sys.stderr)
        sys.exit(1)
