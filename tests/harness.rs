use std::time::Duration;

use anyhow::Ok;
use fuels::{
    accounts::{wallet::WalletUnlocked, ViewOnlyAccount},
    macros::abigen,
    programs::{
        contract::{Contract, LoadConfiguration, StorageConfiguration},
        responses::CallResponse,
    },
    test_helpers::{launch_custom_provider_and_get_wallets, NodeConfig, Trigger, WalletsConfig},
    types::transaction::TxPolicies,
};
use rand::Rng;

abigen!(Contract(
    name = "Example",
    abi = "out/release/example-abi.json",
));

pub struct ExampleContract {
    instance: Example<WalletUnlocked>,
}

impl ExampleContract {
    pub async fn deploy(wallet: &WalletUnlocked) -> anyhow::Result<Self> {
        let storage_configuration = StorageConfiguration::default()
            .add_slot_overrides_from_file("out/release/example-storage_slots.json")?;

        let config = LoadConfiguration::default().with_storage_configuration(storage_configuration);

        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();

        let id = Contract::load_from("out/release/example.bin", config)?
            .with_salt(salt)
            .deploy(wallet, TxPolicies::default())
            .await?;

        let example_contract = Example::new(id, wallet.clone());

        Ok(Self {
            instance: example_contract,
        })
    }

    pub async fn with_account(&self, account: &WalletUnlocked) -> anyhow::Result<Self> {
        Ok(Self {
            instance: Example::new(self.instance.contract_id().clone(), account.clone()),
        })
    }

    // Contract methods
    pub async fn get_last_update_time(&self) -> anyhow::Result<u64> {
        Ok(self
            .instance
            .methods()
            .get_last_update_time()
            .simulate()
            .await?
            .value)
    }

    pub async fn refrest_last_update_time(&self) -> anyhow::Result<CallResponse<()>> {
        Ok(self
            .instance
            .methods()
            .refrest_last_update_time()
            .call()
            .await?)
    }

    pub async fn check_if_current_time_older_than_last_update_time(
        &self,
    ) -> anyhow::Result<CallResponse<()>> {
        Ok(self
            .instance
            .methods()
            .check_if_current_time_older_than_last_update_time()
            .simulate()
            .await?)
    }
}

// Function to initialize the wallets
pub async fn init_wallets() -> Vec<WalletUnlocked> {
    let wallets_config = WalletsConfig::new(Some(3), Some(1), Some(1_000_000_000));

    let provider_config = NodeConfig {
        block_production: Trigger::Interval {
            block_time: Duration::from_secs(1),
        },
        ..NodeConfig::default()
    };

    launch_custom_provider_and_get_wallets(wallets_config, Some(provider_config), None)
        .await
        .unwrap()
}

#[tokio::test]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize wallets
    let wallets = init_wallets().await;

    // Users
    let admin = &wallets[0];
    let bob = &wallets[1];

    // Deploy contract
    let contract = ExampleContract::deploy(admin).await?;

    // Create 10 blocks (10 seconds)
    admin.try_provider()?.produce_blocks(10, None).await?;

    // Call 1 - get_last_update_time (should be 0)
    let last_update_time = contract
        .with_account(admin)
        .await?
        .get_last_update_time()
        .await?;

    println!("last_update_time: {}", last_update_time);
    assert!(last_update_time == 0);

    // Create 10 blocks (10 seconds) - use admin provider to get the latest block time
    admin.try_provider()?.produce_blocks(10, None).await?;

    // Call 2 - refrest_last_update_time
    contract
        .with_account(admin)
        .await?
        .refrest_last_update_time()
        .await?;

    // Create 10 blocks (10 seconds) - use admin provider to get the latest block time
    admin.try_provider()?.produce_blocks(10, None).await?;

    // Call 3 - get_last_update_time (should be 10)
    let last_update_time = contract
        .with_account(admin)
        .await?
        .get_last_update_time()
        .await?;

    println!("last_update_time: {}", last_update_time);

    // Call 4 - check_if_current_time_older_than_last_update_time (should succeed, but reverts)
    // Maybe there is a bug here, because `.simulate()` is used ?`
    contract
        .with_account(admin)
        .await?
        .check_if_current_time_older_than_last_update_time()
        .await?;

    // This also fails
    // Call 5 - check_if_current_time_older_than_last_update_time (should succeed, but reverts)
    contract
        .with_account(bob)
        .await?
        .check_if_current_time_older_than_last_update_time()
        .await?;

    // Call 6 (this does not revert) - using `call()` instead of `simulate()`
    contract
        .instance
        .methods()
        .check_if_current_time_older_than_last_update_time()
        .call()
        .await?;

    Ok(())
}
