## Summary

<!-- What changed and why — 1–3 lines. -->

## Related issues

Closes #

## Type of change

- [ ] `feat:` — new feature
- [ ] `fix:` — bug fix
- [ ] `docs:` — documentation only
- [ ] `refactor:` — no behavior change
- [ ] `perf:` — performance improvement
- [ ] `test:` — adding or fixing tests
- [ ] `chore:` — maintenance / dependencies
- [ ] `ci:` — CI/CD changes
- [ ] `BREAKING CHANGE` — incompatible ABI or CLI change

## How tested

<!-- Check every gate you ran. $HOST_PKGS is the package set for your OS, as listed in CONTRIBUTING.md. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy "${HOST_PKGS[@]}" --all-targets -- -D warnings`
- [ ] `cargo test "${HOST_PKGS[@]}" --lib`
- [ ] `cargo test -p acdesktop`
- [ ] `cargo test -p acdesktop-ffi --tests`
- [ ] `ACDESKTOP_E2E_EXCLUSIVE=1 bash tests/e2e/run.sh` (requires `--release` build + AX permission; run when behavior changes)

## Checklist

- [ ] PR title uses a conventional-commit prefix (`type: description`, lowercase, no capital after colon)
- [ ] `cargo fmt` and `cargo clippy -D warnings` are clean
- [ ] All tests pass
- [ ] No `unwrap()` added in non-test code
- [ ] No inline comments added (only `///` doc-comments on public items)
- [ ] All modified files are within the 400 LOC limit
- [ ] Skill docs updated if a command, flag, or JSON output changed (`skills/acdesktop/` or the platform skill under `skills/acdesktop-macos/`, `skills/acdesktop-windows/` or `skills/acdesktop-linux/`)
- [ ] README / other docs updated if the public interface changed
