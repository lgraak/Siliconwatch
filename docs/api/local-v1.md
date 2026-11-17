# Local read-only API v1

Prototype 009, November 17, 2026. **Accepted by Chris** as technically complete
with its documented limitations on November 17, 2026. Acceptance does not establish
production readiness or authorize publication or deployment.
The application and daemon are Rust-only. This contract is intended for future
GUI clients; it does not introduce a GUI or deployment service.

## Enablement and trust boundary

```sh
cargo run -- daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

The endpoint is `$XDG_RUNTIME_DIR/siliconwatch/api.sock`. API service is opt-in;
without the flag the existing foreground daemon has no listener. The runtime
root must exist, be an absolute path, have mode **0700**, and belong to the
matching, nonzero real/effective UID. Do not run through sudo. Every ancestor
must be a real directory owned by root or this UID and not writable by other
users. Symlinks and parent traversal are rejected. The sole shared-directory
exception is root-owned mode-1777 `/tmp`, immediately followed by a private,
current-user-owned mode-0700 directory. The complete socket pathname must be
shorter than 108 bytes. No environment path is accepted without these checks.

Startup creates a new `siliconwatch` child with mode 0700 and socket with mode
0600. Any existing child, including an empty directory or stale socket, causes
startup failure. There is no automatic stale unlink or takeover. Verify that no
prior instance is running and inspect ownership before manually removing its
known stale socket and empty directory; never recursively delete a runtime tree.

Each accepted stream must pass Linux `SO_PEERCRED` UID equality before any request
is parsed. A failed credential query or unauthorized UID is closed silently.
Credentials identify the connection-time principal, not an executable; an
authorized descriptor can subsequently be passed to another process. Clients
should likewise check server peer credentials. The private directory,
socket mode and peer check form a current-user boundary; they do not isolate
hostile programs running as the same UID, privileged root, or a compromised
kernel. Trusted local filesystem ownership and permission behavior is assumed.
No TCP, abstract socket, D-Bus, remote access, multiuser sharing, authentication
service, mutation, or hardware-control operation is exposed.

On normal shutdown, inode/device/type/UID identities must match the process's
created child and socket before cleanup. Only that socket and empty child are
removed. Replacements or uncertain ownership are left untouched and reported as
failure. A crash or SIGKILL may leave a stale endpoint. The root runtime directory
is never removed by the daemon. Same-UID/root replacement races are outside the
security boundary; identity checks reduce accidental destructive cleanup.

## Framing and methods

One connection carries one UTF-8 JSON object terminated by LF and receives one
JSON object terminated by LF, then closes. Process only the first LF frame;
additional bytes or pipelined frames are discarded. There is no HTTP, request ID,
subscription, scan operation or persistent session. Empty/partial-at-EOF frames,
invalid JSON/UTF-8, duplicate keys, unknown fields, missing fields, arrays and wrong
types are rejected. JSON whitespace before the final LF is allowed. API version
must be an unsigned integer, not a string or floating-point number.

```json
{"api_version":1,"method":"get_info"}
```

```json
{"api_version":1,"method":"get_snapshot"}
```

Append a literal LF to each request. `get_info` reports fixed protocol information;
`get_snapshot` assesses the latest complete cache publication without acquiring
hardware. No arguments besides `api_version` and `method` are accepted.

Every response includes numeric `api_version: 1`, a public 32-character lowercase
hex `instance_id`, and boolean `ok`. The instance token comes from 128 bits of
kernel entropy at each start; startup fails if bounded nonblocking entropy
acquisition fails. It is neither a secret nor a physical-device identity. A changed
token invalidates comparisons of attempt sequences and monotonic timestamps
across restarts. Do not use it as authentication.

Success has `result` and no `error`; failure has `error` and no `result`. API v1,
embedded snapshot schema **2**, and embedded identity schema **1** are independent.
The [machine-readable schema](local-v1.schema.json) describes complete message
shapes emitted by this implementation. Clients may ignore future additive response fields;
unknown major versions or enum values must never be treated as current or healthy.
Incompatible meaning or field changes require a new API version. Constraints
involving time, trust and the relationship between fields are
specified below; schema validation alone cannot establish freshness or identity.

## Exact result fields

All listed fields are present. “Nullable” means JSON null, not omission. API
sequence and time metadata are **decimal strings** to preserve precision in GUI
languages: sequence and wall milliseconds are unsigned 64-bit quantities;
monotonic nanoseconds and durations use unsigned 128-bit quantities. `"0"` is
valid. These strings have no sign, exponent, fraction or leading zeros.

`get_info.result` contains:

| Field | Type / value |
| --- | --- |
| `supported_versions` | `[1]` |
| `methods` | `["get_info", "get_snapshot"]` |
| `snapshot_schema_version`, `identity_schema_version` | integers 2 and 1 |
| `read_only` | `true` |
| `limits` | the numeric fields in the resource table below |

`get_snapshot.result` contains:

| Field | Type / meaning |
| --- | --- |
| `lifecycle` | `starting`, `running`, `stopping`, or `stopped` at assessment |
| `assessed_at_ns` | decimal monotonic time since this cache instance began |
| `stale_after_ns` | positive decimal duration configured by `--stale-after-ms` |
| `producer` | `active` or `stopped` |
| `service_current` | boolean conjunction described below |
| `physical_age_known` | `false`; provider measurement age is unknown |
| `latest` | `{state, attempt, error}` |
| `observation` | nullable `{attempt, role, freshness, snapshot}` |

`latest.state` is `empty`, `succeeded`, or `failed`. Empty has null attempt/error;
succeeded has an attempt and null error; failed has an attempt and
`{kind, raw_os_error, message}`. Acquisition error `kind` is a Rust I/O kind name
(string, not a stable closed enum), `raw_os_error` is a nullable signed 32-bit
integer, and `message` is a diagnostic string. Clients must not parse diagnostic
prose as a protocol. A scan-level failure is data in an `ok:true` result, not a
protocol error.

Each `attempt` contains `sequence`, `started_ns`, `finished_ns`, `published_ns`,
`wall_start_unix_ms`, and `wall_time_error`. The first four are non-null decimal
strings. Wall time is either a decimal string and null error, or null with an
error string. Start/finish bracket the acquisition; publication can be later.
The observation has its **own** attempt: after failure, never relabel older sensor
values with the latest failed attempt's timestamps. Wall time is acquisition
start context, not a per-sensor measurement or response timestamp.

`observation.role` is `latest_attempt`, `historical_after_failure`, or
`historical_after_stop` (stopped takes precedence). An initial failed attempt with
no prior successful scan has null observation. `freshness` contains `state`,
`lower_ns`, and `upper_ns`. State is `no_sensor_observations` with both bounds
null for an empty sensor list; otherwise `within_budget` or `stale` with decimal
bounds. At assessment, lower = assessed − acquisition finish, upper = assessed −
acquisition start. Stale means **upper > configured threshold**; equality is
within budget. These are software observation bounds, not physical measurement
ages, and do not establish quality for individual rows.

`service_current` is true only when lifecycle is running, producer active,
latest attempt succeeded, observation role is latest_attempt and freshness is
within_budget. It may still be true with invalid/faulted rows: inspect every row's
status and all warnings. Partial scans retain all rows/errors. “Succeeded” means
that the scanner produced a snapshot, including empty or partial snapshots;
it does not certify hardware health, complete coverage or useful readings.

`assessed_at_ns` is the response assessment instant, not transmission or receipt
time. Preflight/encoding and transport follow it. The client cannot compare this
monotonic clock with its own clock or assume the response is still current on
receipt. Adding elapsed time since local receipt does **not** produce an upper
bound on current age: pre-receipt delay remains unknown. These responses are not
suitable for safety or protective decisions. Known system suspend requires cache
recreation by restarting the daemon; automatic suspend detection is not implemented.

### Embedded snapshot and identity schemas

The unchanged snapshot v2 object has `schema_version: 2`, `root` (UTF-8 path
string), `sensors` (array), and `warnings` (array of `{path, message}` strings).
Each sensor has all these fields:

| Field | Type / meaning |
| --- | --- |
| `identity` | identity v1 object below |
| `device`, `device_path`, `attribute`, `source_path` | strings, preserving source evidence |
| `label` | nullable string |
| `raw_text` | nullable string retained by acquisition, including invalid text; the existing scanner trims surrounding whitespace |
| `raw_value` | nullable exact signed 64-bit JSON integer |
| `value` | nullable finite JSON number, canonical conversion separate from raw data |
| `unit` | nullable string; current supported units `°C`, `RPM`, `V` |
| `status` | `ok`, `unavailable`, `permission_denied`, `unsupported`, `invalid`, `error`, or `fault` |
| `error` | nullable diagnostic string |

Temperature conversion is millidegrees / 1000 into Celsius; voltage is millivolts
/ 1000 into volts; fan speed remains RPM. Celsius remains default. Fahrenheit is
a future presentation preference, not a raw acquisition transformation. A large
`raw_value` can exceed JavaScript Number precision: use a lossless JSON parser or
retain `raw_text`; do not round-trip it through a binary64 integer. No calibration,
correction or fabricated value is applied. An erroneous row can still retain raw
and converted values; its status governs usability.

Identity v1 fields are `schema_version: 1`, nullable string `candidate`, `status`,
`evidence`, and string-array `issues`. Status is `best_effort`,
`insufficient_evidence`, `unreadable_evidence`, `conflicting_evidence`,
`ambiguous_channel`, or `collision`. Evidence contains nullable strings `provider`,
`parent`, `device_link`, `canonical_parent`, `channel`, and `measurement`. A
candidate is a best-effort source tuple encoded as a string, never proof of
physical identity; keep status/evidence/issues with it. Duplicate sources are
retained. Unknown measurements remain explicit. No identity is synthesized by
the API. See [Prototype 002](../milestones/002-stable-sensor-identity.md).

Non-UTF-8 paths and nonfinite converted numbers cannot be represented faithfully:
the whole response becomes `serialization_failed`, with no lossy substitution.

## Limits and errors

| `limits` field | Value | Scope |
| --- | ---: | --- |
| `request_bytes` | 4096 | entire first frame including LF |
| `response_bytes` | 1048576 | entire response including LF |
| `active_clients` | 1 | one worker/active stream; no application queue or thread per client |
| `request_deadline_ms` | 1000 | absolute budget from accept, not reset after partial input |
| `response_deadline_ms` | 2000 | absolute budget from frame completion, rejection or timeout; includes parse, assessment, preflight, encode and write |
| `max_sensors` | 4096 | per snapshot |
| `max_warnings` | 1024 | per snapshot |
| `max_identity_issues_per_sensor` | 64 | per row |
| `max_text_bytes` | 16384 | per dynamic text/path field, UTF-8 bytes |
| `max_total_text_bytes` | 262144 | snapshot text/path fields, acquisition error messages and wall-time errors; repeated occurrences count |

Bounds are inclusive. Generated decimal metadata, the instance token and I/O-kind
names do not count toward the text preflight sum; the final encoded-byte cap still
covers the entire response. The kernel has its own pending-connection backlog; clients
may wait there, fail or time out before server acceptance. There is no fairness
promise, and a same-UID client can monopolize the service. Clients need their own
connection/end-to-end deadlines. Use one request at a time, poll no faster than
the configured acquisition interval, and back off on busy/error responses. Idle accept polling uses a 50 ms cancellable
wait. Deadlines bound cooperative API work, not scheduler latency or arbitrary
kernel/filesystem stalls; this is not a real-time service.

Before serialization, preflight checks counts, text bytes, paths and values.
Encoding uses a capped buffer and periodic deadline/cancellation checks; no
snapshot clone or row truncation is used. Full encoding finishes before any
response socket write. A cache/lifecycle try-lock that is busy returns
`cache_busy`; poisoned or invalid service state is fatal. The lifecycle guard is
released immediately after assessment; the cache guard lasts through bounded
preflight/encoding and is released before socket I/O. Slow clients cannot retain
cache generations or prevent publication by holding that guard. Acquisition and
its existing cache allocations are not bounded by these API response limits.

Error shape is `{code, message, retryable}`. Only `unsupported_version` also has
`supported_versions: [1]`. Messages are fixed diagnostics, not an interface to
parse. Codes and retry flags:

| Code | Retryable |
| --- | --- |
| `invalid_request` | false |
| `unsupported_version` | false |
| `unknown_method` | false |
| `request_too_large` | false |
| `request_timeout` | true |
| `cache_busy` | true |
| `response_too_large` | false |
| `serialization_failed` | false |
| `service_unavailable` | false |

Ordinary protocol/client errors end only that connection. The daemon continues
serving after recoverable acquisition failures. Response-cap failure returns a
whole error, never a truncated successful snapshot. Cancellation, expired
response budget, disconnect or unauthorized peer can close without a complete
JSON error. A slow writer can receive a partial frame before closure; discard it.
Service-unavailable state closes acquisition admission and wakes the coordinator;
its error is best effort and may be lost during cancellation.

SIGINT/SIGTERM first closes acquisition admission, cancels the active API socket,
joins the API worker and cleans its endpoint, then joins the acquisition owner.
Only after all created workers are joined is final Stopped reported by the daemon.
A final `stopped` API reply is not guaranteed: the listener closes during shutdown.
EOF or refusal does not prove graceful completion. An already admitted acquisition
may finish and publish after Stopping, with lifecycle still overriding currentness. API startup
failure does not start acquisition; later worker failure/unwind wakes the signal
coordinator and follows cleanup. Shutdown cannot cancel an already admitted
blocked hardware operation. Stderr and filesystem/kernel operations can also
delay completion. There is no production systemd deployment.

## Response examples

Examples below use synthetic times and paths, not hardware evidence. Each wire
message occupies one line plus LF; pretty printing here is for readability.

```json
{"api_version":1,"instance_id":"0123456789abcdef0123456789abcdef","ok":false,"error":{"code":"unsupported_version","message":"Unsupported API version","retryable":false,"supported_versions":[1]}}
```

The informational method returns fixed limits and versions:

```json
{
  "api_version": 1,
  "instance_id": "0123456789abcdef0123456789abcdef",
  "ok": true,
  "result": {
    "supported_versions": [
      1
    ],
    "methods": [
      "get_info",
      "get_snapshot"
    ],
    "snapshot_schema_version": 2,
    "identity_schema_version": 1,
    "read_only": true,
    "limits": {
      "request_bytes": 4096,
      "response_bytes": 1048576,
      "active_clients": 1,
      "request_deadline_ms": 1000,
      "response_deadline_ms": 2000,
      "max_sensors": 4096,
      "max_warnings": 1024,
      "max_identity_issues_per_sensor": 64,
      "max_text_bytes": 16384,
      "max_total_text_bytes": 262144
    }
  }
}
```

An initially empty cache is a successful response, with no observation:

```json
{"api_version":1,"instance_id":"0123456789abcdef0123456789abcdef","ok":true,"result":{"lifecycle":"starting","assessed_at_ns":"100","stale_after_ns":"3000000000","producer":"active","service_current":false,"physical_age_known":false,"latest":{"state":"empty","attempt":null,"error":null},"observation":null}}
```

A recent successful scan can contain partial row failures. This complete response
preserves both a canonical Celsius conversion and an invalid row with raw text:

```json
{
  "api_version": 1,
  "instance_id": "0123456789abcdef0123456789abcdef",
  "ok": true,
  "result": {
    "lifecycle": "running",
    "assessed_at_ns": "300",
    "stale_after_ns": "3000000000",
    "producer": "active",
    "service_current": true,
    "physical_age_known": false,
    "latest": {
      "state": "succeeded",
      "attempt": {
        "sequence": "1",
        "started_ns": "100",
        "finished_ns": "200",
        "published_ns": "250",
        "wall_start_unix_ms": "1000",
        "wall_time_error": null
      },
      "error": null
    },
    "observation": {
      "attempt": {
        "sequence": "1",
        "started_ns": "100",
        "finished_ns": "200",
        "published_ns": "250",
        "wall_start_unix_ms": "1000",
        "wall_time_error": null
      },
      "role": "latest_attempt",
      "freshness": {
        "state": "within_budget",
        "lower_ns": "100",
        "upper_ns": "200"
      },
      "snapshot": {
        "schema_version": 2,
        "root": "/fixture",
        "sensors": [
          {
            "identity": {
              "schema_version": 1,
              "candidate": null,
              "status": "insufficient_evidence",
              "evidence": {
                "provider": "fixture",
                "parent": null,
                "device_link": null,
                "canonical_parent": null,
                "channel": "temp1_input",
                "measurement": "temperature_input_millidegree_celsius"
              },
              "issues": [
                "no supported parent-device evidence"
              ]
            },
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
          },
          {
            "identity": {
              "schema_version": 1,
              "candidate": null,
              "status": "insufficient_evidence",
              "evidence": {
                "provider": "fixture",
                "parent": null,
                "device_link": null,
                "canonical_parent": null,
                "channel": "temp2_input",
                "measurement": "temperature_input_millidegree_celsius"
              },
              "issues": [
                "no supported parent-device evidence"
              ]
            },
            "device": "fixture",
            "device_path": "/fixture/hwmon0",
            "attribute": "temp2_input",
            "label": null,
            "source_path": "/fixture/hwmon0/temp2_input",
            "raw_text": "invalid",
            "raw_value": null,
            "value": null,
            "unit": "°C",
            "status": "invalid",
            "error": "invalid integer: invalid digit found in string"
          }
        ],
        "warnings": []
      }
    }
  }
}
```

A later acquisition failure retains the original observation and its own times.
This complete response remains within its age budget but is historical and not
service-current. Stopped producer state would use `historical_after_stop` instead:

```json
{
  "api_version": 1,
  "instance_id": "0123456789abcdef0123456789abcdef",
  "ok": true,
  "result": {
    "lifecycle": "running",
    "assessed_at_ns": "500",
    "stale_after_ns": "3000000000",
    "producer": "active",
    "service_current": false,
    "physical_age_known": false,
    "latest": {
      "state": "failed",
      "attempt": {
        "sequence": "2",
        "started_ns": "400",
        "finished_ns": "450",
        "published_ns": "460",
        "wall_start_unix_ms": "1100",
        "wall_time_error": null
      },
      "error": {
        "kind": "NotFound",
        "raw_os_error": 2,
        "message": "No such file or directory (os error 2)"
      }
    },
    "observation": {
      "attempt": {
        "sequence": "1",
        "started_ns": "100",
        "finished_ns": "200",
        "published_ns": "250",
        "wall_start_unix_ms": "1000",
        "wall_time_error": null
      },
      "role": "historical_after_failure",
      "freshness": {
        "state": "within_budget",
        "lower_ns": "300",
        "upper_ns": "400"
      },
      "snapshot": {
        "schema_version": 2,
        "root": "/fixture",
        "sensors": [
          {
            "identity": {
              "schema_version": 1,
              "candidate": null,
              "status": "insufficient_evidence",
              "evidence": {
                "provider": "fixture",
                "parent": null,
                "device_link": null,
                "canonical_parent": null,
                "channel": "temp1_input",
                "measurement": "temperature_input_millidegree_celsius"
              },
              "issues": [
                "no supported parent-device evidence"
              ]
            },
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
          },
          {
            "identity": {
              "schema_version": 1,
              "candidate": null,
              "status": "insufficient_evidence",
              "evidence": {
                "provider": "fixture",
                "parent": null,
                "device_link": null,
                "canonical_parent": null,
                "channel": "temp2_input",
                "measurement": "temperature_input_millidegree_celsius"
              },
              "issues": [
                "no supported parent-device evidence"
              ]
            },
            "device": "fixture",
            "device_path": "/fixture/hwmon0",
            "attribute": "temp2_input",
            "label": null,
            "source_path": "/fixture/hwmon0/temp2_input",
            "raw_text": "invalid",
            "raw_value": null,
            "value": null,
            "unit": "°C",
            "status": "invalid",
            "error": "invalid integer: invalid digit found in string"
          }
        ],
        "warnings": []
      }
    }
  }
}
```

## Example diagnostic client

This Python standard-library client is optional development tooling; Python is
not part of the installed application. Run it as the daemon's current user.
It checks server UID and response framing and preserves Python integer precision.
Use `get_info` in place of `get_snapshot` to inspect protocol limits.

```sh
python3 - <<'PYCLIENT'
import json, os, socket, struct, time
path = os.path.join(os.environ["XDG_RUNTIME_DIR"], "siliconwatch", "api.sock")
limit = 1_048_576
end = time.monotonic() + 5
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
    peer.settimeout(max(0.001, end - time.monotonic()))
    peer.connect(path)
    _, uid, _ = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    if uid != os.getuid() or uid == 0:
        raise RuntimeError("unexpected server UID")
    peer.sendall(b'{"api_version":1,"method":"get_snapshot"}\n')
    frame = bytearray()
    while not frame.endswith(b"\n"):
        remaining = end - time.monotonic()
        if remaining <= 0 or len(frame) >= limit:
            raise RuntimeError("incomplete or oversized response")
        peer.settimeout(remaining)
        chunk = peer.recv(min(65536, limit - len(frame)))
        if not chunk:
            raise RuntimeError("response ended without LF")
        frame.extend(chunk)
    if b"\n" in frame[:-1]:
        raise RuntimeError("multiple response frames")
    response = json.loads(frame)
    if response.get("api_version") != 1:
        raise RuntimeError("unsupported response version")
    print(json.dumps(response, ensure_ascii=False, indent=2))
PYCLIENT
```

A GUI should additionally validate the response schema, track instance changes,
retain per-row diagnostics, label historical/unknown freshness clearly and bound
its own retries. Do not infer authorization for hardware actions from telemetry.
