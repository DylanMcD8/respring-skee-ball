#import <UIKit/UIKit.h>
#import <QuartzCore/QuartzCore.h>
#include <stdlib.h>

// This display link requests ProMotion and measures the OS callback cadence.
// Game rendering still uses the original render path; this callback draws nothing.
@interface ChuzzleDisplayTiming : NSObject
@property(nonatomic, strong) CADisplayLink *link;
@property(nonatomic, strong) NSFileHandle *log;
@property(nonatomic) CFTimeInterval sampleStart;
@property(nonatomic) NSUInteger callbacks;
@end

static ChuzzleDisplayTiming *monitor;

void ChuzzleFrameTimingSetActive(BOOL active) {
    monitor.link.paused = !active;
    monitor.sampleStart = 0;
    monitor.callbacks = 0;
}

@implementation ChuzzleDisplayTiming
- (void)tick:(CADisplayLink *)link {
    CFTimeInterval now = CACurrentMediaTime();
    if (self.sampleStart == 0) {
        self.sampleStart = now;
        return;
    }
    self.callbacks += 1;
    CFTimeInterval elapsed = now - self.sampleStart;
    if (elapsed < 5.0) return;
    CFTimeInterval scheduled = link.targetTimestamp - link.timestamp;
    NSString *line = [NSString stringWithFormat:
        @"elapsed=%.3f callbacks=%lu callback_hz=%.2f scheduled_hz=%.2f low_power=%d\n",
        elapsed, (unsigned long)self.callbacks, self.callbacks / elapsed,
        scheduled > 0 ? 1.0 / scheduled : 0,
        NSProcessInfo.processInfo.lowPowerModeEnabled];
    // Diagnostic I/O must never terminate gameplay; bound this session log.
    @try {
        if (self.log.offsetInFile < 256 * 1024) {
            [self.log writeData:[line dataUsingEncoding:NSUTF8StringEncoding]];
        } else {
            [self.log closeFile];
            self.log = nil;
        }
    } @catch (NSException *exception) { self.log = nil; }
    self.sampleStart = now;
    self.callbacks = 0;
}
@end

void ChuzzleConfigureFrameTiming(NSString *documents) {
    NSInteger maximum = UIScreen.mainScreen.maximumFramesPerSecond;
    NSInteger requested = MIN(60, maximum);
    // Explicit overrides allow baseline comparisons and 120 Hz logic tests on
    // a simulator whose display may report only 60 Hz. Default respects hardware.
    const char *override = getenv("CHUZZLE_TARGET_FPS");
    if (override && strcmp(override, "60") == 0) requested = MIN(60, maximum);
    if (override && strcmp(override, "120") == 0) requested = 120;
    requested = MAX(60, requested);
    NSString *fps = [NSString stringWithFormat:@"%ld", (long)requested];
    setenv("CHUZZLE_TARGET_FPS", fps.UTF8String, 1);

    const char *diagnostics = getenv("CHUZZLE_RUNTIME_DIAGNOSTICS");
    if (!diagnostics || strcmp(diagnostics, "1") != 0) return;

    NSString *path = [documents stringByAppendingPathComponent:@"display-timing.log"];
    NSString *header = [NSString stringWithFormat:
        @"requested_fps=%ld screen_max_fps=%ld low_power=%d\n"
         "CADisplayLink callbacks and scheduled intervals; not proof of physical scanout.\n",
        (long)requested, (long)maximum, NSProcessInfo.processInfo.lowPowerModeEnabled];
    [header writeToFile:path atomically:YES encoding:NSUTF8StringEncoding error:nil];
    monitor = [ChuzzleDisplayTiming new];
    monitor.log = [NSFileHandle fileHandleForWritingAtPath:path];
    @try { [monitor.log seekToEndOfFile]; }
    @catch (NSException *exception) { monitor.log = nil; }
    monitor.link = [CADisplayLink displayLinkWithTarget:monitor selector:@selector(tick:)];
    NSInteger displayRate = MIN(requested, maximum);
    monitor.link.preferredFrameRateRange = CAFrameRateRangeMake(MIN(60, displayRate), displayRate, displayRate);
    [monitor.link addToRunLoop:NSRunLoop.mainRunLoop forMode:NSRunLoopCommonModes];
}
