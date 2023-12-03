pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hashed_password: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hashed_password)
}
