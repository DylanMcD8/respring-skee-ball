//! Modern host lifecycle for the original application's foreground-only loop.
use super::*;
use crate::abi::{CallFromHost, GuestFunction};
use crate::frameworks::{audio_toolbox::{audio_queue, audio_unit}, foundation::ns_run_loop, openal};
use crate::mem::ConstPtr;
use crate::objc::msg_send;
use std::time::Instant;

pub fn is_inactive(env: &Environment) -> bool {
    env.framework_state.uikit.ui_application.inactive_since.is_some()
}

fn notify(env: &mut Environment, method: &str, notification: &'static str) {
    let pool: id = msg_class![env; NSAutoreleasePool new];
    let app: id = msg_class![env; UIApplication sharedApplication];
    let delegate: id = msg![env; app delegate];
    if env.objc.object_has_method_named(&env.mem, delegate, method) {
        let selector = env.objc.register_host_selector(method.to_owned(), &mut env.mem);
        let _: () = msg_send(env, (delegate, selector, app));
    }
    let center: id = msg_class![env; NSNotificationCenter defaultCenter];
    let name = get_static_str(env, notification);
    let _: () = msg![env; center postNotificationName:name object:app userInfo:nil];
    let _: () = msg![env; pool drain];
}

fn chuzzle_app(env: &Environment) -> Option<u32> {
    if env.bundle.bundle_identifier() != "com.popcap.Chuzzle" { return None; }
    let app = env.mem.read(ConstPtr::<u32>::from_bits(0xbce70));
    (app != 0 && env.mem.read(ConstPtr::<u32>::from_bits(app)) == 0xb6da0).then_some(app)
}

fn tick(env: &Environment) -> Option<u32> {
    chuzzle_app(env).map(|app| env.mem.read(ConstPtr::<u32>::from_bits(app + 0x1ac)))
}

fn host_audio(env: &mut Environment, active: bool) {
    #[cfg(all(target_os = "ios", feature = "ios-static"))]
    env.on_parent_stack_in_coroutine(move |_, _| {
        extern "C" { fn ChuzzleHostAudioActive(active: i32); }
        unsafe { ChuzzleHostAudioActive(i32::from(active)); }
    });
    #[cfg(not(all(target_os = "ios", feature = "ios-static")))]
    let _ = (env, active);
}

pub fn resign_active(env: &mut Environment) {
    if is_inactive(env) { return; }
    // Set the guard before invoking guest callbacks, which can yield/reenter.
    env.framework_state.uikit.ui_application.inactive_since = Some(Instant::now());
    super::super::ui_touch::cancel_all(env);
    notify(env, "applicationWillResignActive:", UIApplicationWillResignActiveNotification);
    openal::pause_for_lifecycle(env);
    audio_queue::pause_for_lifecycle(env);
    audio_unit::pause_for_lifecycle(env);
    host_audio(env, false);
    env.framework_state.uikit.ui_application.pause_tick = tick(env);
    echo!("[CHUZZLE-LIFECYCLE] paused pid={} tick={:?}", std::process::id(), tick(env));
}

// Keep the previous on-disk game files outside the guest sandbox. This is
// recovery material, not an automatic restore or a change to the game's format.
fn backup_previous_game_files(env: &Environment) {
    if env.bundle.bundle_identifier() != "com.popcap.Chuzzle" { return; }
    let base = crate::paths::user_data_base_path();
    let source = base.join(crate::paths::SANDBOX_DIR).join("com.popcap.Chuzzle").join("Documents");
    let backup = base.join("Chuzzle Save Backups");
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(&backup)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() { continue; }
            let name = entry.file_name();
            let upper = name.to_string_lossy().to_ascii_uppercase();
            if !(upper.ends_with(".DAT") || upper.ends_with(".CFG")) { continue; }
            let bytes = std::fs::read(entry.path())?;
            crate::fs::atomic::replace(&backup.join(name), &bytes)?;
        }
        Ok(())
    })();
    if let Err(error) = result { log!("Previous-save backup unavailable: {error}"); }
}

pub fn enter_background(env: &mut Environment) {
    resign_active(env);
    if env.framework_state.uikit.ui_application.in_background { return; }
    env.framework_state.uikit.ui_application.in_background = true;
    notify(env, "applicationDidEnterBackground:", UIApplicationDidEnterBackgroundNotification);
    backup_previous_game_files(env);
    // Call only the original save routine, never Quit or the termination
    // observer: both destroy state that must survive a temporary interruption.
    if let Some(app) = chuzzle_app(env) {
        let game: MutPtr<u8> = env.mem.read(ConstPtr::from_bits(app + 0x1701c));
        if !game.is_null() {
            assert_eq!(env.mem.read(ConstPtr::<u32>::from_bits(0x4e498)), 0xe92d40f0,
                "Chuzzle GameInterface::Save signature");
            let save = GuestFunction::from_addr_with_thumb_bit(0x4e498);
            let _: () = save.call_from_host(env, (game, ConstPtr::<u8>::null()));
            echo!("[CHUZZLE-LIFECYCLE] original game save completed");
        }
    }
    if !env.is_app_picker {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
        let saved: bool = msg![env; defaults synchronize];
        echo!("[CHUZZLE-LIFECYCLE] background defaults_saved={saved}");
        let _: () = msg![env; pool drain];
    }
    #[cfg(all(target_os = "ios", feature = "ios-static"))]
    env.on_parent_stack_in_coroutine(|_, _| {
        extern "C" { fn ChuzzleFinishBackgroundSave(); }
        unsafe { ChuzzleFinishBackgroundSave(); }
    });
}

pub fn enter_foreground(env: &mut Environment) {
    if !env.framework_state.uikit.ui_application.in_background { return; }
    env.framework_state.uikit.ui_application.in_background = false;
    notify(env, "applicationWillEnterForeground:", UIApplicationWillEnterForegroundNotification);
}

pub fn become_active(env: &mut Environment) {
    if !is_inactive(env) { return; }
    enter_foreground(env);
    let elapsed = env.framework_state.uikit.ui_application.inactive_since.unwrap().elapsed();
    let before = env.framework_state.uikit.ui_application.pause_tick;
    let after = tick(env);
    ns_run_loop::resume_after_pause(env, elapsed);
    host_audio(env, true);
    openal::resume_after_lifecycle(env);
    audio_queue::resume_after_lifecycle(env);
    audio_unit::resume_after_lifecycle(env);
    // Clear the guard only after restoring host resources.
    env.framework_state.uikit.ui_application.inactive_since = None;
    notify(env, "applicationDidBecomeActive:", UIApplicationDidBecomeActiveNotification);
    echo!("[CHUZZLE-LIFECYCLE] resumed pid={} paused_seconds={:.3} tick_before={before:?} tick_after={after:?} frozen={}",
        std::process::id(), elapsed.as_secs_f64(), before == after);
}
