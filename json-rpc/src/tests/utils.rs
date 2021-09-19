// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Error, Result};
use diem_config::{
    config::{
        RoleType, DEFAULT_BATCH_SIZE_LIMIT, DEFAULT_CONTENT_LENGTH_LIMIT, DEFAULT_PAGE_SIZE_LIMIT,
    },
    utils,
};
use diem_crypto::HashValue;
use diem_mempool::{MempoolClientSender, SubmissionStatus};

use diem_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    account_state_blob::{AccountStateBlob, AccountStateWithProof},
    block_info::BlockInfo,
    chain_id::ChainId,
    contract_event::{ContractEvent, EventWithProof},
    epoch_change::EpochChangeProof,
    event::EventKey,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::{
        AccumulatorConsistencyProof, AccumulatorRangeProof, SparseMerkleProof,
        TransactionAccumulatorProof, TransactionInfoWithProof, TransactionListProof,
    },
    transaction::{
        SignedTransaction, Transaction, TransactionInfo, TransactionListWithProof,
        TransactionWithProof, Version,
    },
    vm_status::KeptVMStatus,
};
use diemdb::test_helper::arb_blocks_to_commit;

use crate::tests::genesis::generate_genesis_state;
use diem_client::BlockingClient;
use diem_proptest_helpers::ValueGenerator;
use diem_types::account_config::FreezingBit;
use futures::channel::{
    mpsc::{channel, Receiver},
    oneshot,
};
use move_core_types::{
    language_storage::TypeTag,
    move_resource::MoveResource,
    value::{MoveStructLayout, MoveTypeLayout},
};
use move_vm_types::values::{Struct, Value};
use proptest::prelude::*;
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    net::SocketAddr,
    sync::Arc,
};
use storage_interface::{DbReader, Order, StartupInfo, TreeState};
use tokio::runtime::Runtime;

