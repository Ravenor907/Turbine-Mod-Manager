//! The controller pointer, laid out like Steam's desktop controls: the right stick moves a mouse pointer, the left
//! stick is a scroll wheel turned by circling it (clockwise scrolls down, like a wheel rolled toward you), and pushing
//! it straight up or down also scrolls. This file is only the arithmetic: stick positions in, where the pointer is and
//! how many wheel steps to send out. backend.rs reads the sticks (pad.rs) and sends the mouse and wheel events
//! (qtglue.cpp). It needs nothing but the standard library, so the build tests it on its own.

use std::f64::consts::PI;

/// Stick travel ignored around the middle (sticks rest a little off center).
pub const DEADZONE: f64 = 0.14;
/// Full right stick: the pointer crosses the screen's width in about this many seconds.
pub const CROSS_SECS: f64 = 0.85;
/// The pointer hides after this long without the right stick moving.
pub const HIDE_MS: f64 = 3000.0;
/// The left stick counts as on the wheel's rim past this much travel.
pub const RIM: f64 = 0.55;
/// One wheel step for this much turning (twelve steps a full turn).
pub const STEP_RAD: f64 = PI / 6.0;
/// Pushed straight up or down past this much travel, held this long while not turning: scrolls by itself.
pub const PUSH: f64 = 0.65;
pub const PUSH_HOLD_MS: f64 = 220.0;

/// The sticks as -1..1 (x right, y down, the way SDL reports them) and the triggers as 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sticks {
    pub lx: f64,
    pub ly: f64,
    pub rx: f64,
    pub ry: f64,
}

/// SDL's axis value (-32768..32767) as -1..1.
pub fn unit(v: i16) -> f64 {
    (f64::from(v) / 32767.0).clamp(-1.0, 1.0)
}

/// How fast the pointer goes for a stick pushed `m` (0..1) of the way out: slow and precise near the middle, quick
/// at the edge, in screen widths per second.
pub fn speed(m: f64) -> f64 {
    if m <= DEADZONE {
        return 0.0;
    }
    let t = ((m - DEADZONE) / (1.0 - DEADZONE)).clamp(0.0, 1.0);
    // A gentle start (precise aiming on small buttons) that bends up to full speed.
    (0.18 * t + 0.82 * t * t * t) / CROSS_SECS
}

/// What one look at the sticks did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tick {
    /// The pointer moved (or just showed up).
    pub moved: bool,
    /// It was showing and has just hidden.
    pub hid: bool,
    /// Wheel steps to send: positive scrolls down, negative up.
    pub wheel: i32,
}

#[derive(Clone, Debug)]
pub struct Pointer {
    pub x: f64,
    pub y: f64,
    pub shown: bool,
    /// Time since the right stick last moved it.
    idle_ms: f64,
    /// Left stick: the angle at the last look while on the rim, the turning not yet sent as a step, how fast it is
    /// turning (smoothed, radians a second), how long it has been pushed straight, and pushed steps not yet sent.
    angle: Option<f64>,
    turned: f64,
    spin: f64,
    pushed_ms: f64,
    pushed: f64,
    /// Held down by a trigger (keeps the pointer showing while dragging).
    pub holding: bool,
}

impl Default for Pointer {
    fn default() -> Self {
        Pointer { x: -1.0, y: -1.0, shown: false, idle_ms: 0.0, angle: None, turned: 0.0, spin: 0.0, pushed_ms: 0.0, pushed: 0.0, holding: false }
    }
}

