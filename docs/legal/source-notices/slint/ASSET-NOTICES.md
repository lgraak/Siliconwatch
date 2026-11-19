# Slint 1.13.1 artwork and attribution

SiliconWatch uses Slint's Fluent widget style for its Linux GUI. Slint supplies
the copyright holders and license assignments in the accompanying `REUSE.toml`.
This file and the archived Slint artwork come from commit
`70ad5e38344ed600c2fc763da9787f2e7ef3db7f`, as recorded in the crates'
`.cargo_vcs_info.json`. Archive paths below are relative to the named crate.
[Slint's source annotations](https://github.com/slint-ui/slint/blob/70ad5e38344ed600c2fc763da9787f2e7ef3db7f/REUSE.toml)

## Linux GUI icons

Fluent UI System Icons, Copyright (c) 2020 Microsoft Corporation. The complete
original MIT license is in `FluentUI-MIT.txt`. The GUI embeds 12 SVGs from
`i-slint-compiler-1.13.1/widgets/fluent/`. The files are `_arrow_forward.svg`,
`_check-mark.svg`, `_chevron-down.svg`, `_chevron-up.svg`, `_dismiss.svg`,
`_down.svg`, `_dropdown.svg`, `_eye_hide.svg`, `_eye_show.svg`, `_left.svg`,
`_right.svg` and `_up.svg`.
[Fluent UI System Icons license](https://github.com/microsoft/fluentui-system-icons/blob/06f559c29c761ca979894d07a14b3a44c94a24d1/LICENSE)

Material Icons, the Material Design icon set from Google, is credited by Slint
under the Apache License 2.0. Nine embedded Fluent files have byte-identical
copies in Slint's other widget styles under this credit. These are
`_arrow_forward.svg`, `_check-mark.svg`, `_chevron-down.svg`, `_chevron-up.svg`,
`_down.svg`, `_dropdown.svg`, `_left.svg`, `_right.svg` and `_up.svg`.
The terms are in `Apache-2.0.txt`.
[Material Icons attribution](https://github.com/material-icons/material-icons/blob/af0ed9c0e1276bad43c4d6ca8e8aaa283e425195/README.md) · [Material Icons license](https://github.com/material-icons/material-icons/blob/af0ed9c0e1276bad43c4d6ca8e8aaa283e425195/LICENSE)

## Corresponding-source archive

The `i-slint-compiler-1.13.1.crate` archive preserves all 75 widget SVGs from
Slint's pinned source without changes. The archive includes the following groups.

| Archive path | Attribution | License text |
| --- | --- | --- |
| `widgets/fluent/_*.svg` (19 files) | Microsoft Corporation, Fluent UI System Icons | `FluentUI-MIT.txt` |
| `widgets/material/_*.svg` (15), `widgets/cupertino/_*.svg` (19), `widgets/qt/_*.svg` (7) | Material Icons | `Apache-2.0.txt` |
| `widgets/cosmic/_*.svg` (13) | “Cosmic Icons” by System76 | `CC-BY-SA-4.0.txt` |
| `widgets/common/MadeWithSlint-logo-{dark,light}.svg` (2) | Copyright © SixtyFPS GmbH <info@slint.dev> | `CC-BY-ND-4.0.txt` |

System76 licenses the Cosmic Icons artwork under Creative Commons
Attribution-ShareAlike 4.0 International. SixtyFPS licenses the two Made With
Slint logos under Creative Commons Attribution-NoDerivatives 4.0 International.
[Cosmic Icons and attribution](https://github.com/pop-os/cosmic-icons/blob/f1a282a0822aed1f0083dd2ab2e6c65bd5b401eb/README.md) · [Slint's widget artwork](https://github.com/slint-ui/slint/tree/70ad5e38344ed600c2fc763da9787f2e7ef3db7f/internal/compiler/widgets)

The separate `i-slint-backend-linuxkms-1.13.1.crate` archive includes
`mouse-pointer.svg`. Slint credits “Copyright © 2018 Dave Gandy & Fork Awesome”
and records MIT for that file. Its SVG path data is identical to Fork Awesome
1.2.0's mouse-pointer glyph. Slint changed the viewport, dimensions and stroke.
Fork Awesome expressly places the original glyph and SVG under SIL OFL 1.1,
Copyright (c) 2018, Fork Awesome, with Reserved Font Name Fork Awesome.
`Fork-Awesome-OFL-1.1.txt` preserves that attribution and the terms for the
glyph. `Fork-Awesome-MIT.txt` preserves the separately referenced original MIT
notice and Fork Awesome's statement of its scope. SiliconWatch uses the winit
backend for the Linux GUI. The KMS pointer is in the corresponding-source
archive.
[Fork Awesome pointer](https://github.com/ForkAwesome/Fork-Awesome/blob/1e3849530d0266ece3a883649e1398414b92241d/src/icons/svg/mouse-pointer.svg) · [Original license assignments and terms](https://github.com/ForkAwesome/Fork-Awesome/blob/1e3849530d0266ece3a883649e1398414b92241d/LICENSES) · [Slint pointer](https://github.com/slint-ui/slint/blob/70ad5e38344ed600c2fc763da9787f2e7ef3db7f/internal/backends/linuxkms/mouse-pointer.svg)

The accompanying `REUSE.toml` also credits SixtyFPS GmbH for
`i-slint-core-macros-1.13.1/link-data.json` and for the annotations themselves.
SiliconWatch uses Slint's GPL-3.0-only option for those files. The DejaVu font
in `i-slint-common-1.13.1` carries its own attribution and license text
inside that crate.
