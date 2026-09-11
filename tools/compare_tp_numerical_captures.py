#!/usr/bin/env python3
"""Compare actual TP1 projection/head operands; no model or timing acceptance."""

import argparse
import hashlib
import heapq
import importlib.util
import json
import math
from pathlib import Path
import struct

REPLAY_SHA256 = "0008f8425f00c2f1668a584fad99a2234bfd27277b261fdaba0f0fcc1916e3e1"
WATCH = (9856, 17689)
IDENTICAL_FIELDS = (
    "controller_sha256", "worker_sha256", "artifact_hsaco_id", "artifact_manifest_id",
    "artifact_handoff_id", "model_bundle_id", "requests_sha256", "tensor_parallel",
    "device_unique_id", "output_head_pruning", "runtime_operational", "runtime_cache_admission",
    "queue_rollover", "collective", "running_worker_sha256", "batch_tokens", "prefill_chunk", "prefix_cache",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_replay(path):
    require(hashlib.sha256(path.read_bytes()).hexdigest() == REPLAY_SHA256, "replay helper SHA-256")
    spec = importlib.util.spec_from_file_location("ferric_frozen_numerical_replay", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def pair_identity(left, right):
    for key in IDENTICAL_FIELDS:
        require(key in left["identity"] and left["identity"][key] == right["identity"][key], f"paired identity: {key}")
    require(left["identity"]["projection"] == "baseline" and right["identity"]["projection"] == "mfma",
            "pair must compare baseline with MFMA")
    require(left["identity"]["session_id"] != right["identity"]["session_id"], "independent capture sessions required")
    for key in ("batch_ordinal", "scheduler_batch_id", "pool_batch_id", "execution_rows"):
        require(left[key] == right[key], f"paired batch/row identity: {key}")
    for key in ("role", "layer", "rows", "n", "k", "world", "tag"):
        require(left["projection"][key] == right["projection"][key], f"paired projection shape: {key}")


def difference(helper, baseline, candidate, width, columns):
    require(width in (2, 4) and len(baseline) == len(candidate) and columns > 0, "paired output extent")
    left, right = helper.words(baseline, width), helper.words(candidate, width)
    require(len(left) % columns == 0, "paired row extent")
    changed, details, absolute = 0, [], []
    for index, (a, b) in enumerate(zip(left, right, strict=True)):
        decode = helper.bf16 if width == 2 else lambda value: struct.unpack("<f", struct.pack("<I", value))[0]
        av, bv = decode(a), decode(b)
        require(math.isfinite(av) and math.isfinite(bv), "nonfinite paired output")
        absolute.append(abs(av-bv))
        if a != b:
            changed += 1
            if len(details) < 32:
                details.append({"row":index//columns, "column":index%columns, "baseline_bits":a,
                                "mfma_bits":b, "baseline":av, "mfma":bv})
    return {"elements":len(left), "different_bits":changed, "first_changed_elements":details,
            "maximum_absolute_difference":max(absolute, default=0),
            "rms_difference":math.sqrt(math.fsum(value*value for value in absolute)/max(1,len(absolute)))}


def check_head_summary(helper, logits, summary, row_identity):
    require(len(logits) == helper.VOCABULARY, "full head vocabulary extent")
    finite = [(helper.bf16(bits), token, bits) for token, bits in enumerate(logits)]
    top = heapq.nlargest(16, finite, key=lambda entry: (entry[0], -entry[1]))
    expected = [{"token":token, "bf16_bits":bits, "value":value} for value, token, bits in top]
    watch = [{"token":token, "bf16_bits":logits[token], "value":helper.bf16(logits[token])} for token in WATCH]
    require(summary["row_identity"] == row_identity and summary["top16"] == expected,
            "final head row/top16 differs from actual full logits")
    require(summary["watch"] == watch and summary["gpu_choice"] == top[0][1], "final head watch/choice")
    require(summary["top1_minus_top2"] == top[0][0]-top[1][0]
            and summary["top_two_tied"] == (top[0][0] == top[1][0]), "head tie/margin")
    return expected, watch


def analyze_head(helper, manifest, payloads):
    head = manifest["head"]
    get = lambda entry: payloads[entry["file"]]
    rows, n, k = head["rows"], head["n"], head["k"]
    require(type(rows) is int and 1 <= rows <= 16 and n == helper.VOCABULARY and k == 4096, "head shape")
    require(rows <= len(manifest["execution_rows"]) and len(head["request_rows"]) == rows, "head row identity count")
    logits = helper.words(get(head["logits"]))
    inputs = helper.words(get(head["input"]))
    weights = helper.words(get(head["selected_weights_nk"]))
    tokens = head["selected_weight_tokens"]
    require(tokens == sorted(set(tokens)) and set(WATCH) <= set(tokens)
            and len(tokens) <= 258 and all(type(token) is int and 0 <= token < n for token in tokens), "selected head weight tokens")
    require(len(logits) == rows*n and len(inputs) == rows*k and len(weights) == len(tokens)*k, "head operand byte extent")
    token_index = {token:index for index,token in enumerate(tokens)}
    results = []
    for row in range(rows):
        row_logits = logits[row*n:(row+1)*n]
        top, watch = check_head_summary(helper, row_logits, head["request_rows"][row], manifest["execution_rows"][row])
        references = []
        for token in sorted({entry["token"] for entry in top} | set(WATCH)):
            require(token in token_index, "top head weight row absent")
            index = token_index[token]
            serial, precise = helper.sampled_dot(inputs[row*k:(row+1)*k], weights[index*k:(index+1)*k])
            observed_bits = row_logits[token]
            observed = helper.bf16(observed_bits)
            references.append({"token":token,"observed_bits":observed_bits,"observed":observed,
                               "serial_fp32":serial,"serial_bf16_bits":helper.narrow(serial),
                               "matches_serial_bf16":observed_bits == helper.narrow(serial),
                               "fp64_product_sum":precise,"observed_abs_error_fp64":abs(observed-precise)})
        by_token = {entry["token"]:entry for entry in references}
        results.append({"row_identity":manifest["execution_rows"][row],"top16":top,"watch":watch,
                        "top1_minus_top2":head["request_rows"][row]["top1_minus_top2"],
                        "top_two_tied":head["request_rows"][row]["top_two_tied"],
                        "watch_9856_minus_17689":{
                            "observed_bf16":by_token[9856]["observed"]-by_token[17689]["observed"],
                            "serial_fp32":by_token[9856]["serial_fp32"]-by_token[17689]["serial_fp32"],
                            "fp64":by_token[9856]["fp64_product_sum"]-by_token[17689]["fp64_product_sum"]},
                        "references":references})
    return results


def compare(helper, left, right, left_payloads, right_payloads):
    pair_identity(left, right)
    projection_a, projection_b = left["projection"], right["projection"]
    data = lambda payloads, entry: payloads[entry["file"]]
    require(data(left_payloads,projection_a["weights_nk"]) == data(right_payloads,projection_b["weights_nk"]),
            "original projection weights changed")
    require(projection_a["output"]["element_bytes"] == projection_b["output"]["element_bytes"], "projection output dtype drift")
    reports = [helper.analyze(left,left_payloads), helper.analyze(right,right_payloads)]
    head_reports = [analyze_head(helper,left,left_payloads), analyze_head(helper,right,right_payloads)]
    require(left["head"]["rows"] == right["head"]["rows"], "head active rows differ")
    left_tokens, right_tokens = left["head"]["selected_weight_tokens"], right["head"]["selected_weight_tokens"]
    left_weights = data(left_payloads,left["head"]["selected_weights_nk"])
    right_weights = data(right_payloads,right["head"]["selected_weights_nk"])
    common = sorted(set(left_tokens) & set(right_tokens))
    for token in common:
        a, b = left_tokens.index(token)*8192, right_tokens.index(token)*8192
        require(left_weights[a:a+8192] == right_weights[b:b+8192], "shared selected LM weight row differs")
    return {"schema":"FerricTpNumericalCapturePairV1","authority":"none","performance_qualified":False,
            "model_parity_qualified":False,"identical_policy_except_projection":True,
            "original_projection_weights_identical":True,"common_head_weight_tokens_exact":common,
            "projection_input":difference(helper,data(left_payloads,projection_a["input"]),data(right_payloads,projection_b["input"]),2,projection_a["k"]),
            "projection_output":difference(helper,data(left_payloads,projection_a["output"]),data(right_payloads,projection_b["output"]),projection_a["output"]["element_bytes"],projection_a["n"]),
            "head_input":difference(helper,data(left_payloads,left["head"]["input"]),data(right_payloads,right["head"]["input"]),2,4096),
            "head_logits":difference(helper,data(left_payloads,left["head"]["logits"]),data(right_payloads,right["head"]["logits"]),2,helper.VOCABULARY),
            "projection_references":{"baseline":reports[0],"mfma":reports[1]},
            "head_references":{"baseline":head_reports[0],"mfma":head_reports[1]},
            "watch_label_scope":"token IDs only; no tokenizer labels assumed",
            "analysis_scope":"actual active operand differences, full transpose, sampled serial/FP64 projection and top/watch head references; not an established MFMA instruction-order model or production fix",
            "custody_scope":"requires separate zero-exit/Closed/trace/reference validation; no timing acceptance"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--baseline-manifest-sha256", required=True)
    parser.add_argument("--mfma", type=Path, required=True)
    parser.add_argument("--mfma-manifest-sha256", required=True)
    parser.add_argument("--replay", type=Path, default=Path(__file__).with_name("replay_tp_numerical_capture.py"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    helper = load_replay(args.replay)
    left,left_payloads,left_sha,_ = helper.load_capture(args.baseline,args.baseline_manifest_sha256)
    right,right_payloads,right_sha,_ = helper.load_capture(args.mfma,args.mfma_manifest_sha256)
    result = compare(helper,left,right,left_payloads,right_payloads)
    result.update({"baseline_manifest_sha256":left_sha,"mfma_manifest_sha256":right_sha,
                   "baseline_identity":left["identity"],"mfma_identity":right["identity"],
                   "replay_sha256":REPLAY_SHA256,"source_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest()})
    with args.output.open("x",encoding="utf-8") as output:
        json.dump(result,output,sort_keys=True,indent=2,allow_nan=False)
        output.write("\n")


if __name__ == "__main__":
    main()
