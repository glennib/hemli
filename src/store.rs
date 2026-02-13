use crate::error::HemliError;
use crate::model::StoredSecret;

pub fn service_name(namespace: &str) -> String {
    format!("hemli:{namespace}")
}

pub fn get_secret(namespace: &str, name: &str) -> Result<Option<StoredSecret>, HemliError> {
    let entry = keyring::Entry::new(&service_name(namespace), name)?;
    match entry.get_password() {
        Ok(json) => {
            let secret: StoredSecret = serde_json::from_str(&json)?;
            Ok(Some(secret))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn set_secret(namespace: &str, name: &str, secret: &StoredSecret) -> Result<(), HemliError> {
    let entry = keyring::Entry::new(&service_name(namespace), name)?;
    let json = serde_json::to_string(secret)?;
    entry.set_password(&json)?;
    Ok(())
}

pub fn delete_secret(namespace: &str, name: &str) -> Result<(), HemliError> {
    let entry = keyring::Entry::new(&service_name(namespace), name)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_format() {
        assert_eq!(service_name("myapp"), "hemli:myapp");
        assert_eq!(service_name("prod"), "hemli:prod");
    }

    #[test]
    #[ignore] // Requires OS keyring access
    fn get_set_delete_roundtrip() {
        let ns = "hemli-test-roundtrip";
        let name = "test-secret";

        // Clean up first
        let _ = delete_secret(ns, name);

        // Get should return None
        let result = get_secret(ns, name).unwrap();
        assert!(result.is_none());

        // Set
        let secret = StoredSecret::new("test-value".into(), None, None, None);
        set_secret(ns, name, &secret).unwrap();

        // Get should return the secret
        let result = get_secret(ns, name).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "test-value");

        // Delete
        delete_secret(ns, name).unwrap();

        // Get should return None again
        let result = get_secret(ns, name).unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[ignore] // Requires OS keyring access
    fn delete_nonexistent_is_ok() {
        let result = delete_secret("hemli-test-nonexistent", "nonexistent");
        assert!(result.is_ok());
    }
}
