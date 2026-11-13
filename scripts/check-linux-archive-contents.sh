#!/usr/bin/env bash
#
# Check that the CLI and FFI archives carry the Linux clipboard owner helper. A channel
# that omits the helper installs a program whose clipboard-set has no owner, and
# nothing else in the pipeline notices.
#
# Usage:
#   check-linux-archive-contents.sh cli <cli.tar.gz>
#   check-linux-archive-contents.sh ffi <ffi.tar.gz>
#   check-linux-archive-contents.sh all <cli.tar.gz> <ffi.tar.gz>
# Exit: 0 every named channel carries the helper, 1 a channel does not, 2 bad arguments

set -euo pipefail

# A reader that quits at its first match (grep -q, awk exit) can close the pipe
# before printf has written everything; printf then dies of SIGPIPE and pipefail
# reports a failure for a check that passed. Every reader below drains its input.

HELPER=acdesktop-linux-clipboard-owner
LIBRARY=libacdesktop_ffi.so

usage() {
  echo "Usage: $0 cli <archive> | ffi <archive> | all <cli-archive> <ffi-archive>" >&2
  exit 2
}

problems=0
fail() {
  echo "FAIL: $1" >&2
  problems=$((problems + 1))
}

archive_listing() {
  tar -tzf "$1" | sed 's|^\./||' | sort
}

executable_members() {
  tar -tvzf "$1" | awk '$1 ~ /^-..x/ { print $NF }' | sed 's|^\./||' | sort
}

check_cli() {
  local archive=$1 listing expected executable
  [ -r "$archive" ] || { fail "cannot read CLI archive $archive"; return; }
  listing=$(archive_listing "$archive")
  expected=$(printf 'acdesktop\n%s\n' "$HELPER" | sort)
  if [ "$listing" != "$expected" ]; then
    fail "CLI archive $archive lists '$(printf '%s' "$listing" | tr '\n' ' ')', expected exactly '$(printf '%s' "$expected" | tr '\n' ' ')'"
    return
  fi
  executable=$(executable_members "$archive")
  if [ "$executable" != "$expected" ]; then
    fail "CLI archive $archive members are not all executable: '$(printf '%s' "$executable" | tr '\n' ' ')'"
    return
  fi
  echo "OK: CLI archive carries acdesktop and $HELPER, both executable"
}

check_ffi() {
  local archive=$1 listing root member executable
  [ -r "$archive" ] || { fail "cannot read FFI archive $archive"; return; }
  listing=$(archive_listing "$archive")
  root=$(printf '%s\n' "$listing" | awk -F/ 'NF > 1 && !found { print $1; found = 1 }')
  if [ -z "$root" ]; then
    fail "FFI archive $archive has no top-level directory"
    return
  fi
  for member in "lib/$LIBRARY" "lib/$HELPER" include/acdesktop.h LICENSE README.md SHA256SUMS; do
    if ! printf '%s\n' "$listing" | grep -Fx "$root/$member" >/dev/null; then
      fail "FFI archive $archive does not list $root/$member"
    fi
  done
  executable=$(executable_members "$archive")
  if ! printf '%s\n' "$executable" | grep -Fx "$root/lib/$HELPER" >/dev/null; then
    fail "FFI archive $archive: $root/lib/$HELPER is not executable"
  fi
  readme=$(tar -xzOf "$archive" "$root/README.md" 2>/dev/null || true)
  for phrase in "glibc 2.35" "SONAME" "-rpath" "one at a time" "ad_snapshot" "carry text only" "$HELPER" "wl-clipboard"; do
    if ! printf '%s\n' "$readme" | grep -F -- "$phrase" >/dev/null; then
      fail "FFI archive README does not state '$phrase'"
    fi
  done
  [ "$problems" -eq 0 ] && echo "OK: FFI archive carries $LIBRARY and $HELPER side by side"
  return 0
}

[ "$#" -ge 1 ] || usage
channel=$1
shift
case "$channel" in
  cli) [ "$#" -eq 1 ] || usage; check_cli "$1" ;;
  ffi) [ "$#" -eq 1 ] || usage; check_ffi "$1" ;;
  all)
    [ "$#" -eq 2 ] || usage
    check_cli "$1"
    check_ffi "$2"
    ;;
  *) usage ;;
esac

[ "$problems" -eq 0 ]
