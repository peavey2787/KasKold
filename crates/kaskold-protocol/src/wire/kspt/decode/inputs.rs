use super::super::io::Reader;
use super::super::{valid_sighash, DecodeError, Input, Signature, WireError};
use super::DecodeSink;

const MIN_INPUT_WIRE_BYTES: usize = 59;

pub(super) fn validate_input_capacity(count: u32, remaining: usize) -> Result<(), WireError> {
    let count = usize::try_from(count).map_err(|_| WireError::CountOverflow)?;
    let minimum = count
        .checked_mul(MIN_INPUT_WIRE_BYTES)
        .ok_or(WireError::CountOverflow)?;
    if minimum > remaining {
        return Err(WireError::CountOverflow);
    }
    Ok(())
}

pub(super) fn read_inputs<S: DecodeSink>(
    reader: &mut Reader<'_>,
    count: u32,
    sink: &mut S,
) -> Result<(), DecodeError<S::Error>> {
    for index in 0..count {
        read_input(reader, index, sink)?;
    }
    Ok(())
}

fn read_input<S: DecodeSink>(
    reader: &mut Reader<'_>,
    index: u32,
    sink: &mut S,
) -> Result<(), DecodeError<S::Error>> {
    let input = read_input_value(reader)?;
    let signature_count = read_signature_count(reader)?;
    sink.input(index, input, signature_count)
        .map_err(DecodeError::Sink)?;
    read_signatures(reader, index, signature_count, sink)?;
    read_redeem(reader, index, sink)
}

fn read_input_value<'a>(reader: &mut Reader<'a>) -> Result<Input<'a>, WireError> {
    let previous_tx_id = reader.array::<32>()?;
    let previous_index = reader.u32()?;
    let amount = reader.u64()?;
    let sequence = reader.u64()?;
    let sig_op_count = reader.u8()?;
    if usize::from(sig_op_count) > super::super::MAX_SIGNATURE_RECORDS {
        return Err(WireError::TooManySignatures);
    }
    Ok(Input {
        previous_tx_id,
        previous_index,
        amount,
        sequence,
        sig_op_count,
        script_version: reader.u16()?,
        script: reader.script()?,
    })
}

fn read_signature_count(reader: &mut Reader<'_>) -> Result<u8, WireError> {
    let count = reader.u8()?;
    if usize::from(count) > super::super::MAX_SIGNATURE_RECORDS {
        return Err(WireError::TooManySignatures);
    }
    Ok(count)
}

fn read_signatures<S: DecodeSink>(
    reader: &mut Reader<'_>,
    index: u32,
    count: u8,
    sink: &mut S,
) -> Result<(), DecodeError<S::Error>> {
    let mut seen = [false; 256];
    for slot in 0..count {
        let signature = read_signature(reader, &mut seen)?;
        sink.signature(index, slot, signature)
            .map_err(DecodeError::Sink)?;
    }
    Ok(())
}

fn read_signature(reader: &mut Reader<'_>, seen: &mut [bool; 256]) -> Result<Signature, WireError> {
    let position = reader.u8()?;
    if seen[usize::from(position)] {
        return Err(WireError::DuplicateSignaturePosition);
    }
    seen[usize::from(position)] = true;
    let sighash = reader.u8()?;
    if !valid_sighash(sighash) {
        return Err(WireError::InvalidSigHashType);
    }
    Ok(Signature {
        position,
        sighash,
        bytes: reader.array::<64>()?,
    })
}

fn read_redeem<S: DecodeSink>(
    reader: &mut Reader<'_>,
    index: u32,
    sink: &mut S,
) -> Result<(), DecodeError<S::Error>> {
    let redeem_len = usize::from(reader.u16()?);
    if redeem_len > super::super::MAX_REDEEM_SIZE {
        return Err(WireError::RedeemTooLong.into());
    }
    let redeem = reader.bytes(redeem_len)?;
    sink.redeem(index, redeem).map_err(DecodeError::Sink)
}
