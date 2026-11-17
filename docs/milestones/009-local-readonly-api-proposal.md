# Alpha 1, Prototype 009: Local Read-Only API

**Status: PROPOSAL — awaiting Chris’s architectural review and separate implementation authorization.**

Planning completed November 17, 2026. PR #7 has been merged and verified. This document proposes an experimental Linux, current-user API; no API listener, application code, dependency installation, deployment or Prototype 010 work was performed. **Decision 43 remains Pending and unchanged.** Numeric limits and policy choices below are recommendations, not accepted guarantees.

## 1. Verified foundation and planning outcome

Chris authorized a regular merge of [PR #7](https://github.com/lgraak/Siliconwatch/pull/7), verification, and this proposal. Fresh premerge checks confirmed the expected sole contribution, author-matching DCO signoff, eleven-file manifest and hashes, successful final-HEAD CI, main target and mergeability.

The resulting merge is **231034f2822b9c91e8781c20579729374328280d**. Its two parents, in order, are:

1. Previous main: **28d7177cc334e357646023e5a2292b0cc434e0a7**.
2. Reviewed contribution: **0df9ac1e342b617748bf65ca8ac38e0d67636461**.

Its tree **fdbe8247730bdd6642378ce68ebd9f9e4f4825b9** equals the reviewed contribution tree. GitHub marks PR #7 merged; the contribution is reachable from fetched origin/main, which points to the merge. No squash, rebase, force-push, branch deletion or branch-protection change occurred.

**Planning completion: Complete; implementation: not started.** The only proposed repository change is this document, on local branch `codex/alpha1-prototype009` in a separate worktree created from the verified merge. No new contribution commit or publication is authorized.

## 2. Current architecture and constraints

Reviewed [README](../../README.md), [design decisions](../DESIGN_DECISIONS.md), [Prototype 007 cache report](007-shared-snapshot-cache.md), [Prototype 008 report](008-headless-daemon-lifecycle.md), and [readiness review](008-publication-readiness.md), together with the source contracts below.

| Existing component | Relevant contract |
| --- | --- |
| [daemon](../../src/daemon.rs) | Main coordinates signals; one acquisition worker owns the cache. Private lifecycle and inspection; cooperative shutdown joins the worker. |
| [snapshot cache](../../src/snapshot_cache.rs) | Unique non-cloneable CacheOwner; cloneable CacheReader; borrowed read guard; one latest attempt plus, after failure, at most one historical successful observation. |
| [hwmon model](../../src/hwmon.rs) | Snapshot schema 2; existing Serialize implementations retain raw text/value, converted value/unit, paths, identities and diagnostics. Snapshot is not Clone. |
| [identity](../../src/identity.rs) | Identity schema 1; candidate is best-effort evidence, not physical-identity proof. |
| [CLI](../../src/main.rs) | Existing sensors/sample contracts and daemon lifecycle messages must remain compatible. |
| [Cargo configuration](../../Cargo.toml) | Rust 2024, minimum Rust 1.85; serde, serde_json and signal-hook. Locked signal-hook is 0.4.6. |

CacheReader::read currently blocks on an RwLock. Assessment borrows data and has no existing wire serialization. Daemon inspection currently reduces state to private reporting labels. These are implementation constraints, not an already available API.

The acquisition owner scans and publishes independently of readers. API requests must receive only a reader and lifecycle view: **no CacheOwner, acquisition method, scan callback, root-selection request, or refresh operation**. New reads cannot schedule or accelerate hardware access. Existing foreground sensors/sample remain separate commands, unchanged.

## 3. Recommended transport and explicit design departures

Recommend an **opt-in pathname Unix-domain stream socket, with a small versioned JSON-line protocol**, initially restricted to the daemon’s current non-root user.

| Alternative | Benefits | Costs / recommendation |
| --- | --- | --- |
| Pathname Unix socket + JSON lines | Filesystem isolation, kernel peer credentials, existing JSON serialization, small fixed protocol | Linux-specific permission assumptions; clients need a Unix-stream protocol implementation. Recommended for this prototype. |
| TCP loopback HTTP | Familiar HTTP clients and tooling | Loopback is not user authentication; other local users can connect. Requires a separately designed credential and HTTP parsing policy, including browser-origin concerns. Reject for 009. |
| HTTP over Unix socket | Retains socket permissions with HTTP tooling | Adds HTTP grammar/limits without a requirement for routing, headers or streaming. Defer. |
| Session/system D-Bus | Existing local identity/policy mechanisms; matches accepted initial direction | Requires bus integration and explicit bus policy/dependency choices. Valid alternative if Chris prefers continuity with Decision 5. |

**Decision 5 currently names D-Bus as the initial API direction.** The custom Unix-socket protocol is a proposed, narrowly scoped departure, not a claim that the decision already permits it. Chris must approve the exception or select D-Bus before implementation; any accepted change needs its own design-record update.

**Decision 26 anticipates one shared system-wide daemon.** This proposal allows one opt-in instance in the current user’s runtime namespace, not multi-user access to that future shared service. Concurrent logins for the same user share that namespace. It does not authorize deploying multiple collectors system-wide. Cross-user telemetry, service-account ownership and administrative policy remain unresolved.

The installed runtime stays Rust-only. No HTTP framework, async runtime, authentication service or new Python runtime is justified.

## 4. Proposed Decision 43 policy — Pending

Recommend the following text for review, without editing or accepting Decision 43:

> For Prototype 009, an explicitly enabled local API allows only clients whose kernel-reported effective UID at connection establishment matches the daemon’s non-root effective UID. A private pathname Unix socket exposes only versioned informational and snapshot reads. Read access grants no configuration, acquisition scheduling or hardware-control authority. Cross-user, privileged and remote access remain unavailable pending separate policy approval.

Require real UID = effective UID, both nonzero, for this initial API mode. This is a proposed restriction on the new mode; it does not change existing daemon behavior without the opt-in flag. No elevation, setuid installation, groups, ACL-based sharing, tokens, secrets or PID allowlist.

Before parsing request bytes, obtain Linux SO_PEERCRED, validate the returned structure and compare the peer UID with the captured daemon UID. Failure or mismatch closes the connection without sensor data. Future clients should check the server UID too. The credential authenticates a connected principal, not an executable, and reflects connection-time identity; already-authorized descriptors may be passed to another process. Root, privileged namespace manipulation and malicious same-UID code are outside this isolation boundary. [Linux Unix-domain socket semantics](https://man7.org/linux/man-pages/man7/unix.7.html)

Owner-only permissions alone would be smaller, but kernel UID validation adds an explicit principal check. Group access would need membership/revocation policy; system-wide read access would reveal potentially identifying source paths and diagnostics. Neither is selected here. Administrator configuration and protective-action permissions remain separately unresolved. No automatic data redaction is proposed for this authorized local reader: preserve telemetry truth and document its sensitivity.

### Socket location, ownership and cleanup

Proposed opt-in usage, **not implemented**:

```text
siliconwatch daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

Use `$XDG_RUNTIME_DIR/siliconwatch/api.sock`. Without --local-api, preserve the existing execution path. In API mode, missing/invalid runtime-directory configuration is a setup error; do not silently use a shared /tmp socket or bind TCP. This is a deliberately narrower prototype policy than the XDG specification’s suggested fallback.

Require a trusted local runtime filesystem with Linux permission semantics as an environment assumption; standard metadata checks alone do not establish filesystem locality. Validate an absolute runtime directory owned by the effective UID with mode 0700. Reject symlink components, relative paths, unsafe ownership and writable ancestors controlled by other users. Ancestors must be root- or effective-UID-owned with no group/other write. For fixture directories, permit only the explicit exception of a root-owned sticky /tmp ancestor followed by an already-owned private user directory; reject other shared writable ancestors. Do not infer safety from canonicalization or a leaf lstat alone. Trusted root and same-UID processes must not concurrently replace this namespace. [XDG runtime-directory requirements](https://specifications.freedesktop.org/basedir/latest/)

Create the fixed siliconwatch child exclusively with mode 0700, verifying actual ownership/mode before use. **Any existing child or socket is a startup error**, including a stale object: never unlink, chmod or take over an existing endpoint. Bind a pathname socket, set/verify mode 0600 before accepting, and reject overlong paths without truncation. Creation-time restrictions and the private parent avoid a world-accessible interval; do not change process-wide umask after threads start. Abstract sockets are excluded because filesystem permissions do not apply to them.

Record device/inode/type identity of this instance’s directory and socket. On shutdown, after API worker exit, unlink only its identity-matching socket and remove only its own matching empty directory; never recursively remove a directory. On mismatch, leave the replacement untouched and report cleanup failure. These checks prevent accidental replacement cleanup within the trusted namespace; they are not atomic protection against a hostile same-UID replacer. A crash can leave a stale directory. Recovery is explicit/manual after the user establishes no instance is running; failed connection attempts alone do not authorize deletion. No automatic stale recovery or PID file is included.

## 5. Proposed Rust boundaries and dependency review

Keep these concrete boundaries small; filenames are prospective, not created:

| Future file/change | Responsibility |
| --- | --- |
| src/local_api.rs | Bounded framing, permission/setup helpers, one API worker, stream cancellation and endpoint ownership. Split only if actual size warrants it. |
| src/local_api/wire.rs | Versioned borrowed response DTOs, explicit enum mappings, preflight limits and capped in-memory serialization. |
| src/local_api/tests.rs; tests/local_api_cli.rs | Deterministic contract tests and isolated socket/process fixtures. |
| src/daemon.rs | Minimal read-only service view, API worker ownership and failure/stop integration; preserve acquisition ownership/scheduling. |
| src/snapshot_cache.rs | Narrow try_read entry point reusing ReadView/assess; distinguish contention from poisoning. Existing read and cache publication semantics stay unchanged. |
| src/main.rs; src/lib.rs | Opt-in argument parsing and minimal internal module exposure. |
| docs/api/local-v1.md | Public protocol, limits, examples, permission assumptions and compatibility rules, written during authorized implementation. |

Standard-library UnixListener/UnixStream, synchronization, time, filesystem and existing serde/serde_json suffice for most behavior. **Stable Rust 1.85 does not expose UnixStream::peer_cred**; its API is unstable. Recommend a direct, narrowly audited libc declaration using the already locked **0.2.191** version for real/effective UID queries, SO_PEERCRED and a small instance-token entropy wrapper. Do not rely on an undeclared transitive dependency. [Rust 1.85 source](https://raw.githubusercontent.com/rust-lang/rust/1.85.0/library/std/src/os/unix/net/stream.rs)

The cached libc manifest declares Rust 1.65 and MIT OR Apache-2.0. Declared compatibility and prior transitive builds do not test future wrappers. Keep unsafe code confined to reviewed functions checking sizes, return values, EINTR, buffer bounds and ownership. A safe narrow crate such as rustix is an alternative if wrapper review demonstrates a concrete benefit, but requires a separate feature/lockfile audit; no choice is installed now. No framework is proposed.

Create one public 128-bit random instance token per cache creation, rendered as 32 lowercase hex characters. It identifies the monotonic epoch; it is not an authentication secret or mathematical uniqueness guarantee. Proposed libc getrandom with GRND_NONBLOCK fills 16 bytes using bounded partial/EINTR retries; zero progress, unavailable entropy or unsupported/error results fail API setup. Never fall back to PID, wall time or a weak generator. Tests inject a deterministic token. [Linux getrandom](https://man7.org/linux/man-pages/man2/getrandom.2.html)

## 6. Versioned request/response contract

One connection carries **one UTF-8 JSON object followed by LF**, then one response object and LF, then close. No HTTP, compression, subscriptions, keepalive, pagination, client-specified paths or pipelining. Read only the first bounded frame; never execute additional frames. Do not wait for EOF to respond or promise detection of bytes sent later. A closed/partial response is not a valid record.

The only request fields are api_version (integer 1) and method (get_info or get_snapshot). Reject unknown/duplicate fields, extra JSON values within the first frame, malformed UTF-8/JSON and unknown methods. Parse with explicit typed request structures and retain serde_json’s recursion limit. Negotiate by get_info or a structured unsupported_version response advertising [1]; do not silently downgrade.

Every response has api_version, instance_id, ok, and exactly one of result or error. API version **1**, embedded Snapshot schema **2**, and embedded identity schema **1** are independent. Exact field names, types, enum spellings and nullability are a public experimental contract. Clients may ignore unknown additive response fields; unknown major versions or enum values must not be treated as current/healthy. Incompatible meaning or field changes require a new API version.

get_info reports supported API versions, methods, snapshot/identity versions, read_only=true, limits and the instance token. It does not inspect hardware or claim sensor health. get_snapshot returns this point-in-time envelope:

| Field in result | Contract |
| --- | --- |
| lifecycle | starting, running, stopping or stopped; unavailable/poisoned lifecycle yields an API error rather than invented state. |
| assessed_at_ns; stale_after_ns | Decimal-string nonnegative nanoseconds, relative to this cache’s origin / configured threshold. |
| producer | active or stopped, as assessed by the existing cache. |
| service_current | Derived conjunction below; not a per-row success flag or continuing liveness guarantee. |
| physical_age_known | Always false for this implementation. |
| latest | state=empty/succeeded/failed; attempt=null only for empty; error=null except failed. |
| observation | null if no successful observation; otherwise its own attempt, role, freshness and full unchanged snapshot. |

Each attempt includes sequence, started_ns, finished_ns, published_ns, wall_start_unix_ms and wall_time_error. Encode sequence, durations and optional wall milliseconds as decimal strings (null for absent wall time); Duration nanoseconds use checked u128 conversion, without u64 narrowing. Strings avoid precision loss in future JavaScript clients. Wall time is optional correlation only; offsets/sequences are comparable only within the same instance_id.

A failed latest attempt includes the original acquisition error kind as an opaque diagnostic string, raw_os_error as nullable i32, and message. Clients branch on latest.state, not Rust debug spelling or localized messages. Snapshot errors/identity diagnostics are preserved separately. Reading a failed acquisition is a **successful API retrieval** (ok=true, latest.state=failed), not a transport/server failure.

An observation has role latest_attempt, historical_after_failure or historical_after_stop, copied from cache semantics. Its attempt is the retained successful attempt, which may differ from latest.attempt. Freshness has state no_sensor_observations / within_budget / stale and nullable lower_ns/upper_ns (both null only for no sensor observations).

Preserve all Snapshot fields (schema_version, root, sensors, warnings), every Sensor field and ordering, and identity candidate/status/evidence/issues. In particular raw_text and exact i64 raw_value remain distinct from canonical value/unit. raw_text is the scanner’s stored text, not a claim of original untrimmed sysfs bytes. Preserve nulls, unsupported inputs, ambiguity and questionable readings. Existing raw_value JSON numbers must be parsed losslessly by clients; ordinary floating-point JSON numbers cannot represent every i64. Do not round or stringify that existing schema field. Reject non-UTF8 paths or otherwise unrepresentable data explicitly instead of lossy path conversion; preflight non-finite converted values rather than silently emitting null.

### Freshness, failures and historical state

Service-current requires **Running AND active producer AND latest succeeded AND observation role latest_attempt AND within_budget**. Each row’s status remains independently authoritative.

| Scenario | Required representation |
| --- | --- |
| Cache has never published | latest empty, observation null, service_current false. |
| Successful empty inventory | latest succeeded, full empty snapshot, no_sensor_observations, service_current false. |
| Partial/unsupported rows | Preserve all rows/statuses; global currentness never validates every sensor. |
| Latest success aged beyond threshold | freshness stale, service_current false; do not relabel the cache role. |
| Whole acquisition fails with prior success | latest failed with its error/attempt; observation retains older successful attempt, historical_after_failure while active; non-current. |
| Failure before any success | latest failed; observation null. |
| Later successful acquisition | New latest attempt replaces failure normally; no fabricated repair/history. |
| Stopping while admitted scan completes | Lifecycle overrides currentness even if producer is active and cache role remains latest_attempt. Preserve both dimensions. |
| Owner stopped | historical_after_stop overrides prior failure role; latest failure, if any, remains explicit. |

Userspace age bounds are [assessed_at - observation.finished, assessed_at - observation.started]. The upper bound **greater than** threshold is stale; equality is within budget. Physical sensor age remains unknown. Use cache metadata, never scheduler offsets or receipt wall time. These bounds apply only at server assessment. Clients may display time since receipt separately, but adding it to transmitted bounds omits transport delay and cannot establish a current upper age bound or freshness at receipt. Any receiving-side freshness bound needs a separately specified RTT/clock/suspend model and is deferred. A response is valid at its assessment instant, not continuously through transmission. Known suspend requires the existing cache recreation contract; automatic suspend detection is excluded.

### Example interactions — proposed, not executed

Wire request (one line including final LF):

```json
{"api_version":1,"method":"get_snapshot"}
```

Complete illustrative response for an empty starting cache (pretty-printed here for readability; actual framing is compact JSON plus one LF):

```json
{
  "api_version": 1,
  "instance_id": "0123456789abcdef0123456789abcdef",
  "ok": true,
  "result": {
    "lifecycle": "starting",
    "assessed_at_ns": "1000000",
    "stale_after_ns": "3000000000",
    "producer": "active",
    "service_current": false,
    "physical_age_known": false,
    "latest": {"state": "empty", "attempt": null, "error": null},
    "observation": null
  }
}
```

For a fixture success, the returned snapshot must preserve a row such as raw_text="42000", raw_value=42000, value=42.0, unit="°C", together with its full paths/identity/status. It must not replace raw_value with 42 or Fahrenheit. If attempt 2 subsequently fails, latest.attempt.sequence="2" and its error coexist with observation.attempt.sequence="1" and historical_after_failure; a young observation still has service_current=false.

Complete synthetic failure-with-history response; these are invented fixture values, not new measurements:

```json
{
  "api_version": 1,
  "instance_id": "0123456789abcdef0123456789abcdef",
  "ok": true,
  "result": {
    "lifecycle": "running",
    "assessed_at_ns": "2000000000",
    "stale_after_ns": "3000000000",
    "producer": "active",
    "service_current": false,
    "physical_age_known": false,
    "latest": {
      "state": "failed",
      "attempt": {"sequence":"2","started_ns":"1000000000","finished_ns":"1010000000","published_ns":"1020000000","wall_start_unix_ms":null,"wall_time_error":"fixture wall clock unavailable"},
      "error": {"kind":"NotFound","raw_os_error":2,"message":"fixture root unavailable"}
    },
    "observation": {
      "attempt": {"sequence":"1","started_ns":"100000000","finished_ns":"110000000","published_ns":"120000000","wall_start_unix_ms":null,"wall_time_error":"fixture wall clock unavailable"},
      "role": "historical_after_failure",
      "freshness": {"state":"within_budget","lower_ns":"1890000000","upper_ns":"1900000000"},
      "snapshot": {
        "schema_version": 2,
        "root": "/fixture",
        "sensors": [{
          "identity": {"schema_version":1,"candidate":null,"status":"insufficient_evidence","evidence":{"provider":"fixture","parent":null,"device_link":null,"canonical_parent":null,"channel":"temp1","measurement":null},"issues":["fixture has no parent-device identity"]},
          "device": "fixture",
          "device_path": "/fixture/hwmon0",
          "attribute": "temp1_input",
          "label": null,
          "source_path": "/fixture/hwmon0/temp1_input",
          "raw_text": "42000",
          "raw_value": 42000,
          "value": 42.0,
          "unit": "°C",
          "status": "ok",
          "error": null
        }],
        "warnings": []
      }
    }
  }
}
```

Capability request and complete illustrative reply:

```json
{"api_version":1,"method":"get_info"}
{"api_version":1,"instance_id":"0123456789abcdef0123456789abcdef","ok":true,"result":{"supported_versions":[1],"methods":["get_info","get_snapshot"],"snapshot_schema_version":2,"identity_schema_version":1,"read_only":true,"limits":{"request_bytes":4096,"response_bytes":1048576,"active_clients":1,"request_deadline_ms":1000,"response_deadline_ms":2000,"max_sensors":4096,"max_warnings":1024,"max_identity_issues_per_sensor":64,"max_text_bytes":16384,"max_total_text_bytes":262144}}}
```

Unknown version:

```json
{"api_version":99,"method":"get_info"}
{"api_version":1,"instance_id":"0123456789abcdef0123456789abcdef","ok":false,"error":{"code":"unsupported_version","message":"Unsupported API version","retryable":false,"supported_versions":[1]}}
```

No cache acquisition is implied by any interaction. Public documentation during implementation must include complete successful nonempty and failure-with-history fixture responses, all enum values, errors, timing arithmetic and lossless parsing guidance.

## 7. Resource bounds, locks and concurrent clients

Recommend **one additional API worker**, one active connection, no user-space request queue and no thread per connection. Main remains signal coordinator; the existing acquisition worker remains the only owner. Concurrent clients wait in the OS-managed listen backlog or receive connection errors. This is deliberate head-of-line behavior; there is no fairness guarantee or claimed bound on total kernel socket memory. The first GUI must use one request at a time, poll no faster than the configured acquisition interval and back off on busy/errors.

Proposed fixed initial limits, advertised by get_info:

| Resource | Proposed bound / behavior |
| --- | --- |
| Request frame | 4 KiB including LF, enforced while accumulating, not after unbounded read_line. |
| Complete encoded response | 1 MiB including LF; preallocate a capped buffer, never stream borrowed telemetry to a socket. |
| Request / response-phase budget | 1 s from acceptance for request framing; 2 s from completed/rejected frame (or read timeout) for assessment, preflight, encoding and all response writes together; absolute monotonic deadlines. |
| Active application connections | One accepted stream plus at most one cancellation clone, one listener; no accumulated clients or responses. |
| Snapshot preflight | At most 4096 sensors, 1024 warnings, 64 identity issues per sensor. |
| Input text/path bounds | At most 16 KiB each; at most 256 KiB aggregate source text/path bytes, checked arithmetic. |
| Idle accept wake | Nonblocking accept with a stop-aware condition-variable wait of at most 50 ms requested duration; scheduling is not real-time. |

All text/path fields, including root, device, source paths, labels, raw text, errors, identity evidence/candidate/issues, warnings and wall-time errors, count toward preflight. Check collection counts and string/path lengths before traversing contents; stop at the first exceeded bound. Preflight then capped serialization bounds API work/output; it does not bound the existing cache’s payload size, allocator overhead, thread stacks or process RSS.

For get_snapshot: try cache read lock, then try lifecycle lock (same order as existing inspection); assess/capture lifecycle while both held. Release lifecycle lock immediately. Preflight and encode borrowed DTOs into bounded memory while retaining the cache view, then **release the view before all socket I/O**. Distinguish WouldBlock from poison; return cache_busy without spinning, sleeping or rescanning. No nested cache reads or reverse lock order. Use the captured lifecycle/assessment consistently even if a later stop changes the world.

Preflight checks cancellation and the response-phase deadline between bounded entries; the capped writer checks both between writes. Encoding and transmission share that same deadline: beginning a write never starts a fresh budget. On any size/serialization error discard the entire partial buffer, drop guards, and return a small fixed error response only within the remaining response budget. On expiry, close; a timeout error record is not guaranteed. No truncated sensor list, omitted diagnostic, partial snapshot or retained per-client snapshot clone. Encoding can still delay publication for its bounded traversal; std RwLock offers no writer-fairness or hard latency guarantee. Acceptance must test contention rather than assume a bound. [Rust RwLock behavior](https://doc.rust-lang.org/std/sync/struct.RwLock.html)

Recompute remaining timeout before each partial read/write; reject elapsed/zero remaining before calling timeout setters. EINTR, WouldBlock and partial progress never reset the absolute budget. Application cancellation shuts down the active stream clone to wake I/O. No socket timeout is a hard end-to-end shutdown guarantee. [Rust UnixStream timeouts and shutdown](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html)

## 8. Lifecycle, failures and cleanup

Recommended integration sequence after approval:

1. Parse options, register signals, validate directory/UIDs, obtain instance token and prepare a private listener before acquisition. Setup failure starts no scan; clean only resources created by this invocation.
2. Create the existing cache/control once. Spawn one API worker with reader/view only; spawn acquisition under the existing stop/admission gate. Main must not introduce a competing readiness wait before signal coordination. API startup/fatal exit before acquisition readiness requests stop and closes signal delivery.
3. The API worker can expose Starting/empty state. Acquisition startup notice remains a point-in-time transition, not evidence of healthy sensors or a request-completion deadline.
4. Signals or fatal worker errors close acquisition admission and API acceptance. Registering an active stream clone and observing the API stop predicate share one mutex, so cancellation cannot miss a just-accepted connection. Never hold that mutex while doing socket I/O or joining.
5. Cancel active API I/O, wake idle accept, join the API worker and remove its endpoint before waiting indefinitely for an admitted hardware read. Complete the original owner cleanup/acquisition join before reporting Stopped. Keep signal registration alive through all joins; repeated signals remain idempotent.
6. An API worker fatal error/panic wakes the signal coordinator and participates in the same cleanup path. Every post-spawn failure must join both workers; never detach a thread or report clean shutdown while one remains alive.

The existing spawn_or_stop helper immediately marks Stopped when acquisition-thread creation fails. It cannot be reused unchanged in API mode after an API worker has been spawned: that worker must first be cancelled/joined and its endpoint cleaned up. Preserve the old no-API path; classify final state only after all actually created workers are joined.

Routine malformed/oversized requests, denied peers, timeouts, disconnects, broken pipes and response-too-large errors terminate only that request. They do not terminate acquisition or emit unbounded per-client logs. Unexpected accept failures, internal invariant errors and API worker panic are fatal when API mode was explicitly requested; avoid running silently without the requested endpoint. Cache/control poison or invalid monotonic state must close service admission and report failure, never return a healthy snapshot.

Suggested stable API error codes: invalid_request, unsupported_version, unknown_method, request_too_large, request_timeout, cache_busy, response_too_large, serialization_failed and service_unavailable. Error messages are bounded static public text. Set retryable=true only for request_timeout and cache_busy, meaning a later connection may succeed with the same request, without a success guarantee. Set it false for all other codes; do not automatically retry those without changing the request, resolving the condition or restarting the service. Only unsupported_version adds supported_versions. Unauthenticated peers receive no JSON. Once transport output is broken, no error response is guaranteed. A client accepts only a complete LF-terminated response.

A stop racing an already encoded response may prevent transmission; check cancellation before the first write. Already sent bytes cannot be retracted, so point-in-time semantics remain essential. Stopping/Stopped can be represented by the DTO and tested, but **a final Stopped reply is not promised**: the listener closes during shutdown. EOF/refusal is not proof of graceful completion. Signal/exit semantics and unbounded hardware-read/cache/output shutdown limitations from Prototype 008 remain; no second-signal forced exit or hard deadline is added.

## 9. Deterministic acceptance tests and verification plan

No API tests below have been implemented or executed. Use temporary current-user-owned filesystem/socket fixtures, fake clocks, controlled cache publications, barriers and injected OS-call results; never inspect real hwmon in CI.

| Area | Required acceptance evidence |
| --- | --- |
| Permissions | Exact directory/socket modes, real/effective UID rejection, peer-UID match/mismatch and getsockopt failures. Actual same-UID socket integration plus injected mismatch; real cross-UID test only where safely provisioned, with any skip reported. |
| Namespace | Missing/relative runtime root, symlink at every component, wrong owner/mode, unsafe ancestor, explicit sticky-/tmp fixture case, overlong path, pre-existing file/socket/dir, second instance and identity-changed cleanup. Nothing pre-existing is removed or chmodded. |
| Entropy | Deterministic token injection; partial/zero/EINTR/EAGAIN/ENOSYS failures; no weak fallback or acquisition on setup failure. |
| Protocol | Exact methods/version, duplicates/unknown fields, invalid UTF-8/JSON, unsupported version, frame boundary ±1 byte, missing LF, extra values, first-frame-only behavior, disconnects and partial writes. |
| Contract preservation | Full golden fixture responses preserve schema 2/identity 1, ordering, large i64 values, raw/converted distinction, paths, missing/unsupported/fault rows, identity collisions and diagnostics; non-UTF8/nonfinite data fail explicitly. |
| Freshness/history | Never-published vs empty success; exact-threshold equality; stale; first failure; historical success after failure; recovery; stopped-role precedence; stopping with late admitted publication; unknown physical age and wall-clock diagnostics. |
| Concurrency | API reads cause zero acquisition calls; unique owner invariant; cache/control contention returns busy; no lock inversion; partial network writes occur only after guard release; publication progresses while a client reads slowly. |
| Bounds | Every collection/text limit, aggregate overflow, JSON escaping/output expansion, over-cap entire-response rejection, one active client, no user-space queue or retained history. |
| Deadlines | Absolute budgets survive trickled input, partial writes and repeated interruption; timeout/cancel cleanup releases buffer/guards and reaps fixture processes. |
| Lifecycle | Stop during accept/registration/read/encoding/write, idle wake, setup/thread-spawn failures, API/acquisition panic, poisoned control/cache and blocked admitted read. Coordinator wakes, API closes, and neither worker is detached. |
| Compatibility | Existing sensors/sample/daemon behavior with API disabled, all existing Rust/Python/DCO checks and schema fixtures remain unchanged. |

After separate implementation approval: run cargo fmt --check; configured cargo clippy --locked -- -D warnings; cargo test --locked on Rust 1.85.0 and installed stable; release fixture build; strict Python tests with zero skips; DCO-checker tests. Audit the exact future libc declaration/features/lockfile/license and wrapper safety. No new hardware benchmark is needed for architecture correctness. Supplemental all-target Clippy’s two baseline findings remain separately scoped; do not fix them opportunistically.

## 10. Security limits, exclusions and approval questions

This isolates different unprivileged UIDs under a trusted local runtime filesystem. It does not sandbox clients sharing the daemon UID, stop root, prevent same-user denial of service, guarantee fairness, correct sensors, conceal hardware metadata from authorized readers, or certify production readiness. Filesystem namespace validation and signal/accept races need careful implementation review. Responses over the cap are unavailable as a whole; no pagination fallback is authorized.

Explicitly exclude API implementation in this planning task and, from the proposed 009 implementation scope: GUI, database/history, remote access, TLS, authentication service, hardware controls, protective actions, calibration, adaptive polling, alarms/notifications, production systemd deployment, subscriptions, mutations and Prototype 010. Celsius remains the default presentation unit; future global/per-sensor Fahrenheit preferences belong in the GUI/presentation layer and never rewrite telemetry.

Chris’s review is required for:

1. The custom Unix-socket exception to Decision 5, or selection of D-Bus instead.
2. Current-user, non-root access with same-UID kernel checks and the explicit temporary restriction relative to Decision 26.
3. Proposed Decision 43 policy and its still-unresolved future cross-user/configuration/protection permissions.
4. Opt-in flag, strict XDG namespace/stale-object policy and manual recovery.
5. Protocol version/framing, complete-response semantics, instance-token design and provisional resource/deadline limits.
6. Narrow direct libc use and small try_read/lifecycle-view additions, without changing acquisition semantics.
7. Only after architectural acceptance, separate implementation authorization and any necessary dated design-record update. Do not mark Decision 43 Accepted/Implemented merely because this proposal exists.

## 11. Recommended implementation sequence

After approval, keep one small Prototype 009 milestone: first wire DTO/preflight and deterministic serialization tests; then minimal nonblocking service view; then permission/namespace helpers and fixture listener; then bounded client processing; finally daemon shutdown integration, compatibility regression checks and public API documentation. Stop for review if a framework, cache redesign, additional acquisition owner or broader permissions become necessary. Do not start a later milestone automatically.

Acceptance requires one read-only owner-restricted endpoint, zero request-triggered scans, exact raw/identity/diagnostic preservation, explicit failure/history/freshness semantics, bounded API buffers/clients, no socket I/O under cache guards, coordinated cleanup and the fixture checks above. Numeric limits remain subject to Chris’s review; there is no performance certification.

## 12. Actual planning verification, preservation and Git status

Inspected [Alpha 1 CI run 44071512407](https://github.com/lgraak/Siliconwatch/actions/runs/44071512407) for exact contribution HEAD 0df9ac1e342b617748bf65ca8ac38e0d67636461. Both jobs and all configured steps passed: Rust 1.85.0 formatting, configured Clippy, **105 executable tests + 2 compile-fail doctests**, release fixture build, **153 Python tests without skips**, **6 DCO-checker tests**, and the sole contribution signoff. These are observed Prototype 008 hosted results, not tests of an implemented API. Local DCO checking with full SHAs passed again before merge.

No Cargo/Python test suite, API listener, functional API execution, hardware measurement or benchmark was run for this documentation-only planning task. Rust 1.85 compatibility of future API code remains unverified. Source/contract review, primary-source documentation research, merge-parent/tree/reachability checks and document checks are the applicable planning verification. No package was installed.

Preservation checks cover the original checkout’s 78-file baseline, Prototype 007’s 75 files and Prototype 008’s 81 files; original intentional changes and clean 007/008 states remain preserved. Agent-skills setup, the intentional Prototype 006 report, nine prior feature branches, 19 published benchmark artifacts across the three earlier worktrees, ten retained archive payloads, 78 recorded capture hashes, 21 source hashes, two binaries and four completion-artifact checks were preserved. All 225 private evidence files remain excluded. Prototype 004’s 90 recorded-size checks over 88 captures do not establish historical byte identity where original hashes are absent.

The new worktree is **SiliconWatch-prototype009**, branch **codex/alpha1-prototype009**, HEAD/base **231034f2822b9c91e8781c20579729374328280d**. Its sole intended change is this untracked proposal. Index empty; no upstream, new commit, push or PR. Existing application, Cargo, CI, earlier reports and DESIGN_DECISIONS.md remain unchanged. Merge and planning evidence/continuation stay outside all worktrees.

## Revision history

| Date | Change |
| --- | --- |
| 2026-11-17 | Verified regular merge of PR #7; prepared local API architecture/permissions proposal only. Decision 43 remains Pending; implementation awaits Chris. |

## HANDOFF TO CHATGPT

Chris authorized the regular merge of Prototype 008 PR #7 and **Prototype 009 architecture planning only**. [PR #7](https://github.com/lgraak/Siliconwatch/pull/7) is merged as **231034f2822b9c91e8781c20579729374328280d**. Its parents are exactly previous main **28d7177cc334e357646023e5a2292b0cc434e0a7** and reviewed contribution **0df9ac1e342b617748bf65ca8ac38e0d67636461**. The merge tree equals the reviewed contribution tree, and the original contribution remains reachable from fetched origin/main at that merge. Feature branches and history were preserved.

Before merging, the exact eleven-file manifest and hashes, sole contribution, author-matching DCO signoff, main target and mergeability were reverified. [Hosted CI run 44071512407](https://github.com/lgraak/Siliconwatch/actions/runs/44071512407) passed formatting, configured Clippy, **105 Rust tests plus two doctests, 153 Python tests without skips, six DCO-checker tests**, and contribution signoff verification. These are Prototype 008 results; no API implementation or runtime validation is claimed.

A separate **SiliconWatch-prototype009** worktree now holds **codex/alpha1-prototype009** at the merge. Its only change is the untracked proposal **docs/milestones/009-local-readonly-api-proposal.md**. The index is empty, with no upstream, new contribution commit, push or PR. Original checkout changes, Prototype 007/008 worktrees, agent skills, intentional Prototype 006 changes, historical archives and all 225 private evidence files remain preserved.

The proposal recommends an opt-in, current-user Unix-domain socket with private directory/socket permissions, kernel same-UID checks and two versioned read-only methods. One bounded API worker consumes the existing cache; requests cannot acquire hardware or create another owner. Responses preserve raw readings, converted values, identities, diagnostics and distinct failure/historical states. Cache guards are released before socket I/O. Explicit limits, deadlines, lifecycle cleanup and deterministic fixture tests are specified. A narrow direct libc declaration is proposed for Linux identity/credential and instance-token operations; nothing was installed or changed.

**Decision 43 remains Pending and unchanged.** Chris must review the permissions recommendation, the proposed departure from Decision 5’s D-Bus direction, and the current-user restriction relative to Decision 26’s shared service. Transport limits, manual stale-socket recovery and dependency choices also await approval.

Planning is complete, with no unresolved merge blocker. Future implementation still needs Rust 1.85 verification; same-UID/root isolation, fairness, physical sensor age and hard shutdown deadlines are not guaranteed. No new hardware measurements, deployment, GUI, controls or Prototype 010 work occurred. The repository’s **/handoff** skill was invoked and an external continuation saved. Recommended next action: **Chris’s architectural review, followed only by separate implementation authorization.**
