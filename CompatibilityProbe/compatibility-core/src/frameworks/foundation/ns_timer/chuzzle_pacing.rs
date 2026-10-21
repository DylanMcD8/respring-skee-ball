//! Chuzzle 1.0-specific render/update separation. Original code stays translated.
//!
//! drawView calls AppInterface::Update(float), which discards its argument and
//! dispatches to App::BaseUpdate through ChuzzleApp's first vtable slot. On an
//! intermediate render we temporarily select the original no-op App::Update.
//! The rest of drawView (including drawing and presentation) runs unchanged.
//! Guard every address against the known executable; never patch instructions.
use super::chuzzle_clock::LogicClock;
use crate::mem::{ConstPtr, Mem, MutPtr};
use std::time::{Duration, Instant};

const APP_GLOBAL: u32 = 0xbce70;
const VTABLE: u32 = 0xb6da0;
const UPDATE: u32 = 0x782a0;
const NOOP: u32 = 0x7961c;

pub struct Pacing {
    clock: LogicClock,
    high_refresh: bool,
    diagnostics: bool,
    sample_start: Instant,
    renders: u32,
    updates: u32,
}

pub struct Frame {
    counter: ConstPtr<u32>,
    before: u32,
    update: bool,
}

fn read(mem: &Mem, address: u32) -> u32 {
    mem.read(ConstPtr::<u32>::from_bits(address))
}

impl Pacing {
    pub fn new(fps: f64) -> Self {
        Self { clock: LogicClock::default(), high_refresh: fps > 60.0,
            diagnostics: std::env::var("CHUZZLE_RUNTIME_DIAGNOSTICS").as_deref() == Ok("1"),
            sample_start: Instant::now(), renders: 0, updates: 0 }
    }

    pub fn begin(&mut self, mem: &mut Mem) -> Frame {
        assert_eq!(read(mem, UPDATE), 0xe92d4090, "Chuzzle BaseUpdate signature");
        assert_eq!(read(mem, NOOP), 0xe12fff1e, "Chuzzle no-op signature");
        assert_eq!(read(mem, VTABLE), UPDATE, "Chuzzle update vtable slot");
        assert_eq!(read(mem, VTABLE + 4), 0x77a60, "Chuzzle draw vtable slot");
        let app = read(mem, APP_GLOBAL);
        assert_ne!(app, 0, "Chuzzle app initialized before drawView");
        assert_eq!(read(mem, app), VTABLE, "Chuzzle app vtable");
        let counter = ConstPtr::<u32>::from_bits(app + 0x1ac);
        let before = mem.read(counter);
        let update = !self.high_refresh || self.clock.should_update(Instant::now());
        if !update { mem.write(MutPtr::<u32>::from_bits(VTABLE), NOOP); }
        Frame { counter, before, update }
    }

    pub fn finish(&mut self, mem: &mut Mem, frame: Frame) {
        // Restore before checking/logging, including if the callback misbehaved.
        mem.write(MutPtr::<u32>::from_bits(VTABLE), UPDATE);
        let actual_updates = mem.read(frame.counter).wrapping_sub(frame.before);
        assert_eq!(actual_updates, u32::from(frame.update), "Unexpected Chuzzle simulation tick count");
        if !self.diagnostics { return; }
        self.renders += 1;
        self.updates += actual_updates;
        let now = Instant::now();
        let elapsed = now.duration_since(self.sample_start);
        if elapsed >= Duration::from_secs(5) {
            echo!("[CHUZZLE-FRAME-PACING] elapsed={:.3} renders={} updates={} render_hz={:.2} update_hz={:.2}",
                elapsed.as_secs_f64(), self.renders, self.updates,
                f64::from(self.renders) / elapsed.as_secs_f64(),
                f64::from(self.updates) / elapsed.as_secs_f64());
            self.sample_start = now;
            self.renders = 0;
            self.updates = 0;
        }
    }
}
