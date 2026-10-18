//! The Android app's ties to the system, through JNI where only Java can tell: the parts of
//! the screen under the status bar, the navigation bar and a camera cutout (Android 15 and
//! later always draw apps under them), the user's languages, and the back button.

use eframe::egui::{Context, Event, Key, RawInput, SafeAreaInsets, epaint::MarginF32};
use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};

/// Runs `f` with this thread's JNI environment and the app's activity, in a local reference
/// frame: the app's thread stays attached to Java, so its local references would pile up.
fn with_activity<T>(f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<T>) -> Option<T> {
    let context = ndk_context::android_context();
    // SAFETY: android-activity sets ndk-context's JavaVM and activity before `android_main`
    // runs, and both live as long as the process. The activity is a global reference that
    // android-activity owns; a `JObject` never deletes it.
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) }.ok()?;
    let activity = unsafe { JObject::from_raw(context.context().cast()) };
    let mut env = vm.attach_current_thread_permanently().ok()?;
    let result = env.with_local_frame(32, |env| f(env, &activity));
    // A Java exception would stay pending and break the next JNI call.
    if env.exception_check().unwrap_or(false) {
        env.exception_clear().ok();
    }
    result.ok()
}

/// Android's version (`Build.VERSION.SDK_INT`).
fn sdk_version(env: &mut JNIEnv) -> jni::errors::Result<i32> {
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
        .i()
}

/// How far the system bars and a display cutout reach into the app's view, in pixels (left,
/// top, right, bottom). Before Android 15 the system usually keeps the view clear of the bars
/// itself, so only the part of the insets that overlaps the view counts.
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
        // The content view (NativeActivity's surface) within the window.
        let id = env.get_static_field("android/R$id", "content", "I")?.i()?;
        let content = env
            .call_method(
                &window,
                "findViewById",
                "(I)Landroid/view/View;",
                &[JValue::Int(id)],
            )?
            .l()?;
        if content.is_null() {
            return Ok(Some(insets));
        }
        let location = env.new_int_array(2)?;
        env.call_method(
            &content,
            "getLocationInWindow",
            "([I)V",
            &[(&location).into()],
        )?;
        let mut position = [0; 2];
        env.get_int_array_region(&location, 0, &mut position)?;
        let mut size = |view: &JObject, name| env.call_method(view, name, "()I", &[])?.i();
        let (window_width, window_height) = (size(&decor, "getWidth")?, size(&decor, "getHeight")?);
        let (width, height) = (size(&content, "getWidth")?, size(&content, "getHeight")?);
        let [left, top] = position;
        let gaps = [
            left,
            top,
            window_width - left - width,
            window_height - top - height,
        ];
        Ok(Some(std::array::from_fn(|i| (insets[i] - gaps[i]).max(0))))
    })
    .flatten()
}

/// Called with every frame's input: tells egui which parts of the screen the system bars
/// cover (egui's safe area, which the layout keeps clear), and turns the back button into
/// Esc, which closes the open menu, popup or flyout.
pub(super) fn input_hook(ctx: &Context, raw_input: &mut RawInput) {
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
    for event in &mut raw_input.events {
        if let Event::Key { key, .. } = event
            && *key == Key::BrowserBack
        {
            *key = Key::Escape;
        }
    }
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
