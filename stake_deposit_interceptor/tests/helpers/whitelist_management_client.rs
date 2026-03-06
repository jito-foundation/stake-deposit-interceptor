use jito_bytemuck::AccountDeserialize;
use jito_whitelist_management_client::{
    instructions::{
        AddAdminBuilder, AddToWhitelistBuilder, InitializeWhitelistBuilder, RemoveAdminBuilder,
        RemoveFromWhitelistBuilder,
    },
    programs::JITO_WHITELIST_MANAGEMENT_ID,
};
use jito_whitelist_management_core::whitelist::Whitelist;
use solana_commitment_config::CommitmentLevel;
use solana_keypair::{Keypair, Signer};
use solana_program_test::BanksClient;
use solana_pubkey::Pubkey;
use solana_transaction::Transaction;

pub struct WhitelistManagementProgramClient {
    /// Banks client
    pub banks_client: BanksClient,

    /// Payer keypair
    payer: Keypair,
}

impl WhitelistManagementProgramClient {
    #[allow(dead_code)]
    pub const fn new(banks_client: BanksClient, payer: Keypair) -> Self {
        Self {
            banks_client,
            payer,
        }
    }

    #[allow(dead_code)]
    pub async fn get_whitelist(&mut self) -> Whitelist {
        let pda = self.get_whitelist_pda();
        let account = self.banks_client.get_account(pda).await.unwrap().unwrap();
        let whitelist =
            jito_whitelist_management_core::whitelist::Whitelist::try_from_slice_unchecked(
                &account.data,
            )
            .unwrap();
        *whitelist
    }

    pub fn get_whitelist_pda(&self) -> Pubkey {
        Pubkey::new_from_array(
            jito_whitelist_management_core::whitelist::Whitelist::find_program_address(
                &JITO_WHITELIST_MANAGEMENT_ID,
            )
            .0
            .to_bytes(),
        )
    }

    #[allow(dead_code)]
    pub async fn do_initialize_whitelist(&mut self, initial_admin: Pubkey) -> () {
        let whitelist_pda = self.get_whitelist_pda();

        self.initialize_whitelist(whitelist_pda, initial_admin)
            .await
    }

    #[allow(dead_code)]
    pub async fn initialize_whitelist(&mut self, whitelist: Pubkey, initial_admin: Pubkey) {
        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = InitializeWhitelistBuilder::new()
            .payer(self.payer.pubkey())
            .whitelist(whitelist)
            .initial_admin(initial_admin)
            .instruction();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    #[allow(dead_code)]
    pub async fn do_add_admin(&mut self, admin: &Keypair, new_admin: Pubkey) {
        let whitelist_pda = self.get_whitelist_pda();

        self.add_admin(admin, whitelist_pda, new_admin).await
    }

    #[allow(dead_code)]
    pub async fn add_admin(&mut self, admin: &Keypair, whitelist: Pubkey, new_admin: Pubkey) {
        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = AddAdminBuilder::new()
            .admin(admin.pubkey())
            .whitelist(whitelist)
            .new_admin(new_admin)
            .instruction();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            blockhash,
        ))
        .await
    }

    #[allow(dead_code)]
    pub async fn do_remove_admin(&mut self, admin: &Keypair, admin_to_remove: Pubkey) {
        let whitelist_pda = self.get_whitelist_pda();

        self.remove_admin(admin, whitelist_pda, admin_to_remove)
            .await
    }

    #[allow(dead_code)]
    pub async fn remove_admin(
        &mut self,
        admin: &Keypair,
        whitelist: Pubkey,
        admin_to_remove: Pubkey,
    ) {
        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = RemoveAdminBuilder::new()
            .admin(admin.pubkey())
            .whitelist(whitelist)
            .admin_to_remove(admin_to_remove)
            .instruction();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            blockhash,
        ))
        .await
    }

    #[allow(dead_code)]
    pub async fn do_add_to_whitelist(&mut self, admin: &Keypair, signer_to_add: Pubkey) {
        let whitelist_pda = self.get_whitelist_pda();

        self.add_to_whitelist(admin, whitelist_pda, signer_to_add)
            .await
    }

    #[allow(dead_code)]
    pub async fn add_to_whitelist(
        &mut self,
        admin: &Keypair,
        whitelist: Pubkey,
        signer_to_add: Pubkey,
    ) {
        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = AddToWhitelistBuilder::new()
            .admin(admin.pubkey())
            .whitelist(whitelist)
            .signer_to_add(signer_to_add)
            .instruction();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            blockhash,
        ))
        .await
    }

    #[allow(dead_code)]
    pub async fn do_remove_from_whitelist(&mut self, admin: &Keypair, signer_to_remove: Pubkey) {
        let whitelist_pda = self.get_whitelist_pda();

        self.remove_from_whitelist(admin, whitelist_pda, signer_to_remove)
            .await
    }

    #[allow(dead_code)]
    pub async fn remove_from_whitelist(
        &mut self,
        admin: &Keypair,
        whitelist: Pubkey,
        signer_to_remove: Pubkey,
    ) {
        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = RemoveFromWhitelistBuilder::new()
            .admin(admin.pubkey())
            .whitelist(whitelist)
            .signer_to_remove(signer_to_remove)
            .instruction();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            blockhash,
        ))
        .await
    }

    pub async fn process_transaction(&mut self, tx: &Transaction) {
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                tx.clone(),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();
    }
}
