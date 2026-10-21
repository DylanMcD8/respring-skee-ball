#include "render_scale.h"
#include <SDL.h>
#include <SDL_main.h>
#include <stdlib.h>
#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>
#import <UIKit/UIKit.h>
#import <OpenAL/alc.h>

#import <OpenGLES/EAGL.h>
#import <objc/runtime.h>

// SDL owns one drawable view per context. Its window can be created before
// UIKit connects the scene; attach it once a foreground scene is available.
@interface EAGLContext (SkeeSDLDrawable)
@property(nonatomic, readonly) UIView *sdlView;
@end
void SkeeEnsureDrawableScene(void) {
    EAGLContext *context=EAGLContext.currentContext;
    if (![context respondsToSelector:@selector(sdlView)]) return;
    static UIWindow *drawableWindow;
    UIView *view=context.sdlView;
    UIWindow *window=view.window;
    if (window) drawableWindow=window;
    else if (drawableWindow && view) {
        // SDL keeps multiple contexts associated with one window, but creating
        // a guest context replaces the compositor view without clearing the
        // old view's SDL association. Restore the view that is presenting.
        drawableWindow.rootViewController.view=view;
        window=drawableWindow;
    }
    if (!window || window.windowScene) return;
    for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
        if (![scene isKindOfClass:UIWindowScene.class] ||
            scene.activationState == UISceneActivationStateBackground ||
            scene.activationState == UISceneActivationStateUnattached) continue;
        UIWindowScene *windowScene=(UIWindowScene *)scene;
        window.windowScene=windowScene;
        id<UIWindowSceneDelegate> delegate=(id)scene.delegate;
        if ([delegate respondsToSelector:@selector(window)] && delegate.window != window)
            delegate.window.hidden=YES;
        if ([delegate respondsToSelector:@selector(setWindow:)]) delegate.window=window;
        [window makeKeyAndVisible];
        if (@available(iOS 27.1,*)) {
            // Duo inner displays resize independently of physical orientation.
            [window.rootViewController setNeedsUpdateOfSupportedInterfaceOrientations];
        } else if (@available(iOS 16.0,*)) {
            [window.rootViewController setNeedsUpdateOfSupportedInterfaceOrientations];
            UIWindowSceneGeometryPreferencesIOS *geometry=[[UIWindowSceneGeometryPreferencesIOS alloc]
                initWithInterfaceOrientations:UIInterfaceOrientationMaskPortrait];
            [windowScene requestGeometryUpdateWithPreferences:geometry errorHandler:^(NSError *error) {
                NSLog(@"Skee-Ball portrait orientation: %@",error);
            }];
        }
        break;
    }
}

static BOOL (*skeeOriginalPresent)(id, SEL, NSUInteger);
static BOOL SkeePresent(id context, SEL selector, NSUInteger target) {
    SkeeEnsureDrawableScene();
    if (@available(iOS 27.1,*)) {
        if ([context respondsToSelector:@selector(sdlView)]) {
            UIView *view = [context sdlView];
            UIWindow *window = view.window;
            CGRect bounds = window.windowScene.coordinateSpace.bounds;
            if (window && bounds.size.width > 0 && bounds.size.height > 0) {
                if (!CGRectEqualToRect(window.frame, bounds)) window.frame = bounds;
                if (!CGSizeEqualToSize(view.bounds.size, window.bounds.size)) {
                    view.bounds = (CGRect){CGPointZero, window.bounds.size};
                    view.center = CGPointMake(CGRectGetMidX(window.bounds), CGRectGetMidY(window.bounds));
                    [view setNeedsLayout];
                    NSLog(@"Skee-Ball drawable resized: %.0fx%.0f", view.bounds.size.width, view.bounds.size.height);
                }
            }
        }
    }
    return skeeOriginalPresent(context, selector, target);
}

extern void ChuzzleFrameTimingSetActive(BOOL active);
static UIBackgroundTaskIdentifier backgroundSave;

void ChuzzleFinishBackgroundSave(void) {
    if (backgroundSave == UIBackgroundTaskInvalid) return;
    UIBackgroundTaskIdentifier task = backgroundSave;
    backgroundSave = UIBackgroundTaskInvalid;
    [UIApplication.sharedApplication endBackgroundTask:task];
}

