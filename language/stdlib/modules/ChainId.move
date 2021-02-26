address 0x1 {
/// The chain id distinguishes between different chains (e.g., testnet and the main Diem network).
/// One important role is to prevent transactions intended for one chain from being executed on another.
/// This code provides a container for storing a chain id and functions to initialize and get it.
module ChainId {
    use 0x1::CoreAddresses;
    use 0x1::Errors;
    use 0x1::DiemTimestamp;
    use 0x1::Signer;

    resource struct ChainId {
        id: u8
    }

    /// The `ChainId` resource was not in the required state
    const ECHAIN_ID: u64 = 0;

    /// Publish the chain ID `id` of this Diem instance under the DiemRoot account
    public fun initialize(lr_account: &signer, id: u8) {
        DiemTimestamp::assert_genesis();
        CoreAddresses::assert_diem_root(lr_account);
        assert(!exists<ChainId>(Signer::address_of(lr_account)), Errors::already_published(ECHAIN_ID));
        move_to(lr_account, ChainId { id })
    }

    /// Return the chain ID of this Diem instance
    public fun get(): u8 acquires ChainId {
        DiemTimestamp::assert_operating();
        borrow_global<ChainId>(CoreAddresses::LIBRA_ROOT_ADDRESS()).id
    }

    // =================================================================
    // Module Specification

    spec module {} // Switch to module documentation context

    /// # Initialization

    /// When Diem is operating, the chain id is always available.
    spec module {
        invariant [global] DiemTimestamp::is_operating() ==> exists<ChainId>(CoreAddresses::LIBRA_ROOT_ADDRESS());
    }

    /// # Helper Functions

    spec define spec_get_chain_id(): u8 {
        global<ChainId>(CoreAddresses::LIBRA_ROOT_ADDRESS()).id
    }
}
}
