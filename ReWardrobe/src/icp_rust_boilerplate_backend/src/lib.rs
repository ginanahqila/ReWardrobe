#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct NFTItem {
    id: u64,
    name: String,
    description: String,
    owner: String,
    rental_price: u64,
    available: bool,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for NFTItem {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for NFTItem {
    const MAX_SIZE: u32 = 2048;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static MARKETPLACE: RefCell<StableBTreeMap<u64, NFTItem, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct NFTItemPayload {
    name: String,
    description: String,
    owner: String,
    rental_price: u64,
}

#[ic_cdk::query]
fn get_item(id: u64) -> Result<NFTItem, String> {
    MARKETPLACE.with(|storage| storage.borrow().get(&id).cloned())
        .ok_or_else(|| format!("NFT item with ID {} not found", id))
}

#[ic_cdk::update]
fn add_item(payload: NFTItemPayload) -> NFTItem {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let item = NFTItem {
        id,
        name: payload.name,
        description: payload.description,
        owner: payload.owner,
        rental_price: payload.rental_price,
        available: true,
        created_at: time(),
        updated_at: None,
    };

    MARKETPLACE.with(|storage| storage.borrow_mut().insert(id, item.clone()));
    item
}

#[ic_cdk::update]
fn update_item(id: u64, payload: NFTItemPayload) -> Result<NFTItem, String> {
    MARKETPLACE.with(|storage| {
        let mut marketplace = storage.borrow_mut();
        if let Some(item) = marketplace.get_mut(&id) {
            item.name = payload.name;
            item.description = payload.description;
            item.owner = payload.owner;
            item.rental_price = payload.rental_price;
            item.updated_at = Some(time());
            Ok(item.clone())
        } else {
            Err(format!("NFT item with ID {} not found", id))
        }
    })
}

#[ic_cdk::update]
fn delete_item(id: u64) -> Result<NFTItem, String> {
    MARKETPLACE.with(|storage| storage.borrow_mut().remove(&id))
        .ok_or_else(|| format!("NFT item with ID {} not found", id))
}

#[ic_cdk::update]
fn toggle_availability(id: u64) -> Result<NFTItem, String> {
    MARKETPLACE.with(|storage| {
        let mut marketplace = storage.borrow_mut();
        if let Some(item) = marketplace.get_mut(&id) {
            item.available = !item.available;
            item.updated_at = Some(time());
            Ok(item.clone())
        } else {
            Err(format!("NFT item with ID {} not found", id))
        }
    })
}

// Generate candid interface
ic_cdk::export_candid!();
