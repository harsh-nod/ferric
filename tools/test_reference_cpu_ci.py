"""Off-roster stdlib regression/hostile controls for ordinary reference CI."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile


SOURCE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("reference_cpu_ci", SOURCE / "tools/reference_cpu_ci.py")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class ProfileTests(unittest.TestCase):
    def test_site_inventory_rejects_all_symlink_entries_and_ancestors(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            site = root / "venv/lib/python3.12/site-packages"
            site.mkdir(parents=True)
            payload = site / "payload.py"
            payload.write_text("pass\n")
            outside = root / "outside"
            outside.mkdir()
            foreign = outside / "foreign.py"
            foreign.write_text("pass\n")
            self.assertEqual(ci.site_inventory_files(site), {payload})
            for target in (foreign, root / "missing", outside):
                link = site / "extra"
                link.symlink_to(target, target_is_directory=target == outside)
                with self.subTest(target=target), self.assertRaisesRegex(ValueError, "symlink"):
                    ci.site_inventory_files(site)
                link.unlink()
            linked_site = root / "linked-site"
            linked_site.symlink_to(site, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "ancestor"):
                ci.site_inventory_files(linked_site)
            linked_prefix = root / "linked-prefix"
            linked_prefix.symlink_to(root / "venv", target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "ancestor"):
                ci.site_inventory_files(linked_prefix / "lib/python3.12/site-packages")

    def test_known_unlisted_vendor_payloads_do_not_repair_record(self):
        names = ["triton/backends/amd/lib/" + n for n in ("libelf.so", "libnuma.so", "libtinfo.so")]
        old = [["triton/backends/amd/lib/libelf.so.1", "sha256=known-defective", "123"]]
        result = ci.vendor_record_rows(old, names, Path("/private/venv"))
        self.assertEqual(result[old[0][0]], old[0][1:])
        for name in names:
            self.assertEqual(result[name], ["", ""])
        for hostile in ("triton/backends/amd/lib/unowned.so", "elsewhere/libelf.so", "../escape"):
            with self.assertRaises(ValueError):
                ci.vendor_record_rows(old, names + [hostile], Path("/private/venv"))

    def test_only_frozen_non_site_payload_mapping(self):
        prefix = Path("/private/venv")
        self.assertEqual(ci.installed_member("pkg/module.py", prefix),
                         prefix / "lib/python3.12/site-packages/pkg/module.py")
        self.assertEqual(ci.installed_member("sympy-1.14.0.data/data/share/man/man1/isympy.1", prefix),
                         prefix / "share/man/man1/isympy.1")
        for name in ("pkg.data/headers/file.h", "pkg.data/scripts/worker", "pkg.data/data/unknown",
                     "sympy-1.14.0.data/data/../../escape"):
            with self.assertRaises(ValueError):
                ci.installed_member(name, prefix)

    def test_only_top_level_metadata_identifies_distribution(self):
        self.assertEqual(ci.wheel_dist_info([
            "setuptools-81.0.0.dist-info/METADATA",
            "setuptools/_vendor/packaging-24.0.dist-info/METADATA"]), "setuptools-81.0.0.dist-info")
        for names in ([], ["pkg/vendor.dist-info/METADATA"],
                      ["a.dist-info/METADATA", "b.dist-info/METADATA"]):
            with self.assertRaisesRegex(ValueError, "metadata roster"):
                ci.wheel_dist_info(names)

    def test_actual_frozen_linux_closure(self):
        wheels = ci.selected_wheels(SOURCE)
        self.assertEqual(len(wheels), 28)
        self.assertEqual(len({w["filename"] for w in wheels}), 28)
        self.assertNotIn("colorama", {w["name"] for w in wheels})
        self.assertEqual(next(w["version"] for w in wheels if w["name"] == "torch"), "2.12.1+rocm7.2")

    def test_lock_and_version_drift_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            ref = Path(tmp) / "benches/m1/reference"
            ref.mkdir(parents=True)
            for name in ("uv.lock", "pyproject.toml"):
                (ref / name).write_bytes((SOURCE / "benches/m1/reference" / name).read_bytes())
            project = ref / "pyproject.toml"
            old = project.read_text()
            for wrong in (old.replace("==3.12.*", ">=3.12"), old.replace("numpy==1.26.4", "numpy==1.26.3")):
                project.write_text(wrong)
                with self.assertRaises(ValueError):
                    ci.selected_wheels(tmp)
            project.write_text(old)
            with (ref / "uv.lock").open("a") as out:
                out.write("\n# drift\n")
            with self.assertRaisesRegex(ValueError, "lock drift"):
                ci.selected_wheels(tmp)

    def test_python_version_rejected(self):
        with patch.object(sys, "version_info", (3, 13)):
            with self.assertRaisesRegex(ValueError, "Python3.12"):
                ci.selected_wheels(SOURCE)

    def test_hostile_urls_rejected(self):
        for value in ("http://files.pythonhosted.org/x.whl", "https://evil.invalid/x.whl",
                      "https://files.pythonhosted.org/%2e%2e%2fx.whl",
                      "https://user@files.pythonhosted.org/x.whl", "https://files.pythonhosted.org/x.whl?x=1",
                      "https://files.pythonhosted.org/x.whl#x", "https://files.pythonhosted.org:444/x.whl"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                ci.wheel_url(value)

    def test_hostile_paths_rejected(self):
        for value in ("/absolute", "../escape", "a/../../escape", "a\\escape", "a\x00escape", ""):
            with self.subTest(value=value), self.assertRaises(ValueError):
                ci.safe_member(value)

    def test_fresh_capacity_and_insufficient_space(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertTrue(ci.capacity([tmp]))
            volume = type("Volume", (), {"f_bavail": ci.MIN_FREE - 1, "f_frsize": 1})()
            with patch.object(ci.os, "statvfs", return_value=volume):
                with self.assertRaisesRegex(ValueError, "40 GiB"):
                    ci.capacity([tmp])

    def test_archive_hash_size_path_and_duplicate_controls(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "test.whl"
            with zipfile.ZipFile(path, "w") as out:
                out.writestr("package/file.py", b"x")
            wheel = dict(hash="sha256:" + ci.digest(path), size=path.stat().st_size)
            self.assertEqual(ci.checked_wheel(path, wheel), path.stat().st_size)
            for wrong in (dict(wheel, hash="sha256:" + "0" * 64), dict(wheel, size=0)):
                with self.assertRaises(ValueError):
                    ci.checked_wheel(path, wrong)
            with zipfile.ZipFile(path, "w") as out:
                out.writestr("../escape", b"x")
            with self.assertRaisesRegex(ValueError, "unsafe"):
                ci.checked_wheel(path, dict(hash="sha256:" + ci.digest(path)))
            with zipfile.ZipFile(path, "w") as out:
                import warnings
                out.writestr("file", b"x")
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore", UserWarning)
                    out.writestr("file", b"y")
            with self.assertRaisesRegex(ValueError, "duplicate"):
                ci.checked_wheel(path, dict(hash="sha256:" + ci.digest(path)))

    def test_failure_propagates(self):
        with self.assertRaises(subprocess.CalledProcessError):
            ci.run_checked([sys.executable, "-I", "-B", "-c", "raise SystemExit(7)"])

    def test_filtered_discovery_not_admitted(self):
        for command in ([], ci.DISCOVERY + ["-k", "unit"], ["python3", "-m", "unittest"]):
            with self.assertRaisesRegex(ValueError, "original full"):
                ci.execute(SOURCE, Path("/nonexistent"), command)

    def test_clean_offline_environment(self):
        env = ci.offline_environment(Path("/tmp/owned"))
        for name in ("ROCR_VISIBLE_DEVICES", "HIP_VISIBLE_DEVICES", "CUDA_VISIBLE_DEVICES", "HSA_VISIBLE_DEVICES"):
            self.assertEqual(env[name], "-1")
        for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE", "HF_DATASETS_OFFLINE"):
            self.assertEqual(env[name], "1")
        self.assertNotIn("PYTHONPATH", env)
        self.assertNotIn("LD_LIBRARY_PATH", env)

    def test_mandatory_workflow_and_original_discovery(self):
        workflow = (SOURCE / ".github/workflows/ci.yml").read_text()
        job = workflow.split("\n  reference-cpu:\n", 1)[1].split("\n  qwen3-aggregate-device:", 1)[0]
        self.assertIn("runs-on: ubuntu-24.04", job)
        self.assertNotIn("continue-on-error", job)
        self.assertNotIn("if:", job)
        self.assertIn("-- python3 -I -B -m unittest discover -s benches/m1/reference -p 'test*.py'", job)
        self.assertIn("reference_cpu_ci.py prepare", job)
        self.assertIn("sudo apt-get install --yes --no-install-recommends python3.12-venv python3-pip", job)
        self.assertLess(job.index("reference_cpu_ci.py preflight"), job.index("sudo apt-get"))
        self.assertIn("test_reference_cpu_ci.py", job)
        rust = workflow.split("\n  rust:\n", 1)[1].split("\n  reference-cpu:", 1)[0]
        self.assertNotIn("-m unittest discover -s benches/m1/reference", rust)
        self.assertIn("python3 -I -B benches/m1/reference/test-policy.py", rust)
        self.assertEqual(len(list((SOURCE / "benches/m1/reference").iterdir())), 31)


if __name__ == "__main__":
    unittest.main()