/// Creates JSON RPC server for a Validator node
/// Should only be used for unit-tests
#[allow(unused)]
pub fn test_bootstrap(
    address: SocketAddr,
    diem_db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> Runtime {
    crate::bootstrap(
        address,
        DEFAULT_BATCH_SIZE_LIMIT,
        DEFAULT_PAGE_SIZE_LIMIT,
        DEFAULT_CONTENT_LENGTH_LIMIT,
        &None,
        &None,
        diem_db,
        mp_sender,
        RoleType::Validator,
        ChainId::test(),
    )
}

/// Lightweight mock of DiemDB
#[derive(Clone)]
#[allow(unused)]
pub struct MockDiemDB {
    pub version: u64,
    pub genesis: HashMap<AccountAddress, AccountStateBlob>,
    pub all_accounts: HashMap<AccountAddress, AccountStateBlob>,
    pub all_txns: Vec<(Transaction, KeptVMStatus)>,
    pub events: Vec<(u64, ContractEvent)>,
    pub account_state_with_proof: Vec<AccountStateWithProof>,
    pub timestamps: Vec<u64>,
}

impl DbReader for MockDiemDB {
    fn get_latest_account_state(
        &self,
        address: AccountAddress,
    ) -> Result<Option<AccountStateBlob>> {
        if let Some(blob) = self.genesis.get(&address) {
            Ok(Some(blob.clone()))
        } else if let Some(blob) = self.all_accounts.get(&address) {
            Ok(Some(blob.clone()))
        } else {
            Ok(None)
        }
    }

    fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures> {
        Ok(LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::new(
                    0,
                    self.version,
                    HashValue::zero(),
                    HashValue::zero(),
                    self.version,
                    self.get_block_timestamp(self.version).unwrap(),
                    None,
                ),
                HashValue::zero(),
            ),
            BTreeMap::new(),
        ))
    }

    fn get_txn_by_account(
        &self,
        address: AccountAddress,
        seq_num: u64,
        _ledger_version: u64,
        fetch_events: bool,
    ) -> Result<Option<TransactionWithProof>, Error> {
        Ok(self
            .all_txns
            .iter()
            .enumerate()
            .find(|(_, (x, _))| {
                if let Ok(t) = x.as_signed_user_txn() {
                    t.sender() == address && t.sequence_number() == seq_num
                } else {
                    false
                }
            })
            .map(|(v, (x, status))| TransactionWithProof {
                version: v as u64,
                transaction: x.clone(),
                events: if fetch_events {
                    Some(
                        self.events
                            .iter()
                            .filter(|(ev, _)| *ev == v as u64)
                            .map(|(_, e)| e)
                            .cloned()
                            .collect(),
                    )
                } else {
                    None
                },
                proof: TransactionInfoWithProof::new(
                    TransactionAccumulatorProof::new(vec![]),
                    TransactionInfo::new(
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        0,
                        status.clone(),
                    ),
                ),
            }))
    }

    fn get_transactions(
        &self,
        start_version: u64,
        limit: u64,
        ledger_version: u64,
        fetch_events: bool,
    ) -> Result<TransactionListWithProof, Error> {
        // ensure inputs are validated before we enter mock DB
        assert!(
            start_version <= ledger_version,
            "start_version: {}, ledger_version: {}",
            start_version,
            ledger_version
        );
        assert!(limit > 0, "limit: {}", limit);
        let limit = std::cmp::min(limit, ledger_version - start_version + 1);
        let mut transactions = vec![];
        let mut txn_infos = vec![];
        self.all_txns
            .iter()
            .skip(start_version as usize)
            .take(limit as usize)
            .for_each(|(t, status)| {
                transactions.push(t.clone());
                txn_infos.push(TransactionInfo::new(
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    0,
                    status.clone(),
                ));
            });
        let first_transaction_version = transactions.first().map(|_| start_version);
        let proof = TransactionListProof::new(AccumulatorRangeProof::new_empty(), txn_infos);

        let events = if fetch_events {
            let events = (start_version..start_version + transactions.len() as u64)
                .map(|version| {
                    self.events
                        .iter()
                        .filter(|(v, _)| *v == version)
                        .map(|(_, e)| e)
                        .cloned()
                        .collect()
                })
                .collect::<Vec<_>>();
            Some(events)
        } else {
            None
        };

        Ok(TransactionListWithProof {
            transactions,
            events,
            first_transaction_version,
            proof,
        })
    }

    fn get_events(
        &self,
        key: &EventKey,
        start: u64,
        _order: Order,
        limit: u64,
    ) -> Result<Vec<(u64, ContractEvent)>> {
        let events = self
            .events
            .iter()
            .filter(|(_, e)| {
                e.key() == key
                    && start <= e.sequence_number()
                    && e.sequence_number() < start + limit
            })
            .cloned()
            .collect();
        Ok(events)
    }

    fn get_events_with_proofs(
        &self,
        _key: &EventKey,
        _start: u64,
        _order: Order,
        _limit: u64,
        _known_version: Option<u64>,
    ) -> Result<Vec<EventWithProof>> {
        unimplemented!()
    }

    fn get_state_proof(
        &self,
        known_version: u64,
    ) -> Result<(
        LedgerInfoWithSignatures,
        EpochChangeProof,
        AccumulatorConsistencyProof,
    )> {
        let li = self.get_latest_ledger_info()?;
        let proofs = self.get_state_proof_with_ledger_info(known_version, li.clone())?;
        Ok((
            LedgerInfoWithSignatures::new(li.ledger_info().clone(), BTreeMap::new()),
            proofs.0,
            proofs.1,
        ))
    }

    fn get_state_proof_with_ledger_info(
        &self,
        _known_version: u64,
        _ledger_info: LedgerInfoWithSignatures,
    ) -> Result<(EpochChangeProof, AccumulatorConsistencyProof)> {
        Ok((
            EpochChangeProof::new(vec![], false),
            AccumulatorConsistencyProof::new(vec![]),
        ))
    }

    fn get_account_state_with_proof(
        &self,
        _address: AccountAddress,
        _version: Version,
        _ledger_version: Version,
    ) -> Result<AccountStateWithProof> {
        Ok(self
            .account_state_with_proof
            .get(0)
            .ok_or_else(|| format_err!("could not find account state"))?
            .clone())
    }

    fn get_startup_info(&self) -> Result<Option<StartupInfo>> {
        unimplemented!()
    }

    fn get_account_state_with_proof_by_version(
        &self,
        address: AccountAddress,
        _version: u64,
    ) -> Result<(
        Option<AccountStateBlob>,
        SparseMerkleProof<AccountStateBlob>,
    )> {
        Ok((
            self.get_latest_account_state(address)?,
            SparseMerkleProof::new(None, vec![]),
        ))
    }

    fn get_latest_state_root(&self) -> Result<(u64, HashValue)> {
        unimplemented!()
    }

    fn get_latest_tree_state(&self) -> Result<TreeState> {
        unimplemented!()
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        _start_epoch: u64,
        _end_epoch: u64,
    ) -> Result<EpochChangeProof> {
        unimplemented!()
    }

    fn get_epoch_ending_ledger_info(&self, _: u64) -> Result<LedgerInfoWithSignatures> {
        unimplemented!()
    }

    fn get_block_timestamp(&self, version: u64) -> Result<u64> {
        Ok(match self.timestamps.get(version as usize) {
            Some(t) => *t,
            None => *self.timestamps.last().unwrap(),
        })
    }

    fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue> {
        Ok(HashValue::zero())
    }
}

