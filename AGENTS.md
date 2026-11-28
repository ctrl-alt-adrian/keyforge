# Keyforge — Engineering Instructions
## 1. Engineering Authority and Scope
The owner retains final authority over architecture, dependencies, framework selection, product behavior, design direction, and major refactors. Correctness, maintainability, usability, and performance take precedence over implementation speed.
- Implement the requested outcome. Preserve unrelated behavior and do not expand scope without approval.
- Resolve routine implementation details using established project conventions. Escalate decisions that materially change architecture, UX contracts, compatibility, dependencies, or data integrity.
- Do not conceal defects, incomplete work, failed checks, or deviations from the design.
- A task-specific owner instruction may override a standing rule only within its stated scope. Historical permissions are not standing authorization.
## 2. Project Architecture and Active Contracts
Keyforge uses a framework-independent Rust core and an active GPUI frontend. Retain independent Cargo workspaces for the GPUI, Iced, and egui implementations where present. Do not introduce a root Cargo workspace, GUI dependencies into the core, or a universal widget abstraction. Keep the framework-selection ADR proposed until the owner explicitly approves it. Do not modify retained prototypes or introduce packaging changes unless requested.
- Keep parsing, validation, domain rules, configuration serialization, and firmware/build orchestration independent of GPUI. The frontend owns presentation, interaction, and transient UI state.
- Real GitHub repository loading, saving/build integration, and the remaining editing screens are authorized application capabilities. Remote writes and workflow triggers must remain explicit application actions, never development checks.
- mocks/Keyforge-v0.5.figcraft, including both interaction contracts, is the current UX reference unless superseded by the owner. Valid live edits stage immediately; Save and build remains explicit. Preserve unknown source and invalid drafts. Retain existing backend capabilities without reintroducing remote build controls into the v0.5 UX.
- Inspect the actual checkout and Cargo manifests before assuming which crates, prototypes, dependencies, or features are present.
## 3. Research Before Implementation
Before modifying code:
1. Read applicable AGENTS.md files, the relevant design/interaction contracts, and current implementation.
2. Trace the existing data flow, state ownership, commands, and failure paths affected by the task.
3. Search for existing components, helpers, types, validation rules, and similar behavior before creating new ones.
4. Verify unfamiliar APIs against the project's pinned versions and primary documentation or dependency source. Do not invent API signatures, assume current upstream APIs are installed, or implement from memory when uncertain.
5. Identify the smallest coherent change that satisfies the requested outcome. For complex tasks, form a short implementation plan and identify material risks before editing.
Use targeted rg, git, and file reads. Expand the investigation only as evidence requires. Do not repeatedly scan the repository, reread unchanged files, or generate speculative architecture documents for routine changes.
## 4. Rust Engineering Standards
- Write idiomatic Rust consistent with rustfmt, Clippy, the Rust API Guidelines, and established crate conventions. Prefer clear names, explicit control flow, and cohesive functions over compressed or clever code.
- Use enums, structs, newtypes, Option, and Result to represent domain concepts and invariants. Avoid stringly typed state and combinations of flags that allow contradictory states.
- Respect ownership and borrowing. Borrow when ownership is unnecessary; clone when ownership or lifetime boundaries justify it. Do not use gratuitous cloning, allocation, reference counting, or synchronization to bypass design problems.
- Prefer exhaustive match, if let, let else, and ? where they improve clarity. Use iterators when they remain readable; use loops when they communicate intent better. Avoid unnecessary nesting.
- Propagate recoverable errors with actionable context using existing error types and conventions. Do not silently discard errors, substitute misleading defaults, or use unwrap, expect, or panic for recoverable production failures. An invariant-based panic requires a clear justification.
- Keep visibility narrow; expose only the API needed by callers. Document non-obvious invariants, error contracts, safety requirements, and rationale, not obvious syntax.
- Prefer safe Rust. Do not introduce unsafe, new runtime dependencies, major upgrades, or broad lint suppressions without owner approval. Do not use lint allowances to hide defects.
- Use existing patterns for logging, error handling, concurrency, and configuration. Never log credentials, tokens, or sensitive repository contents.
## 5. Reuse, Duplication, and Abstraction
Before introducing a function, method, component, type, or conditional, determine whether its responsibility already exists.
- Maintain one authoritative implementation of each business rule, validation rule, state transition, and derived value. Reuse or consolidate equivalent behavior instead of copying it into another screen or handler.
- Route equivalent UI actions through the same command or domain operation. Do not duplicate mutation, parsing, serialization, or validation logic across handlers.
- When multiple branches implement the same policy, consolidate the predicate or operation at the correct ownership boundary. Avoid scattering equivalent platform, capability, or behavior checks throughout the codebase.
- Prefer composition of existing components and functions. Extend an existing interface when the semantics fit; do not add flags or parameters that turn it into an unrelated collection of behaviors.
- Do not introduce near-identical components, wrapper functions that add no value, speculative traits, generic factories, or abstractions for hypothetical future requirements.
- Do not force reuse between superficially similar code with different responsibilities. Abstract shared semantics, not coincidentally similar syntax. Keep trivial local expressions local where extraction would reduce clarity.
- Consolidate duplication encountered within the task when it is necessary and safe. Do not turn a focused change into a repository-wide cleanup.
## 6. GPUI, Design System, and Interaction Quality
Treat the design system as a functional implementation contract, not a visual suggestion. Read the relevant native design documents and inspect the existing implementation before changing UI.
- Reuse existing GPUI components, design tokens, typography, spacing, colors, icons, borders, focus treatments, and interaction patterns. Keep theme-dependent values centralized; do not hardcode duplicate styles in individual views.
- Compose UI from cohesive reusable elements. Follow the installed GPUI version's entity, context, action, focus, subscription, and task lifecycle conventions.
- Keep rendering primarily concerned with presentation. Do not perform I/O, parsing, expensive computation, repeated task creation, or unintended domain mutations from render paths.
- Maintain a single authoritative owner for each piece of state. Derive values where appropriate; keep transient drafts and committed/staged data distinct when their semantics differ.
- Use stable identities and appropriate state ownership so focus, selection, scroll position, and in-progress edits survive unrelated updates. Notify or refresh only the affected state/view scope where supported.
- Keep keyboard and pointer interactions consistent; support appropriate focus order, shortcuts, disabled states, discoverability, and accessible semantics where supported by GPUI.
- Design for loading, empty, error, invalid, disabled, and success states where applicable. Verify behavior at supported window sizes and themes; do not introduce avoidable clipping or layout instability.
- Do not duplicate controls, permanently expose irrelevant configuration fields, introduce redundant state, or create feature-specific editing windows without a justified UX requirement.
- Use progressive disclosure for advanced or conditional configuration. For compound key behaviors, expose a coherent editing flow based on the selected behavior and its supported parameters, rather than overlapping controls or duplicated action editors.
Features without existing designs
Identify the closest approved interaction pattern and compose existing components using the established visual language. Determine where the feature belongs, what the user must configure, and how validation, cancellation, and saving should behave. A missing mockup is not permission to invent a new design system. Routine extensions may proceed; novel navigation, interaction models, or material design-system departures require owner approval. Record consequential design assumptions in the handoff.
## 7. Performance, Reliability, and Security
- Keep the UI responsive during repository loading, file operations, network requests, and firmware builds. Use the project's established asynchronous execution mechanisms; do not block the UI thread.
- Avoid duplicate requests, unnecessary state notifications, unbounded queues, excessive logging, polling, and repeated transformations. Reuse existing cached or derived data when valid.
- Handle cancellation, stale asynchronous results, overlapping operations, and task/subscription cleanup where relevant. Prevent outdated results from overwriting newer user state.
- Preserve source fidelity, invalid drafts, unrelated configuration, and user edits. Do not silently discard unsupported ZMK syntax or overwrite configuration after a failed validation or save.
- Use safe filesystem and process operations. Validate external input and paths; avoid shell interpolation of untrusted data. Preserve existing credential handling and explicit external-action boundaries.
- Preserve Windows, Linux, and macOS compatibility. Avoid machine-specific assumptions in application code and do not claim cross-platform verification without evidence.
- Optimize demonstrated or reasonably identifiable bottlenecks. Do not introduce speculative caching, concurrency, or unsafe optimization that complicates correctness. Measure when claiming a performance improvement.
## 8. Testing Policy — PR Gate
Testing is outcome-oriented and risk-based. New automated tests are deferred until PR preparation by default.
During implementation and bug fixing
- Do not create, write, update, or expand automated tests unless the owner explicitly requests test authoring for the active task.
- Run relevant existing checks and tests when useful. Preserve existing assertions and fixtures. Do not delete, weaken, skip, or rewrite tests merely to make checks pass.
- If a legitimate contract change invalidates existing tests before test authoring is authorized, report the mismatch and defer test updates to the PR gate. Do not claim the affected checks pass.
- A request to implement, fix, refactor, build, verify, or push does not authorize test authoring. Track material untested behavior or regression risks in the task handoff; do not create speculative test scaffolding.
During PR preparation
An explicit request to prepare, create, or open a PR authorizes focused test authoring for that PR after production changes are complete, unless the owner directs otherwise. Test authorization does not independently authorize a remote push or PR creation.
- Review existing coverage first. Add or update only tests that establish meaningful confidence in the changed behavior. A documentation or purely presentational change may need no new automated tests.
- Prioritize domain rules, configuration preservation and round trips, behavior composition, state transitions, user-visible outcomes, error recovery, and regression cases for confirmed defects.
- Prefer core tests for domain logic and integration tests for important boundaries. Use GPUI interaction tests only when they establish a meaningful UI contract not adequately covered elsewhere.
- Assert observable outcomes, not internal implementation details, arbitrary theme values, trivial getters, framework behavior, or incidental call counts. An exception requires a genuine contract or risk.
- Reuse fixtures and existing test infrastructure. Keep tests deterministic, isolated, and readable. Do not introduce a test framework or dependency without approval.
- Do not target arbitrary coverage percentages or test counts, enumerate every permutation without risk-based justification, or add tests for unchanged behavior already adequately covered.
- Update obsolete tests only for an intentional, authorized contract change. Explain why the previous expectation no longer applies.
- Report what was added, what behavior it protects, and what material risks remain unverified.
## 9. Verification and Development Efficiency
Use the actual workspace manifests and existing repository scripts/CI configuration. Apply checks to affected crates first, expanding when shared contracts or release risk justify it.
- During implementation, format changed Rust code, run targeted cargo check, and run relevant existing tests or lint checks as appropriate. Use the configured toolchain and features.
- Before PR readiness, run applicable formatting, Clippy, test, and build checks required by the repository or affected workspaces. Prefer existing CI-equivalent commands. If full verification is impractical, report the exact gaps.
- Reuse build artifacts and caches. Avoid repeated full-workspace builds, redundant test runs, dependency downloads, and cargo clean without a demonstrated need.
- Diagnose failures from the earliest actionable error. Do not repeatedly retry unchanged failing commands or suppress warnings/errors to obtain a green result.
- Review git diff for duplicated logic, unnecessary code, debug artifacts, unrelated formatting, regressions, and unintended design changes before declaring completion.
- Compilation is not runtime verification. Distinguish checked, compiled, tested, manually exercised, and unverified behavior. The owner retains final acceptance of the implementation.
## 10. Git and Worktree Discipline
- Create a new Git worktree for each new independent task beneath the main repository's ignored .worktrees/ directory. Keep related follow-ups and corrections in the active task's worktree.
- Inspect branch, worktree, working-tree status, and intended base before changes. Do not assume uncommitted work in the main checkout exists in a new worktree; report missing prerequisites rather than copying or discarding them silently.
- Do not modify the main checkout, unrelated worktrees, or user changes. Do not use destructive Git operations, force pushes, history rewrites, or automatic cleanup without authorization.
- Keep diffs focused. Do not mix unrelated refactors, dependency updates, generated files, or formatting churn with the requested change.
- Commit, push, open PRs, merge, and remove worktrees only within the owner's explicit authorization. Preserve local work if an operation is blocked.
## 11. Documentation and Handoff
The master knowledge-base entry is /home/adrian/Dropbox/keyforge/GPUI Foundation.md. Native design documents live in mocks/. Follow the Mirukai-style handbook structure in the vault: docs/ for guides, decisions/ for canonical decisions, handoffs/ for active execution/evidence, and handoffs/archive/ for superseded records. Repository documentation/ADRs are pointers.

Update the relevant guide and handoff in the same task when behavior, architectural decisions, or verification evidence changes. Avoid duplicated narratives and documentation churn. Keep records factual, concise, and current. Do not assume the historical base commit 96e2234 contains the uncommitted application. If the vault or design reference is unavailable, report the limitation; do not fabricate its contents or silently create a competing canonical record.

At completion, report briefly:
- Outcome, affected behavior, and important implementation decisions.
- Verification actually performed, results, and remaining gaps.
- Material risks, assumptions, blockers, or owner decisions needed.
- Worktree/branch and commit or PR state, when applicable.
Do not provide exhaustive file-by-file summaries, large code dumps, or repeated explanations unless requested.
## 12. Primary Engineering References
Consult only the references relevant to the task. The pinned dependency source and installed toolchain take precedence over examples targeting other versions.
- Rust documentation: https://doc.rust-lang.org/stable/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/checklist.html
- Clippy: https://doc.rust-lang.org/clippy/
- Cargo: https://doc.rust-lang.org/cargo/
- GPUI upstream: https://github.com/zed-industries/zed/tree/main/crates/gpui
- ZMK documentation: https://zmk.dev/docs
