//! Controllers through SDL. In Game Mode, Steam hands non-Steam apps a virtual Xbox pad, the same path games use;
//! SDL's controller mappings cover Xbox 360 and Series pads and the handhelds' built-in controls the same way.
//! Buttons and the triggers become plain presses ("rt", "lt"); the sticks are kept as positions for the controller
//! pointer (pointer.rs): the right stick moves it; the left stick is the D-pad when pushed and held, a wheel when swept.

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::{EventPump, GameControllerSubsystem, Sdl, TimerSubsystem};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// How far a trigger must move to count as a press, and where it lets go again (so it never flickers).
const PRESS_AT: i16 = 20_000;
const RELEASE_AT: i16 = 12_000;
/// The same tap seen on two devices (Steam's virtual pad and the raw one) arrives within this many milliseconds.
const TWIN_MS: u128 = 40;

pub struct Pads {
    _sdl: Option<Sdl>,
    subsystem: Option<GameControllerSubsystem>,
    pump: Option<EventPump>,
    open: Vec<GameController>,
    /// Which pad holds each button down right now.
    held: HashMap<&'static str, u32>,
    /// When each button was last let go, and on which pad.
    last_up: HashMap<&'static str, (u32, Instant)>,
    /// Triggers currently counted as pressed, per pad.
    axis_down: HashMap<(u32, &'static str), bool>,
    /// Each pad's sticks: left x, left y, right x, right y (SDL's -32768..32767, y down).
    sticks: HashMap<u32, [i16; 4]>,
    /// Presses are dropped until then (just after Turbine's window comes back to the front).
    quiet_until: Option<Instant>,
    timer: Option<TimerSubsystem>,
    /// How long the last presses waited in SDL's queue before Turbine read them (ms), for Labs > Performance check.
    waits: VecDeque<u32>,
    /// A controller went away since the last `take_dropped_out` (holds in the interface stop).
    dropped_out: bool,
}

impl Pads {
    /// Opens SDL's controller support. If SDL isn't available Turbine still runs, with keyboard and touch only.
    pub fn open() -> Self {
        // Steam sets these for apps it starts so they only see its virtual pad; in Desktop Mode that can leave
        // Turbine with no pad at all. Every pad SDL can see counts (twins are told apart by `dedupe`).
        for k in ["SDL_GAMECONTROLLER_IGNORE_DEVICES", "SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT"] {
            if let Ok(v) = std::env::var(k) {
                crate::log::line("pad", &format!("{k} was set ({} characters); cleared so SDL sees every controller", v.len()));
                std::env::remove_var(k);
            }
        }
        sdl2::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
        let sdl = sdl2::init().ok();
        let subsystem = sdl.as_ref().and_then(|s| s.game_controller().ok());
        let pump = sdl.as_ref().and_then(|s| s.event_pump().ok());
        let timer = sdl.as_ref().and_then(|s| s.timer().ok());
        if sdl.is_none() {
            crate::log::line("pad", "SDL isn't available: keyboard and touch only");
        }
        let mut pads = Pads {
            _sdl: sdl,
            subsystem,
            pump,
            open: Vec::new(),
            held: HashMap::new(),
            last_up: HashMap::new(),
            axis_down: HashMap::new(),
            sticks: HashMap::new(),
            quiet_until: None,
            timer,
            waits: VecDeque::new(),
            dropped_out: false,
        };
        if let Some(sub) = pads.subsystem.as_ref() {
            let count = sub.num_joysticks().unwrap_or(0);
            crate::log::line("pad", &format!("SDL lists {count} joystick device(s)"));
            for i in 0..count {
                let name = sub.name_for_index(i).unwrap_or_else(|_| "unnamed".into());
                let guid = pads._sdl.as_ref().and_then(|s| s.joystick().ok()).and_then(|j| j.device_guid(i).ok()).map(|g| g.string()).unwrap_or_default();
                crate::log::line("pad", &format!("Device {i}: {name}, GUID {guid}, {}", if sub.is_game_controller(i) { "a game controller" } else { "not a game controller (no mapping)" }));
                if sub.is_game_controller(i) {
                    if let Ok(c) = sub.open(i) {
                        crate::log::line("pad", &format!("Controller found: {}", c.name()));
                        pads.open.push(c);
                    }
                }
            }
        }
        pads
    }

    /// Where the sticks are now. With more than one pad (Steam's virtual pad and the raw one report the same
    /// stick), each stick is taken from the pad that has it pushed furthest.
    pub fn sticks(&self) -> crate::pointer::Sticks {
        use crate::pointer::unit;
        let mut s = crate::pointer::Sticks::default();
        let (mut lm, mut rm) = (0.0, 0.0);
        for v in self.sticks.values() {
            let (lx, ly, rx, ry) = (unit(v[0]), unit(v[1]), unit(v[2]), unit(v[3]));
            if lx.hypot(ly) > lm {
                lm = lx.hypot(ly);
                s.lx = lx;
                s.ly = ly;
            }
            if rx.hypot(ry) > rm {
                rm = rx.hypot(ry);
                s.rx = rx;
                s.ry = ry;
            }
        }
        s
    }

