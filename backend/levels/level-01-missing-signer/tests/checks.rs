//! Checks for Level 1. Players see each check's name and, on a failure, why.
//!
//! Every check builds a fresh VM, so checks cannot affect each other.

use std::path::Path;

use litesvm::LiteSVM;
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::Check;
use crate::support::{SOL, account_data, balance, funded_user, ix_data, load, seed_account, send};

pub const CHECKS: &[Check] = &[
    Check { name: "attacker cannot withdraw without the owner's signature", run: attacker_cannot_withdraw },
    Check { name: "owner can still withdraw", run: owner_can_withdraw },
];

const PROGRAM_ID: &str = "3DGDBs4E5jhTevbUhK5JU5qQHKM3f8q9ifYhn5cZUhKc";

struct World {
    svm: LiteSVM,
    program: Address,
    owner: Keypair,
    vault: Address,
}

/// `owner_signs` is the whole game: it sets whether the transaction claims the
/// `owner` account signed. A real attacker sets it to false, because they do
/// not hold the owner's key.
fn withdraw_ix(
    program: Address,
    vault: Address,
    owner: Address,
    owner_signs: bool,
    destination: Address,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(owner, owner_signs),
            AccountMeta::new(destination, false),
        ],
        data: ix_data("withdraw", &amount.to_le_bytes()),
    }
}

/// A fresh VM with the player's program loaded and a vault holding `amount`
/// that it can pay out.
fn funded_vault(so: &Path, amount: u64) -> Result<World, String> {
    let (mut svm, program) = load(so, PROGRAM_ID)?;
    let owner = funded_user(&mut svm)?;
    let data = account_data("Vault", &[owner.pubkey().as_ref()]);
    let vault = seed_account(&mut svm, program, data, amount)?;
    Ok(World { svm, program, owner, vault })
}

fn attacker_cannot_withdraw(so: &Path) -> Result<(), String> {
    let mut world = funded_vault(so, SOL)?;
    let attacker = funded_user(&mut world.svm)?;
    let before = balance(&world.svm, &world.vault);

    // The attacker names the real owner but cannot sign for them, so the
    // owner is passed with owner_signs = false. Only the attacker signs.
    let loot = Keypair::new().pubkey();
    let ix = withdraw_ix(world.program, world.vault, world.owner.pubkey(), false, loot, SOL / 2);
    // Whether the transaction failed is not the test: the vault's balance is.
    let _ = send(&mut world.svm, ix, &attacker);

    match balance(&world.svm, &world.vault) {
        after if after == before => Ok(()),
        after => Err(format!("vault went from {before} to {after}")),
    }
}

fn owner_can_withdraw(so: &Path) -> Result<(), String> {
    let mut world = funded_vault(so, SOL)?;
    let destination = Keypair::new().pubkey();
    let amount = SOL / 2;
    let ix = withdraw_ix(world.program, world.vault, world.owner.pubkey(), true, destination, amount);
    send(&mut world.svm, ix, &world.owner)?;
    match balance(&world.svm, &destination) {
        got if got == amount => Ok(()),
        got => Err(format!("destination received {got}, expected {amount}")),
    }
}
