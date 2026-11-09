#!/usr/bin/env python3
"""R131 isolated v7 candidate regression tests.

Pure state-machine tests only: no svc-workflow API, DB, Definition mutation,
release, or production access.
The shared platform_receipts mapping simulates retained platform readback;
it is not runtime/platform evidence.

B1 mutation self-check, executed 2026-11-10 against this 11-test suite:
temporarily delete ONE verify conjunct and run
python3 -m unittest discover -s tests/r131 -p 'v7_candidate_regression.py'
  and actual_readback                       -> rc=1, only test_6 fails
  and p.readback_version == p.version        -> rc=1, only test_7 fails
  and bool(p.url)                           -> rc=1, only test_8 fails
  and bool(p.post_id)                        -> rc=1, only test_9 fails
Each mutant ran all 11 tests; all original gates were restored byte-for-byte
before adding this record. The unmutated suite passed all 11 tests (rc=0).
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
    failure_reason: str = ""
    failure_receipt: bool = False
    operation_key: str = "op-1"


class V7Candidate:
    def __init__(self, *, platform_receipts=None):
        self.seen_operations = set()
        self.platform_receipts = platform_receipts if platform_receipts is not None else {}
        self.readback_operations = []
        self.publish_attempts = 0

    def publish(self, p: Publication, *, url="https://example.test/post/1", post_id="p-1", readback_version="v7-content-1", fail=False):
        if not p.authorized:
            p.state = "waiting_authorization"
            return "WAITING_AUTHORIZATION"
        if p.operation_key in self.seen_operations or p.operation_key in self.platform_receipts:
            return "DUPLICATE_IGNORED"
        self.seen_operations.add(p.operation_key)
        self.publish_attempts += 1
        if fail:
            p.state = "publish_failed"
            p.notified = True
            p.failure_reason = "platform rejected publication"
            p.failure_receipt = True
            return "PUBLISH_FAILED"
        p.url, p.post_id = url, post_id
        p.readback_version = readback_version
        p.published_receipt = True
        self.platform_receipts[p.operation_key] = (url, post_id, readback_version)
        p.state = "verify"
        return "VERIFY_REQUIRED"

    def recover(self, p: Publication):
        """Read the simulated platform result by operation key before resuming."""
        self.readback_operations.append(p.operation_key)
        receipt = self.platform_receipts.get(p.operation_key)
        if receipt is None:
            return "RECOVERY_BLOCKED"
        p.url, p.post_id, p.readback_version = receipt
        p.published_receipt = True
        self.seen_operations.add(p.operation_key)
        p.state = "verify"
        return "VERIFY_REQUIRED"

    def terminate_failure(self, p: Publication):
        if not (p.state == "publish_failed" and p.notified
                and p.failure_receipt and p.failure_reason):
            return "FAILURE_TERMINATION_BLOCKED"
        p.state = "failed_terminal"
        return "FAILURE_TERMINATED"

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

    def test_3_failure_notifies_and_terminates_without_done(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p, fail=True), "PUBLISH_FAILED")
        self.assertTrue(p.notified)
        self.assertTrue(p.failure_receipt)
        self.assertTrue(p.failure_reason)
        self.assertEqual(p.state, "publish_failed")
        self.assertEqual(app.verify(p), "VERIFY_BLOCKED")
        self.assertEqual(p.state, "publish_failed")
        self.assertEqual(app.terminate_failure(p), "FAILURE_TERMINATED")
        self.assertEqual(p.state, "failed_terminal")
        self.assertEqual(app.verify(p), "VERIFY_BLOCKED")
        self.assertEqual(p.state, "failed_terminal")
        self.assertFalse(p.published_receipt)

    def test_4_lost_receipt_restart_reads_back_without_duplicate_publish(self):
        platform_receipts = {}
        p, app = Publication(authorized=True), V7Candidate(platform_receipts=platform_receipts)
        self.assertEqual(app.publish(p), "VERIFY_REQUIRED")
        retained_receipts = dict(platform_receipts)
        # New process-local objects lose the receipt; only platform data survives.
        restarted = V7Candidate(platform_receipts=platform_receipts)
        resumed = Publication(authorized=True, state="publishing", operation_key=p.operation_key)
        self.assertFalse(resumed.published_receipt)
        self.assertEqual(restarted.verify(resumed), "VERIFY_BLOCKED")
        self.assertEqual(restarted.recover(resumed), "VERIFY_REQUIRED")
        self.assertEqual(restarted.readback_operations, [p.operation_key])
        self.assertEqual(resumed.state, "verify")
        self.assertEqual((resumed.url, resumed.post_id, resumed.readback_version),
                         retained_receipts[p.operation_key])
        self.assertTrue(resumed.published_receipt)
        self.assertIn(p.operation_key, restarted.seen_operations)
        self.assertEqual(restarted.verify(resumed), "DONE")
        self.assertEqual(resumed.state, "done")
        self.assertEqual(restarted.publish(resumed), "DUPLICATE_IGNORED")
        self.assertEqual((app.publish_attempts, restarted.publish_attempts), (1, 0))
        self.assertEqual(platform_receipts, retained_receipts)

    def test_5_legacy_v5_v6_resume_and_parent_visibility(self):
        for legacy_version in ("v5", "v6"):
            p, app = Publication(authorized=True, version="v7-content-1", operation_key=f"resume-{legacy_version}"), V7Candidate()
            p.state = "publishing"  # legal successor resumes at publishing
            self.assertEqual(app.publish(p), "VERIFY_REQUIRED")
            self.assertEqual(app.verify(p), "DONE")
            self.assertTrue(p.published_receipt)  # parent can read the durable receipt

    def test_6_missing_actual_readback_blocks_verify(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p), "VERIFY_REQUIRED")
        self.assertEqual(app.verify(p, actual_readback=False), "VERIFY_BLOCKED")
        self.assertEqual(p.state, "verify")

    def test_7_mismatched_readback_version_blocks_verify(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p, readback_version="v7-content-other"), "VERIFY_REQUIRED")
        self.assertEqual(app.verify(p), "VERIFY_BLOCKED")
        self.assertEqual(p.state, "verify")

    def test_8_missing_url_blocks_verify(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p, url=""), "VERIFY_REQUIRED")
        self.assertEqual(app.verify(p), "VERIFY_BLOCKED")
        self.assertEqual(p.state, "verify")

    def test_9_missing_post_id_blocks_verify(self):
        p, app = Publication(authorized=True), V7Candidate()
        self.assertEqual(app.publish(p, post_id=""), "VERIFY_REQUIRED")
        self.assertEqual(app.verify(p), "VERIFY_BLOCKED")
        self.assertEqual(p.state, "verify")

    def test_10_failure_termination_requires_notification_reason_and_receipt(self):
        for missing in ("notified", "failure_reason", "failure_receipt"):
            with self.subTest(missing=missing):
                p, app = Publication(authorized=True), V7Candidate()
                self.assertEqual(app.publish(p, fail=True), "PUBLISH_FAILED")
                setattr(p, missing, "" if missing == "failure_reason" else False)
                self.assertEqual(app.terminate_failure(p), "FAILURE_TERMINATION_BLOCKED")
                self.assertEqual(p.state, "publish_failed")
                self.assertEqual(app.verify(p), "VERIFY_BLOCKED")
                self.assertEqual(p.state, "publish_failed")

    def test_11_restart_without_platform_receipt_stays_blocked(self):
        p, restarted = Publication(authorized=True, state="publishing"), V7Candidate()
        self.assertEqual(restarted.recover(p), "RECOVERY_BLOCKED")
        self.assertEqual(restarted.readback_operations, [p.operation_key])
        self.assertEqual(p.state, "publishing")
        self.assertFalse(p.published_receipt)
        self.assertEqual(restarted.publish_attempts, 0)
        self.assertEqual(restarted.verify(p), "VERIFY_BLOCKED")


if __name__ == "__main__":
    unittest.main(verbosity=2)
