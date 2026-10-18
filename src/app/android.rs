//! The Android app's ties to the system, through JNI where only Java can tell: the parts of
//! the screen under the status bar, the navigation bar and a camera cutout (NativeActivity
//! draws under them), the user's languages, and the back button.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui::{Context, Event, Key, RawInput, SafeAreaInsets, epaint::MarginF32};
use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use winit::platform::android::activity::AndroidApp;

/// The app's activity, which `android_main` gets from Android.
static APP: OnceLock<AndroidApp> = OnceLock::new();

/// Keeps the activity for the JNI calls (`android_main` calls this first).
pub(crate) fn init(app: &AndroidApp) {
    APP.set(app.clone()).ok();
}

/// Runs `f` with this thread's JNI environment and the app's activity, in a local reference
/// frame: the app's thread stays attached to Java, so its local references would pile up.
/// The first failure goes to the log, with the Java exception if there is one.
fn with_activity<T>(f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<T>) -> Option<T> {
    static FAILED: AtomicBool = AtomicBool::new(false);
    let app = APP.get()?;
    // SAFETY: android-activity's JavaVM and activity live as long as the app. The activity
    // (the NativeActivity, not ndk-context's Application) is a global reference that
    // android-activity owns, and a `JObject` never deletes it.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
    let mut env = vm.attach_current_thread_permanently().ok()?;
    let result = env.with_local_frame(32, |env| f(env, &activity));
    let first = result.is_err() && !FAILED.swap(true, Ordering::Relaxed);
    // A Java exception would stay pending and break the next JNI call.
    if env.exception_check().unwrap_or(false) {
        if first {
            env.exception_describe().ok();
        }
        env.exception_clear().ok();
    }
    result
        .inspect_err(|error| {
            if first {
                log::warn!("JNI call failed: {error}");
            }
        })
        .ok()
}

/// Android's version (`Build.VERSION.SDK_INT`).
fn sdk_version(env: &mut JNIEnv) -> jni::errors::Result<i32> {
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
        .i()
}

/// How far the system bars and a display cutout reach into the window, in pixels (left, top,
/// right, bottom). NativeActivity draws on the whole window's surface, under the bars, on
/// every Android version, whatever space its content view leaves them.
fn covered_edges() -> Option<[i32; 4]> {
    with_activity(|env, activity| {
        let window = env
            .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?
            .l()?;
        let decor = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?
            .l()?;
        let insets = env
            .call_method(
                &decor,
                "getRootWindowInsets",
                "()Landroid/view/WindowInsets;",
                &[],
            )?
            .l()?;
        if insets.is_null() {
            return Ok(None);
        }
        let insets = if sdk_version(env)? >= 30 {
            let kind = |env: &mut JNIEnv, name| {
                env.call_static_method("android/view/WindowInsets$Type", name, "()I", &[])?
                    .i()
            };
            let kinds = kind(env, "systemBars")? | kind(env, "displayCutout")?;
            let edges = env
                .call_method(
                    &insets,
                    "getInsets",
                    "(I)Landroid/graphics/Insets;",
                    &[JValue::Int(kinds)],
                )?
                .l()?;
            let mut edge = |name| env.get_field(&edges, name, "I")?.i();
            [edge("left")?, edge("top")?, edge("right")?, edge("bottom")?]
        } else {
            let mut edge = |name| env.call_method(&insets, name, "()I", &[])?.i();
            [
                edge("getSystemWindowInsetLeft")?,
                edge("getSystemWindowInsetTop")?,
                edge("getSystemWindowInsetRight")?,
                edge("getSystemWindowInsetBottom")?,
            ]
        };
        Ok(Some(insets))
    })
    .flatten()
}

/// Called with every frame's input: tells egui which parts of the screen the system bars
/// cover (egui's safe area, which the layout keeps clear), and handles the back button.
/// While the app has something to close (`back_closes`: a menu, popup, flyout or the
/// Settings page), back is Esc, which closes it; otherwise it leaves the app, as on the first
/// screen of any Android app (Esc would clear the calculator, as in Windows).
pub(super) fn input_hook(ctx: &Context, raw_input: &mut RawInput, back_closes: bool) {
    if let Some([left, top, right, bottom]) = covered_edges() {
        let pixels_per_point =
            raw_input.viewport().native_pixels_per_point.unwrap_or(1.0) * ctx.zoom_factor();
        let points = |pixels: i32| pixels as f32 / pixels_per_point;
        raw_input.safe_area_insets = Some(SafeAreaInsets(MarginF32 {
            left: points(left),
            right: points(right),
            top: points(top),
            bottom: points(bottom),
        }));
    }
    let mut leave = false;
    raw_input.events.retain_mut(|event| match event {
        Event::Key { key, pressed, .. } if *key == Key::BrowserBack => {
            if back_closes {
                *key = Key::Escape;
                return true;
            }
            leave |= *pressed;
            false
        }
        _ => true,
    });
    if leave {
        move_task_to_back();
    }
}

/// Sends the app to the background, as the back button does on an app's first screen.
fn move_task_to_back() {
    with_activity(|env, activity| {
        env.call_method(activity, "moveTaskToBack", "(Z)Z", &[JValue::Bool(1)])?;
        Ok(())
    });
}

/// The user's languages, most preferred first (`LocaleList`, or the default `Locale` before
/// Android 7).
pub(super) fn languages() -> Vec<String> {
    with_activity(|env, _| {
        let tags = if sdk_version(env)? >= 24 {
            let list = env
                .call_static_method(
                    "android/os/LocaleList",
                    "getDefault",
                    "()Landroid/os/LocaleList;",
                    &[],
                )?
                .l()?;
            env.call_method(&list, "toLanguageTags", "()Ljava/lang/String;", &[])?
                .l()?
        } else {
            let locale = env
                .call_static_method(
                    "java/util/Locale",
                    "getDefault",
                    "()Ljava/util/Locale;",
                    &[],
                )?
                .l()?;
            env.call_method(&locale, "toLanguageTag", "()Ljava/lang/String;", &[])?
                .l()?
        };
        let tags: String = env.get_string(&JString::from(tags))?.into();
        Ok(tags.split(',').map(str::to_string).collect())
    })
    .unwrap_or_default()
}
