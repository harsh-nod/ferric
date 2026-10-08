"""Fixed-original stdlib host report; no model, retainer, or helper execution."""
from decimal import Decimal, ROUND_HALF_EVEN, localcontext
import hashlib
from html import escape
import io
import json
import os
from pathlib import Path, PurePosixPath
import resource
import shutil
import signal
import stat
import sys
import tarfile

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
ARCHIVE_PATH = E / "guarded-mlp-readiness40-tail-forward-duration-retention-v228-v1.tar.gz"
ARCHIVE = dict(bytes=4595635, sha256="d22c87f34e686653caa3cdd5584e8b37569845455eb2d398f19d81902209403e")
RECEIPT_PATH = E / "guarded-mlp-forward-phases-native-source-v228-v1/native-retention-result-v1.json"
RECEIPT = dict(bytes=125598, sha256="2164b10d39096dbc81f4f4f24d6fd2e083dfc0b5ad9e3326bbd1a9fb07128b9d")
TERMINAL = dict(bytes=812186, sha256="aa5d9b023c8ca9b7cb916bc60d41282804769479b294a01ab619787ed0a4dc56")
OUTPUT = E / "guarded-mlp-forward-duration-report-v228-v1"
PHASES = ("input", "metadata", "embedding", "bank", "layers", "tail", "frame", "fence", "commit")
SCOPES = ("bank", "layers", "tail")
CALLBACKS = ("before", "discover", "after", "root_generation")
PARENT = ("prepare_write", "flush_to_frame_read", "validate_retain_commit")
HOUR_NS = 3_600_000_000_000


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path, expected=None, cap=16 << 20):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= cap, "bounded ordinary input")
    with path.open("rb") as stream:
        raw = stream.read(cap + 1)
        opened = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(opened) == stamp(path.lstat())
            and len(raw) == before.st_size and (expected is None or pin(raw) == expected), "input pin/drift")
    return raw


def pairs(items):
    value = {}
    for key, item in items:
        require(key not in value, "duplicate JSON key")
        value[key] = item
    return value


def decode(raw):
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))


def uint(value, limit=(1 << 64) - 1):
    require(type(value) is int and 0 <= value <= limit, "exact bounded unsigned integer")
    return value


def total(values, limit=(1 << 64) - 1):
    answer = sum(uint(v, limit) for v in values)
    require(answer <= limit, "bounded integer sum")
    return answer


def display(value, denominator=1_000_000_000, digits=6):
    with localcontext() as context:
        context.prec = 80
        return format((Decimal(value) / Decimal(denominator)).quantize(
            Decimal(1).scaleb(-digits), rounding=ROUND_HALF_EVEN), "f")


def archive_inputs():
    bodies, expanded = {}, 0
    with tarfile.open(fileobj=io.BytesIO(read(ARCHIVE_PATH, ARCHIVE)), mode="r:gz") as archive:
        for member in archive:
            name, parts = member.name, PurePosixPath(member.name).parts
            require(member.isfile() and not member.issym() and not member.islnk()
                    and parts and not name.startswith("/") and "\\" not in name
                    and PurePosixPath(name).as_posix() == name
                    and all(p not in ("", ".", "..") for p in parts)
                    and name not in bodies and 0 <= member.size <= 8 << 20, "safe original member")
            expanded += member.size
            require(len(bodies) < 169 and expanded <= 16 << 20, "closed bounded archive")
            stream = archive.extractfile(member)
            require(stream is not None, "ordinary archive stream")
            body = stream.read(member.size + 1)
            require(len(body) == member.size, "complete original member")
            bodies[name] = body
    manifest = decode(bodies["manifest.json"])
    require(len(bodies) == 169 and len(manifest["files"]) == 168
            and set(manifest["files"]) == set(bodies) - {"manifest.json"}
            and all(pin(bodies[n]) == p for n, p in manifest["files"].items()), "all168 original pins")
    require(manifest["schema"] == "ferric-guarded-mlp-readiness40-tail-forward-duration-retention-v1"
            and manifest["outcomes"] == {"tail_forward": {"name": "complete.json", "sha256": TERMINAL["sha256"]}}
            and pin(bodies["tail_forward/complete.json"]) == TERMINAL, "fixed original outcome")
    receipt = decode(read(RECEIPT_PATH, RECEIPT))
    observation = manifest["observation"]
    require({k: v for k, v in receipt.items() if k not in ("archive", "destination", "members", "expanded_bytes")}
            == observation and receipt["members"] == 169 and receipt["expanded_bytes"] == expanded
            and {k: receipt["archive"][k] for k in ("bytes", "sha256")} == ARCHIVE, "original retention receipt")
    case = observation["cases"]["tail_forward"]
    require(observation["passed"] is True and observation["original_owned_lineage_revalidated"] is True
            and observation["semantic_and_payload_parity_revalidated"] is True
            and case["present"] is True and case["original_passed"] is True
            and case["retained_success_revalidated"] is True and case["original_raw_files"] == 73,
            "healthy retained original case")
    terminal = decode(bodies["tail_forward/complete.json"])
    require(terminal["passed"] is True and terminal["errors"] == []
            and terminal["native_attempts"] == 1 and terminal["generated_tokens_requested"] == 0
            and len(terminal["phases"]) == 11, "one prompt-only original attempt")
    for row in terminal["phases"]:
        require(row["exit_code"] == 0 and row["owned_groups_absent"] is True
                and row["owned_processes_reaped"] is True and row["cleanup_signalled"] is False
                and row["reason"] is None, "original owned retirement")
    return bodies, case["matched_timing"]


