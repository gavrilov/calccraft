# Native Android and iOS apps: feasibility, costs and store publishing

Branch: `research/mobile-apps` · Written: 2026-10-18 · Starting point: `main` at `f87a397`
(v1.2.0) · Status: research only, no code changed.

## Summary

| Question | Answer |
|---|---|
| Can CalcCraft run natively on Android? | **Yes.** eframe supports Android (since 0.30) through winit's `android-activity`, and egui ships an official `hello_android` example. The calculator engines are plain Rust and build for Android unchanged. |
| Can it run natively on iOS? | **Yes, with more caveats.** winit supports iOS (UIKit), eframe has built for iOS in egui's CI since 0.29 and supports the safe area since 0.33, but there is no official template, screen readers (VoiceOver) can't read egui apps on iOS yet, and every build needs a Mac with Xcode. |
| How much work? | Roughly **6–10 weeks** of one developer for both apps (section 5). Android first is cheaper and less risky. |
| What does it cost? | **US$25 once** (Google Play) + **US$99 a year** (Apple Developer Program) + a Mac if you don't have one. No store commission: the app is free, without in-app purchases. |
| How long until it's in the stores? | Google Play: a closed test with **12 testers for 14 days** comes first for a new personal account. App Store: usually a review of a day or two after TestFlight. |
| Biggest risks | Apple's copycat and intellectual property rules (4.1, 5.2) for a Windows Calculator look-alike; Android's edge-to-edge screens (no safe area support in egui yet); clipboard, sharing and regional formats need platform code; no VoiceOver on iOS. |

**Recommendation:** port natively with eframe, Android first, then iOS (section 7). Wrapping
the existing web version (section 2.5) is faster for Android but isn't a native app.

## 1. What already works in our favor

- **The engines are portable.** `engine.rs`, `dates.rs`, `converter.rs` and `graphing.rs` have
  no UI or OS dependencies, and the 295 tests run on Windows, Linux and macOS in CI.
- **The UI already runs on phones.** The web version (WebAssembly, the same egui code) works on
  phones with touch, scales to the screen, and installs as a PWA. The layout, touch input and
  font set (Inter, Noto Sans Math, the icon font, all embedded) are proven on small screens.
- **Platform code is already isolated** behind `cfg(windows)` and
  `cfg(target_arch = "wasm32")`: region, language, share, title bar, keyboard layouts. A mobile
  port adds `cfg(target_os = "android")` and `cfg(target_os = "ios")` branches next to them.
- **The icon is drawn in code** (`src/icon.rs`), so every store icon size (512 × 512 for Play,
  1024 × 1024 for Apple) can be rendered exactly.
- **The licenses allow store distribution.** CalcCraft is MIT; the strings come from
  `microsoft/calculator` (MIT); Inter and Noto Sans Math are SIL OFL; the icons are Fluent System
  Icons (MIT). No Segoe font is bundled. The store app needs a licenses screen or file with
  these notices.
- `build.rs` checks the **target** OS (`CARGO_CFG_TARGET_OS`), so cross-compiling from Windows
  to Android doesn't try to embed Windows resources.

## 2. Technical feasibility

### 2.1 Framework support (eframe 0.36.2, winit 0.30)

| Topic | Android | iOS |
|---|---|---|
| Windowing | winit with `android-activity`; eframe features `android-native-activity` or `android-game-activity`; `NativeOptions::android_app` | winit's UIKit backend; eframe runs `run_app`, which never returns (the `run_and_return` path is compiled out on iOS) |
| Entry point | The crate builds as a `cdylib` named `main` with an `android_main(app: AndroidApp)` function | A `staticlib` with an exported start function, linked into an Xcode project whose `main` calls it |
| Renderer | glow (OpenGL ES, as in `hello_android`) or wgpu (Vulkan, or GLES as a fallback) | **wgpu (Metal)**: eframe's glow backend goes through glutin, which has no iOS backend |
| Official example | `examples/hello_android` (builds with `cargo-apk`) | None |
| Safe area (notch, status bar, gesture bar) | **Not supported yet**: `hello_android` reserves 32 px at the top as a stopgap. Apps targeting Android 15+ are always drawn edge to edge, so we must read the insets ourselves (JNI) | Supported since egui 0.33 (`viewport_rect` and `content_rect`) |
| Back button | Supported since egui 0.32 | Not applicable |
| Soft keyboard and IME | Text input fixed in eframe 0.32; IME composition is limited with NativeActivity (GameActivity is better) | Works through winit; the IME fix of egui 0.36 is for the web version only |
| Clipboard | **None**: egui-winit leaves `arboard` out on Android; needs JNI (`ClipboardManager`) | `arboard` lists only Linux, Mac and Windows; plan on `UIPasteboard` (objc2-ui-kit) |
| Screen readers | AccessKit has an Android adapter and egui-winit always includes `accesskit_winit`; needs a TalkBack test | **None**: AccessKit has no iOS adapter yet, so VoiceOver can't read the app |
| Saving settings | eframe's default storage folder is `None` on Android and iOS ("Saving disabled"); set `NativeOptions::persistence_path` to the app's data folder. eframe saves on suspend on both since 0.31 | Same |

