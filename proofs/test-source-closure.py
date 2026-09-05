#!/usr/bin/env python3
"""Test release source-closure coverage and canonical permission identity."""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import tempfile


SCRIPT = Path(__file__).with_name("source-closure.py")


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def measure(repo: Path, output: Path) -> bytes:
    result = subprocess.run(
        [sys.executable, "-I", str(SCRIPT), str(repo), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if result.returncode != 0:
        fail(f"source-closure fixture measurement failed:\n{result.stdout}")
    return output.read_bytes()


def require_measurement_rejection(repo: Path, output: Path, expected: str) -> None:
    result = subprocess.run(
        [sys.executable, "-I", str(SCRIPT), str(repo), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if result.returncode == 0:
        fail(f"source-closure fixture unexpectedly accepted {expected}")
    if expected not in result.stdout:
        fail(f"source-closure rejection omitted {expected!r}:\n{result.stdout}")


def write(path: Path, content: str, mode: int = 0o644) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="ascii")
    path.chmod(mode)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="ferric-source-closure-test.") as raw:
        root = Path(raw)
        repo = root / "repo"
        files = {
            "Cargo.toml": "[workspace]\n",
            "Cargo.lock": "version = 4\n",
            "rust-toolchain.toml": '[toolchain]\nchannel = "test"\n',
            ".github/workflows/verus.yml": "name: test\n",
            "adapters/example/src/lib.rs": "pub fn adapt() {}\n",
            "benches/m1/benchmark.rs": "fn main() {}\n",
            "crates/example/src/lib.rs": "pub fn example() {}\n",
            "device/example/src/lib.rs": "pub fn kernel() {}\n",
            "docs/README.md": "# Test\n",
            "generated/example.rs": "pub const GENERATED: bool = true;\n",
            "proofs/policy.py": "#!/usr/bin/env python3\n",
            "services/example/src/lib.rs": "pub fn serve() {}\n",
        }
        for relative, content in files.items():
            mode = 0o755 if relative == "proofs/policy.py" else 0o644
            write(repo / relative, content, mode)

        baseline = measure(repo, root / "baseline.records")
        records = baseline.decode("ascii").splitlines()
        if not any(row.startswith("adapters/example/src/lib.rs|644|") for row in records):
            fail("adapter sources are absent from the release closure")
        if not any(row.startswith("benches/m1/benchmark.rs|644|") for row in records):
            fail("benchmark sources are absent from the release closure")
        if not any(row.startswith("device/example/src/lib.rs|644|") for row in records):
            fail("device sources are absent from the release closure")
        if not any(row.startswith("services/example/src/lib.rs|644|") for row in records):
            fail("service sources are absent from the release closure")
        if not any(row.startswith("proofs/policy.py|755|") for row in records):
            fail("executable source mode is not retained")

        external_lock = root / "external-Cargo.lock"
        write(external_lock, files["Cargo.lock"])
        (repo / "Cargo.lock").unlink()
        (repo / "Cargo.lock").symlink_to(external_lock)
        require_measurement_rejection(
            repo, root / "fixed-symlink.records", "source closure contains a symlink"
        )
        (repo / "Cargo.lock").unlink()
        write(repo / "Cargo.lock", files["Cargo.lock"])

        external_services = root / "external-services"
        (repo / "services").rename(external_services)
        (repo / "services").symlink_to(external_services, target_is_directory=True)
        require_measurement_rejection(
            repo, root / "root-symlink.records", "source closure contains a symlink"
        )
        (repo / "services").unlink()
        external_services.rename(repo / "services")

        for path in repo.rglob("*"):
            if path.is_file():
                path.chmod(0o775 if path.stat().st_mode & 0o111 else 0o664)
        shared_checkout = measure(repo, root / "shared.records")
        if shared_checkout != baseline:
            fail("source closure changed with group-writable checkout permissions")

        device_source = repo / "device/example/src/lib.rs"
        device_source.chmod(0o674)
        group_executable = measure(repo, root / "group-executable.records")
        if group_executable != baseline:
            fail("source closure encoded an executable bit Git does not preserve")

        device_source.chmod(0o755)
        executable_drift = measure(repo, root / "executable.records")
        if executable_drift == baseline:
            fail("source closure ignored an executable-bit change")

    print("PASS: release source-closure coverage and permission policy")


if __name__ == "__main__":
    main()
