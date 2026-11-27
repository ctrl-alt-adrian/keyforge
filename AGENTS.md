# Owner preferences

Create Git worktrees under the repository's `.worktrees/` directory, which is ignored by Git.

Do not create, write, update or expand tests unless the user explicitly asks you to write them. Requests to implement, fix, verify, build, push or open a PR do not grant test authoring permission. Preserve existing tests; do not change assertions or fixtures merely to make checks pass without that permission.

Keep verification proportionate to the change. Run relevant existing checks, avoid unnecessary repeated/full-suite runs, and report what was checked. When test authoring is explicitly requested, finish production changes before writing requested tests.

# Keyforge evaluation constraints

One framework-independent Rust core; independent Iced, GPUI and egui Cargo workspaces. No root Cargo workspace, GUI dependencies in the core, universal widget abstraction, or production packaging. Live GitHub writes/builds must be explicit app actions; do not perform them as development checks. Framework selection requires owner approval. Keep its ADR proposed. Record evidence honestly: compilation is not runtime verification.

# GPUI foundation worktree

The master knowledge base is `/home/adrian/Dropbox/keyforge/GPUI Foundation.md`. Native design documents live in `mocks/`. This branch contains `core/` and `frontends/gpui/`; the other evaluation frontends remain uncommitted in the main checkout. Keep framework selection proposed.

The owner will verify the completed implementation. Do not author or change tests until the owner has reviewed and signed off on the implementation. Existing tests remain intact.

# Live application scope update

The owner has explicitly expanded the scope to real GitHub repository loading, saving/build integration and the remaining editing screens. This supersedes the evaluation-only restriction on real services and remaining-screen implementation. Keep the framework-independent core and retained prototypes; do not perform remote writes or trigger workflows as a development check. Tests remain deferred until owner sign-off.

# Documentation maintenance

The master entry is `/home/adrian/Dropbox/keyforge/GPUI Foundation.md`. Follow the Mirukai-style handbook structure under that vault's `docs/`, with canonical decisions in `decisions/`, active execution/evidence in `handoffs/` and superseded records in `handoffs/archive/`. Repository docs/ADRs are pointers. Update the relevant guide and handoff in the same task when behavior or verification changes. Record source/worktree state honestly; base commit `96e2234` does not contain the uncommitted application. Preserve the existing test and framework approval gates.
