use crate::error::{IncidentError, IncidentResult};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_slack_signature(
    signing_secret: &str,
    timestamp: &str,
    body: &str,
    signature: &str,
) -> IncidentResult<()> {
    // Check if timestamp is recent (within 5 minutes), allowing small clock skew.
    let request_time = timestamp
        .parse::<i64>()
        .map_err(|_| IncidentError::InvalidSignature)?;
    let current_time = chrono::Utc::now().timestamp();
    if (current_time - request_time).abs() > 60 * 5 {
        return Err(IncidentError::InvalidSignature);
    }

    // Compute expected signature
    let base_string = format!("v0:{}:{}", timestamp, body);
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())
        .map_err(|_| IncidentError::InvalidSignature)?;
    mac.update(base_string.as_bytes());
    let provided_signature = signature
        .strip_prefix("v0=")
        .ok_or(IncidentError::InvalidSignature)?;
    let provided_bytes =
        hex::decode(provided_signature).map_err(|_| IncidentError::InvalidSignature)?;

    // Constant-time comparison via HMAC verification API.
    mac.verify_slice(&provided_bytes)
        .map_err(|_| IncidentError::InvalidSignature)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_signature() {
        let signing_secret = "test_secret";
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let body = "token=xoxb-test&team_id=T1234";

        // Generate valid signature
        let base_string = format!("v0:{}:{}", timestamp, body);
        let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).unwrap();
        mac.update(base_string.as_bytes());
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        let result = verify_slack_signature(signing_secret, &timestamp, body, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let signing_secret = "test_secret";
        let timestamp = &chrono::Utc::now().timestamp().to_string();
        let body = "token=xoxb-test&team_id=T1234";
        let bad_signature = "v0=wrong";

        let result = verify_slack_signature(signing_secret, timestamp, body, bad_signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_future_timestamp_within_skew_is_allowed() {
        let signing_secret = "test_secret";
        let timestamp = (chrono::Utc::now().timestamp() + 120).to_string();
        let body = "token=xoxb-test&team_id=T1234";
        let base_string = format!("v0:{}:{}", timestamp, body);
        let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).unwrap();
        mac.update(base_string.as_bytes());
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        let result = verify_slack_signature(signing_secret, &timestamp, body, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_future_timestamp_outside_skew_is_rejected() {
        let signing_secret = "test_secret";
        let timestamp = (chrono::Utc::now().timestamp() + 301).to_string();
        let body = "token=xoxb-test&team_id=T1234";
        let base_string = format!("v0:{}:{}", timestamp, body);
        let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).unwrap();
        mac.update(base_string.as_bytes());
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        let result = verify_slack_signature(signing_secret, &timestamp, body, &signature);
        assert!(result.is_err());
    }
}
