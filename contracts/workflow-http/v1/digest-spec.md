# Workflow HTTP Contract V1 — Bundle Digest Specification

**Specification Version:** 1.0.0
**Algorithm:** SHA-256
**Last Updated:** 2026-07-18

## 1. Purpose

This document defines the deterministic, reproducible digest computation for the
Workflow HTTP Contract V1 bundle. Independent auditors can recompute both
`SCHEMA_DIGEST` and `BUNDLE_DIGEST` from a clean checkout without any external
tooling beyond `shasum` (or equivalent SHA-256 utility) and a POSIX shell.

## 2. Encoding Rules (All Digests)

All digests follow these fixed conventions:

| Parameter      | Value                      |
|----------------|----------------------------|
| Character set | File raw UTF-8 bytes        |
| Line endings   | LF (0x0A), as committed     |
| Path format    | Relative to `contracts/workflow-http/v1/` |
| Algorithm      | SHA-256                     |
| Representation | Lowercase hex (64 chars)    |

No temporary files, absolute paths, timestamps, or non-deterministic data
participate in any digest computation.

## 3. SCHEMA_DIGEST

`SCHEMA_DIGEST` is the SHA-256 hash of `openapi.yaml` raw bytes:

```
SCHEMA_DIGEST = SHA-256(openapi.yaml 原始 bytes)
```

### Computation

```bash
sha256sum contracts/workflow-http/v1/openapi.yaml
# or on macOS:
shasum -a 256 contracts/workflow-http/v1/openapi.yaml
```

### Current Value

```
(Recomputed — see manifest.json schema_digest)
```

## 4. BUNDLE_DIGEST

`BUNDLE_DIGEST` covers the entire contract bundle: `manifest.json` plus all files
declared in `manifest.json` → `BUNDLE_FILE_SET`.

### 4.1 File Set

The bundle file set is explicitly listed in `manifest.json` under the
`BUNDLE_FILE_SET` field. Only those files participate in the digest. This
prevents accidental inclusion of temporary files, test outputs, or editor
artifacts.

### 4.2 Self-Reference Handling

`manifest.json` contains the `BUNDLE_DIGEST` field itself. To avoid circular
dependency, the following normalization rule applies:

> **Normalization Rule:** When computing `BUNDLE_DIGEST`, the `BUNDLE_DIGEST`
> value in `manifest.json` is replaced with exactly 64 ASCII `'0'` characters
> before hashing.

All other fields in `manifest.json` are used as-is.

### 4.3 Per-File Entry Format

For each file in `BUNDLE_FILE_SET` (sorted by relative path in ascending
dictionary order), the following bytes are appended to the hash input:

```
<relative_path> + NUL (0x00) + <file_bytes>
```

Where:
- `<relative_path>` is the UTF-8 encoded path relative to
  `contracts/workflow-http/v1/`
- `NUL` is a single 0x00 byte acting as unambiguous separator
- `<file_bytes>` are the file's raw bytes as stored in the repository (LF
  line endings)

### 4.4 Concatenation and Final Hash

1. Sort `BUNDLE_FILE_SET` entries by relative path using simple byte-wise
   dictionary order (same as `LC_ALL=C sort`).
2. For each file in that order, produce the per-file entry as defined in §4.3.
3. Concatenate all per-file entries in that order.
4. Compute SHA-256 of the concatenated bytes.
5. The resulting 64-character lowercase hex string is `BUNDLE_DIGEST`.

### 4.5 Formal Definition

```
normalized_manifest = manifest.json with BUNDLE_DIGEST set to "0000...00" (64 zeros)

hash_input = ""
for each path in sorted(BUNDLE_FILE_SET):
    if path == "manifest.json":
        content = normalized_manifest
    else:
        content = raw bytes of file at path
    hash_input += path.encode("utf-8") + b"\x00" + content

BUNDLE_DIGEST = SHA-256(hash_input).hexdigest().lower()
```

## 5. Verification Command

```bash
contracts/workflow-http/v1/verify-digests.sh
```

This script:
1. Computes `SCHEMA_DIGEST` from `openapi.yaml`
2. Computes `BUNDLE_DIGEST` from the full bundle with normalization
3. Prints both values
4. Exits with status 0 if both match `manifest.json`, non-zero otherwise

## 6. Example

```
$ contracts/workflow-http/v1/verify-digests.sh
SCHEMA_DIGEST=6f5a8630425e8edc212dc0c004c0fff75e098ab086fa79a1086ee733d508d324
BUNDLE_DIGEST=<hex-value>
DIGEST_VERIFICATION=PASS
```

## 7. Changelog

| Version | Date       | Change                    |
|---------|------------|---------------------------|
| 1.0.0   | 2026-07-18 | Initial specification     |
