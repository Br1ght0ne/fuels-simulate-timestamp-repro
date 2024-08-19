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
use pretty_assertions::assert_eq;
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
    pub async fn get_timestamp_simulate(&self) -> anyhow::Result<u64> {
        Ok(self
            .instance
            .methods()
            .get_timestamp()
            .simulate()
            .await?
            .value)
    }

    pub async fn get_timestamp_call(&self) -> anyhow::Result<u64> {
        Ok(self.instance.methods().get_timestamp().call().await?.value)
    }

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
async fn debug() -> Result<(), anyhow::Error> {
    // Initialize wallets
    let wallets = init_wallets().await;

    // Users
    let admin = &wallets[0];
    let bob = &wallets[1];

    // Deploy contract
    let contract = ExampleContract::deploy(admin).await?;

    let time0_simulate = contract.get_timestamp_simulate().await?;
    let time0_call = contract.get_timestamp_call().await?;

    // Create 10 blocks (10 seconds)
    admin.try_provider()?.produce_blocks(10, None).await?;

    let time1_simulate = contract.get_timestamp_simulate().await?;
    let time1_call = contract.get_timestamp_call().await?;
    assert_eq!(
        time1_call,
        time0_call + 11,
        "should have increased timestamp between calls"
    );
    assert_eq!(
        time1_simulate,
        time0_simulate + 11,
        "should have increased timestamp between simulations"
    );

    bob.try_provider()?.produce_blocks(10, None).await?;
    Ok(())
}
