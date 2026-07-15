use secp256k1::{Secp256k1, SecretKey, Keypair, XOnlyPublicKey, Message, schnorr::Signature};
use rand::RngCore;

pub struct Identity {
    pub keypair: Keypair,
    secp: Secp256k1<secp256k1::All>,
}

impl Identity {
    pub fn generate() -> Self {
        let secp = Secp256k1::new();
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let secret = SecretKey::from_byte_array(bytes).expect("valid key");
        let keypair = Keypair::from_secret_key(&secp, &secret);
        Identity { keypair, secp }
    }

    pub fn sign(&self, id_hex: &str) -> String {
        let id_bytes = hex::decode(id_hex).expect("valid hex id");
        let arr: [u8; 32] = id_bytes.try_into().expect("id must be 32 bytes");
        let msg = Message::from_digest(arr);
        let sig: Signature = self.secp.sign_schnorr(msg.as_ref(), &self.keypair);
        hex::encode(sig.to_byte_array())
    }

    pub fn public_key_hex(&self) -> String {
        let (xonly, _) = XOnlyPublicKey::from_keypair(&self.keypair);
        hex::encode(xonly.serialize())
    }
}
