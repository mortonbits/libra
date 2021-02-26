//! account: alice, 1000000GAS
//! account: bob, 10000GAS

// We test processing of autopay at differnt epochs and balance transfers
// Finally, we also check the end_epoch functionality of autopay

// creating the instruction
//! new-transaction
//! sender: alice
script {
  use 0x1::AutoPay;
  use 0x1::Signer;
  fun main(sender: &signer) {
    AutoPay::enable_autopay(sender);
    assert(AutoPay::is_enabled(Signer::address_of(sender)), 0);
    AutoPay::create_instruction(sender, 1, {{bob}}, 2, 5);
    let (payee, end_epoch, percentage) = AutoPay::query_instruction(Signer::address_of(sender), 1);
    assert(payee == {{bob}}, 1);
    assert(end_epoch == 2, 1);
    assert(percentage == 5, 1);
    }
}
// check: EXECUTED

// Processing AutoPay to see if payments are done
//! new-transaction
//! sender: diemroot
script {
  use 0x1::AutoPay;
  use 0x1::DiemAccount;
  use 0x1::GAS::GAS;
  fun main(sender: &signer) {
    let alice_balance = DiemAccount::balance<GAS>({{alice}});
    let bob_balance = DiemAccount::balance<GAS>({{bob}});
    assert(alice_balance==1000000, 1);
    AutoPay::process_autopay(sender);
    
    let alice_balance_after = DiemAccount::balance<GAS>({{alice}});
    assert(alice_balance_after < alice_balance, 2);
    
    let transferred = alice_balance - alice_balance_after;
    let bob_received = DiemAccount::balance<GAS>({{bob}}) - bob_balance;
    assert(bob_received == transferred, 2);
    }
}
// check: EXECUTED