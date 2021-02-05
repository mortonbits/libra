///////////////////////////////////////////////////////////////////////////
// 0L Module
// ValidatorUniverse
///////////////////////////////////////////////////////////////////////////
// Stores all the validators who submitted a vdf proof.
// File Prefix for errors: 2201
///////////////////////////////////////////////////////////////////////////

address 0x1 {
  module ValidatorUniverse {
    use 0x1::Vector;
    use 0x1::Signer;
    use 0x1::CoreAddresses;
    use 0x1::MinerState;

    // resource for tracking the universe of accounts that have submitted a mined proof correctly, with the epoch number.
    resource struct ValidatorUniverse {
        validators: vector<address>
    }

    resource struct JailedBit {
        is_jailed: bool
    }

    // Genesis function to initialize ValidatorUniverse struct in 0x0.
    // This is triggered in new epoch by Configuration in Genesis.move
    // Function code: 01 Prefix: 220101
    public fun initialize(account: &signer){
      // Check for transactions sender is association
      let sender = Signer::address_of(account);
      assert(sender == CoreAddresses::LIBRA_ROOT_ADDRESS(), 220101014010);
      move_to<ValidatorUniverse>(account, ValidatorUniverse {
          validators: Vector::empty<address>()
      });
    }

    // This function is called to add validator to the validator universe.
    // Function code: 02 Prefix: 220102
    // TODO: This is public, anyone can add themselves to the validator universe.
    public fun add_validator(sender: &signer) acquires ValidatorUniverse {
      let addr = Signer::address_of(sender);
      // Miner can only add self to set if the mining is above a threshold.
      assert(MinerState::node_above_thresh(sender, addr), 220102014010);
      let state = borrow_global_mut<ValidatorUniverse>(CoreAddresses::LIBRA_ROOT_ADDRESS());
      let (in_set, _) = Vector::index_of<address>(&state.validators, &addr);
      if (!in_set) {
        Vector::push_back<address>(&mut state.validators, addr);
      }
    }

    // Permissions: Public, VM Only
    public fun remove_validator(vm: &signer, validator: address) acquires ValidatorUniverse {
      assert(Signer::address_of(vm) == CoreAddresses::LIBRA_ROOT_ADDRESS(), 220101014010);

      let state = borrow_global_mut<ValidatorUniverse>(CoreAddresses::LIBRA_ROOT_ADDRESS());
      let (in_set, index) = Vector::index_of<address>(&state.validators, &validator);
      if (in_set) {
        Vector::remove<address>(&mut state.validators, index);
      }
    }

    // A simple public function to query the EligibleValidators.
    // Function code: 03 Prefix: 220103
    public fun get_eligible_validators(vm: &signer): vector<address> acquires ValidatorUniverse {
      assert(Signer::address_of(vm) == CoreAddresses::LIBRA_ROOT_ADDRESS(), 220101014010);
      let state = borrow_global<ValidatorUniverse>(CoreAddresses::LIBRA_ROOT_ADDRESS());
      *&state.validators
    }

    public fun jail(vm: &signer, validator: address) acquires JailedBit{
      assert(Signer::address_of(vm) == CoreAddresses::LIBRA_ROOT_ADDRESS(), 220101014010);
      borrow_global_mut<JailedBit>(validator).is_jailed = true;
    }

    public fun un_jail(sender: &signer, validator: address) acquires JailedBit {
      // only a validator can un-jail themselves.
      assert(Signer::address_of(sender) == validator, 220101014010);
      
      if (!exists<JailedBit>(validator)) {
        move_to<JailedBit>(sender, JailedBit{
          is_jailed: false
        });
      };
      // check the node has been mining before unjailing.
      assert(MinerState::node_above_thresh(sender, validator), 220102014010);
      borrow_global_mut<JailedBit>(validator).is_jailed = false;
    }

    public fun is_jailed(validator: address): bool acquires JailedBit {
      borrow_global_mut<JailedBit>(validator).is_jailed
    }

    public fun genesis_helper(vm: &signer, validator: address) acquires ValidatorUniverse {
      assert(Signer::address_of(vm) == CoreAddresses::LIBRA_ROOT_ADDRESS(), 220101014010);
      // let addr = Signer::address_of(sender);
      // MinerState::node_above_thresh(sender, addr);
      let state = borrow_global_mut<ValidatorUniverse>(CoreAddresses::LIBRA_ROOT_ADDRESS());
      let (in_set, _) = Vector::index_of<address>(&state.validators, &validator);
      if (!in_set) {
        Vector::push_back<address>(&mut state.validators, validator);
      }
    }
  }
}