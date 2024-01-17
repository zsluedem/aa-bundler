use crate::{mempool::ClearOp, AddRemoveUserOp, ReputationEntryOp, UserOperationOp};
use metrics::{counter, describe_counter, describe_gauge, gauge};
use silius_primitives::{UserOperation, UserOperationHash};

const MEMPOOL_SIZE: &str = "silius_mempool_size";
const MEMPOOL_ADD_ERROR: &str = "silius_mempool_add_error";
const MEMPOOL_REMOVE_ERROR: &str = "silius_mempool_remove_error";
const REPUTATION_UO_SEEN: &str = "silius_reputation_uo_seen";
const REPUTATION_UO_INCLUDED: &str = "silius_reputation_uo_included";
const REPUTATION_STATUS: &str = "silius_reputation_status";
const REPUTATION_SET_ENTRY_ERROR: &str = "silius_reputation_set_entry.error";

#[derive(Clone)]
pub struct MetricsHandler<S> {
    inner: S,
}

impl<S> MetricsHandler<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S: AddRemoveUserOp> AddRemoveUserOp for MetricsHandler<S> {
    fn add(&mut self, uo: UserOperation) -> Result<UserOperationHash, crate::MempoolErrorKind> {
        match self.inner.add(uo) {
            Ok(res) => {
                gauge!(MEMPOOL_SIZE).increment(1f64);
                Ok(res)
            }
            Err(e) => {
                counter!(MEMPOOL_ADD_ERROR, "error" => format!("{:?}", e)).increment(1);
                Err(e)
            }
        }
    }

    fn remove_by_uo_hash(
        &mut self,
        uo_hash: &silius_primitives::UserOperationHash,
    ) -> Result<bool, crate::MempoolErrorKind> {
        match self.inner.remove_by_uo_hash(uo_hash) {
            Ok(res) => {
                gauge!(MEMPOOL_SIZE).decrement(1f64);
                Ok(res)
            }
            Err(e) => {
                counter!(MEMPOOL_REMOVE_ERROR, "error" => format!("{:?}", e)).increment(1);
                Err(e)
            }
        }
    }
}

impl<S: UserOperationOp> UserOperationOp for MetricsHandler<S> {
    fn get_by_uo_hash(
        &self,
        uo_hash: &silius_primitives::UserOperationHash,
    ) -> Result<Option<silius_primitives::UserOperation>, crate::MempoolErrorKind> {
        self.inner.get_by_uo_hash(uo_hash)
    }

    fn get_sorted(&self) -> Result<Vec<silius_primitives::UserOperation>, crate::MempoolErrorKind> {
        self.inner.get_sorted()
    }

    fn get_all(&self) -> Result<Vec<silius_primitives::UserOperation>, crate::MempoolErrorKind> {
        self.inner.get_all()
    }
}

impl<S: ClearOp> ClearOp for MetricsHandler<S> {
    fn clear(&mut self) {
        self.inner.clear()
    }
}

impl<S: ReputationEntryOp> ReputationEntryOp for MetricsHandler<S> {
    fn get_entry(
        &self,
        addr: &ethers::types::Address,
    ) -> Result<Option<silius_primitives::reputation::ReputationEntry>, crate::ReputationError>
    {
        self.inner.get_entry(addr)
    }

    fn set_entry(
        &mut self,
        entry: silius_primitives::reputation::ReputationEntry,
    ) -> Result<Option<silius_primitives::reputation::ReputationEntry>, crate::ReputationError>
    {
        let addr = entry.address;
        match self.inner.set_entry(entry.clone()) {
            Ok(res) => {
                gauge!(REPUTATION_UO_SEEN, "address" => format!("{addr:x}"))
                    .set(entry.uo_seen as f64);
                gauge!(REPUTATION_UO_INCLUDED, "address" => format!("{addr:x}"))
                    .set(entry.uo_included as f64);
                gauge!(REPUTATION_STATUS, "address" => format!("{addr:x}"))
                    .set(entry.status as f64);
                Ok(res)
            }
            Err(e) => {
                counter!(REPUTATION_SET_ENTRY_ERROR, "error" => format!("{:?}", e)).increment(1);
                Err(e)
            }
        }
    }

    fn contains_entry(
        &self,
        addr: &ethers::types::Address,
    ) -> Result<bool, crate::ReputationError> {
        self.inner.contains_entry(addr)
    }

    fn get_all(&self) -> Vec<silius_primitives::reputation::ReputationEntry> {
        self.inner.get_all()
    }
}

pub fn describe_mempool_metrics() {
    describe_gauge!(MEMPOOL_SIZE, "The number of user operations in the mempool");
    describe_counter!(MEMPOOL_ADD_ERROR, "The number of errors when adding to the mempool");
    describe_counter!(MEMPOOL_REMOVE_ERROR, "The number of errors when removing from the mempool");
    describe_gauge!(REPUTATION_UO_SEEN, "The number of user operations seen for an address");
    describe_gauge!(
        REPUTATION_UO_INCLUDED,
        "The number of user operations included for an address"
    );
    describe_gauge!(REPUTATION_STATUS, "The status of an address");
    describe_counter!(
        REPUTATION_SET_ENTRY_ERROR,
        "The number of errors when setting a reputation entry"
    )
}
