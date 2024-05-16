use std::cell::RefCell;

use candid::{Decode, Encode};
use ic_cdk::api::{
    management_canister::main::{canister_status, CanisterIdRecord, CanisterStatusResponse},
    stable::stable64_size,
};
use ic_stable_structures::{
    memory_manager::{MemoryManager, VirtualMemory},
    DefaultMemoryImpl,
};

const WASM_PAGE_SIZE: u64 = 64 * 1024;

#[derive(Debug, Clone, candid::CandidType, candid::Deserialize, serde::Serialize)]
pub struct Snapshot {
    pub value: SnapshotValue,
    pub timestamp: u64,
}
type SnapshotValue = u64;
impl ic_stable_structures::Storable for Snapshot {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}
impl ic_stable_structures::BoundedStorable for Snapshot {
    const MAX_SIZE: u32 = 10_000;
    const IS_FIXED_SIZE: bool = false; // temp
}

type MemoryType = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static SNAPSHOTS: std::cell::RefCell<ic_stable_structures::StableVec<Snapshot, MemoryType>> = std::cell::RefCell::new(
        ic_stable_structures::StableVec::init(
            MEMORY_MANAGER.with(|mm| mm.borrow().get(
                ic_stable_structures::memory_manager::MemoryId::new(1)
            ))
        ).unwrap()
    );
}

// Stable Memory
#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_datum(index: u64) -> Snapshot {
    SNAPSHOTS.with(|mem| mem.borrow().get(index).unwrap())
}
#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_data_length() -> u64 {
    SNAPSHOTS.with(|mem| mem.borrow().len())
}
#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_last_datum() -> Option<Snapshot> {
    SNAPSHOTS.with(|mem| {
        let borrowed_mem = mem.borrow();
        let len = borrowed_mem.len();
        borrowed_mem.get(len - 1) // NOTE: Since StableVec does not have last()
    })
}
#[ic_cdk::query]
#[candid::candid_method(query)]
fn get_top_data(n: u64) -> Vec<Snapshot> {
    SNAPSHOTS.with(|mem| {
        let borrowed_mem = mem.borrow();
        let len = borrowed_mem.len();
        let mut res = Vec::new();
        for i in 0..n {
            if i >= len {
                break;
            }
            res.push(borrowed_mem.get(len - i - 1).unwrap());
        }
        res
    })
}
#[ic_cdk::update]
#[candid::candid_method(update)]
pub fn add_datum(value: SnapshotValue) -> Result<(), String> {
    let datum = Snapshot {
        value,
        timestamp: ic_cdk::api::time() / 1_000_000,
    };
    _add_datum(datum)
}
#[ic_cdk::update]
#[candid::candid_method(update)]
fn add_data(value: SnapshotValue, count: u64) {
    let timestamp = ic_cdk::api::time() / 1000000;
    let snapshots = (0..count)
        .map(|_| Snapshot { value, timestamp })
        .collect::<Vec<_>>();
    for (i, snapshot) in snapshots.into_iter().enumerate() {
        if i % 10000 == 0 {
            ic_cdk::println!("Inserting snapshot: {}", i);
        }
        _add_datum(snapshot).unwrap();
    }
}
fn _add_datum(datum: Snapshot) -> Result<(), String> {
    let res = SNAPSHOTS.with(|vec| vec.borrow_mut().push(&datum));
    res.map_err(|e| format!("{:?}", e))
}

// Status
#[ic_cdk::query]
#[candid::candid_method(query)]
async fn status_all() -> CanisterStatusResponse {
    let canister_id = ic_cdk::api::id();
    canister_status(CanisterIdRecord { canister_id })
        .await
        .unwrap()
        .0
}
#[ic_cdk::query]
#[candid::candid_method(query)]
fn status_used_stable_memory() -> u64 {
    stable64_size() * WASM_PAGE_SIZE
}
#[ic_cdk::query]
#[candid::candid_method(query)]
fn status_used_heap_size() -> u64 {
    get_heap_size()
}

fn get_heap_size() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        core::arch::wasm32::memory_size(0) as u64 * WASM_PAGE_SIZE
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    candid::export_service!();

    #[test]
    fn gen_candid() {
        std::fs::write("interface.did", __export_service()).unwrap();
    }
}
