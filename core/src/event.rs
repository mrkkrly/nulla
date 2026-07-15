use secp256k1::{Secp256k1, XOnlyPublicKey, Message};
use secp256k1::schnorr::Signature;

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

use crate::keys::Identity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: u64,
    pub kind: u32,
    pub content: String,
    pub sig: String
}

impl Event {
    pub fn compute_id(pubkey: &str, created_at: u64, kind: u32, content: &str) -> String {
        let serialized = format!(
            "[0,\"{}\",{},{},\"{}\"]",
            pubkey, created_at, kind, content
        );

        let hash = Sha256::digest(serialized.as_bytes());
        hex::encode(hash)
    }

    pub fn create(identity: &Identity, kind: u32, content: &str) -> Event {
        let pubkey = identity.public_key_hex();
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let id = Event::compute_id(&pubkey, created_at, kind, content);
        let sig = identity.sign(&id);
        Event {
            id,
            pubkey,
            created_at,
            kind,
            content: content.to_string(),
            sig
        }

    }

    pub fn verify(&self) -> bool {
        let expected_id = Event::compute_id(
            &self.pubkey, self.created_at, self.kind, &self.content
        );

        if expected_id != self.id {
            return false;
        }

        let Ok(pk_bytes) = hex::decode(&self.pubkey) else { return false};
        let Ok(id_bytes) = hex::decode(&self.id) else { return false};
        let Ok(sig_bytes) = hex::decode(&self.sig) else { return false};

        let Ok(pubkey) = XOnlyPublicKey::from_slice(&pk_bytes) else { return false};
        let Ok(sig) = Signature::from_slice(&sig_bytes) else { return false};
        let Ok(arr) = <[u8; 32]>::try_from(id_bytes) else { return false};

        let msg = Message::from_digest(arr);

        let secp = Secp256k1::verification_only();
        secp.verify_schnorr(&sig, msg.as_ref(), &pubkey).is_ok()


    }
}