fn wrap(a: f64) -> f64 {
    let mut a = a;
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

impl Pointer {
    /// Shows the pointer at this place (the build's screenshot checks, and the first move in a window).
    pub fn place(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
        self.shown = true;
        self.idle_ms = 0.0;
    }

    /// One look at the sticks, `dt_ms` after the last one, in a window `w` x `h`.
    pub fn tick(&mut self, s: &Sticks, dt_ms: f64, w: f64, h: f64) -> Tick {
        let dt_ms = dt_ms.clamp(0.0, 100.0);
        let mut out = Tick::default();
        if self.x < 0.0 || self.y < 0.0 || self.x > w || self.y > h {
            // First time, or the window got smaller: the middle of the window.
            self.x = (w / 2.0).max(0.0);
            self.y = (h / 2.0).max(0.0);
        }
        // Right stick: the pointer.
        let m = s.rx.hypot(s.ry).min(1.0);
        let v = speed(m);
        if v > 0.0 {
            // The same feel on 1280 x 800 and 1920 x 1080 screens: speed is in screen widths.
            let step = v * w.max(1.0) * dt_ms / 1000.0;
            self.x = (self.x + s.rx / m * step).clamp(0.0, (w - 1.0).max(0.0));
            self.y = (self.y + s.ry / m * step).clamp(0.0, (h - 1.0).max(0.0));
            self.idle_ms = 0.0;
            self.shown = true;
            out.moved = true;
        } else {
            self.idle_ms += dt_ms;
            if self.shown && !self.holding && self.idle_ms >= HIDE_MS {
                self.shown = false;
                out.hid = true;
            }
        }
        if self.holding {
            self.idle_ms = 0.0;
        }
        out.wheel = self.wheel(s, dt_ms);
        out
    }

    /// Left stick: steps from circling it, or from pushing it straight up or down.
    fn wheel(&mut self, s: &Sticks, dt_ms: f64) -> i32 {
        let m = s.lx.hypot(s.ly);
        let secs = (dt_ms / 1000.0).max(1e-3);
        let mut steps = 0;
        if m >= RIM {
            // y points down, so the angle grows clockwise as seen on the screen.
            let a = s.ly.atan2(s.lx);
            if let Some(prev) = self.angle {
                let d = wrap(a - prev);
                // A jump of more than a third of a turn in one look is the stick snapping across, not turning.
                if d.abs() < 2.1 {
                    self.turned += d;
                    self.spin = self.spin * 0.8 + (d.abs() / secs) * 0.2;
                }
            }
            self.angle = Some(a);
            while self.turned >= STEP_RAD {
                self.turned -= STEP_RAD;
                steps += 1;
            }
            while self.turned <= -STEP_RAD {
                self.turned += STEP_RAD;
                steps -= 1;
            }
        } else {
            self.angle = None;
            self.turned = 0.0;
            self.spin = 0.0;
        }
        // Pushed straight up or down (within about 25 degrees), not turning: scrolls on its own, faster the further.
        let straight = m >= PUSH && s.lx.abs() <= s.ly.abs() * 0.47;
        if straight && self.spin < 0.8 && steps == 0 {
            self.pushed_ms += dt_ms;
            if self.pushed_ms >= PUSH_HOLD_MS {
                let rate = 4.0 + 14.0 * ((m - PUSH) / (1.0 - PUSH)).clamp(0.0, 1.0);
                self.pushed += rate * secs;
                let whole = self.pushed.floor();
                if whole >= 1.0 {
                    self.pushed -= whole;
                    let n = whole as i32;
                    steps += if s.ly > 0.0 { n } else { -n };
                }
            }
        } else {
            self.pushed_ms = 0.0;
            self.pushed = 0.0;
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f64 = 1280.0;
    const H: f64 = 800.0;

    fn run(p: &mut Pointer, s: Sticks, ms: f64) -> Tick {
        let mut total = Tick::default();
        let mut t = 0.0;
        while t < ms {
            let k = p.tick(&s, 16.0, W, H);
            total.moved |= k.moved;
            total.hid |= k.hid;
            total.wheel += k.wheel;
            t += 16.0;
        }
        total
    }

    #[test]
    fn starts_in_the_middle_hidden_and_shows_on_the_right_stick() {
        let mut p = Pointer::default();
        let t = p.tick(&Sticks::default(), 16.0, W, H);
        assert!(!t.moved && !p.shown);
        assert_eq!((p.x, p.y), (640.0, 400.0));
        let t = p.tick(&Sticks { rx: 1.0, ..Default::default() }, 16.0, W, H);
        assert!(t.moved && p.shown);
        assert!(p.x > 640.0 && p.y == 400.0);
    }

    #[test]
    fn deadzone_and_curve() {
        assert_eq!(speed(0.0), 0.0);
        assert_eq!(speed(DEADZONE), 0.0);
        assert!(speed(0.3) > 0.0);
        // Slow near the middle, much faster at the edge, and always growing.
        assert!(speed(1.0) > speed(0.4) * 6.0);
        let mut last = 0.0;
        for i in 15..=100 {
            let v = speed(i as f64 / 100.0);
            assert!(v > last);
            last = v;
        }
        // A resting stick a little off center doesn't move it.
        let mut p = Pointer::default();
        p.place(100.0, 100.0);
        run(&mut p, Sticks { rx: 0.1, ry: -0.08, ..Default::default() }, 500.0);
        assert_eq!((p.x, p.y), (100.0, 100.0));
    }

    #[test]
    fn full_stick_crosses_the_screen_in_under_a_second_on_both_sizes() {
        for (w, h) in [(1280.0, 800.0), (1920.0, 1080.0)] {
            let mut p = Pointer::default();
            p.place(0.0, h / 2.0);
            let mut ms = 0.0;
            while p.x < w - 1.0 && ms < 5000.0 {
                p.tick(&Sticks { rx: 1.0, ..Default::default() }, 16.0, w, h);
                ms += 16.0;
            }
            assert!((700.0..1000.0).contains(&ms), "{w}: {ms} ms");
            // And stays on the screen.
            p.tick(&Sticks { rx: 1.0, ry: 1.0, ..Default::default() }, 16.0, w, h);
            assert!(p.x <= w - 1.0 && p.y <= h - 1.0);
        }
    }

    #[test]
    fn hides_after_three_seconds_unless_held() {
        let mut p = Pointer::default();
        p.place(10.0, 10.0);
        assert!(!run(&mut p, Sticks::default(), 2900.0).hid);
        assert!(p.shown);
        assert!(run(&mut p, Sticks::default(), 200.0).hid);
        assert!(!p.shown);
        p.place(10.0, 10.0);
        p.holding = true;
        run(&mut p, Sticks::default(), 5000.0);
        assert!(p.shown, "dragging keeps it showing");
    }

    fn circle(p: &mut Pointer, clockwise: bool, turns: f64) -> i32 {
        let steps = (turns * 60.0) as usize; // a turn in about a second
        let mut wheel = 0;
        for i in 0..=steps {
            let a = (i as f64 / 60.0) * 2.0 * PI * if clockwise { 1.0 } else { -1.0 } - PI / 2.0;
            wheel += p.tick(&Sticks { lx: a.cos() * 0.95, ly: a.sin() * 0.95, ..Default::default() }, 16.0, W, H).wheel;
        }
        wheel
    }

    #[test]
    fn circling_the_left_stick_scrolls() {
        let mut p = Pointer::default();
        // Clockwise as seen on the screen (up, right, down, left): down, twelve steps a turn.
        assert_eq!(circle(&mut p, true, 1.0), 12);
        p.tick(&Sticks::default(), 16.0, W, H);
        assert_eq!(circle(&mut p, false, 2.0), -24);
        p.tick(&Sticks::default(), 16.0, W, H);
        // Half a step's turning then letting go sends nothing.
        assert_eq!(circle(&mut p, true, 1.0 / 24.0), 0);
        assert!(!p.shown, "the left stick never shows the pointer");
    }

    #[test]
    fn pushing_straight_scrolls_after_a_moment() {
        let mut p = Pointer::default();
        let down = Sticks { ly: 1.0, ..Default::default() };
        assert_eq!(run(&mut p, down, 200.0).wheel, 0, "a quick flick doesn't scroll");
        let n = run(&mut p, down, 1000.0).wheel;
        assert!((14..=20).contains(&n), "{n} steps a second at full push");
        let mut p = Pointer::default();
        let up = Sticks { ly: -0.75, lx: 0.1, ..Default::default() };
        let n = run(&mut p, up, 1200.0).wheel;
        assert!((-10..=-4).contains(&n), "{n}");
        // Pushed sideways doesn't scroll.
        let mut p = Pointer::default();
        assert_eq!(run(&mut p, Sticks { lx: 1.0, ..Default::default() }, 1000.0).wheel, 0);
    }

    #[test]
    fn sdl_values() {
        assert_eq!(unit(32767), 1.0);
        assert_eq!(unit(-32768), -1.0);
        assert_eq!(unit(0), 0.0);
    }
}
