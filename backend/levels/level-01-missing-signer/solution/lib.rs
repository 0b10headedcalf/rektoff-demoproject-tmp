use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("3DGDBs4E5jhTevbUhK5JU5qQHKM3f8q9ifYhn5cZUhKc");

#[program]
pub mod vault {
    use super::*;

    /// Creates a vault belonging to the signer.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.vault.owner = ctx.accounts.owner.key();
        Ok(())
    }

    /// Anyone can add funds to a vault.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let transfer = system_program::Transfer {
            from: ctx.accounts.depositor.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };
        let cpi = CpiContext::new(ctx.accounts.system_program.key(), transfer);
        system_program::transfer(cpi, amount)
    }

    /// Only the vault's owner should be able to take funds out.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = ctx.accounts.vault.to_account_info();
        let rent_floor = Rent::get()?.minimum_balance(vault.data_len());
        let remaining = vault
            .lamports()
            .checked_sub(amount)
            .ok_or(VaultError::InsufficientFunds)?;
        require!(remaining >= rent_floor, VaultError::InsufficientFunds);

        **vault.try_borrow_mut_lamports()? = remaining;
        **ctx.accounts.destination.try_borrow_mut_lamports()? += amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + Vault::INIT_SPACE,
        seeds = [b"vault", owner.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = owner)]
    pub vault: Account<'info, Vault>,
    pub owner: Signer<'info>,
    #[account(mut)]
    pub destination: SystemAccount<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub owner: Pubkey,
}

#[error_code]
pub enum VaultError {
    #[msg("The vault does not hold enough funds for this withdrawal.")]
    InsufficientFunds,
}