void ChuzzleHostAudioActive(int active) {
    static ALCcontext *pausedContext;
    NSError *error = nil;
    // iOS OpenAL must release its current context across audio-session suspension.
    if (!active) {
        pausedContext = alcGetCurrentContext();
        alcMakeContextCurrent(NULL);
        if (pausedContext) alcSuspendContext(pausedContext);
    }
    [AVAudioSession.sharedInstance setActive:(active != 0)
        withOptions:active ? 0 : AVAudioSessionSetActiveOptionNotifyOthersOnDeactivation error:&error];
    if (active && pausedContext) {
        alcMakeContextCurrent(pausedContext);
        alcProcessContext(pausedContext);
        pausedContext = NULL;
    }
    if (error) NSLog(@"Skee-Ball audio lifecycle: %@", error);
}

@interface ChuzzleSceneDelegate : NSObject <UIWindowSceneDelegate>
@property (nonatomic, strong) UIWindow *window;
@end

@implementation ChuzzleSceneDelegate
- (void)scene:(UIScene *)scene willConnectToSession:(UISceneSession *)session options:(UISceneConnectionOptions *)options {
    if (![scene isKindOfClass:UIWindowScene.class]) return;
    self.window = [[UIWindow alloc] initWithWindowScene:(UIWindowScene *)scene];
    self.window.rootViewController = [UIViewController new];
    self.window.rootViewController.view.backgroundColor = UIColor.blackColor;
    [self.window makeKeyAndVisible];
    NSLog(@"Chuzzle Compatibility scene connected");
}
- (void)sceneWillResignActive:(UIScene *)scene {
    ChuzzleFrameTimingSetActive(NO);
    id<UIApplicationDelegate> delegate = UIApplication.sharedApplication.delegate;
    if ([delegate respondsToSelector:@selector(applicationWillResignActive:)])
        [delegate applicationWillResignActive:UIApplication.sharedApplication];
}
- (void)sceneDidBecomeActive:(UIScene *)scene {
    ChuzzleFrameTimingSetActive(YES);
    id<UIApplicationDelegate> delegate = UIApplication.sharedApplication.delegate;
    if ([delegate respondsToSelector:@selector(applicationDidBecomeActive:)])
        [delegate applicationDidBecomeActive:UIApplication.sharedApplication];
}
- (void)sceneDidEnterBackground:(UIScene *)scene {
    if (backgroundSave == UIBackgroundTaskInvalid) {
        backgroundSave = [UIApplication.sharedApplication beginBackgroundTaskWithName:@"Save Chuzzle"
            expirationHandler:^{ ChuzzleFinishBackgroundSave(); }];
    }
    id<UIApplicationDelegate> delegate = UIApplication.sharedApplication.delegate;
    if ([delegate respondsToSelector:@selector(applicationDidEnterBackground:)])
        [delegate applicationDidEnterBackground:UIApplication.sharedApplication];
}
- (void)sceneWillEnterForeground:(UIScene *)scene {
    id<UIApplicationDelegate> delegate = UIApplication.sharedApplication.delegate;
    if ([delegate respondsToSelector:@selector(applicationWillEnterForeground:)])
        [delegate applicationWillEnterForeground:UIApplication.sharedApplication];
}
@end

extern void ChuzzleConfigureFrameTiming(NSString *documents);

extern int chuzzle_ios_run(const char *ipa, const char *resources, const char *documents);

