# The Android app (prototype)

CalcCraft as a native Android app: Android's `NativeActivity` loads the Rust library
(`libcalculator.so`, see `src/lib.rs`), and eframe draws the same egui interface as on the
desktop with OpenGL ES. There is no Java or Kotlin code. `docs/mobile-apps-research.md` (on
the `research/mobile-apps` branch) explains the choices and what a store release still needs.

**Status:** builds for 64-bit ARM (`arm64-v8a`), Android 7.0 (API 24) and later, targeting
Android 16 (API 36). Tried on a Samsung Galaxy with Android 17: the layout clears the status
bar and the gesture bar, back closes the menu and then leaves the app, and the app comes back
after it.

## Install

```bash
adb install -r target/android/CalcCraft-1.2.0-arm64.apk
```

The APK is signed with the debug key (`~/.android/debug.keystore`). A build signed with
another key can't update it: uninstall first with `adb uninstall io.github.gavrilov.calccraft`.
Warnings and panics are in `adb logcat -s CalcCraft`.

## Build

```bash
pwsh android/build-apk.ps1
```

It writes `target/android/CalcCraft-<version>-arm64.apk`, with the version from `Cargo.toml`
(1.2.0 is version code 10200). It needs:

- the Rust target and cargo-ndk: `rustup target add aarch64-linux-android`,
  `cargo install cargo-ndk`;
- the Android SDK's `platforms;android-36` and `build-tools;36.0.0`, and the NDK
  `29.0.14206865` (in `%LOCALAPPDATA%\Android\Sdk` or `ANDROID_HOME`);
- a JDK 17 or later for `apksigner` and `keytool` (`%LOCALAPPDATA%\Android\jdk-21` or
  `JAVA_HOME`).

The script doesn't use Gradle. cargo-ndk builds the library (a `cdylib` only for Android: the
desktop and web builds use the binary), `llvm-strip` drops its symbol table, `aapt2` links the
manifest and resources, the library goes in uncompressed, `zipalign -P 16` aligns it to 16 KB
pages (as Google Play requires), and `apksigner` signs the APK (v2 and v3).

## How it works

- `Cargo.toml` gives Android its own eframe: NativeActivity (`android-native-activity`) and
  glow, as in egui's `hello_android` example, without wgpu and `arboard`.
- `AndroidManifest.xml`: the app handles every configuration change itself, since winit allows
  one event loop per process; phones stay upright, because the layouts need some 500 points of
  height; the keyboard pans the screen instead of shrinking it; and the app opts out of
  predictive back (`enableOnBackInvokedCallback="false"`), without which Android 16 and later
  handle the back button themselves and the app never sees it.
- `res/`: the launcher icon (adaptive from Android 8; `cargo test write_android_icons --
  --ignored` writes the PNGs from `src/icon.rs`) and a light and a dark theme for the system
  bars and the start-up background.
- `src/app/android.rs` (JNI on the NativeActivity that `android_main` gets; ndk-context's
  context is the Application, which has no window): the system bars' and camera cutout's part
  of the screen becomes egui's safe area, which the layout keeps clear (NativeActivity always
  draws under them); the back button closes the open menu, popup, flyout or Settings page as
  Esc does, and with nothing open moves the app to the background (Esc would clear the
  calculator, as in Windows); and the interface follows the phone's languages.

## Not done yet

- Copy and Paste (no clipboard: egui has none on Android, and `arboard` doesn't build there).
- Share (an `ACTION_SEND` intent).
- Regional formats: numbers and dates are en-US, whatever the phone's region.
- TalkBack: eframe's screen reader support needs GameActivity on Android.
- The status bar's icons follow the phone's dark mode, not the app's own theme setting.
- The graph's hover-only features (its buttons, tracing) have no touch alternative yet.
- Google Play: an App Bundle (AAB) and a release signing key.
