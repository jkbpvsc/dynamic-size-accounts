use anchor_lang::prelude::*;
use anchor_lang::system_program::transfer;
use anchor_lang::system_program::Transfer;
use bytemuck::{checked::try_cast_slice, Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use std::mem::size_of;
use std::ops::{AddAssign, Deref, SubAssign};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod dynamic_accounts_poc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let ds = DynamicState::new();
        ds.pack(&ctx.accounts.state)?;

        Ok(())
    }

    pub fn update(ctx: Context<Update>, add: bool, key: Pubkey) -> Result<()> {
        let mut state = {
            let data = &ctx.accounts.state.try_borrow_data().unwrap();
            DynamicState::unpack(data.deref()).unwrap()
        };

        msg!("Adding {} key {} to state", add, key);

        if add {
            state.keys.push(KeyElement { id: key });
        } else {
            state.keys.retain(|k| k.id != key);
        }

        let new_account_size = 4 + size_of::<KeyElement>() * state.keys.len();
        let current_account_size = ctx.accounts.state.data_len();

        msg!("Current account size: {}", current_account_size);
        msg!("New account size: {}", new_account_size);

        if current_account_size != new_account_size {
            ctx.accounts.state.realloc(new_account_size, false)?;

            let size_diff = new_account_size as i64 - current_account_size as i64;
            let abs_size = size_diff.abs() as usize;
            let rent_increase = size_diff.is_positive();

            let rent = Rent::get()?.minimum_balance(abs_size);

            if rent_increase {
                transfer(
                    CpiContext::new(
                        ctx.accounts.system_program.to_account_info(),
                        Transfer {
                            from: ctx.accounts.signer.to_account_info(),
                            to: ctx.accounts.state.clone(),
                        },
                    ),
                    rent,
                )?
            } else {
                let mut state_lamps = ctx.accounts.state.try_borrow_mut_lamports()?;
                state_lamps.sub_assign(rent);

                let mut signer_lamps = ctx.accounts.signer.try_borrow_mut_lamports()?;
                signer_lamps.add_assign(rent);
            }
        }

        state.pack(&ctx.accounts.state)?;
        msg!("Updated state: {:#?}", state);
        msg!("Lamports: {}", ctx.accounts.state.lamports());

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Test {}

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// CHECK: F
    pub state: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    /// CHECK: F
    pub signer: Signer<'info>,
    #[account(mut)]
    /// CHECK: F
    pub state: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Clone, Copy, Debug)]
struct KeyElement {
    pub id: Pubkey,
}

unsafe impl Zeroable for KeyElement {}
unsafe impl Pod for KeyElement {}

#[derive(Clone, Debug)]
struct DynamicState {
    pub keys: Vec<KeyElement>,
}

impl DynamicState {
    pub fn new() -> Self {
        Self { keys: vec![] }
    }

    pub fn unpack(data: &[u8]) -> Result<Self> {
        let len = u32::from_le_bytes(data[..4].try_into().unwrap());
        let start_index = 4;
        let end_index = start_index + (len as usize * size_of::<KeyElement>());
        msg!("Unpacking from {} to {}", start_index, end_index);
        let keys = try_cast_slice::<u8, KeyElement>(&data[start_index..end_index])
            .unwrap()
            .to_vec();

        Ok(Self { keys })
    }

    pub fn pack(&self, ai: &AccountInfo) -> Result<()> {
        let mut data = vec![];
        data.extend_from_slice(&(self.keys.len() as u32).to_le_bytes());
        data.extend_from_slice(bytemuck::cast_slice(self.keys.as_slice()));

        let mut ai_data = ai.try_borrow_mut_data().unwrap();

        if ai_data.len() != data.len() {
            panic!("Data size mismatch {}:{}", ai_data.len(), data.len());
        }

        ai_data.copy_from_slice(&data);

        Ok(())
    }
}