int SDL_main(int argc, char *argv[]) {
    @autoreleasepool {
        // This workbench runs original ARM code; no Skee-Ball AOT translation yet.
        SDL_SetHint(SDL_HINT_IOS_HIDE_HOME_INDICATOR, "2");
        setenv("CHUZZLE_EXECUTION", "interpreter", 0);
        setenv("CHUZZLE_AOT_STRICT", "0", 0);
        setenv("CHUZZLE_AOT_RUNTIME", "0", 0);
        // Settings.app preferences are host preferences, separate from guest saves.
        NSUserDefaults *settings = NSUserDefaults.standardUserDefaults;
        [settings registerDefaults:@{ @"AllowFullResolution": @YES }];
        BOOL allowFullResolution = [settings boolForKey:@"AllowFullResolution"];
        setenv("CHUZZLE_RENDER_SCALE", allowFullResolution ? "4" : "2", 0);
        setenv("SKEEBALL_FULL_HEIGHT", allowFullResolution ? "1" : "0", 0);
        setenv("SKEEBALL_TEXTURE_DRAWABLE", "1", 0);
        setenv("SKEEBALL_RUNLOOP_POLL_MS", "2", 0);
        // Detailed frame metrics are opt-in for diagnostic launches.
        setenv("CHUZZLE_RUNTIME_DIAGNOSTICS", "0", 0);
        NSError *error = nil;
        AVAudioSession *audio = [AVAudioSession sharedInstance];
        [audio setCategory:AVAudioSessionCategoryAmbient error:&error];
        [audio setActive:YES error:&error];
        if (error) NSLog(@"Audio session: %@", error);
        NSString *resources = NSBundle.mainBundle.resourcePath;
        NSString *ipa = [resources stringByAppendingPathComponent:@"Skee-Ball.ipa"];
        NSString *documents = NSSearchPathForDirectoriesInDomains(NSDocumentDirectory, NSUserDomainMask, YES).firstObject;
        Method present = class_getInstanceMethod(EAGLContext.class, @selector(presentRenderbuffer:));
        skeeOriginalPresent = (void *)method_setImplementation(present, (IMP)SkeePresent);
        ChuzzleConfigureFrameTiming(documents);
        CGSize pixels = UIScreen.mainScreen.nativeBounds.size;
        if (@available(iOS 27.1,*)) {
            for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
                if ([scene isKindOfClass:UIWindowScene.class] && scene.activationState != UISceneActivationStateBackground) {
                    UIWindowScene *ws = (UIWindowScene *)scene;
                    CGSize size = ws.coordinateSpace.bounds.size;
                    if (size.width > 0 && size.height > 0) pixels = CGSizeMake(size.width * ws.screen.scale, size.height * ws.screen.scale);
                    break;
                }
            }
        }
        double width = pixels.width, height = pixels.height;
        const char *fullHeight = getenv("SKEEBALL_FULL_HEIGHT");
        if (!fullHeight || strcmp(fullHeight, "0") != 0) {
            double fit = fmin(width / 320.0, height / 480.0);
            unsigned worldWidth = (unsigned)lround(width / fit);
            unsigned worldHeight = (unsigned)lround(height / fit);
            setenv("SKEEBALL_WORLD_WIDTH", [NSString stringWithFormat:@"%u", worldWidth].UTF8String, 0);
            setenv("SKEEBALL_WORLD_HEIGHT", [NSString stringWithFormat:@"%u", worldHeight].UTF8String, 0);
            NSLog(@"Skee-Ball world: %ux%u, UI 320x480", worldWidth, worldHeight);
        }
        if (strcmp(getenv("CHUZZLE_RUNTIME_DIAGNOSTICS"), "1") == 0) {
            NSDictionary *renderInfo = @{ @"nativeWidth": @(width), @"nativeHeight": @(height),
                @"screenScale": @(UIScreen.mainScreen.scale),
                @"renderScale": @(atoi(getenv("CHUZZLE_RENDER_SCALE"))) };
            [renderInfo writeToFile:[documents stringByAppendingPathComponent:@"rendering-info.plist"] atomically:YES];
        }
        return chuzzle_ios_run(ipa.fileSystemRepresentation, resources.fileSystemRepresentation, documents.fileSystemRepresentation);
    }
}

// Keep the host's UIKit entry point explicit instead of depending on an
// entry point pulled indirectly from the Rust static library's SDL objects.
#undef main
int main(int argc, char *argv[]) {
    backgroundSave = UIBackgroundTaskInvalid;
    if (@available(iOS 27.1,*)) {
        // Native host rotates independently of the original portrait game.
        SDL_SetHintWithPriority(SDL_HINT_ORIENTATIONS,
            "Portrait PortraitUpsideDown LandscapeLeft LandscapeRight", SDL_HINT_OVERRIDE);
    }
    return SDL_UIKitRunApp(argc, argv, SDL_main);
}
