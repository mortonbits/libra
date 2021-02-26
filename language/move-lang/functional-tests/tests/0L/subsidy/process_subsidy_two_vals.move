// ALICE is CASE 1
//! account: alice, 1, 0, validator

// BOB is CASE 4
//! account: bob, 1, 0, validator

// CAROL is CASE 1 AS WELL
//! account: carol, 1, 0, validator

// DAVE is CASE 4
//! account: dave, 1, 0, validator

//! new-transaction
//! sender: alice
script {
    use 0x1::MinerState;
    fun main(sender: &signer) {
      //NOTE: Alice is Case 1, she validates and mines. Setting up mining.
        MinerState::test_helper_mock_mining(sender, 5);

    }
}
//check: EXECUTED


//! new-transaction
//! sender: carol
script {
    use 0x1::MinerState;
    fun main(sender: &signer) {
      //NOTE: Carol is Case 3, she mines but does not validate. Setting up mining.
        MinerState::test_helper_mock_mining(sender, 5);

    }
}
//check: EXECUTED

//! new-transaction
//! sender: diemroot
script {
  use 0x1::Vector;
  use 0x1::Stats;
  use 0x1::GAS::GAS;
  use 0x1::DiemAccount;
  use 0x1::Cases;

  fun main(vm: &signer) {
    // check the case of a network density of 4 active validators.

    let validators = Vector::singleton<address>({{alice}});
    Vector::push_back(&mut validators, {{carol}});

    // create mock validator stats for full epoch
    let i = 0;
    while (i < 16) {
      Stats::process_set_votes(vm, &validators);
      i = i + 1;
    };

    assert(DiemAccount::balance<GAS>({{alice}}) == 1, 7357190102011000);
    assert(DiemAccount::balance<GAS>({{bob}}) == 1, 7357190102021000);
    assert(DiemAccount::balance<GAS>({{carol}}) == 1, 7357190102031000);
    assert(DiemAccount::balance<GAS>({{dave}}) == 1, 7357190102041000);

    assert(Cases::get_case(vm, {{alice}}, 0, 15) == 1, 7357190102051000);
    assert(Cases::get_case(vm, {{bob}}, 0, 15) == 4, 7357190102061000);
    assert(Cases::get_case(vm, {{carol}}, 0, 15) == 1, 7357190102071000);
    assert(Cases::get_case(vm, {{dave}}, 0, 15) == 4, 7357190102081000);
  }
}
// check: EXECUTED


//! new-transaction
//! sender: diemroot
script {
  use 0x1::Subsidy;
  use 0x1::GAS::GAS;
  use 0x1::DiemAccount;
  use 0x1::DiemSystem;
  use 0x1::Debug::print;

  fun main(vm: &signer) {
    let (validators, fee_ratios) = DiemSystem::get_fee_ratio(vm, 0, 15);
    Subsidy::process_subsidy(vm, 100, &validators, &fee_ratios);
    print(&DiemAccount::balance<GAS>({{alice}}));
    assert(DiemAccount::balance<GAS>({{alice}}) == 51, 7357190102091000);
    assert(DiemAccount::balance<GAS>({{bob}}) == 1, 7357190102101000);
    assert(DiemAccount::balance<GAS>({{carol}}) == 51, 7357190102111000);
    assert(DiemAccount::balance<GAS>({{dave}}) == 1, 7357190102121000);

  }
}
// check: EXECUTED
