# RVP patch of libmpv2 6.0.0

This is `libmpv2` 6.0.0 from crates.io (LGPL-2.1, see `LICENSE`), used by
RVP through `[patch.crates-io]` in the workspace manifest. Changes:

- `Mpv::wait_event` returned `None` for a `MPV_EVENT_PROPERTY_CHANGE` whose
  format is `MPV_FORMAT_NONE` (the observed property has no value, for
  example after a file ended). `None` also means "no more events", so a
  caller draining the queue after a wakeup callback stopped early and was
  not woken again. The event is now reported as
  `Event::PropertyUnavailable { name, reply_userdata }` (new variant).
  Found in goal B (`docs/spikes/s3-qt-shell.md`); owner decision
  2026-10-10 to keep libmpv2 with this patch (`docs/PLAN.md`, D1).
- Two wrappers the crate lacked, for the normalizer's control thread
  (`crates/app/src/normalizer.rs`, 2026-10-15):
  `Mpv::create_weak_client` (`mpv_create_weak_client`: a handle that does
  not keep the core alive) and `Mpv::request_log_messages`
  (`mpv_request_log_messages`).
- mpv's software renderer, for M11's thumbnail core
  (`crates/engine/src/thumbnail.rs`, 2026-10-17): `SoftwareRenderContext`
  (`Mpv::create_software_render_context`, `MPV_RENDER_API_TYPE_SW`) with
  `render` into a caller's buffer of `u32` pixels (size, format, stride and
  pointer checked before mpv sees them; `BLOCK_FOR_TARGET_TIME` off), the
  update callback and `update`. Its `Drop` frees the context before the
  callback, so mpv cannot call a freed callback.
- `Mpv::screenshot_raw` (`mpv_command_ret` with `screenshot-raw`), for
  M12's captures and pixel checks (2026-10-18): the reply's map is read
  into a `RawImage` (size, stride, format, a copy of the pixels, the
  stride checked against the data's length) and freed with
  `mpv_free_node_contents`.
- Hooks, for M18's smoothing filter before playback starts
  (`crates/engine/src/engine.rs`, 2026-11-08): `Mpv::hook_add`
  (`mpv_hook_add`), `Mpv::hook_continue` (`mpv_hook_continue`) and
  `Event::Hook { name, id, reply_userdata }` (`MPV_EVENT_HOOK`, which
  upstream reports as `Event::Deprecated`).
- Teardown order (2026-11-21): `Mpv::drop` destroys the handle before it
  frees the wakeup callback's box, and `RenderContext::drop` frees the
  context before the update callback's box, as `SoftwareRenderContext`
  does: mpv may call a callback while its handle or context goes. RVP's
  callbacks owned no memory (a function item; the player's engine is never
  dropped), so nothing RVP ran was affected.
- Flags (2026-11-21) go to mpv and come back as a C `int`, as mpv keeps
  them: `GetData for bool` let mpv write 4 bytes into a 1-byte `bool`,
  `SetData for bool` passed an `i64`, and an observed flag's event read
  the first byte of mpv's `int`.
- `Event::ClientMessage` (2026-11-21): a message without arguments (mpv
  leaves `args` NULL) gives an empty list, not a slice made from a null
  pointer, and an argument that is not UTF-8 gives `Err(InvalidUtf8)`
  instead of a panic (`crates/engine/tests/headless.rs`).
- `create_client` removed (2026-11-21): it passed mpv a name freed before
  the call and took a null handle for a valid one. RVP uses
  `create_weak_client`.
- Leaks (2026-11-21): `with_initializer` destroys the handle, and with it
  the core's thread, when the initializer fails (a refused option;
  `crates/engine/tests/refused_start.rs`), and `GetData for String` frees
  mpv's string when it is not UTF-8.
- Left as upstream has them, unused by RVP:
  - `GetData for MpvStr` keeps mpv's string when it is not UTF-8 (a leak).
  - `protocol.rs` (custom stream protocols; RVP's disc stream is its own,
    `shell/src/disc/stream.rs`): not checked.
  - `RenderParam::ICCProfile`, `NextFrameInfo` and `AdvancedControl`'s
    conversions in `render.rs`: not checked.
