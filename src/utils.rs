use anomaly::format_err;
use std::io::{self, Read, Write};
use std::str::FromStr;
use tracing::{debug, trace};
use cosmrs::crypto::PublicKey;
use tendermint::PublicKey as TPublicKey;

use crate::error::{Error, ErrorKind::IoError};

/// Options for displaying public key
#[derive(Debug)]
pub enum PubkeyDisplay {
    Base64,
    Bech32,
    Json
}

impl FromStr for PubkeyDisplay {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "base64" => Ok(PubkeyDisplay::Base64),
            "bech32" => Ok(PubkeyDisplay::Bech32),
            "json" => Ok(PubkeyDisplay::Json),
            _ => Err("unknown display type".to_owned()),
        }
    }
}

/// prints public key in the desired format
pub fn print_pubkey(
    bech32_prefix: Option<String>,
    ptype: Option<PubkeyDisplay>,
    public: ed25519_dalek::PublicKey,
) {
    match ptype {
        Some(PubkeyDisplay::Bech32) => {
            let prefix = bech32_prefix.unwrap_or_else(|| "cosmosvalconspub".to_owned());
            let mut data = vec![0x16, 0x24, 0xDE, 0x64, 0x20];
            data.extend_from_slice(public.as_bytes());
            println!(
                "public key: {}",
                subtle_encoding::bech32::encode(prefix, data)
            );
        }
        Some(PubkeyDisplay::Json) => {
            let tendermint_key = TPublicKey::Ed25519(public);
            let json_pubkey = PublicKey::from(tendermint_key).to_json();
            println!(
                "public key: {}",
                json_pubkey
            );
        }
        _ => {
            println!(
                "public key: {}",
                String::from_utf8(subtle_encoding::base64::encode(public)).unwrap()
            );
            let id = tendermint::node::Id::from(public);
            println!("address: {}", id);
        }
    }
}

/// Read u16-size payload (for vsock)
pub fn read_u16_payload<S: Read>(stream: &mut S) -> Result<Vec<u8>, Error> {
    let mut len_b = [0u8; 2];
    stream
        .read_exact(&mut len_b)
        .map_err(|e| format_err!(IoError, "error reading len: {}", e))?;

    let l = (u16::from_le_bytes(len_b)) as usize;
    if l > 0 {
        let mut state_raw = vec![0u8; l];
        let mut total = 0;

        while let Ok(n) = stream.read(&mut state_raw[total..]) {
            if n == 0 || n + total > l {
                break;
            }
            total += n;
        }

        if total == 0 {
            return Err(IoError.into());
        }
        state_raw.resize(total, 0);
        Ok(state_raw)
    } else {
        trace!("read empty payload");
        Ok(Vec::default())
    }
}

/// Write u16-sized payload (for vsock)
pub fn write_u16_payload<S: Write>(stream: &mut S, data: &[u8]) -> io::Result<()> {
    if data.len() > u16::MAX as usize {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    debug!("writing u16-sized payload");
    let data_len = (data.len() as u16).to_le_bytes();

    stream.write_all(&data_len)?;
    stream.write_all(data)?;
    stream.flush()?;
    debug!("successfully wrote u16-sized payload");
    Ok(())
}
