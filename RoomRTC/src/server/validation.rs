//! Validación de credenciales de usuario.

/// Valida que el username sea alfanumérico, no vacío y máximo 32 caracteres.
pub fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty() {
        return Err("Username vacío".to_string());
    }
    if username.len() > 32 {
        return Err("Username demasiado largo (máx 32)".to_string());
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Username inválido: solo letras, números o _".to_string());
    }
    Ok(())
}

/// Valida que el password no esté vacío, máximo 64 caracteres y sin caracteres prohibidos.
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.is_empty() {
        return Err("Password vacío".to_string());
    }
    if password.len() > 64 {
        return Err("Password demasiado largo (máx 64)".to_string());
    }
    if password.chars().any(|c| matches!(c, ':' | '|' | '\n' | '\r')) {
        return Err("Password inválido: no usar ':', '|' ni saltos de línea".to_string());
    }
    Ok(())
}