    pub fn connected(&self) -> bool {
        !self.open.is_empty()
    }

    /// The controllers' names, for reports.
    pub fn names(&self) -> Vec<String> {
        self.open.iter().map(|c| c.name()).collect()
    }

    /// (median, worst, count) of how long recent presses waited in SDL's queue, in ms.
    pub fn waits(&self) -> (u32, u32, usize) {
        let mut v: Vec<u32> = self.waits.iter().copied().collect();
        if v.is_empty() {
            return (0, 0, 0);
        }
        v.sort_unstable();
        (v[v.len() / 2], v[v.len() - 1], v.len())
    }

    fn note_wait(&mut self, stamp: u32) {
        let Some(t) = self.timer.as_ref() else { return };
        let wait = t.ticks().saturating_sub(stamp);
        if self.waits.len() >= 50 {
            self.waits.pop_front();
        }
        self.waits.push_back(wait);
    }

    /// Turbine's window just came back to the front: presses made while a game had it (still waiting in SDL's
    /// queue) are read and thrown away, and new ones are ignored for `quiet`, so a button pressed in the game as it
    /// closes never acts in Turbine. The releases among them are returned: a button held in Turbine and let go
    /// while it was in the background must still be let go here (a hold to delete stops).
    pub fn wake(&mut self, quiet: Duration) -> Vec<(&'static str, bool)> {
        let kept = self.waits.len();
        let dropped = self.read();
        // Presses made in a game waited there on purpose; they don't count as Turbine being slow.
        self.waits.truncate(kept);
        if !dropped.is_empty() {
            crate::log::line("pad", &format!("Dropped {} button events made while Turbine was in the background", dropped.len()));
        }
        self.quiet_until = Some(Instant::now() + quiet);
        dropped.into_iter().filter(|(_, down)| !*down).collect()
    }

    /// Whether a controller went away since the last call.
    pub fn take_dropped_out(&mut self) -> bool {
        std::mem::take(&mut self.dropped_out)
    }

    /// True during the quiet time after Turbine comes back to the front (presses are dropped).
    pub fn quiet(&self) -> bool {
        self.quiet_until.is_some_and(|until| Instant::now() < until)
    }

    /// Everything that happened since the last call, as (button, pressed) pairs.
    pub fn poll(&mut self) -> Vec<(&'static str, bool)> {
        let events = self.read();
        if let Some(until) = self.quiet_until {
            if Instant::now() < until {
                // Releases still go through, so nothing stays held down in the interface.
                return events.into_iter().filter(|(_, down)| !*down).collect();
            }
            self.quiet_until = None;
        }
        events
    }

