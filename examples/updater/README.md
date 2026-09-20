# Updater extension

Build the native artifacts once, then run the example:

```sh
bun packages/native/build.ts
bun packages/native/build.ts --extension updater
bun --cwd examples/updater dev
```

Development builds deliberately remain disabled. Replace the demonstration public key and the `[updates.github]` repository in `quickgui.toml` with your own before testing a production update. `updater.Options{AllowDevelopment: true}` enables a deliberately configured development test; it is never enabled by this example automatically.

The example owns one app-wide updater, shows cached update state, exposes manual checks and the automatic-check preference, and quits normally after the portable helper signals readiness. See [the updater guide](../../docs/updater.md) for signing, publication, and platform behavior. Never publish the example with its demonstration key or repository.
