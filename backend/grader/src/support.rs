//! Helpers shared by every level's checks.
//!
//! Checks build the scene themselves (funded users, pre-made program accounts)
//! so the level programs only need the instructions the player reads.

use std::path::Path;
use std::str::FromStr;

use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_address::Address;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const SOL: u64 = 1_000_000_000;

fn discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let hash = Sha256::digest(format!("{namespace}:{name}"));
    hash[..8].try_into().expect("8 bytes")
}

/// Anchor picks an instruction by the first 8 bytes of sha256("global:<name>").
pub fn ix_data(name: &str, args: &[u8]) -> Vec<u8> {
    [&discriminator("global", name)[..], args].concat()
}

/// The bytes of an Anchor `#[account]` struct: sha256("account:<Name>")[..8],
/// then each field in declaration order.
pub fn account_data(name: &str, fields: &[&[u8]]) -> Vec<u8> {
    let mut data = discriminator("account", name).to_vec();
    for field in fields {
        data.extend_from_slice(field);
    }
    data
}

/// A fresh VM with the player's program loaded at `program_id`.
pub fn load(so: &Path, program_id: &str) -> Result<(LiteSVM, Address), String> {
    let mut svm = LiteSVM::new();
    let program = Address::from_str(program_id).map_err(|e| e.to_string())?;
    svm.add_program_from_file(program, so).map_err(|e| format!("load failed: {e:?}"))?;
    Ok((svm, program))
}

/// Sends one instruction. `payer` pays the fee and is the only signature.
pub fn send(svm: &mut LiteSVM, ix: Instruction, payer: &Keypair) -> Result<(), String> {
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{:?}", e.err))
}

pub fn funded_user(svm: &mut LiteSVM) -> Result<Keypair, String> {
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * SOL).map_err(|e| format!("{:?}", e.err))?;
    Ok(user)
}

/// Creates an account owned by `program` at a new address, holding `data` and
/// `spendable` lamports on top of its rent-exempt minimum.
pub fn seed_account(
    svm: &mut LiteSVM,
    program: Address,
    data: Vec<u8>,
    spendable: u64,
) -> Result<Address, String> {
    let address = Keypair::new().pubkey();
    let lamports = svm.minimum_balance_for_rent_exemption(data.len()) + spendable;
    let account = Account { lamports, data, owner: program, executable: false, rent_epoch: 0 };
    svm.set_account(address, account).map_err(|e| format!("seed failed: {e:?}"))?;
    Ok(address)
}

pub fn balance(svm: &LiteSVM, address: &Address) -> u64 {
    svm.get_balance(address).unwrap_or(0)
}
