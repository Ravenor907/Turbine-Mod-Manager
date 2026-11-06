//! Controllers through SDL. In Game Mode, Steam hands non-Steam apps a virtual Xbox pad, the same path games use.
//! Sticks and triggers become plain presses so the interface only deals with buttons.

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::{EventPump, GameControllerSubsystem, Sdl};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How far a stick or trigger must move to count as a press, and where it lets go again (so it never flickers).
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
    /// Stick and trigger directions currently counted as pressed, per pad.
    axis_down: HashMap<(u32, &'static str), bool>,
    /// Presses are dropped until then (just after Turbine's window comes back to the front).
    quiet_until: Option<Instant>,
}

impl Pads {
    /// Opens SDL's controller support. If SDL isn't available Turbine still runs, with keyboard and touch only.
    pub fn open() -> Self {
        sdl2::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
        let sdl = sdl2::init().ok();
        let subsystem = sdl.as_ref().and_then(|s| s.game_controller().ok());
        let pump = sdl.as_ref().and_then(|s| s.event_pump().ok());
        let mut pads = Pads {
            _sdl: sdl,
            subsystem,
            pump,
            open: Vec::new(),
            held: HashMap::new(),
            last_up: HashMap::new(),
            axis_down: HashMap::new(),
            quiet_until: None,
        };
        if let Some(sub) = pads.subsystem.as_ref() {
            let count = sub.num_joysticks().unwrap_or(0);
            for i in 0..count {
                if sub.is_game_controller(i) {
                    if let Ok(c) = sub.open(i) {
                        pads.open.push(c);
                    }
                }
            }
        }
        pads
    }

    pub fn connected(&self) -> bool {
        !self.open.is_empty()
    }

    /// Turbine's window just came back to the front: presses made while a game had it (still waiting in SDL's
    /// queue) are read and thrown away, and new ones are ignored for `quiet`, so a button pressed in the game as it
    /// closes never acts in Turbine.
    pub fn wake(&mut self, quiet: Duration) {
        let _ = self.read();
        self.quiet_until = Some(Instant::now() + quiet);
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
        if let Some(pump) = self.pump.as_mut() {
            for ev in pump.poll_iter() {
                match ev {
                    Event::ControllerDeviceAdded { which, .. } => added.push(which),
                    Event::ControllerDeviceRemoved { which, .. } => removed.push(which),
                    Event::ControllerButtonDown { which, button, .. } => {
                        if let Some(b) = button_name(button) {
                            raw.push((which, b, true));
                        }
                    }
                    Event::ControllerButtonUp { which, button, .. } => {
                        if let Some(b) = button_name(button) {
                            raw.push((which, b, false));
                        }
                    }
                    Event::ControllerAxisMotion { which, axis, value, .. } => {
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
        if let Some(sub) = self.subsystem.as_ref() {
            for index in added {
                if let Ok(c) = sub.open(index) {
                    if !self.open.iter().any(|o| o.instance_id() == c.instance_id()) {
                        self.open.push(c);
                    }
                }
            }
        }
        for id in removed {
            self.open.retain(|c| c.instance_id() != id);
            self.held.retain(|_, pad| *pad != id);
            self.axis_down.retain(|(pad, _), _| *pad != id);
        }
        let now = Instant::now();
        raw.into_iter().filter_map(|(pad, b, down)| self.dedupe(pad, b, down, now).then_some((b, down))).collect()
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
        // The Steam / Guide button belongs to Steam; Turbine never uses it.
        _ => return None,
    })
}

/// Left stick acts as the D-pad; triggers act as buttons. Each yields (name, strength) pairs, strength 0 when idle.
fn axis_names(axis: Axis, value: i16) -> Vec<(&'static str, i16)> {
    let pos = value.max(0);
    let neg = value.checked_neg().unwrap_or(i16::MAX).max(0);
    match axis {
        Axis::LeftX => vec![("right", pos), ("left", neg)],
        Axis::LeftY => vec![("down", pos), ("up", neg)],
        Axis::TriggerLeft => vec![("lt", pos)],
        Axis::TriggerRight => vec![("rt", pos)],
        _ => vec![],
    }
}