// returns MockDiemDB for unit-testing
#[allow(unused)]
pub fn mock_db() -> MockDiemDB {
    let mut gen = ValueGenerator::new();
    let blocks = gen.generate(arb_blocks_to_commit());
    let mut account_state_with_proof = gen.generate(any::<AccountStateWithProof>());

    let mut version = 1;
    let mut all_accounts = HashMap::new();
    let mut all_txns = vec![];
    let mut events = vec![];
    let mut timestamps = vec![0_u64];

    for (txns_to_commit, ledger_info_with_sigs) in &blocks {
        for (idx, txn) in txns_to_commit.iter().enumerate() {
            timestamps.push(ledger_info_with_sigs.ledger_info().timestamp_usecs());
            events.extend(
                txn.events()
                    .iter()
                    .map(|e| ((idx + version) as u64, e.clone())),
            );
        }
        version += txns_to_commit.len();
        let mut account_states = HashMap::new();
        // Get the ground truth of account states.
        txns_to_commit.iter().for_each(|txn_to_commit| {
            account_states.extend(txn_to_commit.account_states().clone())
        });

        // Record all account states.
        for (address, blob) in account_states.into_iter() {
            let mut state = AccountState::try_from(&blob).unwrap();
            let freezing_bit = Value::struct_(Struct::pack(vec![Value::bool(false)]))
                .value_as::<Struct>()
                .unwrap()
                .simple_serialize(&MoveStructLayout::new(vec![MoveTypeLayout::Bool]))
                .unwrap();
            state.insert(FreezingBit::resource_path(), freezing_bit);
            all_accounts.insert(address, AccountStateBlob::try_from(&state).unwrap());
        }

        // Record all transactions.
        all_txns.extend(txns_to_commit.iter().map(|txn_to_commit| {
            (
                txn_to_commit.transaction().clone(),
                txn_to_commit.status().clone(),
            )
        }));
    }

    if account_state_with_proof.blob.is_none() {
        let (_, blob) = all_accounts.iter().next().unwrap();
        account_state_with_proof.blob = Some(blob.clone());
    }

    let account_state_with_proof = vec![account_state_with_proof];

    if events.is_empty() {
        // mock the first event
        let mock_event = ContractEvent::new(
            EventKey::new_from_address(&AccountAddress::random(), 0),
            0,
            TypeTag::Bool,
            b"event_data".to_vec(),
        );
        events.push((version as u64, mock_event));
    }

    let (genesis, _) = generate_genesis_state();
    MockDiemDB {
        version: version as u64,
        genesis,
        all_accounts,
        all_txns,
        events,
        account_state_with_proof,
        timestamps,
    }
}

/// Creates and returns a MockDiemDB, JsonRpcAsyncClient and corresponding server Runtime tuple for
/// testing. The given channel_buffer specifies the buffer size of the mempool client sender channel.
#[allow(unused)]
pub fn create_database_client_and_runtime() -> (MockDiemDB, BlockingClient, Runtime) {
    let (mock_db, runtime, url, _) = create_db_and_runtime();
    let client = BlockingClient::new(url);

    (mock_db, client, runtime)
}

#[allow(unused)]
pub fn create_db_and_runtime() -> (
    MockDiemDB,
    Runtime,
    String,
    Receiver<(
        SignedTransaction,
        oneshot::Sender<anyhow::Result<SubmissionStatus>>,
    )>,
) {
    let mock_db = mock_db();

    let host = "127.0.0.1";
    let port = utils::get_available_port();
    let address = format!("{}:{}", host, port);
    let (mp_sender, mp_events) = channel(1);

    let runtime = test_bootstrap(
        address.parse().unwrap(),
        Arc::new(mock_db.clone()),
        mp_sender,
    );
    (mock_db, runtime, format!("http://{}", address), mp_events)
}
