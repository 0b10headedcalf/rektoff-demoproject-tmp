use anchor_lang::prelude::*;

declare_id!("HhEeHuUkuos2hGCbsxfGMYPmWKZQ1KCHdJHS5DvzcH4B");

/// The withdrawal fee in basis points: 100 = 1%.
pub const FEE_BPS: u64 = 100;

#[program]
pub mod vault {
    use super::*;

    /// Pays `amount` out of the vault. 1% of it goes to the treasury.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // The fix: round the fee up, so a small withdrawal can never round it
        // down to nothing.
        let fee = (amount * FEE_BPS).div_ceil(10_000);

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.treasury.try_borrow_mut_lamports()? += fee;
        **ctx.accounts.destination.try_borrow_mut_lamports()? += amount - fee;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = owner, has_one = treasury)]
    pub vault: Account<'info, Vault>,
    pub owner: Signer<'info>,
    #[account(mut)]
    pub treasury: SystemAccount<'info>,
    #[account(mut)]
    pub destination: SystemAccount<'info>,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub treasury: Pubkey,
}
