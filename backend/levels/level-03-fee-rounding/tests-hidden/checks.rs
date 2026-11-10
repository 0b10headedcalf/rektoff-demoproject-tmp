//! Hidden checks for Level 3. Players see only each check's `name`.
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
    Check { name: "small withdrawals cannot skip the fee", run: small_withdrawals_pay },
    Check { name: "a normal withdrawal pays exactly 1%", run: normal_withdrawal_pays_one_percent },
    Check { name: "only the owner can withdraw", run: stranger_cannot_withdraw },
];

const PROGRAM_ID: &str = "HhEeHuUkuos2hGCbsxfGMYPmWKZQ1KCHdJHS5DvzcH4B";
const FEE_BPS: u128 = 100;

struct World {
    svm: LiteSVM,
    program: Address,
    owner: Keypair,
    vault: Address,
    treasury: Address,
    /// Already funded, so it can receive amounts far below the rent minimum.
    destination: Address,
}

fn withdraw_ix(world: &World, signer: Address, amount: u64) -> Instruction {
    Instruction {
        program_id: world.program,
        accounts: vec![
            AccountMeta::new(world.vault, false),
            AccountMeta::new_readonly(signer, true),
            AccountMeta::new(world.treasury, false),
            AccountMeta::new(world.destination, false),
        ],
        data: ix_data("withdraw", &amount.to_le_bytes()),
    }
}

fn funded_vault(so: &Path, amount: u64) -> Result<World, String> {
    let (mut svm, program) = load(so, PROGRAM_ID)?;
    let owner = funded_user(&mut svm)?;
    let treasury = funded_user(&mut svm)?.pubkey();
    let destination = funded_user(&mut svm)?.pubkey();
    let data = account_data("Vault", &[owner.pubkey().as_ref(), treasury.as_ref()]);
    let vault = seed_account(&mut svm, program, data, amount)?;
    Ok(World { svm, program, owner, vault, treasury, destination })
}

/// Each amount is chosen so that rounding the fee down under-charges: 99 owes a
/// fraction of a lamport, 150 owes 1.5, and 10_001 owes just over 100.
fn small_withdrawals_pay(so: &Path) -> Result<(), String> {
    let mut world = funded_vault(so, SOL)?;
    for amount in [99u64, 150, 10_001] {
        let vault_before = balance(&world.svm, &world.vault);
        let treasury_before = balance(&world.svm, &world.treasury);
        let ix = withdraw_ix(&world, world.owner.pubkey(), amount);
        // Refusing the withdrawal is also an acceptable fix, so a failed
        // transaction is fine. What matters is what moved.
        let _ = send(&mut world.svm, ix, &world.owner);

        let paid_out = vault_before - balance(&world.svm, &world.vault);
        let fee = balance(&world.svm, &world.treasury) - treasury_before;
        // fee / paid_out >= 1%, compared without dividing.
        if (fee as u128) * 10_000 < (paid_out as u128) * FEE_BPS {
            return Err(format!("withdrawing {amount} paid out {paid_out} for a fee of {fee}"));
        }
    }
    Ok(())
}

fn normal_withdrawal_pays_one_percent(so: &Path) -> Result<(), String> {
    let mut world = funded_vault(so, 2 * SOL)?;
    let treasury_before = balance(&world.svm, &world.treasury);
    let destination_before = balance(&world.svm, &world.destination);
    let ix = withdraw_ix(&world, world.owner.pubkey(), SOL);
    send(&mut world.svm, ix, &world.owner)?;

    let fee = balance(&world.svm, &world.treasury) - treasury_before;
    let received = balance(&world.svm, &world.destination) - destination_before;
    if fee == SOL / 100 && received == SOL - SOL / 100 {
        Ok(())
    } else {
        Err(format!("fee was {fee} and the destination received {received}"))
    }
}

fn stranger_cannot_withdraw(so: &Path) -> Result<(), String> {
    let mut world = funded_vault(so, SOL)?;
    let stranger = funded_user(&mut world.svm)?;
    let before = balance(&world.svm, &world.vault);
    let ix = withdraw_ix(&world, stranger.pubkey(), SOL / 2);
    let _ = send(&mut world.svm, ix, &stranger);
    match balance(&world.svm, &world.vault) {
        after if after == before => Ok(()),
        after => Err(format!("vault went from {before} to {after}")),
    }
}
