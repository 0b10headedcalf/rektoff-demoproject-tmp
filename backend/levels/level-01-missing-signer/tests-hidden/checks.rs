//! Hidden checks for Level 1. Players see only each check's `name`.
//!
//! Every check builds a fresh VM, so checks cannot affect each other.

use std::path::Path;
use std::str::FromStr;

use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

use crate::Check;

pub const CHECKS: &[Check] = &[
    Check { name: "attacker can withdraw without signing", run: attacker_cannot_withdraw },
    Check { name: "owner can still withdraw", run: owner_can_withdraw },
    Check { name: "deposits still work", run: deposits_work },
];

const PROGRAM_ID: &str = "3DGDBs4E5jhTevbUhK5JU5qQHKM3f8q9ifYhn5cZUhKc";
const SYSTEM_PROGRAM: Address = Address::new_from_array([0; 32]);
const SOL: u64 = 1_000_000_000;

struct World {
    svm: LiteSVM,
    program: Address,
    owner: Keypair,
    vault: Address,
}

/// Anchor picks an instruction by the first 8 bytes of sha256("global:<name>").
fn ix_data(name: &str, args: &[u8]) -> Vec<u8> {
    let hash = Sha256::digest(format!("global:{name}"));
    [&hash[..8], args].concat()
}

fn send(svm: &mut LiteSVM, ix: Instruction, payer: &Keypair) -> Result<(), String> {
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{:?}", e.err))
}

fn funded_user(svm: &mut LiteSVM) -> Result<Keypair, String> {
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * SOL).map_err(|e| format!("{:?}", e.err))?;
    Ok(user)
}

fn deposit_ix(program: Address, vault: Address, depositor: Address, amount: u64) -> Instruction {
    Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(depositor, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data: ix_data("deposit", &amount.to_le_bytes()),
    }
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

/// A fresh VM with the player's program loaded and a vault holding `amount`.
fn funded_vault(so: &Path, amount: u64) -> Result<World, String> {
    let mut svm = LiteSVM::new();
    let program = Address::from_str(PROGRAM_ID).map_err(|e| e.to_string())?;
    svm.add_program_from_file(program, so).map_err(|e| format!("load failed: {e:?}"))?;

    let owner = funded_user(&mut svm)?;
    let (vault, _bump) =
        Address::find_program_address(&[b"vault", owner.pubkey().as_ref()], &program);

    let init = Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data: ix_data("initialize", &[]),
    };
    send(&mut svm, init, &owner)?;
    send(&mut svm, deposit_ix(program, vault, owner.pubkey(), amount), &owner)?;
    Ok(World { svm, program, owner, vault })
}

fn balance(svm: &LiteSVM, address: &Address) -> u64 {
    svm.get_balance(address).unwrap_or(0)
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

fn deposits_work(so: &Path) -> Result<(), String> {
    let mut world = funded_vault(so, SOL)?;
    let depositor = funded_user(&mut world.svm)?;
    let before = balance(&world.svm, &world.vault);
    let ix = deposit_ix(world.program, world.vault, depositor.pubkey(), SOL);
    send(&mut world.svm, ix, &depositor)?;
    match balance(&world.svm, &world.vault) {
        after if after == before + SOL => Ok(()),
        after => Err(format!("vault went from {before} to {after}")),
    }
}
