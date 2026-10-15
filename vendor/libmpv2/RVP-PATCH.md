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
