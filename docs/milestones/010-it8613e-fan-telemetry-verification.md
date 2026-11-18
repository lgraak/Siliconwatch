# Prototype 010 — IT8613E fan telemetry verification

Date: November 17, 2026 (America/Los_Angeles). **Backend verification: passed. Live GUI qualification: partial.** This is a host-specific, read-only compatibility observation, not Prototype 010 technical acceptance or publication authorization.

## 1. Milestone summary

Chris authorized a narrow check that SiliconWatch discovers and reports the two fan channels newly exposed by the existing Linux `it87` driver. The supplied Sensors screenshot already showed them. This investigation independently confirmed Linux telemetry, two CLI snapshots, ten controlled-daemon API observations, cooperative daemon shutdown and cleanup. No application defect was confirmed and no implementation change was made.

The existing user-test AppImage and prebuilt daemon were reused without compilation or repackaging. The GUI remained open throughout. The earlier daemon was stopped normally before the two CLI acquisitions, then one controlled daemon ran. After its verified shutdown, a normal daemon was restored so the user's existing GUI remained usable. At no point were two SiliconWatch acquisition daemons intentionally run together.

## 2. Environment and driver compatibility

| Item | Observed result |
| --- | --- |
| Hardware | GMKtec NucBox EVO-X1. |
| Firmware | American Megatrends International, LLC.; NucBox EVO-X1 1.03; date 02/18/2025. |
| Active kernel | `7.3.4-1-cachyos`. |
| Driver | `it87` loaded, module init state `live`; installed module under the active kernel's `updates/dkms/`. |
| DKMS revision | `it87/358.bd7a566`, installed for the active kernel; also listed for `6.18.60-2-cachyos-lts`, which was not boot-tested here. |
| Module's own version string | `unknown`; the precise revision above comes from DKMS registration, not an invented `modinfo` version. |
| Detection | Kernel reports IT8613E at `0xa30`, revision 12, then successful hwmon registration named `it8613`. |
| Current registration | `/sys/class/hwmon/hwmon5`, resolving to platform device `it87.2608`. Discovered by name/metadata rather than assuming hwmon5 is permanent. |
| Fan attributes | Exactly `fan2_input` and `fan3_input`; no `fan*_label` files found. |
| Signing/taint | `modinfo` contains signer metadata “DKMS module signing key”; the kernel nevertheless logged failed signature and/or key verification. Module taint `OE`; kernel taint 12288. |

