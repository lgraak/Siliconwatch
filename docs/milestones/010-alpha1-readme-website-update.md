# Prototype 010 — Alpha 1 README and website update

Date: November 17, 2026, America/Los_Angeles. **Status: Complete local documentation and website preparation.** Publication/deployment was not performed. This is not Prototype 010 technical acceptance.

## 1. Milestone summary

Updated the candidate README and the existing Cloudflare static source to describe the implemented Alpha 1 GUI and CLI/headless application accurately. Prepared the requested verified ZIP. Preserved accepted monitoring behavior, existing implementation and historical evidence.

## 2. Implementation details and exact manifest

Repository: `SiliconWatch-prototype010` (local worktree).

| Task change | Path |
| --- | --- |
| Modified | `README.md` |
| Added | `docs/milestones/010-alpha1-readme-website-update.md` |

The existing website source was located and updated in the external Downloads export `siliconwatch-cloudflare(2)/`. After receiving the ZIP, Chris extracted it and confirmed intentionally removing the older Downloads exports and archives. Current verified source/preview: external `siliconwatch-alpha1-website-2026-11-17/` export. Its four files match the delivered ZIP byte-for-byte.

| Task change | Website path |
| --- | --- |
| Modified | `index.html` |
| Modified | `robots.txt` |
| Added | `sitemap.xml` |
| Added | `assets/native-gui-oled.png` |

The original ZIP archives and older export directory were preserved by the agent through delivery; Chris subsequently confirmed deleting them intentionally. This user cleanup is recorded separately from agent changes, and no files were reconstructed or restored. No Rust source, Cargo manifest/lockfile, dependency version, API, daemon, proposal or previous milestone report was edited. No dependency was added. The README retains licensing, full acknowledgments, non-affiliation and existing technical documentation, with a legacy quickstart anchor retained. Appearance import/export is documented as GUI interaction rather than invented CLI flags.

Website original two CSS blocks and wordmark are byte-preserved. Additional CSS supports mobile navigation, visible keyboard focus and improved text contrast. The homepage uses the requested positioning, actual application image, implemented features, local GUI/settings instructions, compatibility guidance, known limits and future roadmap. Canonical and Open Graph URLs target the existing production domain. Robots still permits crawling and now names the new sitemap. No hosting configuration was created or changed.

## 3. Verification results

- Existing production homepage loaded in the browser; its visible content, metadata and stylesheet lengths match the newer export. No existing canonical was present; robots allowed crawling. The sitemap URL failed. Direct HTTP tools returned 403 while normal browser access worked; production HTML byte identity is not claimed.
- Website checked at 320×740, 390×844, 768×1024, 1280×900 and 1440×1000. No horizontal page overflow; mobile navigation remains visible; native screenshot loaded. Desktop/mobile screenshots were inspected in the browser session, not saved as a separate deliverable.
- Dark/light toggling and keyboard skip-to-main/next-Tab focus passed. Downloads navigation reached the correct section; it contains only the repository link. Browser warning/error log was empty. The server recorded an automatic request for an undeclared `/favicon.ico` returning 404; no authored link or required page asset points there.
- Computed text contrast: 182 dark-theme and 181 light-theme text elements passed applicable 4.5:1/3:1 thresholds; minimum observed final ratios 5.32 and4.55. Three light-theme roadmap paragraphs initially measured 4.49:1, were darkened through an added override, and passed on recheck. This is not a complete accessibility conformance audit. Clipboard copying itself was not exercised.
- Static website: 1 H1, 21 unique IDs, 17 valid local links, 1 valid screenshot; both JavaScript blocks passed `node --check`; JSON-LD and sitemap XML parsed. No nonexistent release/download URL or private capture/path was found.
- README: 14 candidate-relative destinations exist; important anchors validated; MarkdownIt/CommonMark rendering inspected at 1280 and 320 pixel widths; both images loaded; 7 code blocks preserve formatting and scroll at mobile width. This is local rendering, not GitHub's hosted renderer.
- Public GitHub repository, main README, API guide, LICENSE and Issues were verified through the GitHub connector. Releases collection was empty. HWiNFO acknowledgment destination remains reachable. Candidate GUI files are not on main, so website GUI documentation uses a local section rather than broken future GitHub paths.
- `cargo +1.85.0 metadata --locked --offline --no-deps --format-version 1`: exit 0; expected package names/default members/toolchain requirements confirmed. Existing packaged `siliconwatch --help`: exit 0; documented CLI flags confirmed. No sensor acquisition was performed for this task.
- `git diff --check`:passed. Formatting, Clippy, Rust/Python suites and application builds were not rerun for these documentation/static-website edits. Earlier milestone results remain historical evidence.
- ZIP CRC test passed; exact four-entry manifest and every archived byte match the verified website sources. No outer parent folder, binaries, private evidence, preview files or agent setup is included.

Private reproducibility records are under `/tmp/siliconwatch-alpha1-docs/`, including feature-command-audit.md, command-and-link-preflight.json, browser-verification.json, website-static-checks.json, readme-link-check.json and website-final-manifest.json.

## 4. Screenshot and hardware validation

Screenshot reused unchanged: `docs/milestones/010-native-gui/main-oled.png`, 1120×680, SHA-256 `493b6a022de1921f11fdb40df13511082d63fec7a5aaabce2770fcabfbcfbca8`. It is an actual native GUI framebuffer from existing verification, displaying deterministic fixture readings. It is clearly captioned as demonstration data, not a wireframe or physical-host measurement. No private user screenshot was copied. No new hardware observations, stress tests or benchmark runs were performed.

The sanitized compatibility example draws only on the completed read-only IT8613E report. Available channels depend on Linux drivers; no CPU/System mapping, universal module recipe, calibration or reboot-persistence guarantee is asserted.

