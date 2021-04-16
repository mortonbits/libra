//! 'query'
use crate::{
    config::OlCliConfig,
    node::metadata::Metadata,
    node::{
        account::{get_account_view, get_annotate_account_blob},
        chain_info,
    },
};

use cli::libra_client::LibraClient;
use libra_types::account_address::AccountAddress;
use num_format::{Locale, ToFormattedString};

#[derive(Debug)]
/// What query do we want to return
pub enum QueryType {
    /// Account balance
    Balance,
    /// Epoch and waypoint
    Epoch,
    /// Network block height
    BlockHeight,
    /// All account resources
    Resources,
    /// How far behind the local is from the upstream nodes
    SyncDelay,
}

/// Get data from a client, with a query type. Will connect to local only if in sync.
pub fn get(
    mut client: LibraClient,
    query_type: QueryType,
    account: AccountAddress,
    config: &OlCliConfig,
) -> String {
    use QueryType::*;
    match query_type {
        Balance => {
            let account_view = get_account_view(&mut client, account);
            for av in account_view.balances.iter() {
                if av.currency == "GAS" {
                    return av.amount.to_formatted_string(&Locale::en);
                }
            }
            "0".to_string()
        }
        BlockHeight => {
            let (chain, _) = chain_info::fetch_chain_info(&mut client);
            chain.unwrap().height.to_string()
        }
        Epoch => {
            let (chain, _) = chain_info::fetch_chain_info(&mut client);

            format!(
                "{} - WAYPOINT: {}",
                chain.clone().unwrap().epoch.to_string(),
                &chain.unwrap().waypoint.unwrap().to_string()
            )
        }
        SyncDelay => Metadata::compare_from_config(config).to_string(),
        Resources => {
            let resources = get_annotate_account_blob(client, account)
                .unwrap()
                .0
                .unwrap();

            format!("{:#?}", resources).to_string()
        }
    }
}

// fn get_account_view(account: AccountAddress) -> AccountView {
//     let (account_view, _) = pick_client()
//       .get_account(account, true)
//       .expect(&format!("could not get account at address {:?}", account));
//     account_view.expect(&format!("could not get account at address {:?}", account))
// }