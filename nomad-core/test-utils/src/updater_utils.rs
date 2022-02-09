use ethers_core::types::{Address, Signature, H160, H256};
use ethers_signers::{LocalWallet, Signer};
use sha3::{digest::Update as DigestUpdate, Digest, Keccak256};

#[derive(Debug, Clone)]
pub struct Updater {
    pub local_domain: u32,
    pub signer: LocalWallet,
    pub address: Address,
}

#[derive(Debug, Clone)]
pub struct Update {
    pub origin: u32,
    pub old_root: H256,
    pub new_root: H256,
    pub signature: Signature,
}

impl Updater {
    pub fn from_privkey(privkey: &str, domain: u32) -> Self {
        let wallet: LocalWallet = privkey.parse().unwrap();
        Self::from_wallet(wallet, domain)
    }

    pub fn from_wallet(wallet: LocalWallet, domain: u32) -> Self {
        Self {
            local_domain: domain,
            signer: wallet.clone(),
            address: wallet.address(),
        }
    }

    pub fn address(&self) -> H160 {
        self.address.into()
    }

    fn domain_hash(&self) -> H256 {
        H256::from_slice(
            Keccak256::new()
                .chain(self.local_domain.to_be_bytes())
                .chain("NOMAD".as_bytes())
                .finalize()
                .as_slice(),
        )
    }

    fn message_hash(&self, old_root: H256, new_root: H256) -> H256 {
        H256::from_slice(
            Keccak256::new()
                .chain(self.domain_hash())
                .chain(old_root)
                .chain(new_root)
                .finalize()
                .as_slice(),
        )
    }

    pub async fn sign_update(
        &self,
        old_root: H256,
        new_root: H256,
    ) -> Result<Update, <LocalWallet as Signer>::Error> {
        let message_hash = self.message_hash(old_root, new_root);
        Ok(Update {
            origin: self.local_domain,
            old_root: H256::from(old_root),
            new_root: H256::from(new_root),
            signature: self.signer.sign_message(message_hash).await?,
        })
    }
}
