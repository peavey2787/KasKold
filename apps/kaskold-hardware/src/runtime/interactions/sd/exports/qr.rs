use super::super::common::context::{SdActionContext, SdIoContext};
use crate::controllers::TouchInput;
// SD controller workflow: qr.
use super::super::{format_auto_name, scan_auto_increment};
/// Offer an auto-numbered 8.3 name as the editable export file name.
fn stage_export_name(ad: &mut crate::runtime::data::AppData, name: [u8; 11]) {
    ad.storage.export_file.filename = name;
    ad.wallet.seeds.pp_input.reset();
    for &byte in name[..8].iter().filter(|&&byte| byte != b' ') {
        ad.wallet.seeds.pp_input.push_char(byte);
    }
}

pub(crate) fn handle_show_qr_popup(ctx: SdIoContext<'_, '_, '_>) -> bool {
    let SdIoContext { ad, delay, i2c, input, .. } = ctx;
    let TouchInput { x, y, is_back } = input;
    let mut needs_redraw = false;

    if is_back {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ShowQrModeChoice));
        needs_redraw = true;
    } else {
        // Two buttons: "Save to SD" and "Back to QR"
        // Save to SD button zone: center-left area
        if (30..=155).contains(&x) && (140..=185).contains(&y) {
            // Save to SD → detect content type for correct extension
            let outgoing = &ad.qr.outgoing.buffer[..ad.qr.outgoing.length];
            if kaskold_protocol::wire::multisig_descriptor::looks_like_descriptor(outgoing) {
                let next = scan_auto_increment(i2c, delay, b"MD", b"TXT");
                stage_export_name(ad, format_auto_name(b"MD", next, b"TXT"));
                crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SdMsDescFilename));
            } else {
                let next = scan_auto_increment(i2c, delay, b"TX", b"KSP");
                stage_export_name(ad, format_auto_name(b"TX", next, b"KSP"));
                crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SdKsptFilename));
            }
            needs_redraw = true;
        }
        // Back to QR button zone: center-right area
        else if (165..=290).contains(&x) && (140..=185).contains(&y) {
            ad.qr.outgoing.frame = 0;
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ShowQR));
            needs_redraw = true;
        }
    }

    needs_redraw
}

pub(crate) fn handle_show_qr_mode_choice(ctx: SdActionContext<'_>) -> bool {
    let SdActionContext { ad, input, .. } = ctx;
    let TouchInput { x, y, is_back } = input;

    if is_back {
        ad.qr.outgoing.frame_count = 0;
        if ad.signing.multisig.creating.n > 0 {
            // Descriptor QR — back to descriptor view
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigDescriptor));
        } else {
            crate::runtime::effects::home(ad);
        }
    } else {
        // "Auto Cycle" button: left
        if (30..=155).contains(&x) && (140..=185).contains(&y) {
            ad.qr.outgoing.manual_frames = false;
            ad.qr.outgoing.frame = 0; // start at frame 0 so the
                                      // frame-0 screen clear in redraw fires and wipes the
                                      // mode-choice text (Manual already resets this).
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ShowQR));
        }
        // "Manual" button: right
        else if (165..=290).contains(&x) && (140..=185).contains(&y) {
            ad.qr.outgoing.manual_frames = true;
            ad.qr.outgoing.frame = 0;
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ShowQR));
        }
    }
    true
}
