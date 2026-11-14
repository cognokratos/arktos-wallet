//! BIP39 mnemonic generation and BIP32 account derivation.
//!
//! The mnemonic is held in [`RecoveryPhrase`], which zeroizes on drop and
//! redacts `Debug`. Seeds are zeroized as soon as the extended key is built,
//! and derived private keys never leave this module: only the public key and
//! address are returned.
//! Library-internal copies (e.g. inside `bip39::Mnemonic` or `bip32::XPrv`
//! chain codes) are outside our control.

use crate::domain::{BitcoinNetwork, DerivationIndex, Network};
use anyhow::{Result, anyhow};
use bip32::{DerivationPath, XPrv};
use bip39::Mnemonic;
use bitcoin::{
    PublicKey,
    address::Address,
    secp256k1::{Secp256k1, XOnlyPublicKey},
};
use std::fmt::{self, Write as _};
use std::str::FromStr;
use tiny_keccak::{Hasher, Keccak};
use zeroize::{Zeroize, Zeroizing};

/// Longest 12-word English mnemonic: 12 × 8 letters + 11 spaces.
const MAX_PHRASE_LEN: usize = 12 * 8 + 11;

/// A BIP39 recovery phrase. Zeroized on drop, never printed.
pub struct RecoveryPhrase(Zeroizing<String>);

impl RecoveryPhrase {
    /// Take ownership of a decrypted phrase without copying it.
    pub fn from_utf8(bytes: Zeroizing<Vec<u8>>) -> Result<Self> {
        let mut bytes = bytes;
        match String::from_utf8(std::mem::take(&mut *bytes)) {
            Ok(phrase) => Ok(Self(Zeroizing::new(phrase))),
            Err(err) => {
                err.into_bytes().zeroize();
                Err(anyhow!("recovery phrase is not valid UTF-8"))
            }
        }
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RecoveryPhrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryPhrase([REDACTED])")
    }
}

/// Generate a 12-word BIP39 mnemonic from 128 bits of OS randomness.
pub fn generate_recovery_passphrase() -> Result<RecoveryPhrase> {
    let mut entropy = Zeroizing::new([0u8; 16]);
    getrandom::fill(entropy.as_mut_slice())
        .map_err(|_| anyhow!("OS random number generator failed"))?;
    let mnemonic = Mnemonic::from_entropy(entropy.as_slice())
        .map_err(|_| anyhow!("failed to build mnemonic from entropy"))?;
    // Pre-sized so formatting never reallocates and leaves a stray copy.
    let mut phrase = Zeroizing::new(String::with_capacity(MAX_PHRASE_LEN));
    write!(phrase, "{mnemonic}").map_err(|_| anyhow!("failed to format mnemonic"))?;
    Ok(RecoveryPhrase(phrase))
}

/// Public result of deriving one account. Contains no secret material.
#[derive(Debug)]
pub struct AccountData {
    pub derivation_path: String,
    /// Compressed SEC1 public key, `0x`-prefixed hex.
    pub public_key: String,
    pub address: String,
}

/// Derive an account's public key and address.
///
/// BIP39 seed → BIP32 key at [`Network::derivation_path`]:
/// Bitcoin uses the BIP86 Taproot path and encodes a P2TR address for the
/// given network; Ethereum uses the BIP44 path `m/44'/60'/0'/0/{index}`.
/// Ethereum addresses are returned in canonical lowercase hex; apply
/// [`eip55_checksum`] for display.
pub fn derive_account_keys(
    phrase: &RecoveryPhrase,
    network: Network,
    index: DerivationIndex,
) -> Result<AccountData> {
    let seed = {
        let mnemonic =
            Mnemonic::parse_normalized(phrase.expose()).map_err(|_| anyhow!("Invalid mnemonic"))?;
        Zeroizing::new(mnemonic.to_seed(""))
    };
    let mut xprv = XPrv::new(seed.as_slice())?;
    drop(seed);

    let derivation_path = network.derivation_path(index);
    let path = DerivationPath::from_str(&derivation_path)
        .map_err(|e| anyhow!("Failed to parse derivation path: {}", e))?;
    for child_num in path.into_iter() {
        xprv = xprv
            .derive_child(child_num)
            .map_err(|e| anyhow!("Failed to derive child key: {}", e))?;
    }

    let public_key = xprv.public_key().to_bytes().to_vec();
    // The extended private key (and its secp256k1 scalar) is zeroized on drop.
    drop(xprv);

    let address = match network {
        Network::Bitcoin(bitcoin_network) => derive_bitcoin_address(&public_key, bitcoin_network)?,
        Network::Ethereum => derive_ethereum_address(&public_key)?,
    };

    Ok(AccountData {
        derivation_path,
        public_key: format!("0x{}", hex::encode(public_key)),
        address,
    })
}

