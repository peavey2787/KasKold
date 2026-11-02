use crate::runtime::interactions::text_files::{self, TextFileSelectionContext, TextFileSelectionWorkflow};

fn accept_content(ad: &mut crate::runtime::data::AppData, content: &[u8]) -> Result<(), &'static str> {
    if content.len() > ad.signing.message.payload.len() {
        return Err("Message too large");
    }
    if content.is_empty() {
        return Err("Message is empty");
    }
    if !text_files::is_readable_text(content) {
        return Err("Message must contain readable text");
    }
    ad.signing.message.payload.fill(0);
    ad.signing.message.payload[..content.len()].copy_from_slice(content);
    ad.signing.message.payload_len = content.len();
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SignMsgPreview));
    Ok(())
}

pub(super) fn handle(context: TextFileSelectionContext<'_, '_, '_>) -> bool {
    text_files::handle_selection::<1_024>(
        context,
        TextFileSelectionWorkflow { back_state: crate::runtime::navigation::continuation!(SignMsgChoice) },
        accept_content,
    )
}

#[cfg(feature = "workflow-test-auto")]
pub(crate) fn workflow_accept_content(
    ad: &mut crate::runtime::data::AppData,
    content: &[u8],
) -> Result<(), &'static str> {
    accept_content(ad, content)
}
