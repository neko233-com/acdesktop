# Common Workflows

Patterns for multi-step desktop automation. Every ref comes from the latest snapshot output.

## First-Time Setup

Check the platform and permissions before any automation.

```bash
acdesktop status
acdesktop permissions
```

- On macOS, the terminal needs Accessibility permission, and Screen Recording for screenshots. If `permissions` reports `denied`, run `acdesktop permissions --guide`. It opens System Settings at the missing permission with a panel that shows the user which app to drag into the list.
- On Windows, there is no permission dialog for same-integrity targets. Input into an elevated target needs an equally elevated terminal.
- On Linux, there is no permission dialog. The session needs a running AT-SPI bus and `status` must report `accessibility` as `granted`. Run the tool in the same desktop session as the target app. If `status` reports it denied, read the bus section of the Linux skill.

Then run `acdesktop skills get platform` for the details of your OS.

## Dense app: skeleton, drill, act, verify

Use this for Electron apps and any app with more than about 50 interactive elements.

```bash
acdesktop snapshot --skeleton --app "Slack" -i --compact
# group "Channels" @s8f3k2p9:e2 (children_count: 42)
acdesktop snapshot --root @s8f3k2p9:e2 -i --compact
acdesktop click @s8f3k2p9:e18
acdesktop snapshot --root @s8f3k2p9:e2 -i --compact
```

- Re-drilling a root replaces that root's refs and the refs of drills made under it. Skeleton refs elsewhere, and drills in other regions, stay valid.
- Interactive elements inside skeleton depth already have normal refs.
- If you know the role or exact name, skip the map and run `find` with `--role --name --exact`.

## Fill a form

```bash
acdesktop snapshot --app "System Settings" -i --compact
acdesktop set-value @s8f3k2p9:e3 "My MacBook Pro"
acdesktop click @s8f3k2p9:e8
acdesktop snapshot --app "System Settings" -i --compact
```

- Prefer `set-value`. Use `clear` then `type` when the field needs real key events.
- Read the result: `delivered_verified` confirms the field value only, not that the app accepted it. Verify the outcome in the UI.
- To move field to field without a ref for each, use `press tab` and `type`.

## Navigate menus

```bash
acdesktop find --app "TextEdit" --surface menubar --name "Save As…" --exact --first
acdesktop click @s8f3k2p9:e5
acdesktop wait --window "Save" --app "TextEdit"
acdesktop snapshot --app "TextEdit" --surface sheet -i
```

- `find --surface menubar` returns one ref instead of the whole menu bar.
- To walk a menu by hand: click the menu bar item, `wait --menu`, `snapshot --surface menu -i`, click the item, `wait --menu-closed`.
- A headless `press` never runs a menu item. Find the item and `click` it instead.

## Dialogs, sheets and alerts

```bash
acdesktop wait --window "Save As" --app "TextEdit" --timeout 5000
acdesktop snapshot --app "TextEdit" --surface sheet -i
acdesktop set-value @s8f3k2p9:e2 "my-document.txt"
acdesktop click @s8f3k2p9:e5
acdesktop snapshot --app "TextEdit" -i
```

Snapshot the surface (`sheet`, `alert`, `popover`), not the window, while an overlay is open. Never use `--skeleton` on a surface. After the dialog closes, snapshot the window again for fresh refs. When an action leaves `data.surfaces` non-empty, answer that overlay next.

## Right-click context menu

```bash
acdesktop right-click @s8f3k2p9:e3
acdesktop snapshot --app "Finder" --surface menu -i
acdesktop click @s8f3k2p9:e7
acdesktop wait --menu-closed --app "Finder" --timeout 2000
```

If `right-click` returns `APP_UNRESPONSIVE`, the menu may have opened. Look at the screen state first. Do not retry blindly. Some OSes need `--headed` for right-click (platform skill).

## Scroll to find an item

```bash
acdesktop snapshot --skeleton --app "App" -i --compact
acdesktop snapshot --root @s8f3k2p9:e2 -i --compact
acdesktop scroll @s8f3k2p9:e8 --direction down --amount 5
acdesktop find --app "App" --name "Target Item" --exact
```

Repeat `scroll` and `find` until `find` returns a ref. Then `click` it. If a list is virtualized and the row never appears, scroll the container instead of `scroll-to`.

## Wait for async UI

```bash
acdesktop click @s8f3k2p9:e5
acdesktop wait --text "Download complete" --app "App" --timeout 30000
acdesktop wait --element @s8f3k2p9:e10 --predicate actionable --timeout 10000
```

