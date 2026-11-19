"""Offline checks of the candidate format, approval gate and exact release promotion."""

import gzip
import hashlib
import io
import json
import shutil
import subprocess
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch
from urllib.request import Request

import candidate
import envelope
import promote
import reconcile

TAG = "v0.1.0-alpha.2"
SHA = "ab" * 20
REPO = "lgraak/Siliconwatch"
KEY = "a0" * 32  # isolated generated fixture content, never an Actions credential
ENV = {"GITHUB_ACTOR_ID": "503", "GITHUB_REPOSITORY_OWNER_ID": "9121312", "GITHUB_REF": "refs/heads/main", "GITHUB_RUN_ATTEMPT": "1", "GITHUB_RUN_ID": "101",
       "GITHUB_REPOSITORY": REPO, "GITHUB_EVENT_NAME": "workflow_dispatch",
       "GITHUB_WORKFLOW_REF": f"{REPO}/.github/workflows/release-publish.yml@refs/heads/main",
       "CANDIDATE_KEY": KEY}


class LocalRelease:
    """An in-memory remote with an operation trace and no network path."""

    def __init__(self, archive):
        self.archive = archive
        self.writes = []
        self.assets = {}
        self.releases = []
        self.tag = None
        self.run = {"id": 100, "run_attempt": 1, "event": "workflow_dispatch", "path": promote.CANDIDATE_WORKFLOW,
                    "head_branch": "main", "head_sha": SHA, "status": "completed", "conclusion": "success",
                    "repository": {"full_name": REPO}, "head_repository": {"full_name": REPO}}
        self.artifact = {"id": 50, "name": "candidate-" + TAG, "expired": False}
        self.reviews = [{"state": "approved", "user": {"login": "lgraak", "id": 9121312}, "environments": [{"name": "release"}]}]
        self.corrupt_upload = False
        self.immutable_on_publish = True

    def download(self, artifact, destination):
        shutil.copyfile(self.archive, destination)

    def upload(self, release_id, path):
        if path.name in self.assets:
            raise AssertionError("overwrite attempted")
        self.writes.append("upload")
        self.assets[path.name] = {"name": path.name, "state": "uploaded", "size": path.stat().st_size,
                                  "digest": "sha256:" + ("bad" if self.corrupt_upload else candidate.sha256(path))}

    def pages(self, path, key=None):
        return {"actions/runs/100/artifacts": [self.artifact], "releases": self.releases,
                "releases/5/assets": list(self.assets.values())}[path]

    def json(self, path, method="GET", *, body=None, missing_ok=False):
        if method == "GET":
            if path == "releases/5":
                return self.releases[0]
            return {"actions/runs/100": self.run, "actions/runs/101/approvals": self.reviews,
                    "git/ref/tags/" + TAG: self.tag}[path]
        self.writes.append((method, path))
        if (method, path) == ("POST", "releases"):
            draft = body | {"id": 5}
            self.releases.append(draft)
            return draft
        if (method, path) == ("POST", "git/refs"):
            if self.tag is not None:
                raise AssertionError("existing tag modified")
            self.tag = {"ref": body["ref"], "object": {"sha": body["sha"]}}
            return self.tag
        if (method, path) == ("PATCH", "releases/5"):
            self.releases[0].update(body)
            self.releases[0]["immutable"] = self.immutable_on_publish
            return self.releases[0] | {"html_url": "https://example.invalid/simulated-release"}
        raise AssertionError((method, path))


