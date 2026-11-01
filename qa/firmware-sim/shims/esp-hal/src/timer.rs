pub mod timg {
    use crate::time::Duration;

    #[derive(Clone, Copy)]
    pub enum MwdtStage {
        Stage0,
        Stage1,
        Stage2,
        Stage3,
    }

    #[derive(Clone, Copy)]
    pub enum MwdtStageAction {
        Off,
        Interrupt,
        ResetCpu,
        ResetSystem,
    }

    #[derive(Default)]
    pub struct Wdt;

    impl Wdt {
        pub fn set_timeout(&mut self, _stage: MwdtStage, _timeout: Duration) {}
        pub fn set_stage_action(&mut self, _stage: MwdtStage, _action: MwdtStageAction) {}
        pub fn enable(&mut self) {}
        pub fn disable(&mut self) {}
        pub fn feed(&mut self) {}
    }

    pub struct TimerGroup {
        pub wdt: Wdt,
    }

    impl TimerGroup {
        pub fn new<T>(_timg: T) -> Self {
            Self { wdt: Wdt }
        }
    }
}

pub mod systimer {
    #[derive(Clone, Copy)]
    pub enum Unit {
        Unit0,
        Unit1,
    }

    pub struct SystemTimer;

    impl SystemTimer {
        pub fn unit_value(_unit: Unit) -> u64 {
            crate::board::clock::read_micros()
        }
    }
}
