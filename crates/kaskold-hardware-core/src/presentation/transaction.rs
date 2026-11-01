//! Pure transaction-screen touch reducer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionScreen {
    ScanQr { return_target: ScanReturn },
    Review,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanReturn {
    MultisigAddKey(u8),
    MainMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionEffect {
    None,
    ScanBack(ScanReturn),
    ReviewBack,
    ReviewAdvance,
    ConfirmBack,
    ConfirmChoice(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionDecision {
    pub effect: TransactionEffect,
    pub redraw: bool,
}

pub fn reduce_touch(
    screen: TransactionScreen,
    x: u16,
    y: u16,
    is_back: bool,
) -> TransactionDecision {
    match screen {
        TransactionScreen::ScanQr { return_target } => reduce_scan(return_target, x, y),
        TransactionScreen::Review => reduce_review(x, y, is_back),
        TransactionScreen::Confirm => reduce_confirm(x, y, is_back),
    }
}

fn reduce_scan(return_target: ScanReturn, x: u16, y: u16) -> TransactionDecision {
    let effect = if x <= 48 && y <= 48 {
        TransactionEffect::ScanBack(return_target)
    } else {
        TransactionEffect::None
    };
    TransactionDecision {
        effect,
        redraw: false,
    }
}

fn reduce_review(x: u16, y: u16, is_back: bool) -> TransactionDecision {
    let effect = if is_back {
        TransactionEffect::ReviewBack
    } else if (205..=305).contains(&x) && (194..=232).contains(&y) {
        TransactionEffect::ReviewAdvance
    } else {
        TransactionEffect::None
    };
    TransactionDecision {
        effect,
        redraw: effect != TransactionEffect::None,
    }
}

fn reduce_confirm(x: u16, y: u16, is_back: bool) -> TransactionDecision {
    let effect = if is_back {
        TransactionEffect::ConfirmBack
    } else if (15..=105).contains(&x) && (188..=228).contains(&y) {
        TransactionEffect::ConfirmChoice(0)
    } else if (215..=305).contains(&x) && (188..=228).contains(&y) {
        TransactionEffect::ConfirmChoice(1)
    } else if (115..=205).contains(&x) && (188..=228).contains(&y) {
        TransactionEffect::ConfirmChoice(2)
    } else {
        TransactionEffect::None
    };
    TransactionDecision {
        effect,
        redraw: effect != TransactionEffect::None,
    }
}