/// BIP86 key-path-only Taproot (P2TR) address: the derived key is the internal
/// key, tweaked with no script tree (BIP341).
fn derive_bitcoin_address(public_key: &[u8], network: BitcoinNetwork) -> Result<String> {
    let pk = PublicKey::from_slice(public_key)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;
    let xonly = XOnlyPublicKey::from(pk.inner);
    let secp = Secp256k1::verification_only();
    Ok(Address::p2tr(&secp, xonly, None, network.to_bitcoin()).to_string())
}

/// Ethereum address: last 20 bytes of Keccak-256 over the uncompressed public
/// key without its 0x04 prefix, as lowercase hex.
fn derive_ethereum_address(public_key: &[u8]) -> Result<String> {
    let public_key = PublicKey::from_slice(public_key)
        .map_err(|e| anyhow!("Invalid public key format: {}", e))?;
    let uncompressed = public_key.inner.serialize_uncompressed();
    let hash = keccak256(&uncompressed[1..]);
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    hash
}

/// EIP-55 mixed-case checksum encoding of a `0x`-prefixed 20-byte hex
/// address (input in any case).
pub fn eip55_checksum(address: &str) -> Result<String> {
    let hex_part = address
        .strip_prefix("0x")
        .filter(|h| h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow!("not a 0x-prefixed 20-byte hex address"))?
        .to_ascii_lowercase();
    let hash = keccak256(hex_part.as_bytes());
    let checksummed: String = hex_part
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let nibble = (hash[i / 2] >> if i % 2 == 0 { 4 } else { 0 }) & 0x0f;
            if c.is_ascii_alphabetic() && nibble >= 8 {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect();
    Ok(format!("0x{checksummed}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Public BIP39 test mnemonic (never use for real funds).
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn phrase(words: &str) -> RecoveryPhrase {
        RecoveryPhrase::from_utf8(Zeroizing::new(words.as_bytes().to_vec())).unwrap()
    }

    fn index(i: u32) -> DerivationIndex {
        DerivationIndex::new(i).unwrap()
    }

    fn derive(network: Network, i: u32) -> AccountData {
        derive_account_keys(&phrase(TEST_MNEMONIC), network, index(i)).unwrap()
    }

    const MAINNET: Network = Network::Bitcoin(BitcoinNetwork::Mainnet);

    // ---- Known derivation vectors (must never change) ----------------------
    // Mainnet values are the BIP86 reference vectors; test-network values were
    // generated independently with Python `bip_utils` (Bip86 coins).

    #[test]
    fn bitcoin_mainnet_bip86_vectors() {
        let a0 = derive(MAINNET, 0);
        assert_eq!(a0.derivation_path, "m/86'/0'/0'/0/0");
        assert_eq!(
            a0.address,
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
        );
        assert_eq!(
            a0.public_key,
            "0x03cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115"
        );
        assert_eq!(
            derive(MAINNET, 1).address,
            "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh"
        );
    }

    #[test]
    fn bitcoin_test_network_bip86_vectors() {
        for network in [BitcoinNetwork::Testnet, BitcoinNetwork::Signet] {
            let a0 = derive(Network::Bitcoin(network), 0);
            assert_eq!(a0.derivation_path, "m/86'/1'/0'/0/0", "{network}");
            assert_eq!(
                a0.address, "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj",
                "{network}"
            );
            assert_eq!(
                a0.public_key,
                "0x0255355ca83c973f1d97ce0e3843c85d78905af16b4dc531bc488e57212d230116"
            );
        }
        let regtest = derive(Network::Bitcoin(BitcoinNetwork::Regtest), 1);
        assert_eq!(regtest.derivation_path, "m/86'/1'/0'/0/1");
        assert_eq!(
            regtest.address,
            "bcrt1p90h6z3p36n9hrzy7580h5l429uwchyg8uc9sz4jwzhdtuhqdl5eqkcyx0f"
        );
    }

    #[test]
    fn bitcoin_addresses_parse_for_their_network_only() {
        use bitcoin::address::NetworkUnchecked;
        for network in BitcoinNetwork::ALL {
            let address = derive(Network::Bitcoin(network), 0).address;
            let parsed: Address<NetworkUnchecked> = address.parse().unwrap();
            assert!(
                parsed.is_valid_for_network(network.to_bitcoin()),
                "{network}"
            );
            if network != BitcoinNetwork::Mainnet {
                assert!(!parsed.is_valid_for_network(bitcoin::Network::Bitcoin));
            }
        }
    }

    #[test]
    fn ethereum_bip44_vectors() {
        let a0 = derive(Network::Ethereum, 0);
        assert_eq!(a0.derivation_path, "m/44'/60'/0'/0/0");
        assert_eq!(a0.address, "0x9858effd232b4033e47d90003d41ec34ecaeda94");
        assert_eq!(
            eip55_checksum(&a0.address).unwrap(),
            "0x9858EfFD232B4033E47d90003D41EC34EcaEda94"
        );
        assert_eq!(
            a0.public_key,
            "0x0237b0bb7a8288d38ed49a524b5dc98cff3eb5ca824c9f9dc0dfdb3d9cd600f299"
        );
        assert_eq!(
            eip55_checksum(&derive(Network::Ethereum, 1).address).unwrap(),
            "0x6Fac4D18c912343BF86fa7049364Dd4E424Ab9C0"
        );
    }

    #[test]
    fn eip55_reference_vectors() {
        // Test vectors from the EIP-55 specification.
        for expected in [
            "0x52908400098527886E0F7030069857D2E4169EE7",
            "0x8617E340B3D01FA5F11F306F4090FD50E238070D",
            "0xde709f2102306220921060314715629080e2fb77",
            "0x27b1fdb04752bbc536007a920d24acb045561c26",
            "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
            "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
            "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
            "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
        ] {
            let hex = &expected[2..];
            let lower = format!("0x{}", hex.to_ascii_lowercase());
            let upper = format!("0x{}", hex.to_ascii_uppercase());
            assert_eq!(eip55_checksum(&lower).unwrap(), expected);
            assert_eq!(eip55_checksum(&upper).unwrap(), expected);
        }
    }

    #[test]
    fn eip55_rejects_malformed_addresses() {
        for bad in [
            "",
            "0x",
            "9858effd232b4033e47d90003d41ec34ecaeda94",
            "0x9858effd",
            "0xzz58effd232b4033e47d90003d41ec34ecaeda94",
        ] {
            assert!(eip55_checksum(bad).is_err(), "{bad:?}");
        }
    }

    // ---- Golden table: every supported network, index edges, two seeds ----
    // Generated with the original `bip32` 0.5 / `k256` 0.13 implementation
    // before it was replaced by `bitcoin::bip32`; any change here is a
    // wallet-breaking change, not a refactor.

    /// BIP39 reference vector (Trezor test vectors, no passphrase).
    const LEGAL_MNEMONIC: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const BTC_MAIN: Network = MAINNET;
    const BTC_TEST: Network = Network::Bitcoin(BitcoinNetwork::Testnet);
    const BTC_SIGNET: Network = Network::Bitcoin(BitcoinNetwork::Signet);
    const BTC_REGTEST: Network = Network::Bitcoin(BitcoinNetwork::Regtest);
    const ETH: Network = Network::Ethereum;

    /// (mnemonic, network, index, path, compressed public key, address)
    #[rustfmt::skip]
    const GOLDEN: &[(&str, Network, u32, &str, &str, &str)] = &[
        (TEST_MNEMONIC, BTC_MAIN, 0, "m/86'/0'/0'/0/0", "0x03cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115", "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"),
        (TEST_MNEMONIC, BTC_MAIN, 1, "m/86'/0'/0'/0/1", "0x0283dfe85a3151d2517290da461fe2815591ef69f2b18a2ce63f01697a8b313145", "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh"),
        (TEST_MNEMONIC, BTC_MAIN, 2, "m/86'/0'/0'/0/2", "0x03a730a23ce8b47248a5ddfb32eef75262d91f0b55b084dd1ff0d2f9e032e0e533", "bc1p0d0rhyynq0awa9m8cqrcr8f5nxqx3aw29w4ru5u9my3h0sfygnzs9khxz8"),
        (TEST_MNEMONIC, BTC_MAIN, 19, "m/86'/0'/0'/0/19", "0x032d381d0397cb4a4403bf48514fbe424a7ff99ea96e44ea29bc69639d01a9f020", "bc1pd6nqrg63ex4025vaw89l9xke00zqjm5vdgm88us6036yqugpfdcq0dfmeg"),
        (TEST_MNEMONIC, BTC_MAIN, 1000, "m/86'/0'/0'/0/1000", "0x021e388da92a34c6a5b81ddae1003e45aad16f2292959f2c4a71e43c957531992f", "bc1pp2pues3jzmnk65mhqse3nlv2s7e7x9mpzag3snu0vukhqgtq4knqq8ax6q"),
        (TEST_MNEMONIC, BTC_MAIN, 2147483647, "m/86'/0'/0'/0/2147483647", "0x03ae1f14be6b67db958f4ad01fb15fec2a051fc135bc2ca9ce2b998852dece926a", "bc1pav8jwqrtxqvj856uya800h8elcjr7m04fv500rawfhc5r9nm2m4s2u4c22"),
        (TEST_MNEMONIC, BTC_TEST, 0, "m/86'/1'/0'/0/0", "0x0255355ca83c973f1d97ce0e3843c85d78905af16b4dc531bc488e57212d230116", "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj"),
        (TEST_MNEMONIC, BTC_TEST, 1, "m/86'/1'/0'/0/1", "0x033058679f6d60b87ef921d98a2a9a1f1e0779dae27bedbd1cdb2f147a07835ac9", "tb1p90h6z3p36n9hrzy7580h5l429uwchyg8uc9sz4jwzhdtuhqdl5eqmpwq6n"),
        (TEST_MNEMONIC, BTC_TEST, 2, "m/86'/1'/0'/0/2", "0x02b68df382cad577d8304d5a8e640c3cb42d77c10016ab754caa4d6e68b6cb296d", "tb1p40qqa84kpphe5vtcwd8zv7v6w7p62cmupf6f60mf8pxdkcv2455q9jyrjg"),
        (TEST_MNEMONIC, BTC_TEST, 19, "m/86'/1'/0'/0/19", "0x0299dddf5c1ca2012dc3c801ca02314597e97901a20669a0e67f6cbbecb70be20c", "tb1p20gfetp0fhwrp739uqsknzc5vzjrcxdau0jqsg8nl3tmc8ypashsjw3tzv"),
        (TEST_MNEMONIC, BTC_TEST, 1000, "m/86'/1'/0'/0/1000", "0x02319bd777a1036786d36bcee25e7a542078753094cde4b25a70c91dc810106e29", "tb1pmhenlqgf3hphwunggufdcdyap2vm9vwgx8qurlml6szy02f50etsum69yq"),
        (TEST_MNEMONIC, BTC_TEST, 2147483647, "m/86'/1'/0'/0/2147483647", "0x034c517607e625c615a9129044a24af9ee3fc1b8be7129fbecbf0d41bb9d22b7d3", "tb1pwkgr80z34h2q7vjh6r7xczrj43hrq54q9jr0vfln227ua9gmj8nqry2py5"),
        (TEST_MNEMONIC, BTC_SIGNET, 0, "m/86'/1'/0'/0/0", "0x0255355ca83c973f1d97ce0e3843c85d78905af16b4dc531bc488e57212d230116", "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj"),
        (TEST_MNEMONIC, BTC_SIGNET, 1, "m/86'/1'/0'/0/1", "0x033058679f6d60b87ef921d98a2a9a1f1e0779dae27bedbd1cdb2f147a07835ac9", "tb1p90h6z3p36n9hrzy7580h5l429uwchyg8uc9sz4jwzhdtuhqdl5eqmpwq6n"),
        (TEST_MNEMONIC, BTC_SIGNET, 2, "m/86'/1'/0'/0/2", "0x02b68df382cad577d8304d5a8e640c3cb42d77c10016ab754caa4d6e68b6cb296d", "tb1p40qqa84kpphe5vtcwd8zv7v6w7p62cmupf6f60mf8pxdkcv2455q9jyrjg"),
        (TEST_MNEMONIC, BTC_SIGNET, 19, "m/86'/1'/0'/0/19", "0x0299dddf5c1ca2012dc3c801ca02314597e97901a20669a0e67f6cbbecb70be20c", "tb1p20gfetp0fhwrp739uqsknzc5vzjrcxdau0jqsg8nl3tmc8ypashsjw3tzv"),
        (TEST_MNEMONIC, BTC_SIGNET, 1000, "m/86'/1'/0'/0/1000", "0x02319bd777a1036786d36bcee25e7a542078753094cde4b25a70c91dc810106e29", "tb1pmhenlqgf3hphwunggufdcdyap2vm9vwgx8qurlml6szy02f50etsum69yq"),
        (TEST_MNEMONIC, BTC_SIGNET, 2147483647, "m/86'/1'/0'/0/2147483647", "0x034c517607e625c615a9129044a24af9ee3fc1b8be7129fbecbf0d41bb9d22b7d3", "tb1pwkgr80z34h2q7vjh6r7xczrj43hrq54q9jr0vfln227ua9gmj8nqry2py5"),
        (TEST_MNEMONIC, BTC_REGTEST, 0, "m/86'/1'/0'/0/0", "0x0255355ca83c973f1d97ce0e3843c85d78905af16b4dc531bc488e57212d230116", "bcrt1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqjeprhg"),
        (TEST_MNEMONIC, BTC_REGTEST, 1, "m/86'/1'/0'/0/1", "0x033058679f6d60b87ef921d98a2a9a1f1e0779dae27bedbd1cdb2f147a07835ac9", "bcrt1p90h6z3p36n9hrzy7580h5l429uwchyg8uc9sz4jwzhdtuhqdl5eqkcyx0f"),
        (TEST_MNEMONIC, BTC_REGTEST, 2, "m/86'/1'/0'/0/2", "0x02b68df382cad577d8304d5a8e640c3cb42d77c10016ab754caa4d6e68b6cb296d", "bcrt1p40qqa84kpphe5vtcwd8zv7v6w7p62cmupf6f60mf8pxdkcv2455qgtw98j"),
        (TEST_MNEMONIC, BTC_REGTEST, 19, "m/86'/1'/0'/0/19", "0x0299dddf5c1ca2012dc3c801ca02314597e97901a20669a0e67f6cbbecb70be20c", "bcrt1p20gfetp0fhwrp739uqsknzc5vzjrcxdau0jqsg8nl3tmc8ypashslhmdhk"),
        (TEST_MNEMONIC, BTC_REGTEST, 1000, "m/86'/1'/0'/0/1000", "0x02319bd777a1036786d36bcee25e7a542078753094cde4b25a70c91dc810106e29", "bcrt1pmhenlqgf3hphwunggufdcdyap2vm9vwgx8qurlml6szy02f50ets3zsr36"),
        (TEST_MNEMONIC, BTC_REGTEST, 2147483647, "m/86'/1'/0'/0/2147483647", "0x034c517607e625c615a9129044a24af9ee3fc1b8be7129fbecbf0d41bb9d22b7d3", "bcrt1pwkgr80z34h2q7vjh6r7xczrj43hrq54q9jr0vfln227ua9gmj8nqwaq83w"),
        (TEST_MNEMONIC, ETH, 0, "m/44'/60'/0'/0/0", "0x0237b0bb7a8288d38ed49a524b5dc98cff3eb5ca824c9f9dc0dfdb3d9cd600f299", "0x9858effd232b4033e47d90003d41ec34ecaeda94"),
        (TEST_MNEMONIC, ETH, 1, "m/44'/60'/0'/0/1", "0x039fd0991d0222b4e1339c1a1a5b5f6d9f6a96672a3247b638ee6156d9ea877a2f", "0x6fac4d18c912343bf86fa7049364dd4e424ab9c0"),
        (TEST_MNEMONIC, ETH, 2, "m/44'/60'/0'/0/2", "0x03880bcb4bf46b49bdb071e307e282b11b9166907d9708f8c706092c44743f3e67", "0xb6716976a3ebe8d39aceb04372f22ff8e6802d7a"),
        (TEST_MNEMONIC, ETH, 19, "m/44'/60'/0'/0/19", "0x03851f3f43111d8f89f3a9d69fa4297c07959e7969ea880d0e6ae38a8d6490c469", "0x5096eee90aa1b783af381669938c688f02bb43d8"),
        (TEST_MNEMONIC, ETH, 1000, "m/44'/60'/0'/0/1000", "0x0296242392ba1bc289a3cc773676dfc8ca38c821a73acfd82f24e53b10669a5390", "0xac2dbb7dc8fa82f47021e01255fbd767a2ec4ead"),
        (TEST_MNEMONIC, ETH, 2147483647, "m/44'/60'/0'/0/2147483647", "0x024c67f2ae0e53b190612dc3477a570c63302202e534628da052ec5fd12ad7bd7c", "0x8848bfc75a28756b521b09afdc120bddddc7d7c9"),
        (LEGAL_MNEMONIC, BTC_MAIN, 0, "m/86'/0'/0'/0/0", "0x02ee57d7981c8489be23cecdc96f30596c700d7f55090029e8a94aeca780a31170", "bc1pgp80xvc97ssyphukzd5w6gmz2z45dzawl8sn3g6r9e95l90yvdfqa9879z"),
        (LEGAL_MNEMONIC, BTC_MAIN, 1, "m/86'/0'/0'/0/1", "0x039c43e4097d76de3ed4db51c7a81adb1d26d29cd3fdfe4eb27b128cdd7e04cb71", "bc1p8t3tw3jvt5urqxjxp8jaex0ecca48l02qm0a85nlaanq2p9lqdvsgglqxq"),
        (LEGAL_MNEMONIC, BTC_MAIN, 2, "m/86'/0'/0'/0/2", "0x0274e58c75593b289b2947f30cdd5b3bb8e95486085a77f24e9d01f8ab1d59b556", "bc1pkhtuh934jgr2k94pfhc2590jn4nk30svhcj2htta0l4hjx7qudpqdkhm7z"),
        (LEGAL_MNEMONIC, BTC_MAIN, 19, "m/86'/0'/0'/0/19", "0x02259659a1d4af94c5d2588acedc0a69f89a1e059db65fd37d0ac84d15dc4fed27", "bc1pulsslv34jldzkqqq6jrl4rgnc7cemjcspssytc6jxyfwfzf6y02sv9mwp2"),
        (LEGAL_MNEMONIC, BTC_MAIN, 1000, "m/86'/0'/0'/0/1000", "0x0296d6c3b4c0921dfa07e80411d4f1af96a55ebe0fdfdb75104440ac5b557bddbc", "bc1pzpesgs9nkddl5wqlvfca57jyg5fw2xleqf53zrzgwl6zj93xmcmqux634k"),
        (LEGAL_MNEMONIC, BTC_MAIN, 2147483647, "m/86'/0'/0'/0/2147483647", "0x0332c48100a29581dc3c85dca019ae9e4f79813ae5ca5b9b93c6122d5cc0fbc1b1", "bc1pq369nghkp8j0ae0t42p4cghl4ysldayyp8nq3eqncryz4xzduvfqz9zdnc"),
        (LEGAL_MNEMONIC, BTC_TEST, 0, "m/86'/1'/0'/0/0", "0x029ff08732287dca062343cf43f763ea84dcd40348a1dd6f90956beb0c59c33b94", "tb1p9y6pr3ec7jfdg6aaa282eu6pxn630ujlmkuwzvmdc6lam3t7dy7qrvhl3c"),
        (LEGAL_MNEMONIC, BTC_TEST, 1, "m/86'/1'/0'/0/1", "0x03bba554603171ffcc5e131cd399de0a1709ec52712c10bde061ed615a891af5dc", "tb1p4355zke8l8sjxjnevk9sytl93qw3ysa48kca74y8uzv22purwwuquulpan"),
        (LEGAL_MNEMONIC, BTC_TEST, 2, "m/86'/1'/0'/0/2", "0x02ea0a024602e0790922722912a0f01dab4ebc873f86319e56f146817e60c19b44", "tb1ptlqnkl4xzy8kext5pmtjur2rpvvh6sq7tw4jdrjmm9fkyvem4feq2xrces"),
        (LEGAL_MNEMONIC, BTC_TEST, 19, "m/86'/1'/0'/0/19", "0x0261d8c64341ae8046c273426443e0877d709338c6e488b5676a4b08014b0729d0", "tb1pltd9hy9z6rne9ten5ytmal7738msnaesyg25erh7a9ldgtj04slqpx56vn"),
        (LEGAL_MNEMONIC, BTC_TEST, 1000, "m/86'/1'/0'/0/1000", "0x03789290056f4b94758b42332bbee6d187a5cf04da4681f5adc873b14c1ea2f35e", "tb1p4cheq3llrcl4smxhve20hqlkfq7dqf3s3vp503t8rl47rxwgggksfsmwpr"),
        (LEGAL_MNEMONIC, BTC_TEST, 2147483647, "m/86'/1'/0'/0/2147483647", "0x03a2dee826af97df40e6d984623f6fe5fa77e57cd5f608df10d4f57cc2913b70c3", "tb1p5rqam34aqmyt8t8ng027hyfevv3tgzyd477g74drj0ec7ql7hh5sve0rvq"),
        (LEGAL_MNEMONIC, BTC_SIGNET, 0, "m/86'/1'/0'/0/0", "0x029ff08732287dca062343cf43f763ea84dcd40348a1dd6f90956beb0c59c33b94", "tb1p9y6pr3ec7jfdg6aaa282eu6pxn630ujlmkuwzvmdc6lam3t7dy7qrvhl3c"),
        (LEGAL_MNEMONIC, BTC_SIGNET, 1, "m/86'/1'/0'/0/1", "0x03bba554603171ffcc5e131cd399de0a1709ec52712c10bde061ed615a891af5dc", "tb1p4355zke8l8sjxjnevk9sytl93qw3ysa48kca74y8uzv22purwwuquulpan"),
        (LEGAL_MNEMONIC, BTC_SIGNET, 2, "m/86'/1'/0'/0/2", "0x02ea0a024602e0790922722912a0f01dab4ebc873f86319e56f146817e60c19b44", "tb1ptlqnkl4xzy8kext5pmtjur2rpvvh6sq7tw4jdrjmm9fkyvem4feq2xrces"),
        (LEGAL_MNEMONIC, BTC_SIGNET, 19, "m/86'/1'/0'/0/19", "0x0261d8c64341ae8046c273426443e0877d709338c6e488b5676a4b08014b0729d0", "tb1pltd9hy9z6rne9ten5ytmal7738msnaesyg25erh7a9ldgtj04slqpx56vn"),
        (LEGAL_MNEMONIC, BTC_SIGNET, 1000, "m/86'/1'/0'/0/1000", "0x03789290056f4b94758b42332bbee6d187a5cf04da4681f5adc873b14c1ea2f35e", "tb1p4cheq3llrcl4smxhve20hqlkfq7dqf3s3vp503t8rl47rxwgggksfsmwpr"),
        (LEGAL_MNEMONIC, BTC_SIGNET, 2147483647, "m/86'/1'/0'/0/2147483647", "0x03a2dee826af97df40e6d984623f6fe5fa77e57cd5f608df10d4f57cc2913b70c3", "tb1p5rqam34aqmyt8t8ng027hyfevv3tgzyd477g74drj0ec7ql7hh5sve0rvq"),
        (LEGAL_MNEMONIC, BTC_REGTEST, 0, "m/86'/1'/0'/0/0", "0x029ff08732287dca062343cf43f763ea84dcd40348a1dd6f90956beb0c59c33b94", "bcrt1p9y6pr3ec7jfdg6aaa282eu6pxn630ujlmkuwzvmdc6lam3t7dy7qw4aeyz"),
        (LEGAL_MNEMONIC, BTC_REGTEST, 1, "m/86'/1'/0'/0/1", "0x03bba554603171ffcc5e131cd399de0a1709ec52712c10bde061ed615a891af5dc", "bcrt1p4355zke8l8sjxjnevk9sytl93qw3ysa48kca74y8uzv22purwwuq3948gf"),
        (LEGAL_MNEMONIC, BTC_REGTEST, 2, "m/86'/1'/0'/0/2", "0x02ea0a024602e0790922722912a0f01dab4ebc873f86319e56f146817e60c19b44", "bcrt1ptlqnkl4xzy8kext5pmtjur2rpvvh6sq7tw4jdrjmm9fkyvem4feq8lf7v2"),
        (LEGAL_MNEMONIC, BTC_REGTEST, 19, "m/86'/1'/0'/0/19", "0x0261d8c64341ae8046c273426443e0877d709338c6e488b5676a4b08014b0729d0", "bcrt1pltd9hy9z6rne9ten5ytmal7738msnaesyg25erh7a9ldgtj04slqvl7uef"),
        (LEGAL_MNEMONIC, BTC_REGTEST, 1000, "m/86'/1'/0'/0/1000", "0x03789290056f4b94758b42332bbee6d187a5cf04da4681f5adc873b14c1ea2f35e", "bcrt1p4cheq3llrcl4smxhve20hqlkfq7dqf3s3vp503t8rl47rxwgggksyf3g5e"),
        (LEGAL_MNEMONIC, BTC_REGTEST, 2147483647, "m/86'/1'/0'/0/2147483647", "0x03a2dee826af97df40e6d984623f6fe5fa77e57cd5f608df10d4f57cc2913b70c3", "bcrt1p5rqam34aqmyt8t8ng027hyfevv3tgzyd477g74drj0ec7ql7hh5spq99e6"),
        (LEGAL_MNEMONIC, ETH, 0, "m/44'/60'/0'/0/0", "0x03a70d1ef368ad99e90d509496e9888ee7404e4f4d360376bf521d769cf0c4de46", "0x58a57ed9d8d624cbd12e2c467d34787555bb1b25"),
        (LEGAL_MNEMONIC, ETH, 1, "m/44'/60'/0'/0/1", "0x025063f04f89890d9fcc6a794e72f064cd98a38a75a52150689abd5292d0d388fb", "0x0d3eb21b6b21833a4939cfff4810e9ae0758e12c"),
        (LEGAL_MNEMONIC, ETH, 2, "m/44'/60'/0'/0/2", "0x036b8e8777a4a4c3e3516d4536f81d2acfec0f204ea625b76accde78a506348c4c", "0xe42f4612e154153b68e241e8fde337e0c4dd6bbd"),
        (LEGAL_MNEMONIC, ETH, 19, "m/44'/60'/0'/0/19", "0x032071280e2a7f87a98a64e236117aa92e86bdc49e997b35f8da7796406312ffa0", "0x1bf35cd18ba074dc21715906e7679f7c49ad5361"),
        (LEGAL_MNEMONIC, ETH, 1000, "m/44'/60'/0'/0/1000", "0x03828ffc392d43c70434cb12974d84f3537f574037ff718ce51a51af5af729f43a", "0x44a2e54ae16dd2060de47124be27a07bf896a681"),
        (LEGAL_MNEMONIC, ETH, 2147483647, "m/44'/60'/0'/0/2147483647", "0x02192164756754a756627ec90740863db79edffc25d57b6054d4ebe58d70ef43e2", "0x577eb46bb8075f5c00bc0607d6e21129d2f70ca7"),
    ];

    #[test]
    fn golden_derivation_table() {
        for &(words, network, i, path, public_key, address) in GOLDEN {
            let account = derive_account_keys(&phrase(words), network, index(i)).unwrap();
            let case = format!("{network:?} index {i} ({}…)", &words[..5]);
            assert_eq!(account.derivation_path, path, "{case}");
            assert_eq!(account.public_key, public_key, "{case}");
            assert_eq!(account.address, address, "{case}");
        }
        // Every supported network and both index edges are covered.
        for network in BitcoinNetwork::ALL
            .map(Network::Bitcoin)
            .into_iter()
            .chain([ETH])
        {
            for i in [0, DerivationIndex::MAX] {
                assert!(
                    GOLDEN.iter().any(|g| g.1 == network && g.2 == i),
                    "{network:?} {i}"
                );
            }
        }
    }

    #[test]
    fn bip86_internal_and_output_keys_match_the_specification() {
        // BIP86 test vectors for "abandon … about", account 0, receive chain:
        // (index, internal key, tweaked output key). The derived public key is
        // the *internal* key; the address commits to the BIP341 output key
        // tweaked with no script tree.
        for (i, internal_key, output_key) in [
            (
                0,
                "cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115",
                "a60869f0dbcf1dc659c9cecbaf8050135ea9e8cdc487053f1dc6880949dc684c",
            ),
            (
                1,
                "83dfe85a3151d2517290da461fe2815591ef69f2b18a2ce63f01697a8b313145",
                "a82f29944d65b86ae6b5e5cc75e294ead6c59391a1edc5e016e3498c67fc7bbb",
            ),
        ] {
            let account = derive(MAINNET, i);
            // Compressed SEC1 key = parity byte ‖ x-only internal key.
            assert_eq!(&account.public_key[4..], internal_key, "index {i}");
            let script = account
                .address
                .parse::<Address<bitcoin::address::NetworkUnchecked>>()
                .unwrap()
                .assume_checked()
                .script_pubkey();
            // OP_1 OP_PUSHBYTES_32 <output key>
            assert_eq!(
                hex::encode(script.as_bytes()),
                format!("5120{output_key}"),
                "index {i}"
            );
        }
    }

    // ---- Mnemonic generation ------------------------------------------------

    #[test]
    fn generated_phrase_is_valid_12_word_bip39() {
        let generated = generate_recovery_passphrase().expect("generate");
        assert_eq!(generated.expose().split_whitespace().count(), 12);
        Mnemonic::parse_normalized(generated.expose()).expect("valid BIP39");
        assert!(generated.0.capacity() >= generated.expose().len());
    }

    #[test]
    fn generated_phrases_are_unique() {
        let a = generate_recovery_passphrase().unwrap();
        let b = generate_recovery_passphrase().unwrap();
        assert_ne!(a.expose(), b.expose());
    }

    // ---- Derivation properties ---------------------------------------------

    #[test]
    fn different_indices_give_different_keys() {
        let (a, b) = (derive(MAINNET, 0), derive(MAINNET, 1));
        assert_ne!(a.public_key, b.public_key);
        assert_ne!(a.address, b.address);
    }

    #[test]
    fn derivation_is_deterministic() {
        for network in [MAINNET, Network::Ethereum] {
            let (a, b) = (derive(network, 3), derive(network, 3));
            assert_eq!(a.address, b.address);
            assert_eq!(a.public_key, b.public_key);
        }
    }

    #[test]
    fn invalid_mnemonic_is_rejected_without_echoing_it() {
        let err =
            derive_account_keys(&phrase("invalid mnemonic words"), MAINNET, index(0)).unwrap_err();
        assert!(!err.to_string().contains("invalid mnemonic words"));
    }

    #[test]
    fn invalid_utf8_phrase_is_rejected() {
        assert!(RecoveryPhrase::from_utf8(Zeroizing::new(vec![0xff, 0xfe])).is_err());
    }

    // ---- Secret hygiene -----------------------------------------------------

    #[test]
    fn debug_output_redacts_secrets() {
        let account = derive(Network::Ethereum, 0);
        let rendered = format!("{account:?} {:?}", phrase(TEST_MNEMONIC));
        // Known private key of m/44'/60'/0'/0/0 must not be reachable anywhere.
        assert!(
            !rendered.contains("1ab42cc412b618bdea3a599e3c9bae199ebf030895b039e9db1e30dafb12b727")
        );
        assert!(!rendered.contains("abandon"));
        assert!(rendered.contains("REDACTED"));
        assert!(rendered.contains(&account.address));
    }
}
