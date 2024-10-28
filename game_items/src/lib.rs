use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedSet};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise};
use serde::{Deserialize, Serialize};

// Metadata for game items
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GameItem {
    pub id: String,
    pub owner_id: AccountId,
    pub metadata: String, // JSON string with item details
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct GameItems {
    owner_id: AccountId,
    items: LookupMap<String, GameItem>,
    owner_to_items: LookupMap<AccountId, UnorderedSet<String>>,
}

#[near_bindgen]
impl GameItems {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            items: LookupMap::new(b"items".to_vec()),
            owner_to_items: LookupMap::new(b"owner_to_items".to_vec()),
        }
    }

    /// Mint a new game item. Only the contract owner can mint.
    pub fn mint(&mut self, id: String, metadata: String) {
        let sender = env::predecessor_account_id();
        assert_eq!(sender, self.owner_id, "Only the owner can mint items");
        assert!(!self.items.contains_key(&id), "Item ID already exists");

        let item = GameItem {
            id: id.clone(),
            owner_id: sender.clone(),
            metadata,
        };

        self.items.insert(&id, &item);

        let mut items_set = self
            .owner_to_items
            .get(&sender)
            .unwrap_or_else(|| UnorderedSet::new(b"owner".to_vec()));
        items_set.insert(&id);
        self.owner_to_items.insert(&sender, &items_set);
    }

    /// Transfer a game item to a new owner.
    pub fn transfer(&mut self, id: String, new_owner_id: AccountId) {
        let sender = env::predecessor_account_id();
        let mut item = self.items.get(&id).expect("Item does not exist");
        assert_eq!(item.owner_id, sender, "Only the owner can transfer this item");

        // Remove item from current owner
        let mut current_owner_items = self.owner_to_items.get(&sender).unwrap();
        current_owner_items.remove(&id);
        self.owner_to_items.insert(&sender, &current_owner_items);

        // Add item to new owner
        let mut new_owner_items = self
            .owner_to_items
            .get(&new_owner_id.clone())
            .unwrap_or_else(|| UnorderedSet::new(b"owner".to_vec()));
        new_owner_items.insert(&id);
        self.owner_to_items.insert(&new_owner_id.clone(), &new_owner_items);

        // Update ownership
        item.owner_id = new_owner_id.clone();
        self.items.insert(&id, &item);
    }

    /// Get details of a specific item by ID.
    pub fn get_item(&self, id: String) -> GameItem {
        self.items.get(&id).expect("Item does not exist")
    }

    /// Get all item IDs owned by a specific account.
    pub fn get_items_by_owner(&self, owner_id: AccountId) -> Vec<String> {
        self.owner_to_items
            .get(&owner_id)
            .map(|set| set.to_vec())
            .unwrap_or_else(Vec::new)
    }
}

// Required for NEAR's testing framework
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_accounts, VMContextBuilder};
    use near_sdk::{testing_env, AccountId};

    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(predecessor);
        builder
    }

    #[test]
    fn test_mint_and_transfer() {
        let accounts = get_accounts();
        let owner = accounts.owner.clone();
        let alice = accounts.alice.clone();
        let bob = accounts.bob.clone();

        // Initialize contract
        let mut context = get_context(owner.clone()).build();
        testing_env!(context);
        let mut contract = GameItems::new(owner.clone());

        // Mint a new item
        contract.mint("item1".to_string(), "{\"name\":\"Sword\",\"power\":10}".to_string());
        let item = contract.get_item("item1".to_string());
        assert_eq!(item.owner_id, owner.clone());
        assert_eq!(item.id, "item1");
        assert_eq!(item.metadata, "{\"name\":\"Sword\",\"power\":10}");

        // Transfer the item to Alice
        context = get_context(owner.clone()).build();
        testing_env!(context);
        contract.transfer("item1".to_string(), alice.clone());
        let item = contract.get_item("item1".to_string());
        assert_eq!(item.owner_id, alice.clone());

        // Attempt unauthorized transfer by Bob
        context = get_context(bob.clone()).build();
        testing_env!(context);
        let result = std::panic::catch_unwind(|| {
            contract.transfer("item1".to_string(), bob.clone());
        });
        assert!(result.is_err());
    }
}
