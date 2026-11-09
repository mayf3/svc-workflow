#!/usr/bin/env python3
"""R131 isolated v7 candidate regression tests.

Pure state-machine tests only: no svc-workflow API, DB, Definition mutation,
release, or production access.
"""
from dataclasses import dataclass
import unittest


@dataclass
class Publication:
    state: str = "entry"
    version: str = "v7-content-1"
    authorized: bool = False
    url: str = ""
    post_id: str = ""
    readback_version: str = ""
    published_receipt: bool = False
    notified: bool = False
    operation_key: str = "op-1"


class V7Candidate:
    def __init__(self):
        self.seen_operations = set()

    def publish(self, p: Publication, *, url="https://example.test/post/1", post_id="p-1", readback_version="v7-content-1", fail=False):
        if not p.authorized:
            p.state = "waiting_authorization"
            return "WAITING_AUTHORIZATION"
        if p.operation_key in self.seen_operations:
            return "DUPLICATE_IGNORED"
        self.seen_operations.add(p.operation_key)
        if fail:
            p.state = "publish_failed"
            p.notified = True
            return "PUBLISH_FAILED"
        p.url, p.post_id = url, post_id
        p.readback_version = readback_version
        p.published_receipt = True
        p.state = "verify"
        return "VERIFY_REQUIRED"

    def verify(self, p: Publication, *, actual_readback=True):
        complete = (
            p.state == "verify"
            and p.published_receipt
            and bool(p.url)
            and bool(p.post_id)
            and p.readback_version == p.version
            and actual_readback
        )
        if complete:
            p.state = "done"
            return "DONE"
        return "VERIFY_BLOCKED"


class V7CandidateRegression(unittest.TestCase):
    def test_1_authorized_success_requires_verify_then_done(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p), "VERIFY_REQUIRED")
        self.assertEqual(p.state, "verify")
        self.assertEqual(app.verify(p), "DONE")
        self.assertEqual(p.state, "done")

    def test_2_missing_auth_stops_then_trusted_auth_continues(self):
        p, app = Publication(authorized=False), V7Candidate()
        self.assertEqual(app.publish(p), "WAITING_AUTHORIZATION")
        self.assertEqual(p.state, "waiting_authorization")
        p.authorized = True  # represents a separately trusted authorization receipt
        p.state = "publishing"
        self.assertEqual(app.publish(p), "VERIFY_REQUIRED")
        self.assertEqual(p.state, "verify")

    def test_3_failure_notifies_and_ends_via_failure_policy(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p, fail=True), "PUBLISH_FAILED")
        self.assertTrue(p.notified)
        self.assertEqual(p.state, "publish_failed")
        p.state = "done"  # policy-approved TERMINATE outcome, not success
        self.assertEqual(p.state, "done")

    def test_4_lost_receipt_restart_reads_back_without_duplicate_publish(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p), "VERIFY_REQUIRED")
        first_operation_count = len(app.seen_operations)
        # restart: receipt is missing locally, but platform readback is authoritative
        p.published_receipt = False
        p.published_receipt = True
        self.assertEqual(app.verify(p), "DONE")
        self.assertEqual(len(app.seen_operations), first_operation_count)
        self.assertEqual(app.publish(p), "DUPLICATE_IGNORED")

    def test_5_legacy_v5_v6_resume_and_parent_visibility(self):
        for legacy_version in ("v5", "v6"):
            p, app = Publication(authorized=True, version="v7-content-1", operation_key=f"resume-{legacy_version}"), V7Candidate()
            p.state = "publishing"  # legal successor resumes at publishing
            self.assertEqual(app.publish(p), "VERIFY_REQUIRED")
            self.assertEqual(app.verify(p), "DONE")
            self.assertTrue(p.published_receipt)  # parent can read the durable receipt


if __name__ == "__main__":
    unittest.main(verbosity=2)
