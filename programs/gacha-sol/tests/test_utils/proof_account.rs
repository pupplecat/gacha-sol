use anyhow::Result;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_token_2022::solana_zk_sdk::encryption::{
    auth_encryption::AeKey,
    elgamal::ElGamalKeypair,
    pod::{
        auth_encryption::PodAeCiphertext,
        elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
    },
};

pub struct SignerProofAccount {
    pub keypair: Keypair,
}

impl Clone for SignerProofAccount {
    fn clone(&self) -> Self {
        Self {
            keypair: self.keypair.insecure_clone(),
        }
    }
}

impl SignerProofAccount {
    pub fn new() -> Self {
        Self {
            keypair: Keypair::new(),
        }
    }
}

impl ProofAccount for SignerProofAccount {
    fn sign(&self, message: &[u8]) -> Signature {
        self.keypair.sign_message(message)
    }

    fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }
}

pub trait ProofAccount {
    fn sign(&self, message: &[u8]) -> Signature;

    fn pubkey(&self) -> Pubkey;

    fn get_ae_key(&self) -> Result<AeKey> {
        let signature = self.sign(b"T0p_s3cr3t_a3_k3y");
        AeKey::new_from_signature(&signature).map_err(|_| anyhow::anyhow!("failed to create AeKey"))
    }

    fn get_pod_elgamal_keypair(&self) -> Result<ElGamalKeypair> {
        let signature = self.sign(b"T0p_s3cr3t_p0d_3lgamal_k3y");
        let elgamal_keypair = ElGamalKeypair::new_from_signature(&signature)
            .map_err(|_| anyhow::anyhow!("failed to create ElGamalKeypair"))?;

        Ok(elgamal_keypair)
    }

    fn get_pod_elgamal_pubkey(&self) -> Result<PodElGamalPubkey> {
        let elgamal_pubkey: PodElGamalPubkey = (*self.get_pod_elgamal_keypair()?.pubkey()).into();
        Ok(elgamal_pubkey)
    }

    fn decrypt_supply(&self, encrypted_supply: &PodAeCiphertext) -> Result<u64> {
        let decrypt_balance = self.get_ae_key()?.decrypt(&(*encrypted_supply).try_into()?);

        if let Some(balance) = decrypt_balance {
            return Ok(balance);
        }

        Err(anyhow::anyhow!("decrypt supply failed"))
    }

    fn encrypt_supply(&self, supply: u64) -> Result<PodAeCiphertext> {
        let encrypted_amount = self.get_ae_key()?.encrypt(supply);
        Ok(encrypted_amount.into())
    }

    fn encrypt_amount_ciphertext(&self, amount: u64) -> Result<PodElGamalCiphertext> {
        let eg = self.get_pod_elgamal_keypair()?;
        let ciphertext = eg.pubkey().encrypt(amount);
        Ok(ciphertext.into())
    }

    fn decrypt_amount_ciphertext(&self, encrypted_amount: &PodElGamalCiphertext) -> Result<u64> {
        let eg = self.get_pod_elgamal_keypair()?;
        let ciphertext = eg.secret().decrypt(&(*encrypted_amount).try_into()?);
        Ok(ciphertext.decode_u32().unwrap())
    }

    // fn decrypt_amount_ciphertext(&self, encrypted_amount: &PodElGamalCiphertext) -> Result<u64> {
    //     let eg = self.get_pod_elgamal_keypair()?;
    //     let ciphertext: ElGamalCiphertext = (*encrypted_amount)
    //         .try_into()
    //         .map_err(|e| anyhow::anyhow!("Failed to convert ciphertext: {:?}", e))?;

    //     let discrete_log = eg.secret().decrypt(&ciphertext);

    //     // Compute discrete logarithm up to a reasonable max (e.g., 2^48 or u64::MAX)
    //     let max = 1u64 << 48; // Covers amounts up to 2^48 (~281 trillion), sufficient for 100 * 10^9
    //     let decrypted_amount = discrete_log
    //         .set_compression_batch_size()
    //         .ok_or_else(|| anyhow::anyhow!("Failed to compute discrete log"))?;

    //     Ok(decrypted_amount)
    // }
}
