# Legacy GUI (Deprecated)

The simple HTML Tauri GUI in `gui/` is deprecated and not part of the supported portable runtime.

Use the Svelte native app path instead:

- Build: `./gui-svelte/build-usb.sh`
- Bundle output: `usb-bundle/`
- Run: `./usb-bundle/RUN.sh`

If you need to work on native runtime behavior, use `gui-svelte/src-tauri/`.
