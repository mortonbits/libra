//! `IpAddrUpdate` subcommand

#![allow(clippy::never_loop)]

use abscissa_core::{Command, Options, Runnable};
use diem_json_rpc_types::views::TransactionView;
use diem_transaction_builder::stdlib as transaction_builder;
use ol_types::config::TxType;
use crate::{entrypoint, submit_tx::{TxError, TxParams, maybe_submit, tx_params_wrapper}};
use std::process::exit;
use std::path::PathBuf;
use ol_types::dialogue::what_ip;
use diem_types::account_address::AccountAddress;


/// `IpAddrUpdate` subcommand
#[derive(Command, Debug, Default, Options)]
pub struct IpAddrUpdateCmd {

}

impl Runnable for IpAddrUpdateCmd {
    fn run(&self) {
        let entry_args = entrypoint::get_args();
        let tx_params = tx_params_wrapper(TxType::Mgmt).unwrap();
        let validator_account = tx_params.signer_address;
        let ip = what_ip();
        let validator_network_address = ip.unwrap().octets().to_vec();
        // set fullnode_network_address to the same value:
        let fullnode_network_address = validator_network_address.clone();

        match update_ipaddr(
            validator_account,
            validator_network_address,
            fullnode_network_address,
            &tx_params,
            entry_args.save_path
        ) {
            Ok(r) => {
              println!("Successfully updated on-chain ip address: {:?}", &r);
            },
            Err(e) => {
              println!("ERROR: could not update on-chain ip address: {:?}", &e);
              exit(1);
            },
        }
    }
}

/// perform tx to update validator's registered ip address on-chain
pub fn update_ipaddr(
    validator_account: AccountAddress,
    validator_network_address: Vec<u8>,
    fullnode_network_address: Vec<u8>,
    tx_params: &TxParams,
    save_path: Option<PathBuf> ) -> Result<TransactionView, TxError> {

    let script =
        transaction_builder::encode_register_validator_config_script_function(
            validator_account,
            tx_params.auth_key.to_vec(),
            validator_network_address,
            fullnode_network_address );

    maybe_submit(
        script,
        &tx_params,
        save_path
  )
}
