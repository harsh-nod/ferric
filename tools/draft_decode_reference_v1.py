#!/usr/bin/env python3
"""Offline CPU oracle for the engineering Draft06B decode profile, not a benchmark.

This is a separate numerical oracle from the older GPU-pinned reference. It
retains complete FP32 logits, verifies two identical executions, and publishes
only after joining the checkpoint with Ferric's independent model admission.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import importlib.util
import os
from pathlib import Path
import struct
import sys

HELPER_SHA = "1d429cc87a8bc46018da9f8d84a7a4328dad05c43de8867ed791226c913f2959"
IMAGE_ID = "sha256:3d7fe5e20f0da24f220aa44fdf352055e8b277f201a4893ac177b380ffcc32ef"
VERSIONS = {"torch": "2.11.0+gitd0c8b1f", "transformers": "5.14.1", "tokenizers": "0.22.2", "safetensors": "0.8.0"}
SOURCES = {
    "models/qwen3/modeling_qwen3.py": "fbdcfeeb1b54135ca67ba7df924da92f4b264e1252b517eea2de989289ebaeab",
    "models/qwen3/configuration_qwen3.py": "49d9ccd0f29ccd977b93dba2004f84b867891b3d0509b71fd8ba3ed01054e64d",
    "utils/loading_report.py": "5e201aa79b46d7831e760984cf178bacaf33d6cedfe507972d2491537eb01fde",
}
RAW_SCHEMA = "FerricIndependentDraftDecodeCpuObservationV1"
REFERENCE_SCHEMA = "FerricDraftDecodeProfileReferenceV1"
STEP_KEYS = ("inputs", "positions", "selected_row", "choice", "cache_tokens")
POLICY = {
    "device": "cpu", "body_dtype": "bfloat16", "head_dtype": "float32",
    "attention": "sdpa", "threads": 2, "interop_threads": 2,
    "async_loading": False, "repetitions": 2,
    "head": "functional-linear-fp32-final-hidden-and-tied-weight",
    "greedy": "lowest-id-exact-maximum", "eos_stopping": False,
    "logits_processors": False, "chat_template": False,
    "add_special_tokens": False, "decode_skip_special_tokens": True,
    "autocast": False, "local_files_only": True, "trust_remote_code": False,
    "full_logits": "every-forward-little-endian-float32",
    "float32_matmul_precision": "highest", "fresh_cache_per_repetition": True,
}


def load_helper(path):
    data = Path(path).read_bytes()
    if hashlib.sha256(data).hexdigest() != HELPER_SHA:
        raise ValueError("frozen helper source drift")
    spec = importlib.util.spec_from_loader("ferric_frozen_decode_helper", loader=None)
    module = importlib.util.module_from_spec(spec)
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def schedule(mode, count, prompt, ordinal, previous):
    if mode not in ("full", "tokenwise") or type(count) is not int or not 2 <= count <= 128:
        raise ValueError("unsupported decode schedule")
    if len(prompt) != 5 or type(ordinal) is not int or not 0 <= ordinal < count + (4 if mode == "tokenwise" else 0):
        raise ValueError("schedule extent")
    positions = list(range(5)) if mode == "full" and ordinal == 0 else [ordinal + (4 if mode == "full" else 0)]
    if positions[-1] >= 5:
        if type(previous) is not int or not 0 <= previous < 151_936:
            raise ValueError("missing preceding independent choice")
        inputs = [previous]
    else:
        inputs = [prompt[position] for position in positions]
    return {"inputs": inputs, "positions": positions, "selected_row": len(inputs) - 1}


def runtime(helper):
    helper.require(sys.version_info[:2] == (3, 12), "Python version")
    versions = {name: importlib.metadata.version(name) for name in VERSIONS}
    dist = importlib.metadata.distribution("transformers")
    sources = {name: helper.file_digest(dist.locate_file("transformers/" + name)) for name in SOURCES}
    helper.require(versions == VERSIONS and sources == SOURCES, "reference runtime/source drift")
    return versions, sources


def payload_name(repetition, ordinal):
    return f"logits-{repetition}-{ordinal}.f32le"


def decode_generated(helper, tokenizer, tokens, count):
    helper.token_list(tokens, count)
    result = list(tokenizer.decode(tokens, skip_special_tokens=True).encode("utf-8"))
    helper.byte_list(result)
    return result


def validate_raw(helper, raw, producer_sha, read_payload):
    helper.exact(raw, ("schema", "performance_qualified", "model", "model_revision", "checkpoint", "draft_payload_sha256", "producer_sha256", "legacy_helper_sha256", "image_id", "versions", "implementation_sources", "policy", "prompt_tokens", "prefill", "new_tokens", "passes"), "CPU observation")
    helper.require(raw["schema"] == RAW_SCHEMA and raw["performance_qualified"] is False, "CPU reference scope")
    helper.require(raw["model"] == helper.MODEL and raw["model_revision"] == helper.REVISION, "model pin")
    helper.require(raw["checkpoint"] == {name: {"bytes": size, "sha256": sha} for name, (size, sha) in helper.FILES.items()}, "checkpoint pin")
    helper.sha(raw["draft_payload_sha256"])
    helper.require(raw["producer_sha256"] == producer_sha and raw["legacy_helper_sha256"] == HELPER_SHA, "producer pin")
    helper.require(raw["image_id"] == IMAGE_ID and raw["versions"] == VERSIONS and raw["implementation_sources"] == SOURCES, "runtime pin")
    helper.require(raw["policy"] == POLICY, "CPU numerical policy")
    mode, count, prompt = raw["prefill"], raw["new_tokens"], raw["prompt_tokens"]
    helper.token_list(prompt, 5)
    schedule(mode, count, prompt, 0, None)
    helper.require(type(raw["passes"]) is list and len(raw["passes"]) == 2, "reference repetition count")
    canonical = []
    for repetition, observed in enumerate(raw["passes"]):
        helper.exact(observed, ("steps", "generated_tokens", "generated_utf8_bytes"), "reference pass")
        helper.require(type(observed["steps"]) is list and len(observed["steps"]) == count + (4 if mode == "tokenwise" else 0), "forward count")
        previous, generated, normalized = None, [], []
        for ordinal, step in enumerate(observed["steps"]):
            helper.exact(step, (*STEP_KEYS, "logits_sha256", "logits_file", "top2"), "reference step")
            expected = schedule(mode, count, prompt, ordinal, previous)
            helper.token_list(step["inputs"], len(expected["inputs"]))
            helper.require(type(step["positions"]) is list and len(step["positions"]) == len(expected["positions"]), "position extent")
            for actual, position in zip(step["positions"], expected["positions"]):
                helper.integer(actual, position, position, "position")
            helper.integer(step["selected_row"], expected["selected_row"], expected["selected_row"], "selected row")
            helper.require({key: step[key] for key in expected} == expected, "causal input schedule")
            helper.integer(step["cache_tokens"], expected["positions"][-1] + 1, expected["positions"][-1] + 1, "cache cursor")
            helper.integer(step["choice"], 0, helper.VOCABULARY - 1, "choice")
            helper.require(step["logits_file"] == payload_name(repetition, ordinal), "payload filename")
            data = read_payload(step["logits_file"])
            helper.require(len(data) == helper.VOCABULARY * 4 and helper.digest(data) == helper.sha(step["logits_sha256"]), "full logits extent/hash")
            top = helper.top_two(struct.unpack(f"<{helper.VOCABULARY}f", data))
            helper.require(helper.encoded(step["top2"]) == helper.encoded(top) and step["choice"] == top["ids"][0], "full logits/argmax disagreement")
            previous = step["choice"]
            if step["cache_tokens"] >= 5:
                generated.append(previous)
            normalized.append({key: value for key, value in step.items() if key != "logits_file"})
        helper.token_list(observed["generated_tokens"], count)
        helper.byte_list(observed["generated_utf8_bytes"])
        bytes(observed["generated_utf8_bytes"]).decode("utf-8", errors="strict")
        helper.require(observed["generated_tokens"] == generated, "published tokens differ")
        canonical.append({**observed, "steps": normalized})
    helper.require(canonical[0] == canonical[1], "independent repeated logits or tokens differ")


def produce(helper, options, producer_sha):
    helper.require(options.image_id == IMAGE_ID, "externally pinned image ID")
    helper.require(all(os.environ.get(name) == "1" for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE")), "offline environment")
    helper.require(os.environ.get("HF_DEACTIVATE_ASYNC_LOAD") == "1", "synchronous model loading")
    helper.require(all(os.environ.get(name) == "2" for name in ("OMP_NUM_THREADS", "MKL_NUM_THREADS", "OPENBLAS_NUM_THREADS", "RAYON_NUM_THREADS")), "bounded CPU threads")
    helper.ensure_fresh_output(options.output)
    source = options.source.resolve(strict=True)
    files, payload_sha = helper.checkpoint(source)
    versions, sources = runtime(helper)
    import torch
    from tokenizers import Tokenizer
    from transformers import Qwen3ForCausalLM

    torch.set_num_threads(2)
    torch.set_num_interop_threads(2)
    torch.set_float32_matmul_precision("highest")
    torch.backends.cuda.matmul.allow_tf32 = False
    tokenizer = Tokenizer.from_file(str(source / "tokenizer.json"))
    prompt = tokenizer.encode(helper.PROMPT, add_special_tokens=False).ids
    helper.token_list(prompt, 5)
    schedule(options.prefill, options.new_tokens, prompt, 0, None)
    model, loading = Qwen3ForCausalLM.from_pretrained(str(source), dtype=torch.bfloat16, attn_implementation="sdpa", local_files_only=True, trust_remote_code=False, output_loading_info=True)
    helper.normalize_loading_info(loading)
    helper.require(all(p.dtype == torch.bfloat16 and p.device.type == "cpu" for p in model.parameters()), "CPU BF16 body")
    embedding, head = model.get_input_embeddings().weight, model.get_output_embeddings().weight
    helper.require(tuple(head.shape) == (helper.VOCABULARY, 1024) and embedding.data_ptr() == head.data_ptr(), "tied head geometry")
    model.eval()
    options.output.mkdir(mode=0o700)
    passes = []
    with torch.inference_mode(), torch.autocast(device_type="cpu", enabled=False):
        head32 = head.float()
        for repetition in range(2):
            cache, previous, steps, generated = None, None, [], []
            for ordinal in range(options.new_tokens + (4 if options.prefill == "tokenwise" else 0)):
                expected = schedule(options.prefill, options.new_tokens, prompt, ordinal, previous)
                result = model.model(input_ids=torch.tensor([expected["inputs"]]), position_ids=torch.tensor([expected["positions"]]), past_key_values=cache, use_cache=True, return_dict=True)
                cache = result.past_key_values
                cursor = expected["positions"][-1] + 1
                helper.require(cache.get_seq_length() == cursor, "CPU cache cursor")
                hidden = result.last_hidden_state
                helper.require(tuple(hidden.shape) == (1, len(expected["inputs"]), 1024) and hidden.dtype == torch.bfloat16, "hidden geometry/precision")
                logits = torch.nn.functional.linear(hidden[0, expected["selected_row"]].float(), head32)
                values = logits.tolist()
                top = helper.top_two(values)
                previous = top["ids"][0]
                helper.require(int(torch.argmax(logits).item()) == previous, "exact lowest-ID argmax")
                data = struct.pack(f"<{helper.VOCABULARY}f", *values)
                name = payload_name(repetition, ordinal)
                with (options.output / name).open("xb") as output:
                    output.write(data)
                steps.append({**expected, "choice": previous, "cache_tokens": cursor, "logits_sha256": helper.digest(data), "logits_file": name, "top2": top})
                if cursor >= 5:
                    generated.append(previous)
            passes.append({"steps": steps, "generated_tokens": generated, "generated_utf8_bytes": decode_generated(helper, tokenizer, generated, options.new_tokens)})
    helper.require(helper.checkpoint(source) == (files, payload_sha) and runtime(helper) == (versions, sources), "source/runtime changed during execution")
    raw = {"schema": RAW_SCHEMA, "performance_qualified": False, "model": helper.MODEL, "model_revision": helper.REVISION, "checkpoint": files, "draft_payload_sha256": payload_sha, "producer_sha256": producer_sha, "legacy_helper_sha256": HELPER_SHA, "image_id": IMAGE_ID, "versions": versions, "implementation_sources": sources, "policy": POLICY, "prompt_tokens": prompt, "prefill": options.prefill, "new_tokens": options.new_tokens, "passes": passes}
    validate_raw(helper, raw, producer_sha, lambda name: helper.read_bound(options.output / name))
    helper.publish(options.output / "raw.json", raw)


def adapt(helper, options, producer_sha):
    raw = helper.read_pinned(options.raw_dir / "raw.json", options.raw_sha256)
    validate_raw(helper, raw, producer_sha, lambda name: helper.read_bound(options.raw_dir / name))
    expected_files = {"raw.json"} | {step["logits_file"] for run in raw["passes"] for step in run["steps"]}
    helper.require({path.name for path in options.raw_dir.iterdir()} == expected_files, "raw file roster")
    identity = helper.read_pinned(options.identity, options.identity_sha256)
    helper.exact(identity, ("schema", "checkpoint", "identity"), "model admission")
    helper.require(identity["schema"] == helper.IDENTITY_SCHEMA and identity["checkpoint"] == raw["checkpoint"], "admitted checkpoint")
    bound = identity["identity"]
    helper.exact(bound, ("model_bundle_id", "draft_model_id", "draft_config_id", "draft_weights_sha256"), "Ferric identity")
    for value in bound.values():
        helper.sha(value)
    helper.require(bound["draft_model_id"] == helper.MODEL_ID and bound["draft_weights_sha256"] == raw["draft_payload_sha256"], "draft role/payload binding")
    observed = raw["passes"][0]
    helper.publish(options.output, {"schema": REFERENCE_SCHEMA, "model": helper.MODEL, "model_revision": helper.REVISION, "identity": bound, "head_precision": "fp32-v10", "prefill": raw["prefill"], "prompt_tokens": raw["prompt_tokens"], "steps": [{key: step[key] for key in STEP_KEYS} for step in observed["steps"]], "generated_tokens": observed["generated_tokens"], "generated_utf8_bytes": observed["generated_utf8_bytes"], "producer": f"independent CPU BF16-body/SDPA FP32-head; producer_sha256={producer_sha}; helper_sha256={HELPER_SHA}; image_id={IMAGE_ID}; raw_sha256={options.raw_sha256}; no performance qualification"})


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--producer-sha256", required=True)
    parser.add_argument("--legacy-helper", type=Path, required=True)
    actions = parser.add_subparsers(dest="action", required=True)
    run = actions.add_parser("produce")
    run.add_argument("--source", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True)
    run.add_argument("--image-id", required=True)
    run.add_argument("--prefill", choices=("full", "tokenwise"), required=True)
    run.add_argument("--new-tokens", type=int, required=True)
    adapter = actions.add_parser("adapt")
    for name in ("raw-dir", "identity", "output"):
        adapter.add_argument("--" + name, type=Path, required=True)
    for name in ("raw-sha256", "identity-sha256"):
        adapter.add_argument("--" + name, required=True)
    options = parser.parse_args(argv)
    helper = load_helper(options.legacy_helper)
    producer_sha = helper.file_digest(Path(__file__))
    helper.require(producer_sha == helper.sha(options.producer_sha256), "producer source drift")
    (produce if options.action == "produce" else adapt)(helper, options, producer_sha)
    helper.require(helper.file_digest(Path(__file__)) == producer_sha, "producer changed during operation")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, TypeError, AttributeError) as error:
        print(f"CPU decode reference rejected: {error}", file=sys.stderr)
        sys.exit(1)