def worker_records(bodies, checked):
    raw = bodies["tail_forward/native/child-stderr.bin"]
    lines = raw.splitlines(keepends=True)
    require(len(raw) <= 69632 and len(lines) == 3 and all(v.endswith(b"\n") for v in lines)
            and all(len(v) <= cap for v, cap in zip(lines, (4096, 65536, 32768))), "three bounded LF records")
    records = [decode(line) for line in lines]
    require(all((json.dumps(v, separators=(",", ":"), ensure_ascii=False, allow_nan=False) + "\n").encode() == line
                for v, line in zip(records, lines)), "canonical original records")
    policy, callbacks, forward = records
    require(policy["schema"] == "FerricReadiness40Position5BankScopedWarmCensusTailPolicyV4"
            and callbacks["schema"] == "FerricReadiness40TailCurrentnessDurationsV1"
            and forward["schema"] == "FerricReadiness40ForwardPhaseDurationsV1", "original schemas")
    require(callbacks["policy_sha256"] == forward["policy_sha256"] == list(hashlib.sha256(lines[0]).digest())
            and forward["currentness_record_sha256"] == list(hashlib.sha256(lines[1]).digest()),
            "LF-inclusive cross-hashes")
    for key in ("session", "worker_sha256", "transcript_sha256"):
        require(policy[key] == callbacks[key] == forward[key], "same original identity")
    require(policy["native_closed"] is True and policy["completed_forwards"] == 40
            and policy["generated_tokens"] == [] and policy["capture_positions"] == [0, 5, 16, 39],
            "healthy prompt-only extent")
    for record in (callbacks, forward):
        require(record["instrumented"] is True and record["host_elapsed_nanoseconds"] is True
                and all(record[k] is False for k in ("numerical_acceptance", "performance_claim", "execution_authority")),
                "host-only flags")
    require(forward["gpu_timing"] is False and forward["disjoint_phases"] is True
            and forward["currentness_durations_nested"] is True and forward["phase_order"] == list(PHASES)
            and callbacks["bank_guarded_body_includes_callbacks"] is True, "fixed nested contract")
    joined = checked["policy"]
    require(joined["policy_record"] == policy and joined["diagnostic_record"] == callbacks
            and joined["forward_record"] == forward
            and {k: joined["file"][k] for k in ("bytes", "sha256")} == pin(raw), "whole original stderr custody")
    require(checked["ordinary"]["currentness_duration_diagnostic"] == callbacks
            and checked["ordinary"]["forward_phase_diagnostic"] == forward, "duplicate admitted observations")
    require(len(forward["forwards"]) == len(callbacks["forwards"]) == 40, "forty exact rows")
    rows, calls = [], {g: {k: 0 for k in CALLBACKS} for g in SCOPES}
    for position, (row, nested) in enumerate(zip(forward["forwards"], callbacks["forwards"])):
        require(set(row) == {"position", "phase_ns", "forward_body_ns"}
                and uint(row["position"]) == uint(nested["position"]) == position
                and set(nested) == {"position", "measured"} and len(row["phase_ns"]) == 9, "ordered typed rows")
        durations = [uint(v, HOUR_NS) for v in row["phase_ns"]]
        require(total(durations, HOUR_NS) == uint(row["forward_body_ns"], HOUR_NS), "nine-phase body sum")
        measured, detail = nested["measured"], None
        if position < 2:
            require(measured is None, "ordinary callback rows unmeasured")
        else:
            require(set(measured) == {*SCOPES, "bank_guarded_body_ns"}, "three callback groups")
            sums = {}
            for g in SCOPES:
                require(set(measured[g]) == set(CALLBACKS), "four callback categories")
                for k in CALLBACKS:
                    part = measured[g][k]
                    require(set(part) == {"calls", "elapsed_ns"}, "typed callback pair")
                    calls[g][k] = total((calls[g][k], uint(part["calls"])))
                    uint(part["elapsed_ns"], HOUR_NS)
                sums[g] = total((measured[g][k]["elapsed_ns"] for k in CALLBACKS), HOUR_NS)
            require([measured["bank"][k]["calls"] for k in CALLBACKS] == [726, 2, 726, 725], "per-row bank census")
            bank_body = uint(measured["bank_guarded_body_ns"], HOUR_NS)
            require(sums["bank"] <= bank_body <= durations[3]
                    and sums["layers"] <= durations[4] and sums["tail"] <= durations[5], "same-process containment")
            detail = dict(callbacks=measured, callback_ns=sums, bank_guarded_body_ns=bank_body,
                          layer_other_ns=durations[4] - sums["layers"])
        rows.append(dict(position=position, phase_ns=dict(zip(PHASES, durations)),
                         forward_body_ns=row["forward_body_ns"], measured_callbacks=detail))
    total((row["forward_body_ns"] for row in rows), HOUR_NS)
    for g, group in zip(SCOPES, ("banks", "layers", "tails")):
        for k, field in zip(CALLBACKS, ("before_calls", "full_discoveries", "after_calls", "generation_probes")):
            require(calls[g][k] == uint(policy["counts"][group][field]), "calls join original policy")
    return rows, dict(policy=pin(lines[0]), currentness=pin(lines[1]), forward=pin(lines[2]))


