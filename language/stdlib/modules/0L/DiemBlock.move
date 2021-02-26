address 0x1 {

/// This module defines a struct storing the metadata of the block and new block events.
module DiemBlock {
    use 0x1::CoreAddresses;
    use 0x1::Errors;
    use 0x1::Event;
    use 0x1::DiemSystem;
    use 0x1::DiemTimestamp;

    //////// 0L ////////
    use 0x1::Reconfigure;
    use 0x1::Stats;
    use 0x1::AutoPay;
    use 0x1::Epoch;

    resource struct BlockMetadata {
        /// Height of the current block
        height: u64,
        /// Handle where events with the time of new blocks are emitted
        new_block_events: Event::EventHandle<Self::NewBlockEvent>,
    }

    struct NewBlockEvent {
        round: u64,
        proposer: address,
        previous_block_votes: vector<address>,

        /// On-chain time during  he block at the given height
        time_microseconds: u64,
    }

    /// The `BlockMetadata` resource is in an invalid state
    const EBLOCK_METADATA: u64 = 0;
    /// An invalid signer was provided. Expected the signer to be the VM or a Validator.
    const EVM_OR_VALIDATOR: u64 = 1;

    /// This can only be invoked by the Association address, and only a single time.
    /// Currently, it is invoked in the genesis transaction
    public fun initialize_block_metadata(account: &signer) {
        DiemTimestamp::assert_genesis();
        // Operational constraint, only callable by the Association address
        CoreAddresses::assert_diem_root(account);

        assert(!is_initialized(), Errors::already_published(EBLOCK_METADATA));
        move_to<BlockMetadata>(
            account,
            BlockMetadata {
                height: 0,
                new_block_events: Event::new_event_handle<Self::NewBlockEvent>(account),
            }
        );
    }
    spec fun initialize_block_metadata {
        include DiemTimestamp::AbortsIfNotGenesis;
        include CoreAddresses::AbortsIfNotDiemRoot;
        aborts_if is_initialized() with Errors::ALREADY_PUBLISHED;
        ensures is_initialized();
        ensures get_current_block_height() == 0;
    }

    /// Helper function to determine whether this module has been initialized.
    fun is_initialized(): bool {
        exists<BlockMetadata>(CoreAddresses::LIBRA_ROOT_ADDRESS())
    }

    /// Set the metadata for the current block.
    /// The runtime always runs this before executing the transactions in a block.
    fun block_prologue(
        vm: &signer,
        round: u64,
        timestamp: u64,
        previous_block_votes: vector<address>,
        proposer: address
    ) acquires BlockMetadata {
        DiemTimestamp::assert_operating();
        // Operational constraint: can only be invoked by the VM.
        CoreAddresses::assert_vm(vm);

        // Authorization
        assert(
            proposer == CoreAddresses::VM_RESERVED_ADDRESS() || DiemSystem::is_validator(proposer),
            Errors::requires_address(EVM_OR_VALIDATOR)
        );
        //////// 0L ////////
        // increment stats
        Stats::process_set_votes(vm, &previous_block_votes);
        Stats::inc_prop(vm, *&proposer);
        
        if (AutoPay::tick(vm)){
            AutoPay::process_autopay(vm);
        };

        ///////////////////

        let block_metadata_ref = borrow_global_mut<BlockMetadata>(CoreAddresses::LIBRA_ROOT_ADDRESS());
        DiemTimestamp::update_global_time(vm, proposer, timestamp);
        block_metadata_ref.height = block_metadata_ref.height + 1;
        Event::emit_event<NewBlockEvent>(
            &mut block_metadata_ref.new_block_events,
            NewBlockEvent {
                round,
                proposer,
                previous_block_votes,
                time_microseconds: timestamp,
            }
        );

         //////// 0L ////////
        // reconfigure
        if (Epoch::epoch_finished()) {
          // TODO: We don't need to pass block height to ReconfigureOL. It should use the BlockMetadata. But there's a circular reference there when we try.
          Reconfigure::reconfigure(vm, get_current_block_height());
        }
    }
    spec fun block_prologue {
        include DiemTimestamp::AbortsIfNotOperating;
        include CoreAddresses::AbortsIfNotVM{account: vm};
        aborts_if proposer != CoreAddresses::VM_RESERVED_ADDRESS() && !DiemSystem::spec_is_validator(proposer)
            with Errors::REQUIRES_ADDRESS;
        ensures DiemTimestamp::spec_now_microseconds() == timestamp;
        ensures get_current_block_height() == old(get_current_block_height()) + 1;

        /// The below counter overflow is assumed to be excluded from verification of callers.
        aborts_if [assume] get_current_block_height() + 1 > MAX_U64 with EXECUTION_FAILURE;
    }

    /// Get the current block height
    public fun get_current_block_height(): u64 acquires BlockMetadata {
        assert(is_initialized(), Errors::not_published(EBLOCK_METADATA));
        borrow_global<BlockMetadata>(CoreAddresses::LIBRA_ROOT_ADDRESS()).height
    }

    spec module { } // Switch documentation context to module level.

    /// # Initialization
    spec module {
        invariant [global] DiemTimestamp::is_operating() ==> is_initialized();
    }
}

}
