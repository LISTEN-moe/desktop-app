use anyhow::Result;

fn entry() -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(
        "moe.listen.desktop",
        "account-session",
    )?)
}

pub fn load_token() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn save_token(token: &str) -> Result<()> {
    Ok(entry()?.set_password(token)?)
}

pub fn delete_token() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
