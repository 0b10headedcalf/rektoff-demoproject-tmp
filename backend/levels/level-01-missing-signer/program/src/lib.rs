use anchor_lang::prelude::*;

declare_id!("3DGDBs4E5jhTevbUhK5JU5qQHKM3f8q9ifYhn5cZUhKc");

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
    #[account(mut, has_one = owner)]
    pub vault: Account<'info, Vault>,
    /// CHECK: matched against vault.owner by has_one above.
    pub owner: UncheckedAccount<'info>,
    #[account(mut)]
    pub destination: SystemAccount<'info>,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
}
