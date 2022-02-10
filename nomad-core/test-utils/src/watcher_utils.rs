use ethers_core::types::{Signature, H160, H256};
use ethers_signers::{LocalWallet, Signer};
use sha3::{digest::Update as DigestUpdate, Digest, Keccak256};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailureNotification {
    /// Domain of failed home
    pub home_domain: u32,
    /// Failed home's updater (as bytes32)
    pub updater: H256,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignedFailureNotification {
    /// Failure notification
    pub notification: FailureNotification,
    /// Signature
    pub signature: Signature,
}

#[derive(Debug, Clone)]
pub struct Watcher {
    pub home_domain: u32,
    pub signer: LocalWallet,
}

impl Watcher {
    pub fn from_privkey(privkey: &str, home_domain: u32) -> Self {
        let wallet: LocalWallet = privkey.parse().unwrap();
        Self::from_wallet(wallet, home_domain)
    }

    pub fn from_wallet(wallet: LocalWallet, home_domain: u32) -> Self {
        Self {
            home_domain,
            signer: wallet,
        }
    }

    pub fn address(&self) -> H160 {
        self.signer.address()
    }

    fn domain_hash(&self) -> H256 {
        H256::from_slice(
            Keccak256::new()
                .chain(self.home_domain.to_be_bytes())
                .chain("NOMAD".as_bytes())
                .finalize()
                .as_slice(),
        )
    }

    fn message_hash(&self, updater: H256) -> H256 {
        H256::from_slice(
            Keccak256::new()
                .chain(self.domain_hash())
                .chain(self.home_domain.to_be_bytes())
                .chain(updater.as_ref())
                .finalize()
                .as_slice(),
        )
    }

    pub async fn sign_failure_notification(
        &self,
        updater: H256,
    ) -> Result<SignedFailureNotification, <LocalWallet as Signer>::Error> {
        let message_hash = self.message_hash(updater);
        Ok(SignedFailureNotification {
            notification: FailureNotification {
                home_domain: self.home_domain,
                updater,
            },
            signature: self.signer.sign_message(message_hash).await?,
        })
    }
}
