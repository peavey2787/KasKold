//! Declarative M5Stack board profile shared by CoreS3 and CoreS3 Lite.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BoardProfile {
    pub(crate) model: &'static str,
    pub(crate) hardware_qualified: bool,
    pub(crate) qualification_note: &'static str,
}

pub(crate) const CORES3: BoardProfile = BoardProfile {
    model: "M5Stack CoreS3",
    hardware_qualified: true,
    qualification_note: "hardware-qualified",
};

pub(crate) const CORES3_LITE: BoardProfile = BoardProfile {
    model: "M5Stack CoreS3 Lite",
    hardware_qualified: false,
    qualification_note: "build-supported; hardware qualification pending",
};

#[must_use]
pub(crate) const fn active() -> BoardProfile {
    if cfg!(feature = "m5stack-lite") {
        CORES3_LITE
    } else {
        CORES3
    }
}
