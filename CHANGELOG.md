# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.10 (2026-06-27)

### Added

- Configurable mailbox capacity via `Actor::mailbox_capacity` (defaults to 64) for tuning backpressure per actor type.
- Module-level and full API documentation for `Link`, `DynLink`, and the `link` module.
- Benchmark infrastructure (criterion overhead bench).

### Changed

- Dual-licensed under `MIT OR Apache-2.0` (previously MIT).
- Overhauled the README with rationale and runnable examples covering static vs. dynamic messages, custom state/props, custom `tick`/`cycle` loops, and `anyhow` error handling.
- Reduced per-actor overhead: collapsed the two tasks per actor into one (panics are now caught inline) and dropped the per-message `Arc` and reply downcast.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release.
 - 175 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Curate changelog for v0.0.10 ([`d32e393`](https://github.com/s-panferov/actor12/commit/d32e39380192dd2e98cc00afe84004c14a6192c6))
    - Merge bench-infra: link docs, README overhaul, dual licensing ([`b1b8459`](https://github.com/s-panferov/actor12/commit/b1b84597f2d54791ca040f36836e7b93cf75f1d7))
    - Merge pull request #5 from s-panferov/docs/readme-license ([`99119d2`](https://github.com/s-panferov/actor12/commit/99119d20bc90d7f218e9c6c4aeb9db63e766a1b1))
    - Overhaul README and dual-license MIT OR Apache-2.0 ([`83814df`](https://github.com/s-panferov/actor12/commit/83814dfca24c6bf77a0e1b08df4bf592bb6bb915))
    - Merge pull request #4 from s-panferov/docs/link-rs ([`8f95e69`](https://github.com/s-panferov/actor12/commit/8f95e690ada2795c3e012fa6232166607a540a1f))
    - Document Link, DynLink, and the link module ([`599b0eb`](https://github.com/s-panferov/actor12/commit/599b0eb96b8e7a70cb2d12209c20bf80593ed522))
    - Merge pull request #3 from s-panferov/bench-infra ([`18ded00`](https://github.com/s-panferov/actor12/commit/18ded006001dda174dc63e7669eda0d1cbf26c9c))
    - Configurable mailbox capacity; drop per-message Arc and reply downcast ([`f2a67ef`](https://github.com/s-panferov/actor12/commit/f2a67ef62d37cb8e80b74c81655942b1c25f7db5))
    - Merge pull request #2 from s-panferov/bench-infra ([`bce7abf`](https://github.com/s-panferov/actor12/commit/bce7abf632bf965c01635bd97f6b573d770c4597))
    - Collapse two tasks per actor into one (catch panics inline) ([`a7c48b0`](https://github.com/s-panferov/actor12/commit/a7c48b09c6a88af22b66a747d2a2e2f30ba51328))
    - Merge pull request #1 from s-panferov/bench-infra ([`a85f7cf`](https://github.com/s-panferov/actor12/commit/a85f7cfeff92460327befde912cd82d0d1b8ec8a))
    - Add benchmark infrastructure (criterion overhead bench) ([`4b42b30`](https://github.com/s-panferov/actor12/commit/4b42b30accdccf07f1f5ed3376cd4873655ae628))
</details>

## v0.0.9 (2026-01-03)

### Changed

- Expose more counts

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release actor12 v0.0.9 ([`02d49cd`](https://github.com/s-panferov/actor12/commit/02d49cd3893bf3412138114240539928cd2ba1df))
    - Even more counts ([`81ff38e`](https://github.com/s-panferov/actor12/commit/81ff38e65733ce30f28d6ab0d79b286b02e5d6f9))
</details>

## v0.0.8 (2026-01-03)

### Changed

- Expose more counts

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release actor12 v0.0.8 ([`2a449ad`](https://github.com/s-panferov/actor12/commit/2a449adb5ec87a171c48e579a111ee2a242eb74b))
    - V0.0.8 ([`4a3ea29`](https://github.com/s-panferov/actor12/commit/4a3ea29316b89107cc8dd8f28fd5729d4968557a))
</details>

## v0.0.7 (2026-01-03)

### Changed

- Exposed counts in actor context

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release actor12 v0.0.7 ([`3c13bf6`](https://github.com/s-panferov/actor12/commit/3c13bf6af90bdcca030c06b2da67d9ed25de2aa0))
    - V0.0.7 ([`41e62c4`](https://github.com/s-panferov/actor12/commit/41e62c4227a626c9832950ffa714ef02d8a016e1))
    - Release actor12 v0.0.6 ([`402255f`](https://github.com/s-panferov/actor12/commit/402255fb4ed8a0370269b17ac8d189a2d6132aad))
    - Expose counts ([`a22b1fe`](https://github.com/s-panferov/actor12/commit/a22b1fed6cbb4638a7379ac7338f12b50e76e6bd))
    - Release actor12 v0.0.6 ([`dbf1721`](https://github.com/s-panferov/actor12/commit/dbf17217d567ef444f3d86e0b9779bc07b923660))
    - Expose counts ([`d1a6728`](https://github.com/s-panferov/actor12/commit/d1a67286b53b024ddc98c1f17d616edd152f56a2))
</details>

## v0.0.6 (2026-01-02)

### Changed

- Improved `Self::tick` API for better ergonomics
- Updated dependencies

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 34 commits contributed to the release over the course of 125 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release actor12 v0.0.6 ([`189637d`](https://github.com/s-panferov/actor12/commit/189637dc2a7ce6e2aad716f776c17aad5af15258))
    - Add changelog entries for v0.0.6 ([`2e4034c`](https://github.com/s-panferov/actor12/commit/2e4034ca79e58aeb420416c9765182da61333e55))
    - Adjusting changelogs prior to release of actor12 v0.0.6 ([`dde598b`](https://github.com/s-panferov/actor12/commit/dde598b98baba5d60f7ee0276470ba913b69475d))
    - Update Cargo.lock ([`d302498`](https://github.com/s-panferov/actor12/commit/d3024981c9eee680edcc53a76866a86ff7d00b6f))
    - Bump version to 0.0.6 ([`7c47cfb`](https://github.com/s-panferov/actor12/commit/7c47cfb97acb0007dc27abf811ccde72dc201d6b))
    - Release actor12 v0.0.5 ([`31717b5`](https://github.com/s-panferov/actor12/commit/31717b5b9020cdcaa5461bdb942225d5537e4c60))
    - Add changelog entries for v0.0.5 ([`4f8fbd7`](https://github.com/s-panferov/actor12/commit/4f8fbd72f54662b23535dc02ac62d76feccf983f))
    - Adjusting changelogs prior to release of actor12 v0.0.5 ([`4d6f479`](https://github.com/s-panferov/actor12/commit/4d6f479be805cb12cb1ff04c932dc7042d2ae7a8))
    - CHANGELOG.md ([`f77c6a4`](https://github.com/s-panferov/actor12/commit/f77c6a492d6c8a56ba147e788338843dff8a4e13))
    - Adjusting changelogs prior to release of actor12 v0.0.5 ([`e5491a6`](https://github.com/s-panferov/actor12/commit/e5491a6a980d93c1dccc283c9d7a3ccd8888851c))
    - CHANGELOG.md ([`c895697`](https://github.com/s-panferov/actor12/commit/c895697086f8c92a04aa2bbb2d9cae14d45052ba))
    - Deps ([`ca7016a`](https://github.com/s-panferov/actor12/commit/ca7016a372915c9f6fbef0c4a1cd6073888fd6c7))
    - Better Self::tick API ([`bffeb57`](https://github.com/s-panferov/actor12/commit/bffeb57aef71a81111b46b0208021d0ac4671189))
    - Release 0.0.5 ([`a21e11f`](https://github.com/s-panferov/actor12/commit/a21e11fac22a93a1a95b1949003b99f35a98bdb2))
    - Add Bazel build system support and refactor handler module ([`0654526`](https://github.com/s-panferov/actor12/commit/06545263f35a1aa221991ad7360d53ef9d494ebd))
    - Add Bazel build artifacts to .gitignore ([`8aba219`](https://github.com/s-panferov/actor12/commit/8aba219b4fd50bfb8f9f978cf578af75c5f80967))
    - Add publish command to justfile ([`dd610cf`](https://github.com/s-panferov/actor12/commit/dd610cf578b828e7f1e2ba9133247f7e8dc021d6))
    - Replace README code examples with docs.rs badge ([`daee4d4`](https://github.com/s-panferov/actor12/commit/daee4d43d7cd8833a94c82e025b7720a70ffbadf))
    - Rename runy-actor to actor12 ([`143e8b4`](https://github.com/s-panferov/actor12/commit/143e8b462bd35a8581bda892ca012a885f15ac5d))
    - Remove CLAUDE.md ([`d393db5`](https://github.com/s-panferov/actor12/commit/d393db5369f11d9ecfbb5ef104fdba12552c6de1))
    - Remove send_message API and keep only existing APIs ([`ba4a168`](https://github.com/s-panferov/actor12/commit/ba4a1680ee13c4ed7c20082ea30cc4165e388e38))
    - Add comprehensive check-all command and use nextest ([`926a258`](https://github.com/s-panferov/actor12/commit/926a258e6401f6d75c814f5b011b862e11210c3a))
    - Replace Astro documentation with Rustdoc ([`26852f1`](https://github.com/s-panferov/actor12/commit/26852f10783cfa4e3b4136de80738abdbad91a19))
    - Make MessageHandle::timeout chainable and remove reply_timeout ([`c919548`](https://github.com/s-panferov/actor12/commit/c91954840aa70580025444b7e2a9062bcdb87c3b))
    - Fix compiler warnings in examples ([`e00b6bb`](https://github.com/s-panferov/actor12/commit/e00b6bbec2842c809206fa531ae9b3a7a0bf6910))
    - Remove empty file ([`445b8cb`](https://github.com/s-panferov/actor12/commit/445b8cb286566d221606bfe158ba7890cd2071d2))
    - Add documentation infrastructure and development tooling ([`2b6c93c`](https://github.com/s-panferov/actor12/commit/2b6c93c53f7c3b4db1fe0671476ce200f925f875))
    - Complete comprehensive integration tests and fix examples ([`a22eab4`](https://github.com/s-panferov/actor12/commit/a22eab4fb587ef384cfad289810bdcbd70636ec5))
    - Complete new two-step API with comprehensive coverage ([`c6e7739`](https://github.com/s-panferov/actor12/commit/c6e7739a0a399136e76cabb4326cc75c37a13ff6))
    - Implement new ergonomic two-step Link API ([`0f43cd9`](https://github.com/s-panferov/actor12/commit/0f43cd9db554c9f12eecba7892faa101ddcbd663))
    - Analyze and redesign Link API for better ergonomics ([`ae90011`](https://github.com/s-panferov/actor12/commit/ae9001198520f14e255db05142ee0e9327bafc7f))
    - Add comprehensive TODO list and improvement roadmap ([`20f0c45`](https://github.com/s-panferov/actor12/commit/20f0c458944fc45d1452d753f233f8fd62ec5e30))
    - Add development learnings documentation ([`1cb3cde`](https://github.com/s-panferov/actor12/commit/1cb3cde2ffc7224f7a6a5c80c0e9a682d2e1774c))
    - Initial commit: Actor12 framework ([`f8f0a01`](https://github.com/s-panferov/actor12/commit/f8f0a0128206f057c6859a1b8f1b7d76d17eb6a9))
</details>

## v0.0.5 (2026-01-02)

### Changed

- Improved `Self::tick` API for better ergonomics

