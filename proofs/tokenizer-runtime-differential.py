#!/usr/bin/env python3
"""Generate or check the pinned Qwen3 tokenizer differential corpus."""

from __future__ import annotations

import argparse
import hashlib
import random
from pathlib import Path

import tokenizers
from tokenizers import Tokenizer


TOKENIZERS_VERSION = "0.22.2"
TOKENIZERS_WHEEL_SHA256 = "369cc9fc8cc10cb24143873a0d95438bb8ee257bb80c71989e3ee290e8d72c67"
SPLIT_RUNTIME = "onig-6.5.3+onig_sys-69.9.3"
SPLIT_UNICODE = "16.0.0"
NFC_RUNTIME = "unicode-normalization-alignments-0.1.12"
NFC_UNICODE = "9.0.0"
TOKENIZER_BYTES = 11_422_654
TOKENIZER_SHA256 = "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4"
SEED = 0xF3_022_022
CASE_COUNT = 640


def corpus() -> list[str]:
    curated = [
        "",
        "hello",
        "Hello world",
        "hello   world",
        "I'm testing!",
        "I'RE I'Ve I'LL I'D I'M I'S I'T",
        "i'Re i'vE i'lL i'D i'M i'S i'T",
        "1 23\nnext",
        "tabs\twork",
        "line\r\nend",
        "line\rend\n",
        " punctuation!?",
        "trailing   ",
        "e\u0301",
        "\u00e9",
        "A\u030a ngstro\u0308m",
        "\u00c5 ngstr\u00f6m",
        "\u4e2d\u6587",
        "\u65e5\u672c\u8a9e",
        "\ud55c\uad6d\uc5b4",
        "\u0627\u0644\u0639\u0631\u0628\u064a\u0629 \u0661\u0662\u0663",
        "\u0939\u093f\u0928\u094d\u0926\u0940 \u0967\u0968\u0969",
        "\u0e20\u0e32\u0e29\u0e32\u0e44\u0e17\u0e22 \u0e51\u0e52\u0e53",
        "\u0395\u03bb\u03bb\u03b7\u03bd\u03b9\u03ba\u03ac",
        "\u0420\u0443\u0441\u0441\u043a\u0438\u0439",
        "emoji \U0001f469\u200d\U0001f4bb\U0001f3f3\ufe0f\u200d\U0001f308",
        "\U000104b0\U000104d8",
        "\U0001e900\U0001e922",
        "\U00011f02\U00011f04",
        "\U0001e4d0\U0001e4eb",
        "\U00011f50\U00011f51",
        "\U0001e2f0\U0001e2f1",
        "a\u0085b",
        "a\u00a0b",
        "a\u1680b",
        "a\u2000\u2001\u200ab",
        "a\u2028\u2029b",
        "a\u202fb",
        "a\u205fb",
        "a\u3000b",
        "x \t\r\n y",
        "x\n\n\r\r\ny",
        "a  b",
        "a   ",
        "  a",
        "!a ?\u03b2 #\U000104b0",
        "99\u00b2\u216b\U00011f50",
        "\u0301a\u0301",
        "\u200ba\u2060b",
        "<think>ok</think>",
        "<|im_start|>user\nHello<|im_end|>",
        "prefix<|endoftext|>suffix",
        "<tool_call>{}</tool_call>",
    ]
    units = [
        "a",
        "Z",
        "\u00e9",
        "e\u0301",
        "\u4e2d",
        "\u03b2",
        "\u0416",
        "\u0634",
        "\u0939",
        "\U000104b0",
        "\U0001e900",
        "\U00011f02",
        "\U0001e4d0",
        "7",
        "\u0661",
        "\u216b",
        "\U00011f50",
        "\U0001e2f0",
        " ",
        "  ",
        "\t",
        "\n",
        "\r\n",
        "\u0085",
        "\u00a0",
        "\u2003",
        "\u2028",
        "\u202f",
        "\u3000",
        "!",
        "??",
        "_",
        "\U0001f469\u200d\U0001f4bb",
        "\u0301",
        "\u200b",
        "'RE",
        "'ve",
        "'Ll",
        "'D",
        "<think>",
        "</think>",
        "<|im_start|>",
        "<|im_end|>",
    ]
    seen = set(curated)
    rng = random.Random(SEED)
    while len(curated) < CASE_COUNT:
        candidate = "".join(rng.choice(units) for _ in range(rng.randrange(1, 18)))
        if candidate not in seen:
            seen.add(candidate)
            curated.append(candidate)
    return curated


def render(tokenizer_path: Path) -> bytes:
    if tokenizers.__version__ != TOKENIZERS_VERSION:
        raise SystemExit(
            f"expected tokenizers {TOKENIZERS_VERSION}, found {tokenizers.__version__}"
        )
    payload = tokenizer_path.read_bytes()
    if len(payload) != TOKENIZER_BYTES:
        raise SystemExit(f"unexpected tokenizer size: {len(payload)}")
    digest = hashlib.sha256(payload).hexdigest()
    if digest != TOKENIZER_SHA256:
        raise SystemExit(f"unexpected tokenizer SHA-256: {digest}")
    oracle = Tokenizer.from_str(payload.decode("utf-8"))
    lines = [
        "# FERRIC-QWEN3-TOKENIZER-DIFFERENTIAL-V1",
        f"# tokenizers={TOKENIZERS_VERSION}",
        f"# tokenizers-wheel-sha256={TOKENIZERS_WHEEL_SHA256}",
        f"# tokenizer-sha256={TOKENIZER_SHA256}",
        f"# split-runtime={SPLIT_RUNTIME}",
        f"# split-unicode={SPLIT_UNICODE}",
        f"# nfc-runtime={NFC_RUNTIME}",
        f"# nfc-unicode={NFC_UNICODE}",
        "# authority=differential-test-only",
        "# nonclaim=no-verus-or-general-tokenizers-equivalence",
        f"# seed={SEED}",
        f"# cases={CASE_COUNT}",
    ]
    for text in corpus():
        ids = oracle.encode(text, add_special_tokens=False).ids
        input_hex = text.encode("utf-8").hex() or "-"
        token_ids = ",".join(str(value) for value in ids) or "-"
        lines.append(f"{input_hex}\t{token_ids}")
    return ("\n".join(lines) + "\n").encode("ascii")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tokenizer", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    rendered = render(args.tokenizer)
    if args.write:
        args.output.write_bytes(rendered)
        print(f"PASS: wrote {CASE_COUNT} exact tokenizers {TOKENIZERS_VERSION} cases")
        return
    try:
        admitted = args.output.read_bytes()
    except OSError as error:
        raise SystemExit(f"cannot read differential corpus: {error}") from error
    if admitted != rendered:
        raise SystemExit("FAIL: tokenizer differential corpus drifted")
    print(f"PASS: {CASE_COUNT} exact tokenizers {TOKENIZERS_VERSION} cases match")


if __name__ == "__main__":
    main()
