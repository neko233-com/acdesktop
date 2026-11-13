#!/usr/bin/env bash
set -euo pipefail

if command -v python3 >/dev/null 2>&1 && python3 -c 'import sys; assert sys.version_info >= (3, 7)' >/dev/null 2>&1; then
  PYTHON=python3
elif command -v python >/dev/null 2>&1 && python -c 'import sys; assert sys.version_info >= (3, 7)' >/dev/null 2>&1; then
  PYTHON=python
else
  echo "Python 3.7 or later is required" >&2
  exit 1
fi
bash scripts/check-release-consistency.sh
VERSION=$("$PYTHON" "$(dirname "${BASH_SOURCE[0]}")/release-version.py")
TAG_NAME="v${VERSION}"
SHOULD_RELEASE=false
NEEDS_GITHUB_RELEASE=false
if RELEASE_DRAFT=$(gh release view "$TAG_NAME" --json isDraft --jq .isDraft 2>/dev/null); then
  case "$RELEASE_DRAFT" in
    false) ;;
    true) SHOULD_RELEASE=true; NEEDS_GITHUB_RELEASE=true ;;
    *) echo "Unable to verify release draft status for ${TAG_NAME}" >&2; exit 1 ;;
  esac
else
  STATUS=$(gh api -i "repos/${GH_REPO}/releases/tags/${TAG_NAME}" 2>/dev/null || true)
  if [[ "$STATUS" != HTTP/*' 404 '* ]]; then
    echo "Unable to verify whether release ${TAG_NAME} exists" >&2
    exit 1
  fi
  SHOULD_RELEASE=true
  NEEDS_GITHUB_RELEASE=true
fi
SOURCE_SHA=$(git rev-parse HEAD)
if [ "$SHOULD_RELEASE" = true ] || [ "$NEEDS_GITHUB_RELEASE" = true ]; then
  if git rev-parse --verify "refs/tags/${TAG_NAME}" >/dev/null 2>&1; then
    SOURCE_SHA=$(git rev-parse "refs/tags/${TAG_NAME}^{commit}")
    TAG_VERSION=$(git show "${SOURCE_SHA}:Cargo.toml" | "$PYTHON" "$(dirname "${BASH_SOURCE[0]}")/release-version.py" -)
    if [ "$TAG_VERSION" != "$VERSION" ]; then
      echo "Existing tag ${TAG_NAME} has inconsistent version metadata" >&2
      exit 1
    fi
  else
    if [ "${RELEASE_DRAFT:-}" = true ]; then
      echo "Draft release ${TAG_NAME} has no matching local tag" >&2
      exit 1
    fi
    gh api "repos/${GH_REPO}/git/refs" -f "ref=refs/tags/${TAG_NAME}" -f "sha=${SOURCE_SHA}"
  fi
fi
{
  echo "tag_name=${TAG_NAME}"
  echo "version=${VERSION}"
  echo "source_sha=${SOURCE_SHA}"
  echo "should_release=${SHOULD_RELEASE}"
  echo "needs_github_release=${NEEDS_GITHUB_RELEASE}"
} >> "$GITHUB_OUTPUT"
