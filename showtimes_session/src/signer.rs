//! A simple re-export for jwt-lc-rs signer data

pub use jwt_lc_rs::signing::*;
use jwt_lc_rs::validator::{SubjectValidator, Validator};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct TestMetadata {
    sub: String,
    time: i64,
}

/// A helper function to test encoding function with subject verification
pub fn test_encode(signer: &jwt_lc_rs::Signer) -> Result<String, jwt_lc_rs::errors::Error> {
    let metadata = TestMetadata {
        sub: "test-encode-subject".to_string(),
        time: jiff::Timestamp::now().as_second(),
    };

    jwt_lc_rs::encode(&metadata, signer)
}

/// A helper function to test decoding function
///
/// Accompanies [`test_encode`]
pub fn test_decode(
    token: &str,
    signer: &jwt_lc_rs::Signer,
) -> Result<(), jwt_lc_rs::errors::Error> {
    let subj = SubjectValidator::new("test-encode-subject");
    let validator = Validator::new(vec![Box::new(subj)]);
    jwt_lc_rs::decode::<TestMetadata>(token, signer, &validator)?;
    Ok(())
}
