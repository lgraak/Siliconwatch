#!/usr/bin/env bash
# Assemble only outputs produced by this successful candidate run.
set -euo pipefail
shopt -s dotglob nullglob
cd "$RUNNER_TEMP"
envelope="$GITHUB_WORKSPACE/scripts/release/envelope.py"
candidate="$GITHUB_WORKSPACE/scripts/release/candidate.py"
python3 "$envelope" unpack "inputs/source/source.gpg" source
mkdir packages reports
cp source/SOURCE-PROVENANCE.json reports/
for distro in debian fedora arch; do
  python3 "$envelope" unpack "inputs/native-$distro/native.gpg" "$distro"
  for asset in "$distro"/*; do
    case "$asset" in
      *.deb|*.rpm|*.pkg.tar.zst) dest=packages ;;
      *.json|*.log) dest=reports ;;
      *) echo "Unexpected native output: $asset" >&2; exit 1 ;;
    esac
    test ! -e "$dest/${asset##*/}"
    mv "$asset" "$dest/"
  done
done
jq -n --arg result "$QUALITY_RESULT" --arg sha "$GITHUB_SHA" \
  --arg url "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID" \
  '{result:$result, source_commit:$sha, run_url:$url}' > reports/ci.json
digest=$(python3 "$candidate" seal --version "$VERSION" --commit "$GITHUB_SHA" \
  --packages packages --notes "$GITHUB_WORKSPACE/docs/releases/$VERSION.md" --reports reports \
  --source "source/siliconwatch-${VERSION#v}-source.tar.gz" --output candidate)
python3 "$candidate" verify --version "$VERSION" --commit "$GITHUB_SHA" --candidate candidate --digest "$digest"
python3 "$envelope" pack candidate candidate.tar.gpg
{
  echo "### Candidate ready for review"
  echo "Version: \`$VERSION\`"
  echo "Source: \`$GITHUB_SHA\`"
  echo "Run: \`$GITHUB_RUN_ID\`"
  echo "Manifest SHA-256: \`$digest\`"
} >> "$GITHUB_STEP_SUMMARY"
