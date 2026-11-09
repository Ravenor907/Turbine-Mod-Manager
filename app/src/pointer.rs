//! The controller pointer and the left stick, laid out like Steam's desktop controls: the right stick moves a mouse
//! pointer, and the left stick is either a D-pad or a scroll wheel. Pushed out and held in one direction it presses
//! that direction (with the D-pad's repeat, done in Main.qml); swept around the rim it turns a scroll wheel
//! (clockwise scrolls down, like a wheel rolled toward you). This file is only the arithmetic: stick positions in,
//! where the pointer is, which direction is pressed and how many wheel notches to send out. backend.rs reads the
//! sticks (pad.rs) and sends the presses, mouse and wheel events (qtglue.cpp). It needs nothing but the standard
//! library, so the build tests it on its own.

use std::f64::consts::PI;

/// Stick travel ignored around the middle (sticks rest a little off center).
pub const DEADZONE: f64 = 0.14;
/// Full right stick: the pointer crosses the screen's width in about this many seconds.
pub const CROSS_SECS: f64 = 0.85;
/// The pointer hides after this long without the right stick moving.
pub const HIDE_MS: f64 = 3000.0;
/// Left stick: past this much travel it counts as pushed; under `LET_GO` it is back in the middle again.
pub const PUSH_ON: f64 = 0.5;
pub const LET_GO: f64 = 0.35;
/// A push that stays within `CONE` of where it started for `COMMIT_MS` is one direction press.
pub const COMMIT_MS: f64 = 80.0;
pub const CONE: f64 = 25.0 * PI / 180.0;
/// Swept this far from where it started before the press, it is the scroll wheel instead. A push that drifts less
/// than this (a curved thumb) is still a press, in the direction it points when `COMMIT_MS` is up.
pub const SWEEP: f64 = 40.0 * PI / 180.0;
/// Once a direction is held, a wobble doesn't scroll: the wheel needs this much turning from where the push began.
pub const HELD_SWEEP: f64 = SWEEP + NOTCH;
/// Held, then snapped across the middle in one look (up to down along the rim): a new press, not the wheel.
pub const SNAP: f64 = 135.0 * PI / 180.0;
/// While only directions count (the quick wheel), a held push moves on to the next direction this far from the
/// held one's axis (45 degrees plus a margin, so the line between two never flickers).
pub const DPAD_TURN: f64 = 55.0 * PI / 180.0;
/// One wheel notch (three rows) for this much turning.
pub const NOTCH: f64 = 30.0 * PI / 180.0;

/// The sticks as -1..1 (x right, y down, the way SDL reports them) and the triggers as 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sticks {
    pub lx: f64,
    pub ly: f64,
    pub rx: f64,
    pub ry: f64,
}

/// The quick wheel's wedge angle for the left stick: degrees from Up, clockwise (0..359), or None while the stick
/// is near the middle (pushed less than half way). The wheel keeps the wedge last picked; a centred stick reads -1,
/// so pushing at the same wedge again (after the D-pad moved on) picks it again.
pub fn wedge_angle(lx: f64, ly: f64) -> Option<i32> {
    if (lx * lx + ly * ly).sqrt() < 0.5 {
        return None;
    }
    let deg = lx.atan2(-ly).to_degrees();
    Some(((deg.round() as i32) % 360 + 360) % 360)
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
    /// Wheel notches to send: positive scrolls down, negative up.
    pub wheel: i32,
    /// The left stick let go of a direction ("up", "down", "left", "right"), then pressed one.
    pub release: Option<&'static str>,
    pub press: Option<&'static str>,
}

/// What the left stick is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Left {
    /// In the middle.
    Idle,
    /// Pushed out at angle `from`, for `ms`, not decided yet.
    Pending { from: f64, ms: f64 },
    /// Holding a direction, pressed at angle `from`.
    Dir { dir: &'static str, from: f64 },
    /// Turning the wheel: the angle at the last look and the turning not yet sent as a notch.
    Wheel { last: f64, turned: f64 },
    /// Let go of in Turbine's eyes (the window went to the back, or the keyboard came up) while still pushed:
    /// nothing until the stick is back in the middle.
    Spent,
}

