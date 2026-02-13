use jiff::SignedDuration;
use jiff::Timestamp;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Sh,
    Cmd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSecret {
    pub value: String,
    pub created_at: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<SourceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<Timestamp>,
}

impl StoredSecret {
    pub fn new(
        value: String,
        source_command: Option<String>,
        source_type: Option<SourceType>,
        ttl_seconds: Option<i64>,
    ) -> Self {
        let created_at = Timestamp::now();
        let expires_at = ttl_seconds.map(|ttl| {
            created_at
                .checked_add(SignedDuration::from_secs(ttl))
                .unwrap()
        });
        Self {
            value,
            created_at,
            source_command,
            source_type,
            ttl_seconds,
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => Timestamp::now() > exp,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_serialization() {
        let secret = StoredSecret::new(
            "my-secret".into(),
            Some("echo hi".into()),
            Some(SourceType::Sh),
            Some(3600),
        );
        let json = serde_json::to_string(&secret).unwrap();
        let deserialized: StoredSecret = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.value, "my-secret");
        assert_eq!(deserialized.source_command.as_deref(), Some("echo hi"));
        assert_eq!(deserialized.source_type, Some(SourceType::Sh));
        assert_eq!(deserialized.ttl_seconds, Some(3600));
        assert!(deserialized.expires_at.is_some());
    }

    #[test]
    fn no_ttl_never_expires() {
        let secret = StoredSecret::new("val".into(), None, None, None);
        assert!(!secret.is_expired());
        assert!(secret.expires_at.is_none());
        assert!(secret.ttl_seconds.is_none());
    }

    #[test]
    fn future_ttl_not_expired() {
        let secret = StoredSecret::new("val".into(), None, None, Some(3600));
        assert!(!secret.is_expired());
    }

    #[test]
    fn past_ttl_is_expired() {
        let mut secret = StoredSecret::new("val".into(), None, None, Some(60));
        // Backdate the secret so it appears expired
        let past = Timestamp::now()
            .checked_add(SignedDuration::from_secs(-120))
            .unwrap();
        secret.created_at = past;
        secret.expires_at = Some(past.checked_add(SignedDuration::from_secs(60)).unwrap());
        assert!(secret.is_expired());
    }

    #[test]
    fn optional_fields_omitted_in_json() {
        let secret = StoredSecret::new("val".into(), None, None, None);
        let json = serde_json::to_string(&secret).unwrap();
        assert!(!json.contains("source_command"));
        assert!(!json.contains("source_type"));
        assert!(!json.contains("ttl_seconds"));
        assert!(!json.contains("expires_at"));
    }

    #[test]
    fn deserialize_from_known_json() {
        let json = r#"{
            "value": "the-secret",
            "created_at": "2025-01-15T10:30:00Z",
            "source_command": "gcloud secrets versions access latest",
            "source_type": "sh",
            "ttl_seconds": 3600,
            "expires_at": "2025-01-15T11:30:00Z"
        }"#;
        let secret: StoredSecret = serde_json::from_str(json).unwrap();
        assert_eq!(secret.value, "the-secret");
        assert_eq!(
            secret.source_command.as_deref(),
            Some("gcloud secrets versions access latest")
        );
        assert_eq!(secret.source_type, Some(SourceType::Sh));
        assert_eq!(secret.ttl_seconds, Some(3600));
        assert!(secret.expires_at.is_some());
    }

    #[test]
    fn source_type_cmd_serde() {
        let secret = StoredSecret::new(
            "val".into(),
            Some("my-cmd arg1".into()),
            Some(SourceType::Cmd),
            None,
        );
        let json = serde_json::to_string(&secret).unwrap();
        assert!(json.contains(r#""source_type":"cmd""#));
        let deserialized: StoredSecret = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source_type, Some(SourceType::Cmd));
    }
}
