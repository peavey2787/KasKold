use crate::credential_policy::{
    validate, validate_entered_password, CredentialError, CredentialPolicyKind, MIN_PASSWORD_CHARS,
};

#[test]
fn credential_policy_rejects_weak_inputs() {
    assert_eq!(
        validate(CredentialPolicyKind::Pin, b""),
        Err(CredentialError::PinTooShort)
    );
    assert_eq!(
        validate(CredentialPolicyKind::Pin, b"12345"),
        Err(CredentialError::PinTooShort)
    );
    assert!(validate(CredentialPolicyKind::Pin, b"123456").is_ok());
    assert!(validate(CredentialPolicyKind::Pin, b"123456789012").is_ok());
    assert_eq!(
        validate(CredentialPolicyKind::Pin, b"1234567890123"),
        Err(CredentialError::PinTooLong)
    );
    assert_eq!(
        validate(CredentialPolicyKind::Pin, b"12a4"),
        Err(CredentialError::PinNotNumeric)
    );
    assert_eq!(
        validate(CredentialPolicyKind::Pin, b"12345x"),
        Err(CredentialError::PinNotNumeric)
    );
    assert_eq!(
        validate(CredentialPolicyKind::Pin, b"123456789012x"),
        Err(CredentialError::PinNotNumeric)
    );
    assert_eq!(MIN_PASSWORD_CHARS, 12);
    assert_eq!(
        validate(CredentialPolicyKind::Password, b"correct7hor"),
        Err(CredentialError::PasswordTooShort)
    );
    assert!(validate(CredentialPolicyKind::Password, b"correct7hors").is_ok());
    assert_eq!(
        validate(CredentialPolicyKind::Password, b"abcdefghijkl"),
        Err(CredentialError::PasswordNeedsDigit)
    );
    assert_eq!(
        validate(CredentialPolicyKind::Password, b"123456789012"),
        Err(CredentialError::PasswordNeedsLetter)
    );
    assert!(validate(CredentialPolicyKind::Password, b"correct7horse").is_ok());
    let mut maximum = [b'a'; 128];
    maximum[127] = b'7';
    assert!(validate(CredentialPolicyKind::Password, &maximum).is_ok());
    let mut too_long = [b'a'; 129];
    too_long[128] = b'7';
    assert_eq!(
        validate(CredentialPolicyKind::Password, &too_long),
        Err(CredentialError::PasswordTooLong)
    );
}

#[test]
fn entered_passwords_keep_opening_ciphertext_sealed_under_older_minimums() {
    assert_eq!(
        validate_entered_password(b""),
        Err(CredentialError::PasswordTooShort)
    );
    assert_eq!(validate_entered_password(b"a"), Ok(()));
    assert_eq!(validate_entered_password(b"abcdefgh"), Ok(()));
    assert_eq!(validate_entered_password(&[b'a'; 128]), Ok(()));
    assert_eq!(
        validate_entered_password(&[b'a'; 129]),
        Err(CredentialError::PasswordTooLong)
    );
}