class Promotion(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scratch = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.scratch.cleanup)
        root = Path(cls.scratch.name)
        cls.notes = root / "notes.md"
        cls.notes.write_text(f"# SiliconWatch {TAG}\n\nFixture release.\n")
        cls.source = root / candidate.source_name(TAG)
        cls.source.write_bytes(b"corresponding source fixture")
        cls.packages = root / "packages"
        cls.packages.mkdir()
        for name in candidate.packages(TAG):
            (cls.packages / name).write_text(name)
        cls.reports = root / "reports"
        cls.reports.mkdir()
        provenance = cls.reports / "SOURCE-PROVENANCE.json"
        provenance.write_text(json.dumps({"schema_version": 1, "version": TAG[1:], "commit": SHA,
                                         "source_archive": cls.source.name, "source_sha256": candidate.sha256(cls.source)}))
        for name in candidate.REPORTS:
            path = cls.reports / name
            if name.startswith("payload-"):
                path.write_text(json.dumps([{"path": "/usr/share/doc/siliconwatch/SOURCE-PROVENANCE.json",
                                            "sha256": candidate.sha256(provenance)}]))
            elif name.endswith(".log"):
                path.write_text("passed\n")
        for distro, suffix in (("debian", "amd64.deb"), ("ubuntu", "amd64.deb"),
                              ("fedora", "x86_64.rpm"), ("arch", "x86_64.pkg.tar.zst")):
            name = f"siliconwatch-{TAG[1:]}-linux-{suffix}"
            receipt = {"schema": "siliconwatch.install.v1", "result": "passed", "distro": distro,
                       "tag": TAG, "source_commit": SHA, "package": name, "sha256": candidate.sha256(cls.packages / name)}
            (cls.reports / f"install-{distro}.json").write_text(json.dumps(receipt))
        (cls.reports / "ci.json").write_text(json.dumps({"result": "success", "source_commit": SHA}))
        cls.bundle = root / "sealed"
        cls.digest = candidate.seal(TAG, SHA, cls.packages, cls.source, cls.notes, cls.reports, cls.bundle)
        cls.archive = root / "encrypted.zip"
        encrypted = root / "candidate.tar.gpg"
        envelope.pack(cls.bundle, encrypted, KEY)
        with zipfile.ZipFile(cls.archive, "w") as archive:
            archive.write(encrypted, encrypted.name)

    def run_promotion(self, remote, publish=True, env=ENV, **overrides):
        options = {"api": remote, "repo": REPO, "version": TAG, "commit": SHA, "run_id": 100,
                   "digest": self.digest, "publish": publish, "env": env}
        promote.promote(**(options | overrides))

    def test_identical_evidence_seals_to_identical_digest(self):
        with tempfile.TemporaryDirectory() as temp:
            copy = Path(temp) / "reports"
            shutil.copytree(self.reports, copy, copy_function=shutil.copyfile)
            result = candidate.seal(TAG, SHA, self.packages, self.source, self.notes, copy, Path(temp) / "bundle")
            self.assertEqual(result, self.digest)

    def test_approved_promotion_uploads_exact_bytes_then_creates_tag_and_publishes(self):
        remote = LocalRelease(self.archive)
        self.run_promotion(remote)
        self.assertEqual(remote.writes, [("POST", "releases"), *(["upload"] * len(candidate.bundle_names(TAG))),
                                         ("POST", "git/refs"), ("PATCH", "releases/5")])
        self.assertEqual(remote.tag["object"]["sha"], SHA)
        self.assertEqual(remote.releases[0]["draft"], False)
        self.assertTrue(remote.releases[0]["prerelease"])

    def test_dry_run_has_zero_writes_even_without_approval(self):
        remote = LocalRelease(self.archive)
        remote.reviews = []
        self.run_promotion(remote, publish=False)
        self.assertEqual(remote.writes, [])

    def test_control_requires_approval_from_named_reviewer_for_this_environment(self):
        for reviews in ([], [{"state": "approved", "user": {"login": "other"}, "environments": [{"name": "release"}]}],
                        [{"state": "approved", "user": {"login": "lgraak"}, "environments": [{"name": "candidate"}]}]):
            with self.subTest(reviews=reviews):
                remote = LocalRelease(self.archive)
                remote.reviews = reviews
                with self.assertRaises(ValueError):
                    self.run_promotion(remote)
                self.assertEqual(remote.writes, [])

    def test_review_identity_and_dispatcher_are_distinct(self):
        remote = LocalRelease(self.archive)
        for dispatcher in ("", "unknown", ENV["GITHUB_REPOSITORY_OWNER_ID"]):
            with self.subTest(dispatcher=dispatcher), self.assertRaisesRegex(ValueError, "independent"):
                self.run_promotion(remote, env=ENV | {"GITHUB_ACTOR_ID": dispatcher})
        remote.reviews[0]["user"]["id"] = 12345
        with self.assertRaisesRegex(ValueError, "approval"):
            self.run_promotion(remote)
        self.assertEqual(remote.writes, [])

    def test_manual_main_publisher_only(self):
        for key, value in (("GITHUB_REF", "refs/tags/" + TAG), ("GITHUB_RUN_ATTEMPT", "2"),
                          ("GITHUB_EVENT_NAME", "pull_request"), ("GITHUB_WORKFLOW_REF", "something else")):
            with self.subTest(key=key):
                remote = LocalRelease(self.archive)
                with self.assertRaises(ValueError):
                    self.run_promotion(remote, env=ENV | {key: value})
                self.assertEqual(remote.writes, [])

    def test_untrusted_candidate_run_or_artifact(self):
        for key, value in (("head_repository", {"full_name": "fork/repo"}), ("head_sha", "0" * 40),
                          ("path", ".github/workflows/other.yml"), ("event", "pull_request"),
                          ("conclusion", "failure"), ("head_branch", "other")):
            with self.subTest(key=key):
                remote = LocalRelease(self.archive)
                remote.run[key] = value
                with self.assertRaises(ValueError):
                    self.run_promotion(remote)
                self.assertEqual(remote.writes, [])
        remote = LocalRelease(self.archive)
        remote.artifact["expired"] = True
        with self.assertRaises(ValueError):
            self.run_promotion(remote)
        self.assertEqual(remote.writes, [])

    def test_collision_refuses_all_mutations(self):
        for already_published in (False, True):
            with self.subTest(already_published=already_published):
                remote = LocalRelease(self.archive)
                if already_published:
                    remote.tag = {"object": {"sha": SHA}}
                else:
                    remote.releases.append({"tag_name": TAG, "draft": True})
                with self.assertRaises(ValueError):
                    self.run_promotion(remote)
                self.assertEqual(remote.writes, [])

    def test_fresh_retry_after_interruption_cannot_overwrite_partial_draft(self):
        remote = LocalRelease(self.archive)
        remote.corrupt_upload = True
        with self.assertRaises(ValueError):
            self.run_promotion(remote)
        first_operations = remote.writes.copy()
        remote.corrupt_upload = False
        with self.assertRaisesRegex(ValueError, "draft already owns"):
            self.run_promotion(remote)
        self.assertEqual(remote.writes, first_operations)
        self.assertIsNone(remote.tag)
        self.assertTrue(remote.releases[0]["draft"])

    def test_upload_digest_failure_stays_draft(self):
        remote = LocalRelease(self.archive)
        remote.corrupt_upload = True
        with self.assertRaises(ValueError):
            self.run_promotion(remote)
        self.assertTrue(remote.releases[0]["draft"])
        self.assertIsNone(remote.tag)

    def test_reconcile_interrupted_tag_and_publish_without_retry_writes(self):
        expected = candidate.verify(self.bundle, TAG, SHA, self.digest)
        metadata = promote.release_metadata(TAG, SHA, self.notes)
        for stage, expected_state in (("before_tag", "draft_without_tag"),
                                      ("after_tag", "draft_with_tag"),
                                      ("after_publication", "public_verified")):
            with self.subTest(stage=stage):
                remote = LocalRelease(self.archive)
                request = remote.json

                def interrupted(path, method="GET", **kwargs):
                    if stage == "before_tag" and (method, path) == ("POST", "git/refs"):
                        raise OSError("interrupted before tag creation")
                    response = request(path, method, **kwargs)
                    if ((stage == "after_tag" and (method, path) == ("POST", "git/refs"))
                            or (stage == "after_publication" and method == "PATCH")):
                        raise OSError("response lost")
                    return response

                with patch.object(remote, "json", side_effect=interrupted), self.assertRaises(OSError):
                    self.run_promotion(remote)
                writes = remote.writes.copy()
                result = reconcile.inspect(remote, TAG, SHA, expected, metadata, release_id=5)
                self.assertEqual(result["state"], expected_state)
                self.assertEqual(result["issues"], [])
                with self.assertRaises(ValueError):
                    self.run_promotion(remote)
                self.assertEqual(remote.writes, writes)

    def test_recovery_checks_immutable_release_and_unique_id(self):
        remote = LocalRelease(self.archive)
        remote.immutable_on_publish = False
        with self.assertRaisesRegex(ValueError, "may already be public"):
            self.run_promotion(remote)
        expected = candidate.verify(self.bundle, TAG, SHA, self.digest)
        metadata = promote.release_metadata(TAG, SHA, self.notes)

        def report():
            return reconcile.inspect(remote, TAG, SHA, expected, metadata, release_id=5)

        writes = remote.writes.copy()
        self.assertEqual(report()["state"], "public_mismatch")
        del remote.releases[0]["immutable"]
        self.assertEqual(report()["state"], "public_mismatch")
        remote.releases[0]["immutable"] = True
        remote.releases.append(remote.releases[0] | {"id": 6})
        self.assertEqual(report()["state"], "ambiguous")
        self.assertEqual(remote.writes, writes)

    def test_reconcile_requires_complete_approved_public_state(self):
        remote = LocalRelease(self.archive)
        expected = candidate.verify(self.bundle, TAG, SHA, self.digest)
        metadata = promote.release_metadata(TAG, SHA, self.notes)

        def report():
            return reconcile.inspect(remote, TAG, SHA, expected, metadata)

        self.assertEqual(report()["state"], "no_visible_tag_or_release")
        self.run_promotion(remote)
        writes = remote.writes.copy()
        remote.assets.pop("SHA256SUMS")
        remote.assets["candidate.json"]["digest"] = None
        remote.tag["object"]["sha"] = "ff" * 20
        result = report()
        self.assertEqual(result["state"], "public_mismatch")
        self.assertEqual(len(result["issues"]), 3)
        remote.releases[0]["draft"] = True
        self.assertEqual(report()["state"], "draft_with_tag")
        remote.releases.clear()
        self.assertEqual(report()["state"], "ambiguous")
        remote.tag["object"]["sha"] = SHA
        self.assertEqual(report()["state"], "tag_only_or_release_hidden")
        self.assertEqual(remote.writes, writes)

    def test_wrong_approved_digest_and_tampering(self):
        remote = LocalRelease(self.archive)
        with self.assertRaises(ValueError):
            self.run_promotion(remote, digest="0" * 64)
        self.assertEqual(remote.writes, [])
        with tempfile.TemporaryDirectory() as tmp:
            copy = Path(tmp) / "bundle"
            shutil.copytree(self.bundle, copy)
            file = copy / "RELEASE-NOTES.md"
            file.write_text("changed")
            with self.assertRaises(ValueError):
                candidate.verify(copy, TAG, SHA, self.digest)
            file.unlink()
            with self.assertRaises(ValueError):
                candidate.verify(copy, TAG, SHA, self.digest)
            file.symlink_to(self.notes)
            with self.assertRaises(ValueError):
                candidate.verify(copy, TAG, SHA, self.digest)

    def test_seal_requires_verification_reports_and_matching_packaged_provenance(self):
        with tempfile.TemporaryDirectory() as tmp:
            logs = Path(tmp) / "logs"
            shutil.copytree(self.reports, logs)
            (logs / "payload-fedora.json").write_text("[]")
            with self.assertRaises(ValueError):
                candidate.seal(TAG, SHA, self.packages, self.source, self.notes, logs, Path(tmp) / "bundle")
            (logs / "install-ubuntu.log").unlink()
            with self.assertRaises(ValueError):
                candidate.seal(TAG, SHA, self.packages, self.source, self.notes, logs, Path(tmp) / "bundle")

    def test_changed_packages_cannot_inherit_passing_install_receipts(self):
        for name in candidate.packages(TAG):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                copies = Path(tmp) / "packages"
                shutil.copytree(self.packages, copies)
                package = copies / name
                package.write_bytes(b"X" * package.stat().st_size)
                output = Path(tmp) / "bundle"
                with self.assertRaisesRegex(ValueError, "Install receipt mismatch"):
                    candidate.seal(TAG, SHA, copies, self.source, self.notes, self.reports, output)
                self.assertFalse(output.exists())

    def test_ubuntu_needs_its_own_success_receipt_for_the_tested_debian_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            reports = Path(tmp) / "reports"
            shutil.copytree(self.reports, reports)
            ubuntu = reports / "install-ubuntu.json"
            original = json.loads(ubuntu.read_bytes())
            for field, incorrect in (("distro", "debian"), ("source_commit", "0" * 40),
                                     ("tag", "v0.1.0-alpha.1"), ("package", "different.deb"),
                                     ("sha256", "0" * 64), ("result", "failed")):
                with self.subTest(field=field):
                    ubuntu.write_text(json.dumps(original | {field: incorrect}))
                    with self.assertRaisesRegex(ValueError, "Install receipt mismatch: install-ubuntu.json"):
                        candidate.seal(TAG, SHA, self.packages, self.source, self.notes, reports, Path(tmp) / "out")
            ubuntu.write_text('{"result":"failed",' + json.dumps(original)[1:])
            with self.assertRaisesRegex(ValueError, "Duplicate field"):
                candidate.seal(TAG, SHA, self.packages, self.source, self.notes, reports, Path(tmp) / "out")
            ubuntu.unlink()
            with self.assertRaisesRegex(ValueError, "missing .*install-ubuntu.json"):
                candidate.seal(TAG, SHA, self.packages, self.source, self.notes, reports, Path(tmp) / "out")

    def test_promotion_rejects_a_validly_hashed_legacy_bundle_without_install_receipts(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle = Path(tmp) / "old-bundle"
            shutil.copytree(self.bundle, bundle)
            with tarfile.open(bundle / "verification-reports.tar.gz", "w:gz", format=tarfile.USTAR_FORMAT) as reports:
                for path in self.reports.iterdir():
                    if not (path.name.startswith("install-") and path.suffix == ".json"):
                        reports.add(path, arcname=path.name)
            files = candidate.metadata(bundle, candidate.payload_names(TAG))
            (bundle / "SHA256SUMS").write_text(candidate.checksums(files))
            files.update(candidate.metadata(bundle, {"SHA256SUMS"}))
            manifest = bundle / "candidate.json"
            manifest.write_text(json.dumps({"schema": 1, "tag": TAG, "source_commit": SHA, "files": files}))
            with self.assertRaisesRegex(ValueError, "Missing verification reports"):
                candidate.verify(bundle, TAG, SHA, candidate.sha256(manifest))

    def test_report_archive_enforces_expanded_size_member_size_and_type(self):
        files = candidate.metadata(self.bundle, candidate.packages(TAG))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "reports.tar.gz"
            path.write_bytes(gzip.compress(b"0" * 1025))
            with patch.object(candidate, "MAX_REPORT_ARCHIVE_BYTES", 1024):
                with self.assertRaisesRegex(ValueError, "Expanded verification report archive"):
                    candidate.verify_install_receipts(path, TAG, SHA, files)
            valid = gzip.decompress((self.bundle / "verification-reports.tar.gz").read_bytes())
            path.write_bytes(gzip.compress(valid + b"hidden data"))
            with self.assertRaisesRegex(ValueError, "end of verification reports"):
                candidate.verify_install_receipts(path, TAG, SHA, files)
            member = tarfile.TarInfo("build-arch.log")
            member.size = candidate.MAX_REPORT_FILE_BYTES + 1
            path.write_bytes(gzip.compress(member.tobuf() + bytes(1024)))
            with self.assertRaisesRegex(ValueError, "Invalid verification report size"):
                candidate.verify_install_receipts(path, TAG, SHA, files)
            member.type = tarfile.XHDTYPE
            path.write_bytes(gzip.compress(member.tobuf() + bytes(1024)))
            with self.assertRaisesRegex(ValueError, "Unexpected verification report member"):
                candidate.verify_install_receipts(path, TAG, SHA, files)

    def test_wrong_decryption_key_preserves_unpublished_state(self):
        remote = LocalRelease(self.archive)
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_promotion(remote, env=ENV | {"CANDIDATE_KEY": "11" * 32})
        self.assertEqual(remote.writes, [])

    def test_helper_roundtrip_and_private_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            package = root / "inputs.gpg"
            envelope.pack(self.reports, package, KEY)
            envelope.unpack(package, root / "reports", KEY)
            candidate.exact_files(root / "reports", candidate.REPORTS)
            for name in sorted(candidate.REPORTS):
                self.assertEqual((root / "reports" / name).read_bytes(), (self.reports / name).read_bytes())
            with self.assertRaises(ValueError):
                envelope.pack(root / "reports", package, KEY)

    def test_archive_names_must_be_flat_and_expected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with zipfile.ZipFile(root / "wrong.zip", "w") as archive:
                archive.writestr("extra", b"data")
            with self.assertRaises(ValueError):
                promote.unpack(root / "wrong.zip", root / "bundle", TAG, KEY)
            def corrupt_tar(source, target, key, home, *, decrypt):
                with tarfile.open(target, "w") as archive:
                    item = tarfile.TarInfo("nested/file")
                    item.size = 1
                    archive.addfile(item, io.BytesIO(b"x"))
            with patch("envelope.crypt", side_effect=corrupt_tar), self.assertRaises(ValueError):
                promote.unpack(self.archive, root / "bundle", TAG, KEY)



class ArtifactTransport(unittest.TestCase):
    def test_github_size_and_digest_bind_downloaded_zip_bytes(self):
        data = b"fixture-archive"
        metadata = {"id": 42, "size_in_bytes": len(data),
                    "digest": "sha256:" + hashlib.sha256(data).hexdigest()}
        cases = [(metadata, True), (metadata | {"size_in_bytes": len(data) - 1}, False),
                 (metadata | {"size_in_bytes": len(data) + 1}, False),
                 (metadata | {"digest": "sha256:" + "0" * 64}, False)]
        api = promote.GitHub(REPO, "fixture")
        for artifact, valid in cases:
            with self.subTest(valid=valid), tempfile.TemporaryDirectory() as temp:
                output = Path(temp) / "download.zip"
                with patch.object(api, "open", return_value=io.BytesIO(data)) as get:
                    if valid:
                        api.download(artifact, output)
                        self.assertEqual(output.read_bytes(), data)
                    else:
                        with self.assertRaises(ValueError):
                            api.download(artifact, output)
                    get.assert_called_once_with("actions/artifacts/42/zip")

    def test_redirect_drops_api_credential_on_storage_download(self):
        handler = promote.DownloadRedirect()
        req = Request("https://api.github.com/test", headers={"Authorization": "Bearer fixture"})
        redirected = handler.redirect_request(req, None, 302, "", {}, "https://storage.example.invalid/asset")
        self.assertIsNone(redirected.get_header("Authorization"))
        with self.assertRaises(ValueError):
            handler.redirect_request(req, None, 302, "", {}, "http://storage.example.invalid/asset")


if __name__ == "__main__":
    unittest.main()
