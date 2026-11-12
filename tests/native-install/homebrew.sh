#!/usr/bin/env bash
set -euo pipefail

FORMULA=${1:?Usage: homebrew.sh FORMULA_PATH}
test -f "$FORMULA"
TAP=local/acdesktop-ci
TAP_PATH="$(brew --repository)/Library/Taps/local/homebrew-acdesktop-ci"
test ! -e "$TAP_PATH"
test ! -e "$(brew --cellar)/acdesktop"
mkdir -p "$TAP_PATH/Formula"
git -C "$TAP_PATH" init
git -C "$TAP_PATH" -c user.name="Homebrew Fixture" -c user.email=fixture@example.invalid \
  commit --allow-empty -m 'test: initialize native installation tap'
cp "$FORMULA" "$(brew --repo "$TAP")/Formula/acdesktop.rb"
brew install "$TAP/acdesktop"
brew test "$TAP/acdesktop"
PREFIX=$(brew --prefix)
BINARY="$PREFIX/bin/acdesktop"
KEG=$(brew --prefix "$TAP/acdesktop")
test -L "$BINARY"
test "$BINARY" -ef "$KEG/bin/acdesktop"
case "$(uname -s)" in
  Darwin) HELPER=acdesktop-macos-helper ;;
  Linux) HELPER=acdesktop-linux-clipboard-owner ;;
esac
test "$PREFIX/bin/$HELPER" -ef "$KEG/bin/$HELPER"
test "$(head -c 2 "$BINARY")" != '#!'
env -i HOME="$HOME" PATH="$KEG/bin" acdesktop version
env -i HOME="$HOME" PATH="$KEG/bin" acdesktop permissions
brew unlink acdesktop
brew link acdesktop
test "$BINARY" -ef "$KEG/bin/acdesktop"
brew uninstall "$TAP/acdesktop"
test ! -e "$BINARY"
test ! -L "$BINARY"
test ! -e "$PREFIX/bin/$HELPER"
brew untap "$TAP"
