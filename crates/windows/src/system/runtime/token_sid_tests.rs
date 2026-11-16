use super::*;

#[test]
fn current_token_user_and_owner_are_valid_sids() {
    let user = token_user_sid(TokenSource::CurrentProcess).unwrap();
    let owner = token_owner_sid(TokenSource::CurrentProcess).unwrap();
    assert!(!user.to_string_form().unwrap().is_empty());
    assert!(!owner.to_string_form().unwrap().is_empty());
}
