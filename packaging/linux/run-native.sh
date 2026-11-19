#!/usr/bin/env bash
# Host entry point. Native package operations run inside disposable containers.
set -euo pipefail
usage() {
    echo "usage: $0 build DIST TAG COMMIT OUTPUT SOURCE_INPUTS | verify DIST TAG COMMIT INPUT" >&2
    exit 2
}
[[ $# -ge 5 ]] || usage
mode=$1 distro=$2 tag=$3 commit=$4
case "$mode:$#" in build:6|verify:5) ;; *) usage ;; esac
number='(0|[1-9][0-9]*)'
[[ $tag =~ ^v${number}\.${number}\.${number}(-((alpha|beta|rc)\.${number}))?$ ]] || usage
[[ $commit =~ ^[0-9a-f]{40}$ ]] || usage
case "$distro" in
    debian) image='debian@sha256:c67e92c8099ba22853ab4cfda7cfb494d253d6012ca82dee98ba8976772af018' ;; # bookworm-slim
    ubuntu) image='ubuntu@sha256:77eb101c607d6e3d4f5ad0c8a02c2136c689b98aba1f359e6502ba517e5c57ec' ;; # 24.04
    fedora) image='fedora@sha256:b189269dfee735c54ebcd3b4e43ff2dc0d4a4b07855468970217c9df57968eb0' ;; # 44
    arch) image='archlinux@sha256:fcee73ea1b96e236248f53a213f71276cc9072d9025b58bce9915e32f19963ae' ;; # base
    *) usage ;;
esac
tools=$(cd "$(dirname "$0")" && pwd)
args=(--rm --platform linux/amd64 --volume "$tools:/release-tools:ro")
if [[ $mode == build ]]; then
    [[ $distro != ubuntu ]] || usage
    mkdir -p "$5"
    out=$(realpath "$5")
    inputs=$(realpath "$6")
    rust=$(rustc +1.85.0 --print sysroot)
    args+=(--volume "$rust:/opt/rust:ro" --volume "$out:/out" --volume "$inputs:/inputs:ro")
else
    inputs=$(realpath "$5")
    receipt_dir=$(mktemp -d "$inputs/.verify-$distro.XXXXXXXX")
    receipt="install-$distro.json"
    trap 'rm -f -- "$receipt_dir/$receipt"; rmdir -- "$receipt_dir"' EXIT
    # A failed retry must not leave a successful receipt from the previous attempt.
    rm -f -- "$inputs/$receipt"
    args+=(--volume "$inputs:/inputs:ro" --volume "$receipt_dir:/receipts")
fi
echo "Native $mode ($distro), source $commit, image $image"
docker run "${args[@]}" "docker.io/library/$image" /bin/bash /release-tools/container.sh "$mode" "$distro" "${tag#v}" "$commit"
if [[ $mode == verify ]]; then
    [[ -s $receipt_dir/$receipt && ! -L $receipt_dir/$receipt ]]
    mv -T -- "$receipt_dir/$receipt" "$inputs/$receipt"
fi
