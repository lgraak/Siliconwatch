# fontdue 0.9.5: local Rust 1.85 compatibility patch

This directory contains the complete 43-file crates.io package, with exactly six
source-line replacements in `fontdue-0.9.5/src/table/parse.rs`. The sibling
`fontdue-0.9.5-rust185.patch` records the full upstream-to-local source diff.

## Provenance

- Package: `fontdue` 0.9.5, <https://crates.io/crates/fontdue/0.9.5>.
- Archive: <https://static.crates.io/crates/fontdue/fontdue-0.9.5.crate>.
- Archive SHA-256: `e796cb6894e6418daacea84b28dd4f98d3b32c74d5bb452ba1ecfbf5ad8e84ca`,
  verified against the registry checksum in the pre-patch project lockfile.
- Repository: <https://github.com/mooman219/fontdue>.
- Packaged `.cargo_vcs_info.json` commit:
  `1a84e6d31ad8f91be74a2d446a8e966e2bf4ba88`.
- Licenses: `MIT OR Apache-2.0 OR Zlib`. All original license files, notices,
  manifests, changelog, VCS metadata and other packaged files are preserved.
  This notice does not replace or modify their terms.

## Approved change and boundary

The upstream package uses `u8::cast_signed`, `u16::cast_signed` and
`u32::cast_signed`, which do not compile on Rust 1.85.0. At upstream lines
40, 46, 52, 167, 172 and 177, each same-width unsigned-to-signed conversion is
replaced with `as i8`, `as i16` or `as i32`. These conversions preserve the
same two's-complement bit interpretation, including the signed boundary.
No parsing logic, bounds checks, public API, feature or dependency requirement
is changed. The package remains version 0.9.5; the root `[patch.crates-io]`
selects this local source, and its workspace exclusion prevents it from
becoming a SiliconWatch workspace member.

`src/font.rs` is byte-identical to the upstream package and retains the
`char::from_u32` validation for malformed font codepoints. Do not replace this
package with 0.9.3 to work around the compiler error. Upstream context:
[issue #172](https://github.com/mooman219/fontdue/issues/172) and its
[maintainer-linked fix](https://github.com/mooman219/fontdue/commit/6f07418745b6d89a648c09873d80bd3a39d69128).

This patch was explicitly authorized for SiliconWatch Prototype 010. It does
not authorize changing the selected Slint 1.13.1 features or Rust 1.85 MSRV.
The milestone report records actual build/test results and any remaining
blockers; this notice alone makes no verification claim.