#[derive(Clone, Debug)]
pub struct Pointer {
    pub x: f64,
    pub y: f64,
    pub shown: bool,
    /// Time since the right stick last moved it.
    idle_ms: f64,
    pub left: Left,
    /// Held down by a trigger or R3 (keeps the pointer showing while dragging).
    pub holding: bool,
    /// Only directions, never the scroll wheel (while the quick wheel is open, so every slot can be reached).
    pub dpad_only: bool,
    /// Never hides (Turbine's browser is open: the pointer is how the page is used). It still counts how long it
    /// has been still, so the page can dim it.
    pub sticky: bool,
}

impl Default for Pointer {
    fn default() -> Self {
        Pointer { x: -1.0, y: -1.0, shown: false, idle_ms: 0.0, left: Left::Idle, holding: false, dpad_only: false, sticky: false }
    }
}

/// The D-pad direction for a stick angle (y down): the larger axis wins.
pub fn direction(a: f64) -> &'static str {
    let (dx, dy) = (a.cos(), a.sin());
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 { "right" } else { "left" }
    } else if dy >= 0.0 {
        "down"
    } else {
        "up"
    }
}

/// The angle of a direction's axis (y down).
fn axis(dir: &str) -> f64 {
    match dir {
        "right" => 0.0,
        "down" => PI / 2.0,
        "left" => PI,
        _ => -PI / 2.0,
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

    /// How long the right stick has left it still, in milliseconds.
    pub fn still_ms(&self) -> f64 {
        self.idle_ms
    }

    /// Turbine stops listening to the left stick for now (it went to the back, or the keyboard came up): a held
    /// direction is let go (returned, so it can be sent as a release) and nothing more happens until the stick has
    /// been back in the middle.
    pub fn let_go(&mut self) -> Option<&'static str> {
        let held = match self.left {
            Left::Dir { dir, .. } => Some(dir),
            _ => None,
        };
        if self.left != Left::Idle {
            self.left = Left::Spent;
        }
        held
    }

    /// The left stick is ignored until it has been back in the middle (under `LET_GO`): the quick wheel just
    /// closed while the stick may still be pushed at a wedge, and that push must not become a D-pad press.
    pub fn hold_off(&mut self) {
        self.left = Left::Spent;
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
            if self.shown && !self.holding && !self.sticky && self.idle_ms >= HIDE_MS {
                self.shown = false;
                out.hid = true;
            }
        }
        if self.holding {
            self.idle_ms = 0.0;
        }
        self.left_stick(s, dt_ms, &mut out);
        out
    }

    /// Left stick: a direction press when pushed and held one way, wheel notches when swept around.
    fn left_stick(&mut self, s: &Sticks, dt_ms: f64, out: &mut Tick) {
        let m = s.lx.hypot(s.ly);
        // y points down, so the angle grows clockwise as seen on the screen.
        let a = s.ly.atan2(s.lx);
        self.left = match self.left {
            Left::Idle => {
                if m >= PUSH_ON {
                    Left::Pending { from: a, ms: 0.0 }
                } else {
                    Left::Idle
                }
            }
            Left::Pending { from, ms } => {
                if m < LET_GO {
                    // A flick shorter than a press: nothing.
                    Left::Idle
                } else {
                    let d = wrap(a - from);
                    let ms = ms + dt_ms;
                    if d.abs() >= SWEEP && !self.dpad_only {
                        let mut w = Left::Wheel { last: from, turned: 0.0 };
                        Self::turn(&mut w, a, out);
                        w
                    } else if ms >= COMMIT_MS {
                        // Held long enough without sweeping: the direction it points now (a push that drifted on
                        // the way out still counts).
                        let dir = direction(a);
                        out.press = Some(dir);
                        Left::Dir { dir, from }
                    } else {
                        Left::Pending { from, ms }
                    }
                }
            }
            Left::Dir { dir, from } => {
                let d = wrap(a - from);
                if m < LET_GO {
                    out.release = Some(dir);
                    Left::Idle
                } else if self.dpad_only {
                    // Turned on to another direction (Up, then on round to Right): that one, so the quick wheel's
                    // in-between slots can be reached the way the D-pad reaches them.
                    let now = direction(a);
                    if now != dir && wrap(a - axis(dir)).abs() >= DPAD_TURN {
                        out.release = Some(dir);
                        out.press = Some(now);
                        Left::Dir { dir: now, from: a }
                    } else {
                        Left::Dir { dir, from }
                    }
                } else if d.abs() >= SNAP {
                    // Snapped across in one look (up to down): let go, and the new way is a new push.
                    out.release = Some(dir);
                    Left::Pending { from: a, ms: 0.0 }
                } else if d.abs() >= HELD_SWEEP {
                    // Held, then swept well round: the wheel after all (a wobble while holding never scrolls).
                    out.release = Some(dir);
                    let mut w = Left::Wheel { last: from, turned: 0.0 };
                    Self::turn(&mut w, a, out);
                    w
                } else {
                    Left::Dir { dir, from }
                }
            }
            Left::Spent => {
                if m < LET_GO {
                    Left::Idle
                } else {
                    Left::Spent
                }
            }
            Left::Wheel { last, turned } => {
                if m < LET_GO {
                    Left::Idle
                } else {
                    let mut w = Left::Wheel { last, turned };
                    Self::turn(&mut w, a, out);
                    w
                }
            }
        };
    }

    /// The wheel turned to angle `a`: whole notches go out, the rest waits.
    fn turn(w: &mut Left, a: f64, out: &mut Tick) {
        if let Left::Wheel { last, turned } = w {
            let d = wrap(a - *last);
            // A jump of more than a third of a turn in one look is the stick snapping across, not turning.
            if d.abs() < 2.1 {
                *turned += d;
            }
            *last = a;
            while *turned >= NOTCH - 1e-9 {
                *turned -= NOTCH;
                out.wheel += 1;
            }
            while *turned <= -NOTCH + 1e-9 {
                *turned += NOTCH;
                out.wheel -= 1;
            }
        }
    }
}

