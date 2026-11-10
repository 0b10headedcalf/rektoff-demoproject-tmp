use anchor_lang::prelude::*;

declare_id!("8LTYjCQ3Cd2DqMm98JpSQN2EnRVbnCjfczLYHddzuiHi");

#[program]
pub mod vault {
    use super::*;

    /// Only the vault's owner should be able to take funds out.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.destination.try_borrow_mut_lamports()? += amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    // The fix: `has_one` requires the signer to be the owner stored in the vault.
    #[account(mut, has_one = owner)]
    pub vault: Account<'info, Vault>,
    pub owner: Signer<'info>,
    #[account(mut)]
    pub destination: SystemAccount<'info>,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
}