def parent_timeline(bodies, checked, rows):
    timeline = decode(bodies["tail_forward/native/host-timing.json"])["timeline"]
    require(timeline == checked["timing"]["timeline"] and checked["timing"]["disjoint_spans"] == 124
            and len(timeline["forwards"]) == 40, "original124 parent spans")
    cursor, spans = 0, 0
    def span(value):
        nonlocal cursor, spans
        require(set(value) == {"start_ns", "end_ns", "elapsed_ns"}
                and uint(value["start_ns"], HOUR_NS) == cursor, "contiguous parent span")
        end, elapsed = uint(value["end_ns"], HOUR_NS), uint(value["elapsed_ns"], HOUR_NS)
        require(end >= cursor and end - cursor == elapsed, "exact parent span")
        cursor, spans = end, spans + 1
        return elapsed
    groups = {k: span(timeline[k]) for k in ("source_preparation", "spawn_to_setup_seal")}
    groups.update({k: 0 for k in PARENT})
    for position, parent in enumerate(timeline["forwards"]):
        require(uint(parent["position"]) == position and uint(parent["generation"]) == position + 1, "parent order")
        values = {k: span(parent[k]) for k in PARENT}
        require(total(values.values(), HOUR_NS) == uint(parent["elapsed_ns"], HOUR_NS), "parent forward sum")
        for k in PARENT:
            groups[k] += values[k]
        rows[position]["parent_ns"] = dict(values, forward=parent["elapsed_ns"])
        # Cross-process intervals have different boundaries, not exclusive residuals.
        rows[position]["parent_minus_body_ns"] = {
            "flush_to_frame_read": values["flush_to_frame_read"] - rows[position]["forward_body_ns"],
            "whole_forward": parent["elapsed_ns"] - rows[position]["forward_body_ns"]}
    for k in ("close_and_retirement", "postcheck_and_ordinary_publication"):
        groups[k] = span(timeline[k])
    require(spans == 124 and cursor == uint(timeline["total_ns"], HOUR_NS)
            == total(groups.values(), HOUR_NS), "whole parent sum")
    return dict(groups_ns=groups, total_ns=cursor, disjoint_spans=spans)