    /// Reads SDL's queue: controllers coming and going, buttons, sticks and triggers.
    fn read(&mut self) -> Vec<(&'static str, bool)> {
        let mut raw: Vec<(u32, &'static str, bool)> = Vec::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut stamps = Vec::new();
        if let Some(pump) = self.pump.as_mut() {
            for ev in pump.poll_iter() {
                match ev {
                    Event::ControllerDeviceAdded { which, .. } => added.push(which),
                    Event::ControllerDeviceRemoved { which, .. } => removed.push(which),
                    Event::ControllerButtonDown { which, button, timestamp } => {
                        if let Some(b) = button_name(button) {
                            raw.push((which, b, true));
                            stamps.push(timestamp);
                        }
                    }
                    Event::ControllerButtonUp { which, button, .. } => {
                        if let Some(b) = button_name(button) {
                            raw.push((which, b, false));
                        }
                    }
                    Event::ControllerAxisMotion { which, axis, value, .. } => {
                        let slot = match axis {
                            Axis::LeftX => Some(0),
                            Axis::LeftY => Some(1),
                            Axis::RightX => Some(2),
                            Axis::RightY => Some(3),
                            _ => None,
                        };
                        if let Some(i) = slot {
                            self.sticks.entry(which).or_insert([0; 4])[i] = value;
                        }
                        for (name, v) in axis_names(axis, value) {
                            let key = (which, name);
                            let was = *self.axis_down.get(&key).unwrap_or(&false);
                            let now = if was { v > RELEASE_AT } else { v > PRESS_AT };
                            if now != was {
                                self.axis_down.insert(key, now);
                                raw.push((which, name, now));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for stamp in stamps {
            self.note_wait(stamp);
        }
        if let Some(sub) = self.subsystem.as_ref() {
            for index in added {
                if let Ok(c) = sub.open(index) {
                    if !self.open.iter().any(|o| o.instance_id() == c.instance_id()) {
                        crate::log::line("pad", &format!("Controller connected: {}", c.name()));
                        self.open.push(c);
                    }
                }
            }
        }
        let now = Instant::now();
        let mut out: Vec<(&'static str, bool)> =
            raw.into_iter().filter_map(|(pad, b, down)| self.dedupe(pad, b, down, now).then_some((b, down))).collect();
        for id in removed {
            if let Some(c) = self.open.iter().find(|c| c.instance_id() == id) {
                crate::log::line("pad", &format!("Controller disconnected: {}", c.name()));
            }
            // Everything it held down is let go (it will never send those releases itself), so no hold keeps
            // running after the controller drops out.
            let mut gone: Vec<&'static str> = self.held.iter().filter(|(_, pad)| **pad == id).map(|(b, _)| *b).collect();
            gone.sort_unstable();
            for b in gone {
                crate::log::line("pad", &format!("Released {b} (its controller went away)"));
                self.held.remove(b);
                out.push((b, false));
            }
            self.dropped_out = true;
            self.open.retain(|c| c.instance_id() != id);
            self.axis_down.retain(|(pad, _), _| *pad != id);
            self.sticks.remove(&id);
        }
        out
    }

    /// One tap seen on two pads counts once.
    fn dedupe(&mut self, pad: u32, button: &'static str, down: bool, now: Instant) -> bool {
        if down {
            if self.held.contains_key(button) {
                return false; // another pad already holds it
            }
            if let Some((other, at)) = self.last_up.get(button) {
                if *other != pad && now.duration_since(*at).as_millis() < TWIN_MS {
                    return false; // the second copy of a tap just let go
                }
            }
            self.held.insert(button, pad);
            true
        } else {
            if self.held.get(button) != Some(&pad) {
                return false;
            }
            self.held.remove(button);
            self.last_up.insert(button, (pad, now));
            true
        }
    }
}

fn button_name(b: Button) -> Option<&'static str> {
    Some(match b {
        Button::A => "a",
        Button::B => "b",
        Button::X => "x",
        Button::Y => "y",
        Button::LeftShoulder => "lb",
        Button::RightShoulder => "rb",
        Button::Start => "start",
        Button::Back => "view",
        Button::DPadUp => "up",
        Button::DPadDown => "down",
        Button::DPadLeft => "left",
        Button::DPadRight => "right",
        Button::LeftStick => "l3",
        Button::RightStick => "r3",
        // The back grips, where a pad has them and Steam passes them on (its default layout sends nothing).
        Button::Paddle1 => "r4",
        Button::Paddle2 => "l4",
        Button::Paddle3 => "r5",
        Button::Paddle4 => "l5",
        // The Steam / Guide button belongs to Steam; Turbine never uses it.
        _ => return None,
    })
}

/// The triggers act as buttons (the sticks are read as positions instead). Yields (name, strength), 0 when idle.
fn axis_names(axis: Axis, value: i16) -> Vec<(&'static str, i16)> {
    let pos = value.max(0);
    match axis {
        Axis::TriggerLeft => vec![("lt", pos)],
        Axis::TriggerRight => vec![("rt", pos)],
        _ => vec![],
    }
}

// ------------------------------------------------------------------------------------------------ pointer events

extern "C" {
    // qtglue.cpp
    fn turbine_window_size(w: *mut f64, h: *mut f64) -> i32;
    fn turbine_pointer_event(kind: i32, x: f64, y: f64, button: i32, buttons: i32, wheel: i32) -> i32;
}

/// A mouse event for Turbine's window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Move,
    Press,
    Release,
    Wheel,
}

/// The main window's size (its own pixels), when it is there.
pub fn window_size() -> Option<(f64, f64)> {
    let (mut w, mut h) = (0.0, 0.0);
    // SAFETY: qtglue.cpp writes two doubles through the pointers it is given, on the interface thread.
    let r = unsafe { turbine_window_size(&mut w, &mut h) };
    (r == 1 && w > 0.0 && h > 0.0).then_some((w, h))
}

/// Delivers a mouse or wheel event to the main window. `button`, `buttons`: Qt's values (1 left, 2 right);
/// `wheel`: steps, positive scrolls down.
pub fn send(kind: Kind, x: f64, y: f64, button: i32, buttons: i32, wheel: i32) {
    let k = match kind {
        Kind::Move => 0,
        Kind::Press => 1,
        Kind::Release => 2,
        Kind::Wheel => 3,
    };
    // SAFETY: a plain call into qtglue.cpp on the interface thread; it only sends events to Turbine's own window.
    let r = unsafe { turbine_pointer_event(k, x, y, button, buttons, wheel) };
    if r != 1 && kind != Kind::Move {
        crate::log::line("pad", "Pointer event not delivered: Turbine's window wasn't found");
    }
}
