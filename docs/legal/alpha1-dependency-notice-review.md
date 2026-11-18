# Alpha 1 dependency notice review

November 17, 2026. This review closes the specific `simd_helpers 0.1.0`
standalone-license-text finding for the assessed unchanged native application
packages. No material redistribution blocker was identified from this finding.
It does not certify every dependency or decide copyright substantiality for
arbitrary generated code. The package binaries and their tested hashes are
unchanged.

## simd_helpers: verified declaration and limited role

The official crate archive has SHA-256
`95890f873bec569a0362c235787f3aca6e1e887302ba4840839bcc6459c42da6`,
matching the project lockfile. All seven packaged files match the cached source.
Its original manifest and macro source match upstream revision
`ca1a2f84aa386d758e98f8a609d990263932fb85`. Both the manifest and registry record
explicitly declare MIT and identify Luca Barbato as author/publisher. The exact
archive and complete repository tree contain no separate license text or
copyright notice. No existing notice has been removed; no copyright year or
holder has been invented. [Pinned manifest](https://raw.githubusercontent.com/lu-zero/simd_helpers/ca1a2f84aa386d758e98f8a609d990263932fb85/Cargo.toml),
[registry record](https://crates.io/api/v1/crates/simd_helpers/0.1.0).

Cargo defines its `license` field as the license under which a package is
released, with `license-file` as an alternative. The absence of a separate file
therefore does not erase the explicit upstream MIT declaration. MIT permits
redistribution subject to retaining its notice terms and existing copyright
notices. [Cargo manifest documentation](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields),
[MIT reference terms](https://spdx.org/licenses/MIT.html).

The selected path is `simd_helpers → rav1e → ravif → image → i-slint-compiler →
slint-build/slint-macros`. These are build/procedural-macro paths. The tool's sole
macro adds conditional `cold` attributes for caller-supplied architectures and
returns the caller's item tokens. It contributes no implementation body or
runtime support routine; its call sites are in the compile-time image/compiler
tooling. Neither the proc-macro binary nor its source is in the native package
payload. This narrow dependency/output analysis supports the finding; it is not
a general exemption for generated code. [Exact macro source](https://github.com/lu-zero/simd_helpers/blob/ca1a2f84aa386d758e98f8a609d990263932fb85/src/lib.rs),
[Rust procedural macro reference](https://doc.rust-lang.org/reference/procedural-macros.html).

The conservative 383-package notice inventory includes build dependencies, which
is why the original package notice identifies this absence. That record remains
accurate. This later review explains why it is not evidence of an unlicensed
runtime component. The adjacent [notice supplement](NOTICE-simd_helpers-0.1.0.txt)
preserves upstream attribution, exact provenance and the standard MIT permission
and disclaimer text. It is explicitly a reference supplement, not a newly
discovered upstream file or a grant made by SiliconWatch. It accompanies the
release preparation separately and is **not represented as already embedded in
the tested native packages**. Reassess before distributing the compiler tool
itself or changing selected features.

## fontdue 0.9.5: exact approved patch

The official archive SHA-256 is
`e796cb6894e6418daacea84b28dd4f98d3b32c74d5bb452ba1ecfbf5ad8e84ca`,
at recorded VCS revision `1a84e6d31ad8f91be74a2d446a8e966e2bf4ba88`.
A fresh comparison covers all 43 files. Exactly one file differs:
`src/table/parse.rs`, with the six approved `u8/u16/u32::cast_signed` replacements
using same-width `as i8/i16/i32` casts, once for each indexed and sequential read.
Reconstructing those substitutions reproduces the vendored bytes exactly. No
other parser or bounds logic changed. The other 42 files, including
`src/font.rs`, its `char::from_u32` validation, the MIT/Apache-2.0/Zlib texts,
manifests, VCS metadata and ignored upstream Cargo.lock, remain unchanged.

The upstream Cargo.lock is 3,081 bytes with SHA-256
`89a337d215e4c22cc8604a6e5f7e2398ca3c02066de8e878538d56c68f2a5762`.
It is an intentional publication input despite the upstream ignore rule. The
patch and rationale remain in [the vendor notice](../../vendor/fontdue-0.9.5-PATCH.md).

Fresh Rust 1.85 verification during package preparation passed four standalone
fontdue tests, all workspace tests and all three target-distribution release
builds. The older milestone also records separate exhaustive u8/u16 and bounded
u32/parser conversion checks on Rust 1.85 and stable; those historical counts
are retained as historical evidence, not relabeled as fresh execution.

## Scope of release preparation

SiliconWatch and Slint use the recorded GPL-3.0-only path. Original upstream
attributions and licenses remain intact. Native packages use distribution
libraries rather than bundling platform libraries; their license metadata is
managed by the distribution. The notice collection is a conservative Cargo
inventory, not a linker-level SBOM or blanket legal certification.

The ultimate release must identify the reviewed merged source matching the
recorded build inputs, including this vendor patch and lockfile. The public base
commit alone does not contain the GUI. No tag or asset upload is authorized by
this review, and no upstream contact or additional permission claim was made.
