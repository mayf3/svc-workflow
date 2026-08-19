# Agent entrypoint

Before non-mechanical work in `mayf3/svc-workflow`:

1. read `.agents/README.md` for the vendored shared Development Grammar;
2. read `.agents/local/README.md` for this repository's authority map and local constraints;
3. read `PRODUCT-BOUNDARY.md`, the directly relevant frozen/effective Architecture, existing accepted authority, and governing Specs under `docs/specs/`;
4. read `.agents/skills/spec-governance/SKILL.md` and only the selected mode file.

Do not implement non-mechanical behavior unless:

- the local governance adoption is accepted and active on `main`; and
- an accepted implementation-authorizing Spec is already present in the implementation PR base and covers the requested change.

Until this adoption is independently reviewed, explicitly accepted, and merged, the vendored files are a proposed governance candidate only. This bootstrap does not authorize product implementation.

Do not treat code, tests, runtime state, chat history, a PR description, or the newest document as higher authority than the repository's accepted local authorities. Report conflicts as authority conflict or conformance drift; do not silently rewrite authority to match implementation.