The kernel taint bits describe an externally built module and an unverified/unsigned-module load classification. Presence of signing metadata does not demonstrate that this kernel trusted the signature. No trust-store, signature, Secure Boot or driver configuration was changed. See the [Linux taint documentation](https://docs.kernel.org/admin-guide/tainted-kernels.html) and [DKMS signing description](https://github.com/dkms-project/dkms#module-signing).

The filtered current-boot journal contained the expected detection and signing/taint messages, with no matching IT87 initialization or resource-conflict error beyond signature verification. A second filtered check after the controlled run returned no entries (journalctl exit 1, empty diagnostic stderr); that means no matches for the specified interval/pattern, not a general kernel-health certification.

The [kernel IT87 documentation](https://docs.kernel.org/hwmon/it87.html) identifies the `it8613` prefix for IT8613E. This observation remains specific to this board, firmware, module build and kernel. It does not establish support for every IT8613E motherboard. Out-of-tree modules require a compatible build and successful load for the kernel in use; later kernel updates and DKMS build results must be checked. Installed DKMS records do not prove successful future boot-time loading. Reboot persistence was not tested, and autoload behavior was not changed.

## 3. Linux, CLI and API observations

All readings below are passive observations under ordinary host activity, without stress, fan-control experiments or imposed workloads. Windows were sequential, not simultaneous: differing values between rows are expected and are not treated as conversion errors.

| Evidence window | Observations | fan2_input | fan3_input |
| --- | ---: | ---: | ---: |
| Direct Linux fan attributes | 8, approximately 1 s apart | 1448–1464 RPM | 3183–3199 RPM |
| Existing CLI `sensors --json` | 2, both exit 0 | 1691–1721 RPM | 3199–3214 RPM |
| Controlled daemon `get_snapshot` | 10, approximately 1.1 s between requests | 1687–1726 RPM | 3199–3214 RPM |

Both direct fan values changed naturally during the short window. Positive, modestly varying values are plausible relative to the earlier user-reported BIOS observations, but this is not independent tachometer calibration. No direct register, PWM or fan-curve operation was performed.

Each CLI and API snapshot contained **24 sensors: 22 `ok`, two unsupported**. The IT8613 source group contained 13 readings, including exactly two fans. Both fan rows had unit `RPM`, status `ok`, null error and null label. For every checked fan row, raw integer = parsed raw text = canonical RPM value; no scaling or unit conversion was applied. Snapshot warnings were empty in the two CLI observations. Unsupported readings remained explicit and were not converted into extra fan channels.

Source identity retained the `it8613` provider, platform-parent evidence, original channel attribute and `fan_speed_input_rpm` measurement. Status was `best_effort` with no identity issues. Identity objects were equal across the two CLI snapshots and all ten controlled API snapshots, including the daemon restart boundary. This establishes consistency for the observed registration, not physical identity across driver changes, hardware replacement or reboot. Raw source paths and exact identity objects remain in private evidence.

The earlier BIOS values supplied by Chris were CPU 1366 RPM and SYS 3214 RPM. Similarity suggests fan2 may be CPU and fan3 may be System, but no reliable kernel labels or physical wiring evidence verified that association. SiliconWatch correctly retains **fan2_input** and **fan3_input**. No CPU/System aliases were added.

## 4. Controlled daemon, API and cleanup

The exact unchanged packaged daemon was hash-verified before use. Command:

```sh
./siliconwatch daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

The prior user-launched daemon was positively identified by executable, PID, start-time token and socket peer credentials before SIGTERM. It terminated and removed its runtime namespace. As a previously detached process it was not waitable by this verification process; its numerical exit status is not claimed.

The CLI ran while no daemon owned acquisition. The controlled daemon then started in the existing normal current-user runtime directory, without permission changes or stale-endpoint removal. The client checked same-user peer credentials and queried only the accepted LF-framed API methods. `get_info` reported API v1, and ten `get_snapshot` observations had running lifecycle, active producer, successful latest attempt, latest-attempt observation, within-budget freshness and `service_current:true`. Observed sequences were 2, 3, 4, 5, 6, 7, 8, 9, 10 and 12; skipped acquisition 11 is consistent with a sampling client and was not reconstructed.

The original GUI's own tray status reported Connected and Recent at last assessment during this controlled run. This confirms application-level reconnection/currentness reporting, not a claim that a tray property exposes individual table values or aggregate history. Physical measurement age remains unknown.

SIGTERM stopped the controlled child with **exit 0**. Its log reported graceful outcome, and both its socket and private runtime child directory disappeared before restoration. The controlled PID is absent. A normal user daemon was then restored with the same arguments; its snapshot was current and the original GUI reconnected. Final inspection found exactly one daemon, the unchanged GUI process and its AppImage runtime helper. No controlled test daemon or acquisition process remains. The restored daemon is intentionally running for the user's continuing GUI test, not installed or deployed as a service.

## 5. Native GUI verification and limits

The two supplied screenshot files are byte-identical and are counted as **one visual observation**. They show black table surfaces, the `it8613 · hwmon5` group with 13 readings, both fan rows in RPM, and no invented CPU/System labels:

| Visible row | Current | Minimum | Maximum | Average |
| --- | ---: | ---: | ---: | ---: |
| fan2_input | 1603 RPM | 1347 RPM | 2327 RPM | 1562 RPM |
| fan3_input | 3214 RPM | 3199 RPM | 3214 RPM | 3204 RPM |

These GUI values predate the later direct/CLI/API windows. The displayed extrema and averages are internally plausible: current and mean lie within the corresponding min/max intervals. Source review confirms bounded min/max/arithmetic mean over distinct eligible publications actually received by the GUI, with RPM formatted to whole numbers. Duplicate reads do not add observations; failures/noncurrent observations do not contribute; new daemon instances reset the epoch. Details shares the same model, row formatter and accumulators. No GUI code or theme setting was changed.

**Qualification limits:** the screenshot does not include the GUI's complete accepted observation history/count, so it cannot independently verify the exact arithmetic average. Means calculated from our ten API requests describe a different observation set and must not be substituted for the GUI's average. The image is not Device Details. Chris reported that Details had not yet been checked, and no subsequent Details observation was supplied. Existing native desktop-control/accessibility qualification is unavailable in this environment; no settings change, automation bypass or substitute test screenshot was used. The live Device Details fan rows, a second visible update, and complete-session aggregate arithmetic remain unverified. This is a limited observation gap, not a confirmed application defect.

## 6. Actual checks, commands and evidence

Raw host observations, CLI JSON, complete API replies, logs, exact command/exit records, source-review notes and preservation baselines are outside the repository in `/tmp/siliconwatch-it8613-verification/`. Hostname, signing-key fingerprint and full raw snapshots are deliberately absent from this sanitized report. Supplied screenshots remain in their original external attachment paths; only hashes and visible fan observations are recorded in the evidence index.

| Check | Result |
| --- | --- |
| `uname -r`, `lsmod`, `modinfo it87`, `dkms status` | All exit 0; installed and active driver state recorded. |
| Current-boot filtered `journalctl` | Exit 0; initialization/signature messages inspected. |
| Eight direct fan reads | Both channels readable each time; natural variation observed. |
| Two existing CLI snapshots | 2/2 exit 0, both fans exact raw/canonical RPM, stable observed identities. |
| Ten controlled cache snapshots | 10/10 successful and within freshness budget; advancing sequences. |
| Controlled daemon stop | Exit 0; owned namespace removed and PID absent. |
| Restored normal session | One daemon; original GUI reports Connected/recent; prior and controlled daemon PIDs absent. |
| Post-run filtered kernel journal | Exit 1 means no matching entries; module remains live and taint unchanged. |
| GUI visual inspection | Supplied Sensors screenshot supports both rows/RPM/OLED-style black surfaces; Details and full-history arithmetic remain unverified. |
| Formatting, Clippy, Rust/Python tests and builds | Not rerun: no application/source/dependency changes. Previous milestone results are historical evidence, not newly executed tests. |

Reproducible command and raw-output indexes are `driver-commands.json`, `runtime-commands.json`, `verify_runtime.py`, `linux-fan-samples.json`, `driver-metadata.json`, `hwmon-discovery.json`, `cli-summary.json`, `api-samples.json`, `runtime-results.json`, `kernel-after.json` and `final-runtime-check.json`. The verifier performs only the documented CLI/API/owned-process lifecycle actions; it does not load/unload modules, invoke `force_id`, bypass ACPI protections or access hardware registers. `telemetry-summary.json` derives counts/ranges and explicitly limits its means to its own sample sets.

A sandbox-scoped initial process listing did not expose desktop processes. A subsequent authorized host-level inspection identified the actual daemon and GUI and was used for every lifecycle action. No absence/cleanup claim relies on the restricted listing.

## 7. Git status and preservation

Worktree: `SiliconWatch-prototype010` (local worktree). Branch: `codex/alpha1-prototype010`. HEAD: `3b6b85732b374d9d3dbc43ca052194475071bda9`. Index remains empty. The pre-existing modified Cargo files/README and untracked GUI, vendor, proposal, screenshots and reports are retained. The only new repository file for this investigation is this report; no implementation file changed.

Final comparison against the fresh preflight baseline passed with only this new report allowed. Existing implementation files, delivered package checksums, Git refs/HEADs/indexes, five worktree registrations, original agent-skills setup, intentional Prototype 006 report changes, benchmark archives and historical private evidence were preserved. The durable implementation manifest matched all 81 recorded paths; all 330 historical worktree files and 225 private evidence files matched their retained baseline. The final audit record is `/tmp/siliconwatch-it8613-verification/preservation-final.json`.

The temporary package baseline and some older temporary planning artifacts were already absent at task start. Durable output manifests and older worktree/evidence baselines were found and checked, and a fresh baseline was saved before runtime verification. The missing temporary records were not recreated or represented as reviewed. Large generated files retain documented metadata-only preservation coverage; exact historical byte preservation is not claimed where no original hash is available.

No staging, commit, push, PR, merge, deployment or Prototype 011 occurred. No driver/module/DKMS/autoload configuration, firmware, fan curve, PWM, daemon behavior, API protocol or GUI implementation was modified. Runtime start/stop and real read-only observations were the authorized execution, not application changes.

## 8. Known limitations and recommended next authorization

Both fan channels are correctly discovered and represented in the observed backend path. No confirmed SiliconWatch application defect was found; no fix is proposed. Hardware calibration, physical CPU/System mapping, complete-session GUI arithmetic, live Device Details and future reboot/kernel compatibility remain unverified. Current driver taint is an observation rather than an application fault.

The next useful action is ordinary human Device Details review for both fan channels and their raw/source/statistical information. A future bounded check after a normally scheduled reboot could verify driver persistence if separately authorized; do not enable autoload, reboot or change kernel configuration automatically. No next prototype is proposed or begun.

## 9. HANDOFF TO CHATGPT

The authorized read-only IT8613E investigation confirmed that Linux and SiliconWatch expose both fan readings on the tested GMKtec NucBox EVO-X1. The `it87` module is live on kernel `7.3.4-1-cachyos`; DKMS reports `it87/358.bd7a566` installed. The kernel detected IT8613E at `0xa30`, revision 12. Its current hwmon registration is named `it8613`; numbering must not be treated as persistent.

Eight direct Linux observations showed fan2_input at 1448–1464 RPM and fan3_input at 3183–3199 RPM, with natural variation and no imposed workload. Two existing prebuilt CLI snapshots exited successfully. They reported 24 sensors: 22 `ok`, two unsupported, including both fan channels. Raw integers, canonical values and RPM units agreed exactly, source metadata was retained, and best-effort identities matched across CLI snapshots and the controlled API observations. No kernel fan labels were present. CPU/System associations remain provisional; no labels were fabricated.

Ten controlled-daemon API snapshots passed: both fans were present, acquisition succeeded, sequences advanced, and freshness remained within budget. Their observed ranges were 1687–1726 RPM and 3199–3214 RPM. These occurred later than the Linux samples, so differing values are not evidence of a conversion error. The controlled daemon stopped cooperatively with exit 0 and removed its socket and private child directory. No controlled test process remains. The original GUI is still running and connected to the restored normal user daemon. Only one acquisition daemon remains; restarting it reset the GUI's statistics epoch.

The supplied screenshot confirms both Sensors rows, RPM units, populated statistics and black table surfaces. The repeated image is byte-identical, not a second time sample. Source review confirms statistics use distinct eligible GUI-received publications and rounded whole-RPM display. The screenshot cannot independently establish the complete-session arithmetic average. Device Details was not observed, so that live GUI check remains unverified. No confirmed application defect was found.

Kernel taint is `OE`/12288. Signing metadata exists, but the kernel logged failed signature/key verification. No matching resource-conflict error was found; future reboot loading and kernel-update compatibility remain unverified. Driver, firmware, PWM, permissions and autoload configuration were unchanged.

The sanitized report and external evidence/handoff record the commands and limitations. Branch and HEAD remain unchanged, the index is empty, and only the new report was added. Preservation checks protect the existing implementation, worktrees, archives and private evidence; missing historical temporary records are explicitly noted. No publication or Prototype 011 work occurred. The next useful action is Chris's Device Details review and, only if desired, separately authorized verification after a normal reboot.

> Publication privacy review — November 17, 2026: personal home-directory paths and local-only file links were replaced with portable descriptions. The transient process ID was removed while preserving the recorded daemon lifecycle outcome. The original report is preserved unchanged in the private external publication-review archive. Historical dates, verification results, failures and counts are unchanged.
