//! This contract demonstrates a sample implementation of the Soroban token
//! interface with freeze/unfreeze functionality.

use crate::admin::{has_administrator, read_administrator, write_administrator};
use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
use crate::storage_types::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD};
use soroban_sdk::token::{self, Interface as _};
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

// New data structure to track frozen accounts
#[derive(Debug)]
struct Account {
    balance: i128,
    is_frozen: bool,
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    pub fn initialize(e: Env, admin: Address, decimal: u32, name: String, symbol: String) {
        if has_administrator(&e) {
            panic!("already initialized")
        }
        write_administrator(&e, &admin);
        if decimal > u8::MAX.into() {
            panic!("Decimal must fit in a u8");
        }

        write_metadata(
            &e,
            TokenMetadata {
                decimal,
                name,
                symbol,
            },
        )
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Ensure account exists (add this line)
        let mut account = self.get_or_create_account(&e, to.clone());
        account.balance += amount;
        self.set_account(&e, to, account);

        TokenUtils::new(&e).events().mint(admin, to, amount);
    }

    // Function to get or create an account with initial frozen state (false)
    fn get_or_create_account(&self, e: &Env, address: Address) -> Account {
        let account = e.storage().get::<Account>(address);
        match account {
            Some(account) => account,
            None => Account {
                balance: 0,
                is_frozen: false, // Initially not frozen
            },
        }
    }

    // Function to set the account state in storage
    fn set_account(&self, e: &Env, address: Address, account: Account) {
        e.storage().put(address, account);
    }

    pub fn set_admin(e: Env, new_admin: Address) {
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_administrator(&e, &new_admin);
        TokenUtils::new(&e).events().set_admin(admin, new_admin);
    }

    // New function to freeze an account
    pub fn freeze_account(&mut self, e: Env, account: Address) {
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let mut target_account = self.get_or_create_account(&e, account);
        target_account.is_frozen = true;
        self.set_account(&e, account, target_account);
    }

    // New function to unfreeze an account
    pub fn unfreeze_account(&mut self, e: Env, account: Address) {
        let admin = read_administrator