/// A in Turbine's browser: a click at the pointer, or Enter on the page's focused link or button. Enter only after
/// the player moved the page's focus with the D-pad (`key`), and only until he moves the pointer (right stick) or
/// clicks: a button clicked a moment ago keeps the page's focus, and A must never press it again by itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyNav {
    pub on: bool,
}

impl KeyNav {
    /// The D-pad moved the page's focus.
    pub fn key(&mut self) {
        self.on = true;
    }
    /// The right stick moved the pointer, or a mouse button went down (RT, R3, LT, or A's own click).
    pub fn pointer(&mut self) {
        self.on = false;
    }
    /// What A does now: true for Enter on the focused element (`focused`: the page has one), false for a click.
    pub fn enter(&self, focused: bool) -> bool {
        self.on && focused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f64 = 1280.0;
    const H: f64 = 800.0;

    #[test]
    fn wedge_angle_by_stick() {
        assert_eq!(wedge_angle(0.0, -1.0), Some(0), "up");
        assert_eq!(wedge_angle(1.0, 0.0), Some(90), "right");
        assert_eq!(wedge_angle(0.0, 1.0), Some(180), "down");
        assert_eq!(wedge_angle(-1.0, 0.0), Some(270), "left");
        assert_eq!(wedge_angle(0.7, -0.7), Some(45), "up-right");
        assert_eq!(wedge_angle(0.2, 0.2), None, "near the middle");
    }

    #[test]
    fn a_stick_still_held_when_the_wheel_closes_is_not_a_press() {
        // Hold View, push the stick at a wedge, let go of View: the wheel closes with the stick still pushed.
        let mut p = Pointer::default();
        p.dpad_only = true;
        p.tick(&Sticks::default(), 16.0, W, H);
        p.dpad_only = false;
        p.hold_off();
        let pushed = Sticks { lx: 0.0, ly: -1.0, ..Sticks::default() };
        let mut pressed = false;
        for _ in 0..20 {
            pressed |= p.tick(&pushed, 16.0, W, H).press.is_some();
        }
        assert!(!pressed, "the push that picked the wedge never lands in the sheet it opened");
        p.tick(&Sticks::default(), 16.0, W, H);
        for _ in 0..10 {
            pressed |= p.tick(&pushed, 16.0, W, H).press.is_some();
        }
        assert!(pressed, "a new push after centring is a press again");
    }

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

    /// Runs the left stick along `path` (one look per 16 ms) and collects what came out.
    fn left(p: &mut Pointer, path: &[(f64, f64)]) -> (Vec<&'static str>, Vec<&'static str>, i32) {
        let (mut presses, mut releases, mut wheel) = (Vec::new(), Vec::new(), 0);
        for (lx, ly) in path {
            let t = p.tick(&Sticks { lx: *lx, ly: *ly, ..Default::default() }, 16.0, W, H);
            presses.extend(t.press);
            releases.extend(t.release);
            wheel += t.wheel;
        }
        (presses, releases, wheel)
    }

    #[test]
    fn a_held_push_is_one_direction() {
        // Acceptance check 1: pushed to 0.6 and held 80 ms is one press, let go is one release.
        let mut p = Pointer::default();
        let mut path = vec![(0.0, -0.6); 6]; // 0, 16, ... 80 ms
        path.extend(vec![(0.0, -0.6); 20]); // held longer: still one press (Main.qml repeats it like the D-pad)
        path.push((0.0, 0.0));
        let (presses, releases, wheel) = left(&mut p, &path);
        assert_eq!(presses, vec!["up"]);
        assert_eq!(releases, vec!["up"]);
        assert_eq!(wheel, 0);
        // Each way, the larger axis winning.
        for ((x, y), want) in [((0.6, 0.1), "right"), ((-0.6, 0.2), "left"), ((0.2, 0.7), "down")] {
            let mut p = Pointer::default();
            let (presses, _, _) = left(&mut p, &vec![(x, y); 7]);
            assert_eq!(presses, vec![want]);
        }
    }

    #[test]
    fn a_flick_is_nothing() {
        let mut p = Pointer::default();
        let mut path = vec![(0.0, 0.9); 4]; // 48 ms
        path.push((0.0, 0.0));
        assert_eq!(left(&mut p, &path), (vec![], vec![], 0));
        // Under the push line it never counts.
        let mut p = Pointer::default();
        assert_eq!(left(&mut p, &vec![(0.45, 0.0); 40]), (vec![], vec![], 0));
    }

    fn sweep(from_deg: f64, to_deg: f64, looks: usize) -> Vec<(f64, f64)> {
        (0..=looks)
            .map(|i| {
                let a = (from_deg + (to_deg - from_deg) * i as f64 / looks as f64).to_radians();
                (a.cos() * 0.9, a.sin() * 0.9)
            })
            .collect()
    }

    #[test]
    fn a_quarter_turn_clockwise_is_three_notches_down() {
        // Acceptance check 1: a 90 degree clockwise sweep (up to right, as seen) is 3 down notches and no direction.
        let mut p = Pointer::default();
        let mut path = sweep(-90.0, 0.0, 8);
        path.push((0.0, 0.0));
        let (presses, releases, wheel) = left(&mut p, &path);
        assert!(presses.is_empty() && releases.is_empty(), "{presses:?} {releases:?}");
        assert_eq!(wheel, 3);
        // Counter-clockwise, a full turn: twelve up.
        let mut p = Pointer::default();
        let (presses, _, wheel) = left(&mut p, &sweep(90.0, -270.0, 40));
        assert!(presses.is_empty());
        assert_eq!(wheel, -12);
        assert!(!p.shown, "the left stick never shows the pointer");
    }

    #[test]
    fn held_then_swept_becomes_the_wheel() {
        let mut p = Pointer::default();
        let mut path = vec![(0.0, -0.9); 7];
        path.extend(sweep(-90.0, 0.0, 12));
        let (presses, releases, wheel) = left(&mut p, &path);
        assert_eq!(presses, vec!["up"]);
        assert_eq!(releases, vec!["up"]);
        assert_eq!(wheel, 3);
    }

    #[test]
    fn a_push_that_drifts_still_presses() {
        // Starts straight up and drifts 35 degrees toward the right on the way out: one press, Up, no wheel.
        let mut p = Pointer::default();
        let mut path = sweep(-90.0, -55.0, 6);
        path.extend(vec![((-55f64).to_radians().cos() * 0.9, (-55f64).to_radians().sin() * 0.9); 10]);
        path.push((0.0, 0.0));
        let (presses, releases, wheel) = left(&mut p, &path);
        assert_eq!(presses, vec!["up"]);
        assert_eq!(releases, vec!["up"]);
        assert_eq!(wheel, 0);
    }

    #[test]
    fn a_wobble_while_holding_does_not_scroll() {
        // Held Right, then wobbled 40 degrees each way and back: still one Right, no wheel notches.
        let mut p = Pointer::default();
        let mut path = vec![(0.9, 0.0); 7];
        path.extend(sweep(0.0, 40.0, 5));
        path.extend(sweep(40.0, -40.0, 10));
        path.extend(sweep(-40.0, 0.0, 5));
        path.extend(vec![(0.9, 0.0); 5]);
        let (presses, releases, wheel) = left(&mut p, &path);
        assert_eq!(presses, vec!["right"]);
        assert!(releases.is_empty(), "{releases:?}");
        assert_eq!(wheel, 0);
    }

    #[test]
    fn snapped_across_is_the_new_direction() {
        let mut p = Pointer::default();
        let mut path = vec![(0.0, -0.9); 7];
        path.extend(vec![(0.0, 0.9); 7]);
        let (presses, releases, wheel) = left(&mut p, &path);
        assert_eq!(presses, vec!["up", "down"]);
        assert_eq!(releases, vec!["up"]);
        assert_eq!(wheel, 0);
    }

    #[test]
    fn the_quick_wheel_reaches_the_slots_in_between() {
        // D-pad only (the quick wheel open): Up, then on round to Right is Up then Right, never wheel notches.
        let mut p = Pointer { dpad_only: true, ..Default::default() };
        let mut path = vec![(0.0, -0.9); 7];
        path.extend(sweep(-90.0, 0.0, 10));
        path.extend(vec![(0.9, 0.0); 3]);
        let (presses, releases, wheel) = left(&mut p, &path);
        assert_eq!(presses, vec!["up", "right"]);
        assert_eq!(releases, vec!["up"]);
        assert_eq!(wheel, 0);
        // A quick sweep from the middle is a direction too, not the wheel.
        let mut p = Pointer { dpad_only: true, ..Default::default() };
        let (presses, _, wheel) = left(&mut p, &sweep(-90.0, 0.0, 8));
        assert_eq!(wheel, 0);
        assert!(!presses.is_empty());
    }

    #[test]
    fn let_go_releases_and_waits_for_the_middle() {
        let mut p = Pointer::default();
        left(&mut p, &vec![(0.0, -0.9); 7]);
        assert_eq!(p.let_go(), Some("up"));
        // Still pushed: nothing more, however long.
        let (presses, releases, _) = left(&mut p, &vec![(0.0, -0.9); 30]);
        assert!(presses.is_empty() && releases.is_empty());
        // Back to the middle, then a new push presses again.
        let mut path = vec![(0.0, 0.0)];
        path.extend(vec![(0.0, 0.9); 7]);
        let (presses, _, _) = left(&mut p, &path);
        assert_eq!(presses, vec!["down"]);
        assert_eq!(Pointer::default().let_go(), None);
    }

    #[test]
    fn presses_and_releases_always_balance() {
        // A long pseudo-random walk of the stick: every release follows its own press.
        let mut p = Pointer::default();
        let mut seed: u64 = 12345;
        let mut rnd = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 33) as f64) / f64::from(u32::MAX >> 1) - 1.0
        };
        let mut held: Option<&'static str> = None;
        let (mut x, mut y) = (0.0f64, 0.0f64);
        for i in 0..20000 {
            if i % 500 == 0 {
                p.dpad_only = !p.dpad_only;
            }
            x = (x + rnd() * 0.3).clamp(-1.0, 1.0);
            y = (y + rnd() * 0.3).clamp(-1.0, 1.0);
            let t = p.tick(&Sticks { lx: x, ly: y, ..Default::default() }, 16.0, W, H);
            if let Some(r) = t.release {
                assert_eq!(held, Some(r));
                held = None;
            }
            if let Some(d) = t.press {
                assert_eq!(held, None);
                held = Some(d);
            }
        }
    }

    #[test]
    fn sdl_values() {
        assert_eq!(unit(32767), 1.0);
        assert_eq!(unit(-32768), -1.0);
        assert_eq!(unit(0), 0.0);
    }

    #[test]
    fn a_sticky_pointer_never_hides_but_counts_its_stillness() {
        let mut p = Pointer::default();
        p.sticky = true;
        run(&mut p, Sticks { rx: 0.8, ..Default::default() }, 100.0);
        assert!(p.shown);
        let t = run(&mut p, Sticks::default(), 10_000.0);
        assert!(p.shown && !t.hid);
        assert!(p.still_ms() >= HIDE_MS);
        p.sticky = false;
        let t = run(&mut p, Sticks::default(), 100.0);
        assert!(!p.shown && t.hid);
    }

    #[test]
    fn a_presses_enter_only_after_the_d_pad() {
        let mut k = KeyNav::default();
        // A button clicked a moment ago keeps the page's focus: A is still a click at the pointer.
        assert!(!k.enter(true));
        k.key();
        assert!(k.enter(true));
        assert!(!k.enter(false));
        // The right stick, or any click, and A clicks at the pointer again (no matter how long it stays still).
        k.pointer();
        assert!(!k.enter(true));
        k.key();
        k.pointer();
        assert!(!k.enter(true));
    }
}