eframe 0.37.0 came out on 2026-10-15. It fixes keeping the safe area insets while input is
added (#8484) and a stuck touch scroll (#8636). Upgrading before the port is worth it.

### 2.2 What changes in CalcCraft

| Area | Today | Android | iOS | Effort |
|---|---|---|---|---|
| Crate layout | Binary only (`src/main.rs`) | Add a library target (`cdylib`) with `android_main`; `main.rs` stays for desktop | Same library as a `staticlib` | 1–2 days |
| Language | Windows API, the browser, else `CALCCRAFT_LANGUAGE` | `sys-locale` crate, or `LocaleList` through JNI | `sys-locale`, or `NSLocale.preferredLanguages` | 1 day |
| Regional formats (decimal separator, grouping, dates) | Windows NLS, the browser's `Intl`, else en-US (as on Linux and macOS today) | `java.text.DecimalFormatSymbols` and `DateFormat` through JNI, or ICU4X | `NSNumberFormatter` and `NSDateFormatter` (objc2-foundation), or ICU4X | 3–5 days; ICU4X would also give Linux and macOS real formats |
| Fonts | Segoe UI, else the embedded Inter; Noto Sans Math only on the web | Inter plus Noto Sans Math, as on the web; CJK fonts can be read from `/system/fonts` | Inter plus Noto Sans Math; CJK, Thai and Amharic left out at first, as on the web | 1–2 days |
| Clipboard (Copy, Paste, the graph's Copy) | `arboard` and egui | JNI `ClipboardManager` | `UIPasteboard` | 2–3 days |
| Share (the graph) | Windows' share sheet; a PNG download on the web | `ACTION_SEND` intent with a `FileProvider` | `UIActivityViewController` | 2–4 days |
| Saving settings | eframe's default folder | `persistence_path` = the app's internal data folder | `persistence_path` = Application Support | 0.5 day |
| Screen edges | None needed | Status and navigation bar insets through JNI | egui's safe area | 2–3 days |
| Back button | None | Closes the open popup, flyout or menu, then Settings | None | 1 day |
| Small heights | `MIN_SIZE` is 320 × 500; the web version scrolls below that | A phone in landscape is about 390 px high: lock phones to portrait at first, allow landscape on tablets | Same | 1–2 days |
| Hover-only UI | The graph's hover buttons and hover tracing | Touch alternatives (tap to trace, buttons always shown); these help the web version too | Same | 3–5 days |
| Keep on top, title bar, keyboard layouts | Windows only (already behind `cfg`) | Hidden, as on the web | Hidden | none |

### 2.3 Build tools

| Tool | What it does | Status | Verdict |
|---|---|---|---|
| `cargo-apk` | Builds an APK from a Rust crate (used by `hello_android`, from a fork) | Builds APKs only; Google Play needs an AAB | Fine for trying things out, not for the store |
| `xbuild` | Cross-builds APK, AAB and iOS packages | Marked "unmaintained" by its authors | Avoid |
| `cargo-mobile2` | Generates Android Studio and Xcode projects, with an egui + winit + wgpu template | Maintained for Tauri's needs only; iOS needs macOS | Useful to generate the first projects, then keep them in the repository |
| **Gradle + `cargo-ndk`** | A small Android project of our own; `cargo-ndk` builds `libmain.so` per ABI, Gradle builds and signs the AAB | Standard Android tooling | **Recommended for Android** |
| **Xcode project + `staticlib`** | Cargo builds `aarch64-apple-ios` (and `aarch64-apple-ios-sim`); Xcode links, signs and archives | Standard Apple tooling | **Recommended for iOS** |

Google Play also requires native code to support **16 KB memory pages** (every app that
targets Android 15 or later). NDK r28 and later align for it by default. With an older NDK, the
Rust linker needs `-C link-arg=-Wl,-z,max-page-size=16384`. Check with `zipalign -c -P 16` and
`llvm-objdump -p libmain.so`.

### 2.4 CI and releases

`release.yml` could get two more jobs:

- **Android:** an `ubuntu-latest` job with the NDK. It runs `cargo ndk -t arm64-v8a` (plus
  `x86_64` for emulators) and `./gradlew bundleRelease`, signs with the upload key kept in
  GitHub secrets, attaches the AAB to the release, and optionally uploads it to Play's internal
  track (fastlane `supply`, or a Play Developer API action with a service account).
- **iOS:** a `macos-latest` job. It runs `cargo build --target aarch64-apple-ios --release`,
  then `xcodebuild archive` and `-exportArchive` with a distribution certificate and provisioning
  profile from secrets, then uploads to TestFlight with an App Store Connect API key (fastlane
  `pilot`, or Transporter).

GitHub's macOS runners are free for public repositories, so CI doesn't need a Mac of our own.
Debugging in the iOS simulator does.

### 2.5 Alternative: wrap the web version

| Option | How | For | Against |
|---|---|---|---|
| Android Trusted Web Activity | Bubblewrap turns the existing PWA into an AAB that opens the site in Chrome, without the address bar | A few days of work; the site already installs and works offline | Not native; needs `/.well-known/assetlinks.json` at the **origin root** (`gavrilov.github.io`, so the `gavrilov.github.io` repository or a custom domain); needs internet for the first start |
| iOS web view (Capacitor, or Tauri 2 mobile) | The built `dist/` inside a `WKWebView`, offline | Reuses the web build | Apple's 4.2 asks for more than "a repackaged website"; slower than native; screen readers only through `web_screen_reader` |

These are fallbacks. The native port gives the same app on every platform from one code base.

## 3. Publishing on Google Play

1. **Developer account:** Play Console, a **US$25 one-time** fee, at least 18 years old, and an
   identity check (government ID and a card in your legal name). Choose a **personal** or an
   **organization** account (an organization needs a D-U-N-S number). New personal accounts
   also verify that they have an Android device, with the Play Console app.
2. **Developer verification** (Android's new program): Play Console developers are covered.
   Google registers about 99 % of Play apps automatically. User protections start on
   2026-09-30 in Brazil, Indonesia, Singapore and Thailand, and go worldwide in 2027.
3. **Technical requirements:**
   - An **Android App Bundle** (AAB), signed with an upload key; Play App Signing holds the app
     signing key.
   - **Target API level 36** (Android 16) for new apps and updates since 2026-08-31 (an
     extension to 2026-11-01 can be requested).
   - **16 KB page size** support (section 2.3); updates without it are refused from 2027-02-01.
   - 64-bit native code (`arm64-v8a`).
   - A `minSdk` of 23 (Android 6), as in `hello_android`, is enough.
4. **Store listing:** app name, short and full description, the 512 × 512 icon, a 1024 × 500
   feature graphic, and phone (and tablet) screenshots. Pick a category such as Tools or
   Education.
5. **Policy forms:**
   - The **Data safety** form, required even when no data is collected.
   - A **privacy policy** URL, required too. A page on the GitHub Pages site is enough.
   - The content rating questionnaire (IARC), target audience, ads (none), and app access (no
     sign-in).
6. **Testing:** internal testing first. Personal accounts created after 2023-11-13 then need a
   **closed test with at least 12 testers opted in for 14 days in a row** before they can apply
   for production. Find the testers early.
7. **Production:** apply for production access, then roll out. Reviews of new apps can take
   several days.
8. **Every year:** raise the target API level when Google raises the requirement (each August).

## 4. Publishing on the App Store

1. **Apple Developer Program:** **US$99 a year**, as an individual (your legal name is the seller
   name) or an organization (a legal entity with a D-U-N-S number). Apple waives the fee for
   some nonprofit, educational and government organizations, not for individuals. If the
   membership lapses, the app leaves the store.
2. **A Mac with Xcode 26 or later:** uploads must be built with the iOS 26 SDK (since
   2026-04-28), and since 2026-09-09 the minimum deployment target is iOS 13, which eframe and
   wgpu meet.
3. **Identifiers and signing:** a bundle ID (for example `io.github.gavrilov.calccraft`), a
   distribution certificate and a provisioning profile (Xcode's automatic signing handles them
   locally; CI needs them as secrets).
4. **The app bundle:**
   - `Info.plist` with `ITSAppUsesNonExemptEncryption` = `NO` (no encryption of our own),
     orientations, a launch screen, and the 1024 × 1024 icon.
   - A **privacy manifest** (`PrivacyInfo.xcprivacy`) with the reasons for the "required reason"
     APIs our binary uses. Rust's standard library and our dependencies probably call
     file-timestamp APIs (`stat`, reason `C617.1` for the app's own files) and system-boot-time
     APIs (`mach_absolute_time` for `Instant`, reason `35F9.1`). Check the binary's symbols
     before the first upload.
5. **App Store Connect:**
   - The app record, with the privacy label ("Data Not Collected").
   - A **privacy policy URL**, required for every app (5.1.1).
   - The new age rating questions (required since 2026-01-31).
   - The category, description, keywords, support URL and screenshots (6.9" iPhone, plus 13"
     iPad for a universal app).
   - The **EU trader status** (Digital Services Act). A trader's address and phone number are
     shown in the EU store.
6. **TestFlight:** internal testers right away; external testers (up to 10,000) after a short
   beta review.
7. **App Review:** submit; reviews usually take a day or two. Points to watch:
   - **4.1 Copycats and 5.2 Intellectual property:** the app deliberately looks like Windows
     Calculator. Keep "Windows" and "Microsoft" out of the name, subtitle, keywords and icon.
     Describe it as an open-source calculator based on Microsoft's MIT-licensed Windows
     Calculator, and keep CalcCraft's own icon. This is the largest rejection risk.
   - **4.2 Minimum functionality:** a native app with these modes is fine; a bare web wrapper
     is at risk.
   - **2.5.2:** everything is in the bundle; the app downloads no code.
8. **Every year:** build with the new Xcode SDK when Apple raises the minimum (usually each
   spring), and renew the membership.

## 5. Costs

### 5.1 Money

| Item | Android | iOS | Notes |
|---|---|---|---|
| Developer account | US$25 once | US$99 a year | |
| Computer | Windows is fine | **A Mac is required** | Use your own; a new Mac mini starts around US$600; a rented cloud Mac costs a monthly fee |
| CI | Free | Free | GitHub-hosted runners are free for public repositories |
| Test devices | An Android phone (also needed for Play's device check) | An iPhone is recommended (the simulator comes with Xcode) | Existing phones are enough |
| Privacy policy and support pages | Free | Free | GitHub Pages |
| Store commission | None | None | Free app, no in-app purchases |
| **First year, with your own Mac and phones** | **US$25** | **US$99** | **US$124**; then US$99 a year |

### 5.2 Work (rough estimate, one developer who knows the code)

| Work | Estimate |
|---|---|
| Shared: library target, language and regional formats, fonts, touch alternatives to hover, portrait layout, licenses screen | 2–3 weeks |
| Android: Gradle and NDK project, CI job, JNI for clipboard, insets, share and back button, tests on devices and with TalkBack | 2–4 weeks |
| iOS: Xcode project, CI job, objc2 for clipboard, share and locale, privacy manifest, signing, TestFlight | 2–3 weeks |
| Store listings: screenshots, texts, privacy policy, forms | 3–5 days |
| **Total** | **about 6–10 weeks**, plus the 14-day closed test on Play |

## 6. Risks and open questions

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Apple rejects the app as a copycat or for IP (4.1, 5.2) | Medium | Blocks iOS | Own name and icon, no Microsoft trademarks in the metadata, credit the MIT-licensed original; explain it in the review notes |
| Google raises an impersonation or IP complaint | Low | Listing removed | Same as above |
| Android edge-to-edge: controls under the status or gesture bar | High without work | Bad first impression | Read insets through JNI until egui supports Android's safe area; check eframe 0.37 |
| Soft keyboard in the Graphing mode's text boxes | Medium | Typing equations is awkward | The graphing keypad types into the boxes already; try GameActivity for full IME |
| No VoiceOver on iOS | Certain for now | Blind users can't use the iOS app | Say so in the listing; follow AccessKit's iOS adapter |
| Mobile tooling churn (`cargo-apk`, `xbuild`) | Medium | Broken builds | Own the Gradle and Xcode projects; use standard tools only |
| The 12-tester rule on Play | Certain for a new personal account | 2+ weeks of delay | Recruit testers before the build is ready, or use an organization account |
| Yearly requirements (target API, Xcode SDK, the US$99 renewal) | Certain | The app disappears if missed | A calendar reminder and a CI job that builds with the newest SDKs |

Still to check with a prototype:

- TalkBack with AccessKit through eframe.
- The real size of the AAB and IPA (the web version's wasm is 11.2 MB, 3.9 MB compressed).
- How the Graphing mode performs on a mid-range phone.

## 7. Recommended plan

1. **Decide:**
   - Personal or organization accounts on both stores.
   - The store name (check that "CalcCraft" is free on both stores).
   - The bundle ID.
   - Whether a Mac is available.
2. **Shared groundwork, on desktop and web first:**
   - Upgrade to eframe 0.37.
   - Split the crate into a library and a thin `main.rs`.
   - Language and regional formats for non-Windows systems (ICU4X or the platform APIs). This
     also helps Linux and macOS.
   - Touch alternatives to hover, and a phone layout.
3. **Android prototype:**
   - Start from `hello_android`, then move to our own Gradle project with `cargo-ndk`.
   - Add clipboard, insets, share and back.
   - Internal test track, then the 14-day closed test, then production.
4. **iOS:**
   - An Xcode project and the `staticlib`, using wgpu (Metal).
   - Add clipboard, share, locale and the privacy manifest.
   - TestFlight, then App Review.
5. **CI:** AAB and IPA jobs in `release.yml`, uploading to Play's internal track and to
   TestFlight on each `v*` tag.

## Sources

- eframe documentation and changelog: <https://docs.rs/eframe/latest/eframe/>,
  <https://github.com/emilk/egui/blob/main/crates/eframe/CHANGELOG.md>,
  <https://github.com/emilk/egui/blob/main/CHANGELOG.md>
- eframe's event loop and storage code: <https://github.com/emilk/egui/blob/main/crates/eframe/src/native/run.rs>,
  <https://github.com/emilk/egui/blob/main/crates/eframe/src/native/file_storage.rs>,
  <https://github.com/emilk/egui/blob/main/crates/egui-winit/Cargo.toml>
- egui's Android example: <https://github.com/emilk/egui/tree/main/examples/hello_android>
- winit on Android and iOS: <https://docs.rs/winit/latest/winit/platform/android/index.html>,
  <https://docs.rs/winit/latest/winit/platform/ios/index.html>
- AccessKit: <https://github.com/AccessKit/accesskit>, <https://docs.rs/accesskit_android>
- arboard: <https://docs.rs/crate/arboard/latest>
- Build tools: <https://github.com/rust-mobile/cargo-apk>, <https://github.com/rust-mobile/xbuild>,
  <https://github.com/tauri-apps/cargo-mobile2>
- Google Play: target API level <https://developer.android.com/google/play/requirements/target-sdk>,
  16 KB pages <https://developer.android.com/guide/practices/page-sizes>,
  testing for new personal accounts <https://support.google.com/googleplay/android-developer/answer/14151465>,
  registration <https://support.google.com/googleplay/android-developer/answer/6112435>,
  Data safety <https://support.google.com/googleplay/android-developer/answer/10787469>,
  developer verification <https://developer.android.com/developer-verification>
- Apple: Developer Program <https://developer.apple.com/programs/>,
  upcoming requirements <https://developer.apple.com/news/upcoming-requirements/>,
  required reason APIs <https://developer.apple.com/documentation/bundleresources/describing-use-of-required-reason-api>,
  App Review Guidelines <https://developer.apple.com/app-store/review/guidelines/>

Prices and requirements are as published on 2026-10-18; check the stores' pages again before
signing up or submitting. The work estimates are rough and assume someone who knows this code.
