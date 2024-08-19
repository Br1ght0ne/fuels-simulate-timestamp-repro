contract;

pub struct ExampleStruct {
    last_update_time: u64,
}

impl ExampleStruct {
    pub fn default() -> Self {
        Self {
            last_update_time: 0,
        }
    }
}

storage {
    example_struct: ExampleStruct = ExampleStruct::default(),
}

abi Example {
    fn get_timestamp() -> u64;

    #[storage(read)]
    fn get_last_update_time() -> u64;

    #[storage(read, write)]
    fn refrest_last_update_time();

    #[storage(read)]
    fn check_if_current_time_older_than_last_update_time();
}

impl Example for Contract {
    fn get_timestamp() -> u64 {
        timestamp()
    }

    #[storage(read)]
    fn get_last_update_time() -> u64 {
        storage.example_struct.read().last_update_time
    }

    #[storage(read, write)]
    fn refrest_last_update_time() {
        storage.example_struct.last_update_time.write(timestamp());
    }

    #[storage(read)]
    fn check_if_current_time_older_than_last_update_time() {
        let last_update_time = storage.example_struct.last_update_time.read();

        let now = timestamp();

        if now < last_update_time {
            // Revert with the difference between the last update time and the current time 
            // This can maybe help in debugging
            revert(last_update_time - now);
        }
    }
}

fn timestamp() -> u64 {
    std::block::timestamp()
}
