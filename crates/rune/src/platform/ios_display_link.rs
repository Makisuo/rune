//! iOS's display link: while motion runs, each display refresh wakes the event loop for the next
//! frame, at the screen's rate (120 Hz on ProMotion, once the app sets
//! `CADisableMinimumFrameDurationOnPhone`), where a 60 Hz timer ignored the display.
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use objc2_foundation_video::{NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
use objc2_video::rc::Retained;
use objc2_video::runtime::NSObject;
use objc2_video::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};

/// What a refresh does: wakes the loop once (a tick waiting to be drawn absorbs the next).
struct Ticks {
    wake: crate::runtime::Waker,
    pending: AtomicBool,
    /// Seconds between refreshes, as the link last reported.
    period: Cell<f64>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements; the class does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RuneDisplayLinkTicks"]
    #[ivars = Ticks]
    struct Ticker;

    impl Ticker {
        #[unsafe(method(tick:))]
        fn tick(&self, link: &CADisplayLink) {
            let ticks = self.ivars();
            let period = link.targetTimestamp() - link.timestamp();
            if period > 0.0 { ticks.period.set(period); }
            if !ticks.pending.swap(true, Ordering::AcqRel) { (ticks.wake)(); }
        }
    }
);

pub(crate) struct DisplayLink {
    link: Retained<CADisplayLink>,
    ticker: Retained<Ticker>,
    running: bool,
    /// When the last frame drew while the link ran.
    last_frame: Option<Instant>,
}

impl DisplayLink {
    /// A paused link on the main run loop; `None` off the main thread.
    pub(crate) fn new(wake: crate::runtime::Waker) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let ticks = Ticks {
            wake,
            pending: AtomicBool::new(false),
            period: Cell::new(1.0 / 60.0),
        };
        let ticker: Retained<Ticker> =
            unsafe { msg_send![super(Ticker::alloc(mtm).set_ivars(ticks)), init] };
        // SAFETY: `tick:` takes the link, as a display link's selector must.
        let link = unsafe { CADisplayLink::displayLinkWithTarget_selector(&ticker, sel!(tick:)) };
        // Motion asks for the screen's fastest rate; a 60 Hz screen caps it there.
        link.setPreferredFrameRateRange(CAFrameRateRange::new(60.0, 120.0, 120.0));
        link.setPaused(true);
        // SAFETY: the main run loop, on the main thread; common modes keep it ticking while a
        // touch tracks.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        Some(Self {
            link,
            ticker,
            running: false,
            last_frame: None,
        })
    }

    pub(crate) fn running(&self) -> bool {
        self.running
    }

    pub(crate) fn refresh_hz(&self) -> f64 {
        1.0 / self.ticker.ivars().period.get()
    }

    /// Starts ticking if paused; it always can.
    pub(crate) fn start(&mut self) -> bool {
        if !self.running {
            self.link.setPaused(false);
            self.running = true;
        }
        true
    }

    /// Whether a frame may draw now, claiming the tick that woke it: one did, or the link is
    /// paused, or the last frame drew at least half a refresh ago. Otherwise (a touch's redraw
    /// right after a tick's frame) the next tick draws it, so a refresh never draws two frames.
    pub(crate) fn claim(&mut self, now: Instant) -> bool {
        let ticks = self.ticker.ivars();
        let ticked = ticks.pending.swap(false, Ordering::AcqRel);
        let half = Duration::from_secs_f64(ticks.period.get() / 2.0);
        let rested = self.last_frame.is_none_or(|last| now.saturating_duration_since(last) >= half);
        let draws = ticked || !self.running || rested;
        if draws && self.running {
            self.last_frame = Some(now);
        }
        draws
    }

    pub(crate) fn stop(&mut self) {
        if self.running {
            self.link.setPaused(true);
            self.running = false;
        }
        self.ticker.ivars().pending.store(false, Ordering::Release);
        self.last_frame = None;
    }
}

impl Drop for DisplayLink {
    // The link retains its target until invalidated.
    fn drop(&mut self) {
        self.link.invalidate();
    }
}
