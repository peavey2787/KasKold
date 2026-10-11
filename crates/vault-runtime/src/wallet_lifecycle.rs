use crate::{VaultCreation, VaultRuntime, VaultRuntimeError};
use hot_wallet::{CreatedWallet, HotWallet, HotWalletError, PLATFORM_WRAPPING_KEY_LEN};

impl VaultRuntime {
    /// Create and unlock a new 12-word Vault wallet.
    pub fn create_wallet_12(&mut self) -> Result<VaultCreation, HotWalletError> {
        self.install_created(HotWallet::create_12()?)
    }

    /// Create and unlock a new 24-word Vault wallet.
    pub fn create_wallet_24(&mut self) -> Result<VaultCreation, HotWalletError> {
        self.install_created(HotWallet::create_24()?)
    }

    /// Create a mnemonic wallet with mandatory Rust CSPRNG entropy plus the
    /// optional additive dice/touch inputs selected by the onboarding flow.
    pub fn create_wallet_with_additive_entropy(
        &mut self,
        word_count: u8,
        dice_rolls: &[u8],
        touch_transcript: &[u8],
        passphrase: &str,
    ) -> Result<VaultCreation, HotWalletError> {
        self.install_created(HotWallet::create_with_additive_entropy(
            word_count,
            dice_rolls,
            touch_transcript,
            passphrase,
        )?)
    }

    fn install_created(&mut self, created: CreatedWallet) -> Result<VaultCreation, HotWalletError> {
        let kpub = created.wallet.export_kpub()?;
        self.install_only_wallet(created.wallet);
        Ok(VaultCreation {
            recovery_phrase: created.recovery_phrase,
            kpub,
        })
    }

    /// Restore and unlock a wallet from an explicitly supplied recovery phrase.
    pub fn restore_wallet(
        &mut self,
        recovery_phrase: &str,
        passphrase: &str,
    ) -> Result<String, HotWalletError> {
        let wallet = HotWallet::restore(recovery_phrase, passphrase)?;
        let kpub = wallet.export_kpub()?;
        self.install_only_wallet(wallet);
        Ok(kpub)
    }

    /// Immediately drop the unlocked wallet. `HotWallet` owns zeroizing secrets.
    pub fn lock_wallet(&mut self) {
        self.clear_all_sessions();
        self.wallets.clear();
        self.wallet_names.clear();
        self.active_wallet = None;
    }

    /// Return an authenticated encrypted wallet blob suitable for native persistence.
    pub fn seal_wallet(
        &self,
        wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    ) -> Result<Vec<u8>, VaultRuntimeError> {
        self.active_wallet()?
            .seal_for_platform(wrapping_key)
            .map_err(VaultRuntimeError::Custody)
    }

    /// Unlock a previously sealed wallet using a short-lived platform key.
    pub fn unlock_sealed_wallet(
        &mut self,
        sealed_wallet: &[u8],
        wrapping_key: &[u8; PLATFORM_WRAPPING_KEY_LEN],
    ) -> Result<String, VaultRuntimeError> {
        let wallet = HotWallet::restore_platform_sealed(sealed_wallet, wrapping_key)
            .map_err(VaultRuntimeError::Custody)?;
        let kpub = wallet.export_kpub().map_err(VaultRuntimeError::Custody)?;
        self.install_only_wallet(wallet);
        Ok(kpub)
    }
}