Use the wait primitive that matches the signal: text, element, window, menu or event. After `launch`, a dialog trigger or a menu click, wait before you snapshot.

## Check before act

```bash
acdesktop is @s8f3k2p9:e6 --property checked
acdesktop check @s8f3k2p9:e6
```

`check` and `uncheck` are idempotent. Use them instead of `toggle`. If a toggle result is `delivered_unverified`, read the state with `is` before you act again.

## Copy text from an element

Read it directly: `acdesktop get @s8f3k2p9:e5 --property value`. If the element does not expose its text, focus it, select all, copy with the OS shortcut through `press`, then run `clipboard-get`.

## Drag and drop

```bash
acdesktop --headed drag --from @s8f3k2p9:e3 --to @s8f3k2p9:e8
acdesktop --headed drag --from-xy 100,200 --to-xy 500,400 --drop-delay 800
```

Drag is physical and needs `--headed`. Confirm the drop by reading the destination state, not the command's `ok`.

## Multiple windows

```bash
acdesktop list-windows --app "Finder"
acdesktop focus-window --window-id w-5678
acdesktop snapshot --app "Finder" --window-id w-5678 -i
```

## Launch, automate, close

```bash
acdesktop launch "Calculator"       # macOS name; on Windows use "calc.exe" or an absolute path
acdesktop snapshot --app "Calculator" -i
acdesktop close-app "Calculator"
```

Read `data.window` in the launch result. If it is absent and you need a window, use `launch --activate` or `wait --event window-opened`. The launch identifier rules differ by OS (platform skill).

## Chromium app: two tools

Slack, VS Code, Discord and similar apps are Chromium based. Use acdesktop for native surfaces and a CDP client for dense web contents.

```bash
acdesktop launch "Slack"       # macOS name; on Windows use the absolute path to slack.exe
acdesktop close-app "Slack"
acdesktop wait --event app-terminated --app "Slack" --timeout 10000
acdesktop launch "Slack" --cdp 9222
agent-browser connect 9222
```

`--cdp` needs a fresh launch, so close a running app first. Drive the web contents with the CDP client. Keep menus, file dialogs, windows, notifications and screenshots on acdesktop. If no CDP client exists, skip `--cdp` and use the skeleton workflow on the accessibility path. It always works.

## Notifications

```bash
acdesktop --headed list-notifications --app "Slack"
acdesktop --headed notification-action 1 "Reply" --expected-app Slack --expected-title "#general"
acdesktop --headed dismiss-notification 1 --expected-app Slack
```

macOS (Notification Center), Windows (Action Center) and Linux (GNOME Shell) support these commands. List first and copy the fingerprint from the listing. Notifications reorder, so a stale index fails with `NOTIFICATION_NOT_FOUND` instead of pressing the wrong row.

## Batch

```bash
acdesktop batch '[
  {"command":"click","args":{"ref_id":"@s8f3k2p9:e1"}},
  {"command":"wait","args":{"ms":200}},
  {"command":"press","args":{"combo":"return"}}
]' --stop-on-error
```

Batch saves process starts. It is not a transaction. Entries after a failure run unless you pass `--stop-on-error`.

## Trace a multi-step run

This example uses `jq` to read the session id returned by `session start`.

```bash
session_start=$(acdesktop session start --name "invoice-bot")
ACDESKTOP_SESSION=$(printf '%s\n' "$session_start" | jq -er '.data.session_id')
export ACDESKTOP_SESSION
acdesktop status
acdesktop trace show --limit 50
acdesktop session end "$ACDESKTOP_SESSION"
```

`status` confirms `session_id` and `tracing`. Use one session per run. Concurrent agents set the same variable and use qualified refs from their own snapshots.

## Anti-patterns

The core skill already states the rules on refs, waits, headed mode and retries. These are the other common mistakes:

1. **Full snapshots of a dense app.** Use `--skeleton` and `--root`, and re-drill only the region that changed. A full tree costs far more tokens.
2. **Coordinates when a ref exists.** Semantic actions are more reliable than coordinate clicks.
3. **Skipping `status` and `permissions`.** A missing permission looks like an empty tree.
4. **Window snapshot while an overlay is open.** Snapshot the surface instead.
5. **Hand-rolled CDP.** Use a real CDP client. Raw WebSocket scripts and app-internal APIs break silently and verify nothing.
6. **Parallel writers on one editor.** Click-to-edit, type and commit are not atomic. Serialize the sequence, and after an uncertain delivery read the state before you act.
