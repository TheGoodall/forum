use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let hasher = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = hasher
        .hash_password_simple(password.as_bytes(), &salt)
        .unwrap() //Should never fail
        .to_string();
    password_hash
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = PasswordHash::new(hash).unwrap();

    let hasher = Argon2::default();
    hasher
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hash_password_unique() {
        let password = "password";
        let hash1 = hash_password(password);
        let hash2 = hash_password(password);
        assert_ne!(hash1, hash2);
        assert_ne!(hash1, "");
    }

    #[test]
    fn verify_password_match() {
        let password = "password123";
        let hash1 = hash_password(password);
        assert!(verify_password(password, &hash1));

        let password2 = "password1234";
        assert_eq!(verify_password(password2, &hash1), false);
    }
}
