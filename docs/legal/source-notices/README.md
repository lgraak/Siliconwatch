# Corresponding source for the Linux packages

The three native packages target x86-64 Linux. Cargo 1.85.0 resolves 442 registry
source archives for a locked, offline workspace build on this target. This includes
62 resolver-selected crates beyond the 380 registry crates in the normal/build
tree. The two workspace crates use the project GPL license. The local fontdue
patch retains its upstream license files and patch history.

The source archive includes the original crates.io archives and their registry
checksums. Cargo's local-registry index has all 590 locked registry entries so the
original unmodified lockfile also resolves target-specific metadata offline.
The remaining 148 entries contain only dependency-resolution metadata. Their
implementation code is excluded, including every version of `block2` and
`clipboard-win`. The archive serves the qualified Linux build. Cross-target
development can use the project's original lockfile and crates.io.

`SOURCE-NOTICES.json` identifies each actual source archive, its checksum,
manifest license and the notice paths in that archive. Any additional files are
identified by path and SHA-256 in the extracted source bundle. The source
assembler pins this complete inventory separately from the Linux binary notice
inventory. Changing a resolver input, crate or reviewed notice requires review.
Cargo 1.85.0's default sparse crates.io cache must be populated by
`cargo +1.85.0 fetch --locked` before source preparation.

## Supplements

- `gbm-MIT.txt` is the complete upstream MIT license of `gbm-sys 0.4.0` from
  the workspace revision in the crate's published VCS record.
  [Upstream license](https://github.com/Drakulix/gbm.rs/blob/2b4047f8e5ab4d050d4ed88f5ee2515069478fac/LICENSE)
- `rust-skia-MIT.txt` is the complete upstream MIT license of `skia-bindings
  0.87.0` and `skia-safe 0.87.0` from their common published workspace revision.
  [Upstream license](https://github.com/rust-skia/rust-skia/blob/e551f334ad5cbdf43abf15acf69efd919f6feebc/LICENSE)
- `drm-fourcc-2.2.0.txt` records the pinned crate's MIT declaration and original
  author attribution. That revision has no standalone license file. The
  supplement contains the referenced MIT terms and the upstream Linux header's
  copyright and permission statement for the generated DRM format identifiers.
  [Crate manifest](https://github.com/danielzfranklin/drm-fourcc-rs/blob/7b9f80060ffd4b35cd236437e65a17fb517d163c/Cargo.toml)
- The indexed notices for `simd_helpers 0.1.0` preserve its original source,
  upstream MIT declaration, author and reference terms. The published revision
  contains no standalone license file. Other selected crates with missing
  workspace-level license files use the reviewed copies from the checksum-pinned
  Alpha 1 notice collection. Slint is distributed under its GPL-3.0-only option.
  [Prior review and upstream evidence](../alpha1-dependency-notice-review.md)

The new source archive retains the original files inside each distribution
crate, including nested third-party notices and copyright headers. The
published-source inventory records paths into each crate without rewriting it.
The binary distribution continues to use its own `THIRD-PARTY.json`.

## Bundled fonts

The sctk-adwaita GUI embeds the 31,236-byte Cantarell Regular font. Its file
matches the GNOME 0.0.5 release SHA-256
`d15f25bcdaba2e25f54c32d4e29e68fedeb1b7a1d8fbc83f7c4475916e93b55f`.
`Cantarell-0.0.5-OFL.txt` preserves the original Understanding Limited and
Jakub Steiner attribution and the complete SIL Open Font License 1.1 from
that same revision, with line endings normalized to LF. Both the binary notice
index and the source inventory link the font to this file.
[GNOME font](https://github.com/GNOME/cantarell-fonts/blob/61362600e104b96c88a2fdc068e1e2c68aef4f16/ttf/Cantarell-Regular.ttf) · [GNOME license](https://github.com/GNOME/cantarell-fonts/blob/61362600e104b96c88a2fdc068e1e2c68aef4f16/COPYING)

The source inventory records each bundled font and test font fixture by
archive path, content digest, and applicable notices. The Slint font database
includes the DejaVu attribution sidecar and full Bitstream/Arev terms.
fontdb's Tuffy fixture carries its public-domain dedication. ttf-parser ships
the Apache terms accompanying the two Google COLR fixtures, two FontTools
faces carrying their own no-rights-reserved credits, and small package test
fixtures. Nested license filenames, including suffixes and plurals, are
indexed wherever they occur in the preserved crate sources.

The sctk-adwaita author credits snakedye/snui in the rounded-headerbar routine
in `src/lib.rs` and references MPL-2.0 for that code. The mapped headerbar
notice in the native packages identifies the file, where to obtain its
complete corresponding source, and the full MPL-2.0 terms. The crate's original
MIT license accompanies it. [Upstream source](https://github.com/PolyMeilex/sctk-adwaita/blob/d5f343c45c04d4a026db57f76760819b7325db00/src/lib.rs#L712-L771)

The upstream Slint workspace assigns copyright and license terms to SVGs in
the compiler crate and Linux KMS backend through its root REUSE.toml. The
`slint/` supplement preserves those original assignments and the additional
artist credits and terms. `SOURCE-NOTICES.json` maps all 75 compiler SVGs and
the KMS pointer to their published upstream paths and credits. The native GUI
uses the Fluent style. Its installed notices link to the supplement through
the compiler's binary notice entry. [Pinned upstream REUSE manifest](https://github.com/slint-ui/slint/blob/70ad5e38344ed600c2fc763da9787f2e7ef3db7f/REUSE.toml)
