#!/usr/bin/env python3
"""Derive full-suite totals and the R3 failure disposition from raw evidence."""
import hashlib
import json
import pathlib
import re

OUT = pathlib.Path(__file__).resolve().parent


def main():
    full = json.loads((OUT / 'full.json').read_text())
    log = (OUT / 'full.log').read_text()
    binaries = [json.loads(line) for line in (OUT / 'full-binary-results.jsonl').read_text().splitlines()]
    results = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;.*$', log, re.M)
    totals = {name: sum(int(row[i + 1]) for row in results)
              for i, name in enumerate(['passed', 'failed', 'ignored', 'measured', 'filtered_out'])}
    r3_log = (OUT.parent / '1359-r3/full.log').read_text()
    failed_names = re.findall(r'^    ([A-Za-z0-9_:]+)$', r3_log[r3_log.rfind('failures:'):], re.M)
    summary = {
        'full_rc': full['rc'], 'elapsed_seconds': full['elapsed_seconds'],
        'completed_binary_count': len(binaries), 'test_summary_count': len(results),
        'totals': totals, 'failed_binaries': [row for row in binaries if row['rc']],
        'binary45': [row for row in binaries if row['binary'] == '45_workflow_prerequisites'],
        'binary48': [row for row in binaries if row['binary'] == '48_outbox_three_channels'],
        'r3_fixed_name_guard_failed_tests': failed_names,
        'optional_messages': [line for line in log.splitlines() if 'NOT_RUN:' in line or 'skipped: set CANONICAL_CREATE_TEST_PRINCIPAL_ID' in line],
        'full_log_sha256': hashlib.sha256((OUT / 'full.log').read_bytes()).hexdigest(),
        'source_hashes_still_match': all(hashlib.sha256((OUT.parents[2] / name).read_bytes()).hexdigest() == value
                                        for name, value in json.loads((OUT / 'validation-input-manifest.json').read_text()).items()),
    }
    (OUT / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    (OUT / 'test-results-verbatim.txt').write_text('\n'.join(line for line in log.splitlines() if line.startswith('test result:')) + '\n')
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    main()