def reduce_rows(rows):
    subsets = {}
    for label, selected in (("first_use", rows[:2]), ("warm", rows[2:]), ("all", rows)):
        phases = {k: total(row["phase_ns"][k] for row in selected) for k in PHASES}
        body = total(row["forward_body_ns"] for row in selected)
        require(total(phases.values()) == body and body > 0, "subset reconciliation")
        subsets[label] = dict(positions=[row["position"] for row in selected], phase_ns=phases, body_ns=body,
            phase_percent={k: display(v * 100, body) for k, v in phases.items()},
            parent_ns={k: total(row["parent_ns"][k] for row in selected) for k in (*PARENT, "forward")},
            signed_parent_minus_body_ns={k: sum(row["parent_minus_body_ns"][k] for row in selected)
                                        for k in ("flush_to_frame_read", "whole_forward")})
    warm = rows[2:]
    categories = {g: {k: dict(calls=total(row["measured_callbacks"]["callbacks"][g][k]["calls"] for row in warm),
                            elapsed_ns=total(row["measured_callbacks"]["callbacks"][g][k]["elapsed_ns"] for row in warm))
                      for k in CALLBACKS} for g in SCOPES}
    nested = dict(categories=categories,
        callback_ns={g: total(categories[g][k]["elapsed_ns"] for k in CALLBACKS) for g in SCOPES},
        bank_guarded_body_ns=total(row["measured_callbacks"]["bank_guarded_body_ns"] for row in warm),
        layer_other_ns=total(row["measured_callbacks"]["layer_other_ns"] for row in warm))
    require(nested["callback_ns"]["layers"] + nested["layer_other_ns"] == subsets["warm"]["phase_ns"]["layers"],
            "nested layer decomposition")
    return subsets, nested


def svg(report):
    warm, nested = report["subsets"]["warm"], report["warm_nested"]
    maximum = max(warm["phase_ns"].values())
    width = lambda n: display(n * 650, maximum, 3)
    items = ['<svg xmlns="http://www.w3.org/2000/svg" width="1100" height="790" viewBox="0 0 1100 790">',
        '<rect width="1100" height="790" fill="white"/>',
        '<g font-family="sans-serif" font-size="15" fill="#17212b">',
        '<text x="32" y="38" font-size="24">Warm forward host phases</text>',
        '<text x="32" y="67">Positions 2-39, one instrumented run. Host time includes checks and waiting.</text>',
        '<text x="230" y="105">Seconds; nine adjacent worker intervals</text>']
    for index, name in enumerate(PHASES):
        y, value = 130 + index * 45, warm["phase_ns"][name]
        items += [f'<text x="32" y="{y + 20}">{escape(name)}</text>',
                  f'<rect x="230" y="{y}" width="{width(value)}" height="28" fill="#27866b"/>',
                  f'<text x="904" y="{y + 20}">{display(value)} s</text>']
    items += ['<text x="32" y="573" font-size="20">Nested layer detail, already inside the layers bar</text>']
    for index, (name, value, color) in enumerate((
            ("Layer callbacks", nested["callback_ns"]["layers"], "#3279ad"),
            ("Other layer interval", nested["layer_other_ns"], "#888d94"))):
        y = 603 + index * 45
        items += [f'<text x="32" y="{y + 20}">{name}</text>',
                  f'<rect x="230" y="{y}" width="{width(value)}" height="28" fill="{color}"/>',
                  f'<text x="904" y="{y + 20}">{display(value)} s</text>']
    items += ['<text x="32" y="726">Nested bars share the phase scale; do not add them to the nine phase bars.</text>',
              '<text x="32" y="753">Not GPU timing, overlap, speedup, throughput, or an optimization ceiling.</text>',
              '</g></svg>\n']
    return "\n".join(items).encode()


