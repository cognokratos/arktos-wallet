//! Developer/operator tool for Arktos key material.
//!
//! Keys are read from environment variables only (never from command-line
//! arguments, which end up in shell history and process listings).

use anyhow::{Context, Result};
use arktos_wallet::config::MASTER_KEY_VAR;
use arktos_wallet::crypto;
use arktos_wallet::keys::{Keyring, MasterKey, generate_master_key_base64};
use clap::{Parser, Subcommand};
use zeroize::Zeroizing;

#[derive(Parser, Debug)]
#[command(
    name = "secret",
    version,
    about = "Arktos key and encrypted-secret tool"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print a new random 32-byte key, base64-encoded (for MASTER_KEY or DATABASE_KEY)
    GenerateKey,
    /// Encrypt a recovery phrase from stdin (or --plaintext) into a v1 envelope
    Encrypt {
        /// Environment variable holding the master key
        #[arg(long, default_value = MASTER_KEY_VAR)]
        key_env: String,
        /// Plaintext (if omitted, reads from stdin)
        #[arg(long)]
        plaintext: Option<String>,
    },
    /// Decrypt a stored `wallets.encrypted_passphrase` envelope and print it
    Decrypt {
        /// Environment variable holding the master key
        #[arg(long, default_value = MASTER_KEY_VAR)]
        key_env: String,
        /// Stored value (if omitted, reads from stdin)
        #[arg(long)]
        ciphertext: Option<String>,
    },
    /// Print the stored HMAC of an API key
    Hash {
        /// Environment variable holding the master key
        #[arg(long, default_value = MASTER_KEY_VAR)]
        key_env: String,
        /// API key to hash (if omitted, reads from stdin)
        #[arg(long)]
        api_key: Option<String>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Command::GenerateKey => {
            println!(
                "{}",
                generate_master_key_base64().context("OS random number generator failed")?
            );
        }
        Command::Encrypt { key_env, plaintext } => {
            let keyring = Keyring::new(&master_key(&key_env)?);
            let plaintext = Zeroizing::new(input(plaintext, "plaintext")?);
            println!(
                "{}",
                crypto::seal(keyring.wallet.seed.aead(), plaintext.as_bytes())?
            );
        }
        Command::Decrypt {
            key_env,
            ciphertext,
        } => {
            let keyring = Keyring::new(&master_key(&key_env)?);
            let stored = input(ciphertext, "ciphertext")?;
            let plaintext = crypto::open(keyring.wallet.seed.aead(), &stored)?;
            println!("{}", String::from_utf8_lossy(&plaintext));
        }
        Command::Hash { key_env, api_key } => {
            let keyring = Keyring::new(&master_key(&key_env)?);
            println!(
                "{}",
                keyring.api_keys.hmac.hash(&input(api_key, "api key")?)
            );
        }
    }
    Ok(())
}

fn master_key(env_name: &str) -> Result<MasterKey> {
    let value =
        Zeroizing::new(std::env::var(env_name).with_context(|| format!("{env_name} is not set"))?);
    MasterKey::from_base64(&value).with_context(|| format!("{env_name} is invalid"))
}

fn input(value: Option<String>, what: &str) -> Result<String> {
    match value {
        Some(v) => Ok(v),
        None => {
            use std::io::Read;
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .with_context(|| format!("failed to read {what} from stdin"))?;
            Ok(s.trim().to_string())
        }
    }
}
