use anyhow::{Context, Result, anyhow};
use arktos_wallet::api_key::ApiKey;
use arktos_wallet::crypto::{decrypt_secret, encrypt_secret};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "secret", version, about = "Decrypt secrets (AES-256-GCM)")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Encrypt plaintext and print DB-safe base64 string
    Encrypt {
        /// Master key string (prefer passing via --key-env)
        #[arg(long)]
        key: Option<String>,

        /// Read master key from this environment variable (recommended)
        #[arg(long)]
        key_env: Option<String>,

        /// Plaintext (if omitted, reads from stdin)
        #[arg(long)]
        plaintext: Option<String>,
    },
    /// Decrypt DB base64 string and print plaintext
    Decrypt {
        /// Master key string (prefer passing via --key-env)
        #[arg(long)]
        key: Option<String>,

        /// Read master key from this environment variable (recommended)
        #[arg(long)]
        key_env: Option<String>,

        /// Base64 ciphertext (if omitted, reads from stdin)
        #[arg(long)]
        ciphertext: Option<String>,
    },
    /// Hash api key using SHA256 and print hex string
    Hash {
        /// Master key string (prefer passing via --key-env)
        #[arg(long)]
        key: Option<String>,

        /// Read master key from this environment variable (recommended)
        #[arg(long)]
        key_env: Option<String>,

        /// API key to hash
        #[arg(long)]
        api_key: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Encrypt {
            key,
            key_env,
            plaintext,
        } => {
            let key = resolve_key(key, key_env)?;
            let plaintext = match plaintext {
                Some(p) => p,
                None => read_all_stdin_trimmed().context("failed to read plaintext from stdin")?,
            };
            let enc = encrypt_secret(&plaintext, &key)?;
            println!("{enc}");
        }
        Command::Decrypt {
            key,
            key_env,
            ciphertext,
        } => {
            let key = resolve_key(key, key_env)?;
            let ciphertext = match ciphertext {
                Some(c) => c,
                None => read_all_stdin_trimmed().context("failed to read ciphertext from stdin")?,
            };
            let pt = decrypt_secret(&ciphertext, &key)?;
            println!("{pt}");
        }
        Command::Hash {
            key,
            key_env,
            api_key,
        } => {
            let key = resolve_key(key, key_env)?;
            let api_key = match api_key {
                Some(a) => a,
                None => read_all_stdin_trimmed().context("failed to read api key from stdin")?,
            };
            let hash = ApiKey::hash(&api_key, &key);
            println!("{hash}");
        }
    }

    Ok(())
}

fn read_all_stdin_trimmed() -> Result<String> {
    use std::io::Read;
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s)?;
    Ok(s.trim().to_string())
}

fn resolve_key(key: Option<String>, key_env: Option<String>) -> Result<String> {
    if let Some(k) = key {
        return Ok(k);
    }
    if let Some(env_name) = key_env {
        let v =
            std::env::var(&env_name).with_context(|| format!("env var {env_name} is not set"))?;
        return Ok(v);
    }
    Err(anyhow!("provide --key or --key-env"))
}
