use blackgold_mint_api::consts::MINT_ADDRESS;
use blackgold_stake_api::prelude::*;
use steel::*;

/// Distributes ORE from the sender to stakers.
pub fn process_distribute(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Distribute::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);
    if amount == 0 {
        return Ok(());
    }

    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, sender_info, blackgold_mint_info, treasury_info, treasury_tokens_info, vesting_info, token_program, blackgold_stake_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    signer_info.is_signer()?;

    sender_info
        .is_writable()?
        .as_associated_token_account(&signer_info.key, &MINT_ADDRESS)?
        .assert(|s| s.amount() >= amount)?;

    blackgold_mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;

    let treasury = treasury_info
        .has_address(&treasury_pda().0)?
        .as_account_mut::<Treasury>(&blackgold_stake_api::ID)?
        .assert_mut_err(
            |t| t.total_staked > 0,
            BlackgoldStakeError::NoDeposits.into(),
        )?;

    let vesting = vesting_info
        .has_address(&vesting_pda().0)?
        .as_account_mut::<Vesting>(&blackgold_stake_api::ID)?;

    treasury_tokens_info
        .is_writable()?
        .as_associated_token_account(&treasury_info.key, &MINT_ADDRESS)?;

    token_program.is_program(&spl_token::ID)?;
    
    blackgold_stake_program.is_program(&blackgold_stake_api::ID)?;

    // Vest any unvested ORE into the treasury.
    vesting.vest(&clock, treasury);

    // Update vesting schedule.
    if vesting.vested_amount >= vesting.initial_amount {
        // Previous schedule complete — start fresh.
        vesting.initial_amount = amount;
        vesting.vested_amount = 0;
        vesting.start_time = clock.unix_timestamp;
    } else {
        // Adjust start_time to preserve vested_amount invariant under new total.
        let new_initial = vesting.initial_amount + amount;
        let new_elapsed = (vesting.vested_amount as u128)
            .saturating_mul(ONE_HOUR as u128)
            .div_ceil(new_initial as u128) as i64;
        vesting.start_time = clock.unix_timestamp - new_elapsed;
        vesting.initial_amount = new_initial;
    }

    // Transfer tokens to treasury for distribution to stakers
    transfer(
        signer_info,
        sender_info,
        treasury_tokens_info,
        token_program,
        amount,
    )?;

    // Log event.
    program_log(
        &[treasury_info.clone()],
        DistributeEvent {
            disc: BlackgoldStakeEvent::Distribute as u64,
            amount,
            total_staked: treasury.total_staked,
            ts: clock.unix_timestamp,
        }
        .to_bytes(),
    )?;

    Ok(())
}
