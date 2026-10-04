#!/usr/bin/env python3
"""Disabled, independent Qwen3 BF16 reference for the fixed 2048/256 workload."""
from __future__ import annotations

import argparse
import hashlib
import inspect
import importlib.metadata
import json
import os
import platform
import socket
import stat
import struct
import sys
import time
from pathlib import Path

PROMPT_ENABLED = False
REFERENCE_ENABLED = True
REVISION = "b968826d9c46dd6066d109eabc6255188de91218"
MODEL_ID = "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b"
VOCABULARY = 151936
INPUT_TOKENS = 2048
OUTPUT_TOKENS = 256
MODEL_FILES = {
    "config.json": (728, "f7c4eadfbbf522470667b797a3c89be2524832d2d599797248dc304fff447c30"),
    "model.safetensors.index.json": (32878, "f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc"),
    "model-00001-of-00005.safetensors": (3996250744, "31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f"),
    "model-00002-of-00005.safetensors": (3993160032, "5991236cea6fe21f3d43cab0f0e84448734fbbe0789816202989f2ddc9d18282"),
    "model-00003-of-00005.safetensors": (3959604768, "c5185c4794be2d8a9784d5753c9922db38df478ce11f9ed0b415b7304d896836"),
    "model-00004-of-00005.safetensors": (3187841392, "b5ee7de71fbf17db3d5704e0c8f2bc7d005ca9e1d7ca2aeb19827b0cfcaa917a"),
    "model-00005-of-00005.safetensors": (1244659840, "20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff"),
    "tokenizer.json": (11422654, "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4"),
    "tokenizer_config.json": (9732, "d5d09f07b48c3086c508b30d1c9114bd1189145b74e982a265350c923acd8101"),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha_file(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        while block := source.read(1 << 20):
            value.update(block)
    return value.hexdigest()


def snapshot(path):
    value = path.stat()
    require(stat.S_ISREG(value.st_mode), f"not a regular file: {path}")
    return [value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns]


def write_new(path, data):
    with path.open("xb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())
    return {"path": path.name, "bytes": len(data), "sha256": digest(data)}


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()


def checked_plan(path, expected, mode):
    data = path.read_bytes()
    require(len(data) <= 65536 and digest(data) == expected, "plan identity")
    plan = json.loads(data)
    require(plan["schema"] == "FerricQwen3LongReferencePlanV1", "plan schema")
    require(plan["harness_sha256"] == sha_file(Path(__file__)), "harness identity")
    require(plan["revision"] == REVISION and plan["model_identity"] == MODEL_ID, "model identity")
    require(plan["input_tokens"] == INPUT_TOKENS and plan["output_tokens"] == OUTPUT_TOKENS, "workload")
    require(plan["host"] == socket.gethostname(), "host identity")
    require(str(Path(sys.executable).absolute()) == plan["python_executable"], "Python executable")
    require(sys.prefix == plan["python_prefix"] and sys.prefix != sys.base_prefix, "isolated venv")
    require(sys.flags.isolated == 1 and sys.dont_write_bytecode and sys.version_info[:2] == (3, 10), "Python -I -B / 3.10")
    require(plan["model_root"] and plan["output_root"], "pending model or output path")
    expected_packages = ({"tokenizers"} if mode == "prepare-prompt" else
                         {"torch", "transformers", "tokenizers", "numpy", "safetensors", "triton-rocm", "accelerate", "psutil"})
    require(set(plan["packages"]) == expected_packages, "package roster")
    for name, version in plan["packages"].items():
        require(version is not None and importlib.metadata.version(name) == version, f"package {name}")
    require(plan["packages"].get("tokenizers") == "0.21.4", "tokenizer version")
    if mode == "reference":
        require(plan["packages"].get("transformers") == "4.51.0", "Transformers version")
        require(plan["packages"].get("torch") == "2.12.1+rocm7.2", "PyTorch version")
        require(plan["packages"].get("accelerate") == "1.14.0" and plan["packages"].get("psutil") == "7.0.0", "loader helper versions")
        require(plan["architecture"] == "gfx950", "reference architecture")
        require(plan["hip_visible_devices"] is not None, "pending GPU selection")
        require(os.environ.get("HIP_VISIBLE_DEVICES") == plan["hip_visible_devices"], "visible GPU")
        require("ROCR_VISIBLE_DEVICES" not in os.environ, "ambiguous GPU remapping")
        require(plan["gpu_unique_id_file"] and plan["gpu_unique_id"], "pending physical GPU identity")
        require(Path(plan["gpu_unique_id_file"]).read_text().strip() == plan["gpu_unique_id"], "GPU unique ID")
    return plan


def checked_sources(root, names):
    records = {}
    for name in names:
        path = root / name
        before = snapshot(path)
        size, expected = MODEL_FILES[name]
        require(before[2] == size and sha_file(path) == expected, f"model source {name}")
        require(snapshot(path) == before, f"source changed while hashing: {name}")
        records[name] = {"stat": before, "bytes": size, "sha256": expected}
    return records


def unchanged_sources(root, records):
    for name, record in records.items():
        require(snapshot(root / name) == record["stat"], f"model source custody changed: {name}")


def raw_decoder(document):
    require(document["model"]["type"] == "BPE" and document["decoder"]["type"] == "ByteLevel", "tokenizer type")
    # Invert the byte-level alphabet directly so invalid UTF-8 output is not
    # silently replaced by a Unicode decoder. Ferric emits these raw bytes.
    direct = list(range(33, 127)) + list(range(161, 173)) + list(range(174, 256))
    byte_order = direct + [value for value in range(256) if value not in direct]
    characters = direct + list(range(256, 256 + 256 - len(direct)))
    alphabet = {chr(character): value for character, value in zip(characters, byte_order)}
    vocabulary = {index: bytes(alphabet[character] for character in token)
                  for token, index in document["model"]["vocab"].items()}
    added = {value["id"]: value for value in document["added_tokens"]}
    special = {index for index, value in added.items() if value["special"]}
    def decode(ids, skip_special):
        result = bytearray()
        for token in ids:
            require(type(token) is int and 0 <= token < VOCABULARY, "token ID extent")
            if token in added:
                if not (skip_special and token in special):
                    result.extend(added[token]["content"].encode("utf-8"))
            else:
                require(token in vocabulary, f"padded or unknown tokenizer ID {token}")
                result.extend(vocabulary[token])
        return bytes(result)
    return decode, special


def tokenizer_and_raw_decoder(root):
    from tokenizers import Tokenizer
    path = root / "tokenizer.json"
    decode, special = raw_decoder(json.loads(path.read_bytes()))
    tokenizer = Tokenizer.from_file(str(path))
    tokenizer.no_padding()
    tokenizer.no_truncation()
    return tokenizer, decode, special


def prepare_prompt(plan):
    root = Path(plan["model_root"])
    sources = checked_sources(root, ["tokenizer.json", "tokenizer_config.json"])
    seed_path = Path(__file__).with_name("prompt-seed.txt")
    seed = seed_path.read_bytes()
    require(digest(seed) == plan["prompt_seed_sha256"], "prompt seed identity")
    require(seed.isascii(), "prompt seed ASCII contract")
    tokenizer, decode, special = tokenizer_and_raw_decoder(root)
    text = "\n\n".join(f"Notebook section {index + 1}\n\n{seed.decode()}" for index in range(8))
    all_ids = tokenizer.encode(text, add_special_tokens=False).ids
    require(len(all_ids) > INPUT_TOKENS, "seed too short")
    ids = all_ids[:INPUT_TOKENS]
    require(not special.intersection(ids) and all(0 <= token < 151643 for token in ids), "prompt contains added/special tokens")
    prompt = decode(ids, skip_special=False)
    decoded = prompt.decode("utf-8", errors="strict")
    require(tokenizer.encode(decoded, add_special_tokens=False).ids == ids, "prompt round trip")
    require(tokenizer.decode(ids, skip_special_tokens=False).encode("utf-8") == prompt, "tokenizer decode parity")
    unchanged_sources(root, sources)
    output = Path(plan["output_root"])
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    text_record = write_new(output / "prompt.txt", prompt)
    ids_record = write_new(output / "prompt.u32le", struct.pack("<2048I", *ids))
    require(0 < len(prompt) <= 16384, "Ferric prompt byte limit")
    workload = {"schema": "FerricQwen3TpWorkloadV2", "requests": [
        {"name": "qwen3-8b-2048-256", "prompt": decoded,
         "new_tokens": OUTPUT_TOKENS, "arrival_tick": 0}]}
    workload_record = write_new(output / "workload.json", json_bytes(workload))
    manifest = {
        "schema": "FerricQwen3LongPromptV1", "revision": REVISION,
        "procedure": "eight numbered ASCII seed sections; first2048 raw tokens; strict decode/re-encode",
        "seed_sha256": digest(seed), "input_token_ids": ids, "input_tokens": INPUT_TOKENS,
        "output_tokens": OUTPUT_TOKENS, "add_special_tokens": False, "chat_template": None,
        "round_trip_verified": True, "tokenizer_sources": sources,
        "files": [text_record, ids_record, workload_record],
        "generated_reference": False,
    }
    write_new(output / "prompt-manifest.json", json_bytes(manifest))


def checked_prompt(plan, tokenizer, decode, special):
    root = Path(plan["prompt_bundle"])
    data = (root / "prompt-manifest.json").read_bytes()
    require(digest(data) == plan["prompt_manifest_sha256"], "frozen prompt identity")
    manifest = json.loads(data)
    require(manifest["schema"] == "FerricQwen3LongPromptV1", "prompt schema")
    require(manifest["revision"] == REVISION and manifest["seed_sha256"] == plan["prompt_seed_sha256"], "prompt procedure binding")
    require(manifest["input_tokens"] == INPUT_TOKENS and manifest["output_tokens"] == OUTPUT_TOKENS, "prompt workload")
    require(manifest["add_special_tokens"] is False and manifest["chat_template"] is None, "raw prompt policy")
    ids = manifest["input_token_ids"]
    require(len(ids) == INPUT_TOKENS and not special.intersection(ids), "prompt ID contract")
    require(sorted(item["path"] for item in manifest["files"]) == ["prompt.txt", "prompt.u32le", "workload.json"], "prompt file roster")
    for record in manifest["files"]:
        require(record["path"] in {"prompt.txt", "prompt.u32le", "workload.json"}, "prompt filename")
        value = (root / record["path"]).read_bytes()
        require(len(value) == record["bytes"] and digest(value) == record["sha256"], "prompt payload identity")
    raw = (root / "prompt.txt").read_bytes()
    require(raw == decode(ids, False), "prompt raw decode")
    require(tokenizer.encode(raw.decode("utf-8"), add_special_tokens=False).ids == ids, "prompt encode")
    require((root / "prompt.u32le").read_bytes() == struct.pack("<2048I", *ids), "prompt binary IDs")
    workload = json.loads((root / "workload.json").read_bytes())
    require(workload == {"schema": "FerricQwen3TpWorkloadV2", "requests": [
        {"name": "qwen3-8b-2048-256", "prompt": raw.decode("utf-8"),
         "new_tokens": OUTPUT_TOKENS, "arrival_tick": 0}]}, "actual Ferric workload binding")
    return ids, manifest


def bf16_diagnostic(raw):
    require(len(raw) == VOCABULARY * 2, "logit row extent")
    best = None
    runner_up = None
    ties = 0
    for token, (bits,) in enumerate(struct.iter_unpack("<H", raw)):
        require(bits & 0x7F80 != 0x7F80, f"nonfinite logit at {token}")
        magnitude = bits & 0x7FFF
        key = 0x8000 - magnitude if bits & 0x8000 else 0x8000 + magnitude
        item = (key, token, bits)
        if best is None or key > best[0]:
            runner_up, best, ties = best, item, 1
        elif key == best[0]:
            ties += 1
            if runner_up is None or key > runner_up[0]:
                runner_up = item
        elif runner_up is None or key > runner_up[0]:
            runner_up = item
    def value(item):
        return struct.unpack("<f", struct.pack("<I", item[2] << 16))[0]
    return {"token_id": best[1], "bf16_bits": best[2], "runner_up_token_id": runner_up[1],
            "runner_up_bf16_bits": runner_up[2], "top_margin": value(best) - value(runner_up),
            "maximum_tie_count": ties, "row_sha256": digest(raw)}


def run_pass(model, torch, prompt, decode, output, ordinal):
    from transformers.cache_utils import DynamicCache
    ids = torch.tensor([prompt], dtype=torch.long, device="cuda")
    cache = DynamicCache()
    require(cache.get_seq_length() == 0, "fresh independent cache")
    generated = []
    diagnostics = []
    torch.cuda.synchronize()
    started = time.monotonic_ns()
    path = output / f"pass-{ordinal}.logits.bf16le"
    with path.open("xb") as logits_file, torch.inference_mode():
        for step in range(OUTPUT_TOKENS):
            length = INPUT_TOKENS + step
            mask = torch.ones((1, length), dtype=torch.long, device="cuda")
            result = model.model(input_ids=ids, attention_mask=mask, past_key_values=cache,
                                 use_cache=True, return_dict=True)
            hidden = result.last_hidden_state
            require(hidden.dtype == torch.bfloat16 and tuple(hidden.shape) == (1, ids.shape[1], 4096), "BF16 hidden state")
            logits = model.lm_head(hidden[:, -1:, :])
            require(logits.dtype == torch.bfloat16 and tuple(logits.shape) == (1, 1, VOCABULARY), "BF16 output head")
            row = logits.reshape(VOCABULARY).detach().contiguous().cpu()
            raw = row.view(torch.uint16).numpy().tobytes()
            diagnostic = bf16_diagnostic(raw)
            token = diagnostic["token_id"]
            logits_file.write(raw)
            generated.append(token)
            diagnostic.update({"step": step, "position": length - 1})
            diagnostics.append(diagnostic)
            cache = result.past_key_values
            require(cache is not None and cache.get_seq_length() == length, "independent cache length")
            require(len(cache.key_cache) == len(cache.value_cache) == 36, "KV cache layer count")
            for key, value in zip(cache.key_cache, cache.value_cache):
                require(key.dtype == value.dtype == torch.bfloat16, "KV cache BF16")
            ids = torch.tensor([[token]], dtype=torch.long, device="cuda")
            del mask, result, hidden, logits, row
        logits_file.flush()
        os.fsync(logits_file.fileno())
    torch.cuda.synchronize()
    elapsed = time.monotonic_ns() - started
    decoded = decode(generated, skip_special=True)
    raw_decoded = decode(generated, skip_special=False)
    files = [write_new(output / f"pass-{ordinal}.tokens.u32le", struct.pack("<256I", *generated)),
             write_new(output / f"pass-{ordinal}.decoded.bin", decoded),
             write_new(output / f"pass-{ordinal}.decoded-preserve-special.bin", raw_decoded),
             {"path": path.name, "bytes": path.stat().st_size, "sha256": sha_file(path)}]
    result = {"ordinal": ordinal, "tokens": generated, "diagnostics": diagnostics,
              "files": files, "diagnostic_elapsed_ns": elapsed,
              "timing_qualified": False, "fresh_cache": True}
    write_new(output / f"pass-{ordinal}.json", json_bytes(result))
    del cache, ids
    torch.cuda.synchronize()
    return result


def move_bf16_model_preserving_fp32_rope(model, torch, device):
    require(all(parameter.dtype == torch.bfloat16 for parameter in model.parameters()), "loaded weights BF16")
    rotary = model.model.rotary_emb
    require(rotary.rope_type == "default", "reference requires the checkpoint's default RoPE")
    require(rotary.inv_freq.dtype == torch.float32, "loaded rotary inverse frequencies FP32")
    inverse_frequencies = rotary.inv_freq.detach().cpu().clone()
    # A dtype-wide model move would also round the FP32 RoPE buffer to BF16.
    # from_pretrained already established BF16 weights; move devices only.
    model.to(device=device).eval()
    require(rotary.inv_freq.dtype == torch.float32 and
            torch.equal(rotary.inv_freq.detach().cpu(), inverse_frequencies), "device move preserves FP32 RoPE")
    return inverse_frequencies


def reference(plan):
    for key in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE", "HF_DATASETS_OFFLINE"):
        os.environ[key] = "1"
    os.environ["TOKENIZERS_PARALLELISM"] = "false"
    os.environ["CUBLAS_WORKSPACE_CONFIG"] = ":4096:8"
    output = Path(plan["output_root"])
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    cache_root = output / "private-cache"
    cache_root.mkdir(mode=0o700)
    for name in ("XDG_CACHE_HOME", "HF_HOME", "TORCH_HOME", "TRITON_CACHE_DIR", "MIOPEN_USER_DB_PATH"):
        os.environ[name] = str(cache_root / name.lower())
    write_new(output / "input-plan.json", json_bytes(plan))
    root = Path(plan["model_root"])
    sources = checked_sources(root, MODEL_FILES)
    tokenizer, decode, special = tokenizer_and_raw_decoder(root)
    prompt, prompt_manifest = checked_prompt(plan, tokenizer, decode, special)
    import torch
    import transformers
    require(sys.byteorder == "little" and torch.cuda.is_available(), "ROCm reference availability")
    require(torch.cuda.device_count() == 1 and str(torch.version.hip).startswith("7.2"), "ROCm visibility/version")
    properties = torch.cuda.get_device_properties(0)
    require(properties.gcnArchName.split(":")[0] == plan["architecture"], "actual GPU architecture")
    require(torch.cuda.mem_get_info()[0] >= 48 * (1 << 30), "48GiB free GPU memory required")
    torch.manual_seed(0)
    torch.backends.cuda.matmul.allow_tf32 = False
    torch.backends.cudnn.allow_tf32 = False
    torch.backends.cuda.matmul.allow_bf16_reduced_precision_reduction = False
    torch.set_float32_matmul_precision("highest")
    torch.use_deterministic_algorithms(True)
    # Keep backend choice explicit; this is standard independent SDPA, not any
    # Ferric/fe2o3 dispatch or a reused optimized-kernel output.
    torch.backends.cuda.enable_flash_sdp(False)
    torch.backends.cuda.enable_mem_efficient_sdp(False)
    torch.backends.cuda.enable_math_sdp(True)
    torch.backends.cuda.enable_cudnn_sdp(False)
    torch.backends.cuda.allow_fp16_bf16_reduction_math_sdp(False)
    model, loading = transformers.AutoModelForCausalLM.from_pretrained(
        str(root), torch_dtype=torch.bfloat16, attn_implementation="sdpa", local_files_only=True,
        trust_remote_code=False, use_safetensors=True, output_loading_info=True)
    require(model.__class__.__name__ == "Qwen3ForCausalLM", "model class")
    require(all(not loading[name] for name in ("missing_keys", "unexpected_keys", "mismatched_keys", "error_msgs")), "model loading mismatch")
    config = {"hidden_size": 4096, "intermediate_size": 12288, "num_hidden_layers": 36,
              "num_attention_heads": 32, "num_key_value_heads": 8, "vocab_size": VOCABULARY,
              "model_type": "qwen3"}
    require(all(getattr(model.config, name) == value for name, value in config.items()), "model config")
    inverse_frequencies = move_bf16_model_preserving_fp32_rope(model, torch, "cuda")
    require(all(parameter.dtype == torch.bfloat16 for parameter in model.parameters()), "all model weights BF16")
    require(model.config._attn_implementation == "sdpa", "actual attention implementation")
    require(len(model.model.layers) == 36, "layer count")
    unchanged_sources(root, sources)
    model_source = Path(inspect.getsourcefile(model.__class__))
    runtime = {"host": socket.gethostname(), "platform": platform.platform(), "python": sys.version,
               "python_executable": sys.executable, "python_prefix": sys.prefix, "packages": plan["packages"],
               "hip": torch.version.hip, "gpu": properties.name, "gcn_arch": properties.gcnArchName,
               "gpu_unique_id": plan["gpu_unique_id"], "hip_visible_devices": plan["hip_visible_devices"],
               "sdpa_backend": "math-only", "sdpa_low_precision_reduction": False,
               "bf16_reduced_precision_matmul_reduction": False, "deterministic_algorithms": True,
               "module_paths": {"torch": torch.__file__, "transformers": transformers.__file__},
               "model_class_module": model.__class__.__module__, "config": config,
               "model_class_source": {"path": str(model_source), "sha256": sha_file(model_source)},
               "rotary_inverse_frequencies": {"dtype": "float32", "device_move_preserved_bytes": True,
                    "elements": inverse_frequencies.numel(),
                    "sha256": digest(inverse_frequencies.numpy().tobytes()),
                    "values": inverse_frequencies.tolist()},
               "torch_build_configuration": torch.__config__.show(),
               "cublas_workspace_config": os.environ["CUBLAS_WORKSPACE_CONFIG"]}
    write_new(output / "runtime.json", json_bytes(runtime))
    passes = [run_pass(model, torch, prompt, decode, output, index) for index in (1, 2)]
    same_tokens = passes[0]["tokens"] == passes[1]["tokens"]
    same_logits = passes[0]["files"][-1]["sha256"] == passes[1]["files"][-1]["sha256"]
    eos = model.generation_config.eos_token_id
    eos = [eos] if type(eos) is int else eos
    require(isinstance(eos, list) and all(type(value) is int for value in eos), "EOS IDs")
    providers = sorted({line.split()[-1] for line in Path("/proc/self/maps").read_text().splitlines()
                        if "/" in line and any(name in line for name in
                        ("libtorch", "libamdhip", "libhsa-runtime", "librocblas", "libhipblas", "libMIOpen"))})
    result = {"schema": "FerricQwen3LongBf16ReferenceV1", "status": "PASS" if same_tokens and same_logits else "FAIL",
              "model_revision": REVISION, "model_identity": MODEL_ID, "model_sources": sources,
              "prompt_manifest_sha256": plan["prompt_manifest_sha256"], "input_token_ids": prompt,
              "output_tokens": OUTPUT_TOKENS, "passes": passes, "runtime": runtime,
              "loaded_provider_libraries": providers,
              "token_passes_equal": same_tokens, "logit_passes_byte_equal": same_logits,
              "eos_ids": eos, "eos_positions": [[i for i, token in enumerate(item["tokens"]) if token in eos] for item in passes],
              "ignore_eos": True, "use_chat_template": False, "speculative_decoding": False,
              "reference_prefill": "one full2048-token independent matrix-model call per pass",
              "head": "BF16 weights/hidden/output; lowest-ID BF16 argmax",
              "timing_qualified": False, "ferric_validation": False}
    unchanged_sources(root, sources)
    write_new(output / "reference.json", json_bytes(result))
    require(same_tokens and same_logits, "two fresh-cache reference passes disagree; retained FAIL")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["prepare-prompt", "reference"])
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--plan-sha256", required=True)
    args = parser.parse_args()
    require(PROMPT_ENABLED if args.mode == "prepare-prompt" else REFERENCE_ENABLED,
            "source-only disabled; separate reviewed enablement and resource lease required")
    plan = checked_plan(args.plan, args.plan_sha256, args.mode)
    (prepare_prompt if args.mode == "prepare-prompt" else reference)(plan)


if __name__ == "__main__":
    main()
