//! The bridge between the QML interface and the engine: one object, `Turbine`, available to QML as a singleton.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, games)]
        #[qproperty(QString, problems)]
        #[qproperty(QString, version)]
        #[qproperty(bool, steamos)]
        #[qproperty(bool, gamemode)]
        #[qproperty(bool, pads)]
        type Turbine = super::TurbineRust;

        /// Looks for games again (Steam and Heroic).
        #[qinvokable]
        fn scan(self: Pin<&mut Self>);

        /// Reads the controllers. QML calls this on a timer while the window is active.
        #[qinvokable]
        fn poll(self: Pin<&mut Self>);

        /// A controller button went down or up: "a", "b", "x", "y", "lb", "rb", "lt", "rt", "start", "view",
        /// "up", "down", "left", "right" (D-pad and left stick).
        #[qsignal]
        fn pad(self: Pin<&mut Self>, button: QString, pressed: bool);
    }
}

use crate::pad::Pads;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

pub struct TurbineRust {
    games: QString,
    problems: QString,
    version: QString,
    steamos: bool,
    gamemode: bool,
    pads: bool,
    reader: Option<Pads>,
}

impl Default for TurbineRust {
    fn default() -> Self {
        Self {
            games: QString::from("[]"),
            problems: QString::default(),
            version: QString::from(crate::VERSION),
            steamos: crate::system::is_steamos(),
            gamemode: crate::system::in_game_mode(),
            pads: false,
            reader: None,
        }
    }
}

impl qobject::Turbine {
    pub fn scan(self: Pin<&mut Self>) {
        let scan = turbine_engine::scan_games(&turbine_engine::home_dir());
        let mut this = self;
        this.as_mut().set_problems(QString::from(scan.problems.join("\n").as_str()));
        this.as_mut().set_games(QString::from(scan.games_json().as_str()));
    }

    pub fn poll(mut self: Pin<&mut Self>) {
        if self.as_mut().rust_mut().reader.is_none() {
            let r = Pads::open();
            self.as_mut().rust_mut().reader = Some(r);
        }
        let (events, connected) = match self.as_mut().rust_mut().reader.as_mut() {
            Some(r) => (r.poll(), r.connected()),
            None => (Vec::new(), false),
        };
        if *self.pads() != connected {
            self.as_mut().set_pads(connected);
        }
        for (button, pressed) in events {
            self.as_mut().pad(QString::from(button), pressed);
        }
    }
}
