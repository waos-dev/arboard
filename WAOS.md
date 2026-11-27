# arboard for WaOS

This fork adds WaOS to arboard 3.7.0: on a WaOS app (a WASI component, `target_os = "wasi"`), `Clipboard` reads and writes plain text through `wasdk::clipboard`, where upstream has no backend for that target. The change is `src/platform/waos.rs`, its switch in `src/platform/mod.rs` and the `wasdk` dependency for that target; everything else is upstream's. It is kept for whoever ports an app to WaOS, and is not offered upstream.

## Using it

```toml
[patch.crates-io]
arboard = { git = "https://github.com/waos-dev/arboard", branch = "waos" }
```

The fork asks for the published `wasdk` (0.8.0). An app that takes `wasdk` from WaOS's repository names it in the same `[patch.crates-io]`, so the fork builds with that one too.

## What it does

`Clipboard::new` opens WaOS's clipboard and returns `Error::ClipboardNotSupported` if the system has none. `get_text` reads the first item offering `text/plain` as a `String`, or returns `Error::ContentNotAvailable` when none does. `set_text` writes one `text/plain` item, and `clear` writes an empty item list. Because arboard's API is synchronous, each call waits on the app's thread with `wasdk::block_on` until the clipboard answers.

## What it leaves out

Images (`get_image`, `set_image`), HTML (`get().html`, `set_html`) and file lists (`get().file_list`, `set().file_list`) are not carried yet: reads return `Error::ContentNotAvailable` and writes return `Error::ConversionFailure`. Every read and write goes through the user's clipboard switches for the app (`clipboard-read` and `clipboard-write`); when a switch is off, the call returns `Error::Unknown` saying so. Unlike the user's own copy, cut and paste view events in `wasdk`, calls through arboard are not tied to a user gesture and only see what is on the clipboard while one of the app's views has the focus.
