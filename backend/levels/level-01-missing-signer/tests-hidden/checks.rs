//! Hidden checks for Level 1. Players see only each check's `name`.
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

    // TODO(you): attack the vault, then decide whether the player's fix held.
    //   1. Build the instruction with withdraw_ix(...). Pass the real owner's
    //      address, but think about what owner_signs must be for an attacker.
    //   2. Send it with send(&mut world.svm, ix, &attacker). The attacker pays.
    //   3. Return Ok(()) only if the vault still holds `before`.
    //      Careful: is `send` returning Err enough on its own to prove that?
    let _ = (&attacker, before);
    Err("exploit check not written yet".to_string())
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
