pub use brdgme_crypto::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn load_key_missing_env_panics_without_opt_in() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("DATABASE_ENCRYPTION_KEY") };
        unsafe { std::env::remove_var("ALLOW_INSECURE_DEFAULT_KEY") };
        let result = std::panic::catch_unwind(|| {
            let _ = load_key().expect("DATABASE_ENCRYPTION_KEY must be set");
        });
        assert!(
            result.is_err(),
            "loader must refuse startup (panic) when DATABASE_ENCRYPTION_KEY and ALLOW_INSECURE_DEFAULT_KEY are both unset"
        );
    }

    #[test]
    fn load_key_missing_env_with_opt_in_loads_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("DATABASE_ENCRYPTION_KEY") };
        unsafe { std::env::set_var("ALLOW_INSECURE_DEFAULT_KEY", "true") };
        let loaded =
            load_key().expect("default key must load when ALLOW_INSECURE_DEFAULT_KEY is enabled");
        assert!(*loaded == *default_key());
        unsafe { std::env::remove_var("ALLOW_INSECURE_DEFAULT_KEY") };
    }
}