## 5. Architecture and design review

The documentation retains the current-user read-only local API, manual foreground daemon and cache-only GUI architecture. It does not imply system-wide sharing, remote access, hardware controls or automatic service/module installation. Planned graphs/history/alarms/protective actions remain explicitly future work. Prototype 010 qualification and technical acceptance remain pending.

## 6. Artifact, deployment and limitations

Requested ZIP (external delivery): `siliconwatch-alpha1-website-2026-11-17.zip` (65,095 bytes).

SHA-256: `a0d06d8c4e66e2e9e8fa3da22dc2aff58652906193777f5b41df8ac7bd9b3502`.

Local preview source is the existing export's `index.html`; open it directly or extract the ZIP with its asset folder intact. A temporary loopback preview server was used for QA and stopped at handoff. No deployment was performed by the agent; production state after Chris received the ZIP has not been rechecked. Its existing manual Cloudflare upload workflow requires authenticated dashboard access; no credential flow, upload, deployment or project/configuration change was attempted. The original export history identifies the existing Worker as `siliconwatch` with custom domain `siliconwatch.cgillett.com`; account configuration was not re-certified in this task.

The ZIP is a website artifact, not an application release. Public GUI downloads, a release tag and release binaries remain absent. README/GUI publication must be coordinated so candidate-relative documentation/images are included with their contribution; this task does not authorize staging or publishing them. Accessibility and broader desktop qualification remain limited as documented in the GUI report.

## 7. Git status and preservation

Branch `codex/alpha1-prototype010`; HEAD `3b6b85732b374d9d3dbc43ca052194475071bda9`; upstream unset; index empty. Existing tracked modifications to Cargo.lock/Cargo.toml are unchanged. README remains modified with this authorized documentation update. Existing untracked GUI/vendor/docs/screenshot files remain; this report is the sole new repository path for this task.

Final preservation comparison passed with the two authorized repository paths, four verified extracted website paths and requested ZIP allowed. Chris's confirmed cleanup of older Downloads exports/archives is recorded as an intentional external change, not an unexplained loss. Full results and exact Git status are recorded in `/tmp/siliconwatch-alpha1-docs/preservation-final.json` and its notes.

Fresh task preflight compared the completed fan-verification state before edits. All original/007/008/009 worktrees, original agent-skills setup, intentional Prototype 006 changes, historical benchmarks/private evidence and existing package are preserved. Existing temporary-evidence absences and large generated-file metadata-only coverage remain explicit; no unavailable evidence was reconstructed. No stage, commit, push, merge, PR, release or tag was created.

## 8. Recommended next authorization

Review the exact README/site manifest and ZIP. For website publication, use the authenticated existing Cloudflare Worker upload workflow while retaining the domain/project; then verify the deployed URL and assets. Git contribution and application release steps require separate authorization. Do not begin Prototype 011. No monitoring implementation change is proposed.

## 9. HANDOFF TO CHATGPT

Chris authorized an Alpha 1 README and website update, then requested a ZIP of the updated website components. The local work is complete; no Git publication, application release, Cloudflare deployment or Prototype 011 work occurred.

The Prototype 010 README now presents SiliconWatch as experimental Rust-based Linux monitoring with both a native Slint GUI and CLI/headless tools. It covers the OLED Black default, Dark and Light themes, hierarchical readings, session statistics, Device Details, tray behavior, portable TOML settings, temperature preferences and explicit data-quality states. Source-build, CLI, foreground-daemon and API/GUI instructions match the candidate. The text distinguishes unpublished GUI work from the currently available public headless/API source and retains licensing, acknowledgments, contribution guidance and technical links.

The existing Cloudflare source was located in the newer downloaded export. Its visible content, metadata and original stylesheet lengths matched the live site at siliconwatch.cgillett.com through the browser. The update preserves the wordmark, established design and original style blocks. It replaces the conceptual illustration with an actual native OLED Black screenshot using clearly labeled demonstration fixtures. Private hardware captures and identifying information remain excluded. The compatibility section explains Linux-driver-dependent coverage and the bounded GMKtec EVO-X1/IT8613E example without universal driver advice.

The website now separates implemented features, known limits and future work. Downloads says Public alpha download coming soon for v0.1.0-alpha.1 and links only to the repository. GitHub currently lists no releases. Canonical/Open Graph metadata, a production sitemap and robots discovery are prepared for the existing domain. No Cloudflare project, route, credential or configuration was changed.

Verification covered desktop/mobile widths from 320 to 1440 pixels, both themes, keyboard skip/focus behavior, image loading, local links, HTML structure, JavaScript syntax, JSON-LD/XML and archive contents. Computed text contrast passed after correcting three light-theme paragraphs. README rendering and all candidate-relative targets passed. These checks do not claim full accessibility certification, hosted GitHub rendering or new Rust/hardware qualification.

The ZIP in Downloads contains exactly index.html, robots.txt, sitemap.xml and assets/native-gui-oled.png. Chris extracted it and confirmed intentionally removing the older Downloads exports; the extracted folder matches the ZIP. Its CRC and every extracted byte match the verified source. Repository changes for this task are README.md and this new report; implementation, proposal, earlier reports and other worktrees remain preserved. Branch/HEAD are unchanged and the index is empty. The external continuation records the exact manifest, verification and preservation limits. Next, Chris can review the ZIP and arrange an authenticated upload to the existing Cloudflare Worker; Git contribution and release publication still require separate authorization.

> Publication privacy review — November 17, 2026: personal home-directory paths and local-only file links were replaced with portable descriptions. The original report is preserved unchanged in the private external publication-review archive. Historical dates, verification results, failures and counts are unchanged.