def markdown(report):
    first, warm = (report["subsets"][k] for k in ("first_use", "warm"))
    nested, parent = report["warm_nested"], report["parent"]
    lines = ["# Forward-Phase Host Attribution", "",
        "One retained TP2 BF16 Readiness40 diagnostic: 40 prompt forwards, zero generated tokens.",
        "This report reads authenticated originals only; it does not rerun a model or GPU.", "",
        "## Worker Phases", "", "First use is positions 0-1; warm is positions 2-39.",
        "Nanosecond reductions are exact integers. Seconds and percentages use Decimal half-even display.", "",
        "| Phase | First Use (s) | Warm (s) | Warm Body Share |", "| --- | ---: | ---: | ---: |"]
    for name in PHASES:
        lines.append(f"| {name} | {display(first['phase_ns'][name])} | {display(warm['phase_ns'][name])} | {warm['phase_percent'][name]}% |")
    lines += [f"| Total worker body | {display(first['body_ns'])} | {display(warm['body_ns'])} | 100.000000% |", "",
        "![Warm host phase attribution](host-phases.svg)", "",
        "## Nested Callbacks", "", "These intervals are already inside the worker phases; they are not additional time.",
        "The first two callback rows are unmeasured, although their forward phases are measured.", "",
        "| Warm Nested Interval | Seconds |", "| --- | ---: |"]
    for label, value in (("Bank callbacks", nested["callback_ns"]["bank"]),
                         ("Bank guarded body, including its callbacks", nested["bank_guarded_body_ns"]),
                         ("Layer callbacks", nested["callback_ns"]["layers"]),
                         ("Other layer interval", nested["layer_other_ns"]),
                         ("Tail callbacks", nested["callback_ns"]["tail"])):
        lines.append(f"| {label} | {display(value)} |")
    lines += ["", "The other layer interval includes arithmetic, polling, checks and other work not classified",
        "as these callbacks. It is neither measured GPU time nor a removable fraction.", "",
        "## Parent Timeline", "", "The following separate table sums the 124 contiguous parent spans.",
        "Forward subsets and worker timings overlap this timeline; do not add them to it.", "",
        "| Parent Group | Seconds |", "| --- | ---: |"]
    for label, value in parent["groups_ns"].items():
        lines.append(f"| {label} | {display(value)} |")
    lines += [f"| Total parent timeline | {display(parent['total_ns'])} |", "",
        "## Signed Boundary Comparisons", "",
        "| Subset | Parent Flush-to-Read Minus Worker Body (ns) | Whole Parent Forward Minus Worker Body (ns) |",
        "| --- | ---: | ---: |"]
    for label in ("first_use", "warm", "all"):
        delta = report["subsets"][label]["signed_parent_minus_body_ns"]
        lines.append(f"| {label} | {delta['flush_to_frame_read']} | {delta['whole_forward']} |")
    delta14 = report["forwards"][14]["parent_minus_body_ns"]
    lines += ["", f"Position 14's flush-to-read difference is {delta14['flush_to_frame_read']} ns.",
        "Differences remain signed and are not clamped. The parent marks Flushed after flush returns;",
        "the worker can already be processing the request. These different process/boundary intervals",
        "are not exclusive unmeasured timers, and neither comparison establishes an optimization ceiling.", "",
        "## Next Measurement", "",
        f"Prioritize the layer interval: {warm['phase_percent']['layers']}% of the warm worker body.",
        f"Its {display(nested['layer_other_ns'], digits=9)} s noncallback remainder is mixed work, not measured GPU compute.",
        "The next bounded measurement should separate the closed layer engine's prefix, MLP and hidden-state work,",
        "then the paired MLP preflight/consume/reserve/publish/poll/retire/terminal path in",
        "engineering_gfx950_peer_combined_mlp_paired_v1.rs:123. Preserve all polling, currentness checks and durability.",
        "This is a measurement priority, not a proposed removal or a predicted speedup.", "",
        "## Source Boundaries", "",
        "- Worker guarded_mlp_long_sequence_v2.rs timestamps at lines 114/125/129/133/137/144/148/172/176/185 delimit the nine phases.",
        "- native_guarded_mlp_readiness_v1.rs:361 constructs Driver outside the timed body. Final row acceptance and wire publication are also outside.",
        "- finite_guarded_mlp_long_wire_v2.rs:380 frame construction/hash work is inside frame; write_frame's recomputation at line 529 is outside.",
        "- Parent readiness.rs:631 reads before Read at line 638; append at line 644 precedes Committed at line 658.",
        "- finite_guarded_mlp_long_wire_v2.rs:557-598 capture validation is inside parent read. readiness_evidence.rs:144-189 capture retention/sync/rehash and per-frame sync_data are inside retain/commit.", "",
        "These are qualified source boundaries, not inferred GPU kernel boundaries.", "",
        "## Original Evidence", "",
        "- [Original 169-member archive](../native-gpu-v1.tar.gz)",
        "- [Original native terminal](native-complete.json)",
        "- [Original three-record stderr](child-stderr.bin)",
        "- [Original parent timing](host-timing.json)",
        "- [Original retention result](retention-result.json)",
        "- [Data-only reporter source](report_forward_phases.py)",
        "- [Exact integer report and provenance](report.json)", "",
        "Archive SHA256: " + ARCHIVE["sha256"] + ".",
        "Native terminal SHA256: " + TERMINAL["sha256"] + ".", "",
        "All 169 archive members and 168 manifest pins are authenticated. The original LF-inclusive",
        "policy/currentness/phase records, cross-hashes, all 40 phase sums, callback containment",
        "and parent span sums are rechecked. Full ordinary admission and historical payload parity",
        "were performed by the separately retained strict retainer; this reducer is not a replacement validator.", "",
        "The reporter is a fixed-input data-only MI350 recipe; it never imports the archived helpers.",
        "The JSON retains each signed row and all twelve callback categories for independent recomputation.", "",
        "No GPU timing, overlap, speedup, generated-token correctness, Full2303 acceptance or 700 tokens/s claim is made.",
        "The independent position-5 numerical diagnostic is not changed by this instrumentation.", ""]
    return "\n".join(lines).encode()


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, "python3 -B report_forward_phases.py only")
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == "smci350-rck-g03-b19-03", "exact unprivileged host")
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10
            and all(os.environ.get(k) == "" for k in ("HIP_VISIBLE_DEVICES", "ROCR_VISIBLE_DEVICES", "CUDA_VISIBLE_DEVICES")),
            "CPU8,9 nice10 hidden GPUs")
    require(not os.path.lexists(OUTPUT), "fresh exclusive report directory")
    require(shutil.disk_usage(E).free >= 40 << 30, "initial40GiB free floor")
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
    resource.setrlimit(resource.RLIMIT_FSIZE, (4 << 20, 4 << 20))
    signal.alarm(90)
    bodies, checked = archive_inputs()
    rows, line_pins = worker_records(bodies, checked)
    parent = parent_timeline(bodies, checked, rows)
    subsets, nested = reduce_rows(rows)
    source = read(Path(__file__).resolve(), cap=1 << 20)
    report = dict(schema="ferric-forward-phase-host-report-v1",
        provenance=dict(archive=ARCHIVE, terminal=TERMINAL, retention_receipt=RECEIPT,
                        reporter=pin(source), records=line_pins),
        phase_order=list(PHASES), forwards=rows, subsets=subsets, warm_nested=nested, parent=parent,
        prompt_forwards=40, generated_tokens=0, native_rerun=False, gpu_timing=False,
        overlap_claim=False, performance_claim=False, optimization_ceiling_claim=False,
        numerical_acceptance=False, generated_token_correctness=False, full2303_acceptance=False)
    outputs = {"report.json": (json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n").encode(),
               "README.md": markdown(report), "host-phases.svg": svg(report)}
    require(all(len(raw) <= 4 << 20 for raw in outputs.values()), "bounded report outputs")
    require(read(Path(__file__).resolve(), pin(source), 1 << 20) == source, "report source unchanged")
    require(shutil.disk_usage(E).free >= 38 << 30, "prewrite38GiB free floor")
    OUTPUT.mkdir(mode=0o700)
    for name, raw in outputs.items():
        with (OUTPUT / name).open("xb") as stream:
            stream.write(raw)
        require(read(OUTPUT / name, pin(raw), 4 << 20) == raw, "original output rehash")
    require(shutil.disk_usage(E).free >= 38 << 30, "final38GiB free floor")
    signal.alarm(0)
    print(json.dumps(dict(directory=str(OUTPUT), files={n: pin(b) for n, b in outputs.items()}), sort_keys=True))


if __name__ == "__main__":
    main()
