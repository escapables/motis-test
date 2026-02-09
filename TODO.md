# USB Bundle Rename + Legacy HTML Removal

Goal: make Svelte the only portable USB app path, rename `usb-bundle-svelte` to `usb-bundle`, and remove legacy HTML bundle creation paths.

Rollback reference:
- Local backup commit: `64de8204` (`Simplify Stockholm metro labels to line and direction`)
- Remote baseline: `8b0496d8` (`origin/master`)

## Steps

- [x] 1. Define migration scope and rollback point.
- [x] 2. Rename runtime folder `usb-bundle-svelte/` -> `usb-bundle/` and keep `data` symlink local (`../data`).
- [x] 3. Update build tooling to output only `usb-bundle/` (Svelte bundle).
- [x] 4. Update docs and references from `usb-bundle-svelte` to `usb-bundle`.
- [x] 5. Remove legacy HTML-bundle-facing docs/instructions so end users only follow Svelte flow.
- [x] 6. Clean up `.gitignore` rules to match the new single-bundle model.
- [x] 7. Verify key flows (`build-usb.sh` path constants + launcher/import scripts) and summarize any residual manual cleanup.
