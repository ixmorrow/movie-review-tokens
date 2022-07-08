use solana_program::{
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    account_info::{next_account_info, AccountInfo},
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
    program::{invoke_signed},
    borsh::try_from_slice_unchecked,
    program_error::ProgramError,
    program_pack::{IsInitialized},
    native_token::LAMPORTS_PER_SOL,
};
use std::convert::TryInto;
use borsh::BorshSerialize;
use crate::instruction::MovieInstruction;
use crate::state::{MovieAccountState, MovieCommentCounter, MovieComment};
use crate::error::ReviewError;
use spl_token::ID as TOKEN_PROGRAM_ID;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
  ) -> ProgramResult {
    let instruction = MovieInstruction::unpack(instruction_data)?;
    match instruction {
        MovieInstruction::AddMovieReview { title, rating, description } => {
            add_movie_review(program_id, accounts, title, rating, description)
        },
        MovieInstruction::UpdateMovieReview { title, rating, description } => {
            update_movie_review(program_id, accounts, title, rating, description)
        },
        MovieInstruction::AddComment { comment } => {
            add_comment(program_id, accounts, comment)
        }
    }
}

 pub fn add_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String
) -> ProgramResult {
    msg!("Adding movie review...");
    msg!("Title: {}", title);
    msg!("Rating: {}", rating);
    msg!("Description: {}", description);

    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let user_ata = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

     
    let (pda, bump_seed) = Pubkey::find_program_address(&[initializer.key.as_ref(), title.as_bytes().as_ref(),], program_id);
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument)
    }

    if rating > 5 || rating < 1 {
        msg!("Rating cannot be higher than 5");
        return Err(ReviewError::InvalidRating.into())
    }

    let review_discriminator = "review";
    let total_len: usize = (4 + review_discriminator.len()) + 1 + 1 + (4 + title.len()) + (4 + description.len());
    if total_len > 1000 {
        msg!("Data length is larger than 1000 bytes");
        return Err(ReviewError::InvalidDataLength.into())
    }

    let account_len: usize = 1000;

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    invoke_signed(
        &system_instruction::create_account(
        initializer.key,
        pda_account.key,
        rent_lamports,
        account_len.try_into().unwrap(),
        program_id,
        ),
        &[initializer.clone(), pda_account.clone(), system_program.clone()],
        &[&[initializer.key.as_ref(), title.as_bytes().as_ref(), &[bump_seed]]],
    )?;

    msg!("PDA created: {}", pda);

    msg!("unpacking state account");
    let mut account_data = try_from_slice_unchecked::<MovieAccountState>(&pda_account.data.borrow()).unwrap();
    msg!("borrowed account data");

    msg!("checking if movie account is already initialized");
    if account_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    account_data.discriminator = "review".to_string();
    account_data.title = title;
    account_data.rating = rating;
    account_data.description = description;
    account_data.is_initialized = true;

    msg!("serializing account");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("state account serialized");

    msg!("create comment counter");
    let counter_discriminator = "counter";
    let counter_len: usize = (4 + counter_discriminator.len()) + 1 + 1;

    let rent = Rent::get()?;
    let counter_rent_lamports = rent.minimum_balance(counter_len);

    let (counter, counter_bump) = Pubkey::find_program_address(&[pda.as_ref(), "comment".as_ref(),], program_id);
    if counter != *pda_counter.key {
        msg!("Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument)
    }

    invoke_signed(
        &system_instruction::create_account(
        initializer.key,
        pda_counter.key,
        counter_rent_lamports,
        counter_len.try_into().unwrap(),
        program_id,
        ),
        &[initializer.clone(), pda_counter.clone(), system_program.clone()],
        &[&[pda.as_ref(), "comment".as_ref(), &[counter_bump]]],
    )?;
    msg!("comment counter created");

    let mut counter_data = try_from_slice_unchecked::<MovieCommentCounter>(&pda_counter.data.borrow()).unwrap();

    msg!("checking if counter account is already initialized");
    if counter_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    
    counter_data.discriminator = "counter".to_string();
    counter_data.counter = 0;
    counter_data.is_initialized = true;
    msg!("comment count: {}", counter_data.counter);
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;

    // mint tokens here
    msg!("deriving mint authority");
    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[b"tokens"], program_id);

    if *mint_auth.key != mint_pda {
        msg!("Mint passed in and mint derived do not match");
        return Err(ReviewError::InvalidPDA.into())
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(ReviewError::IncorrectAccountError.into())
    }

    msg!("Minting 10 tokens to User associated token account");
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            10*LAMPORTS_PER_SOL,
        )?,
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        &[&[b"tokens", &[mint_bump]]],
    )?;

    Ok(())
}

pub fn update_movie_review(program_id: &Pubkey,
accounts: &[AccountInfo],
_title: String,
rating: u8,
description: String
) -> ProgramResult {
    msg!("Updating movie review...");

    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;

    msg!("unpacking state account");
    let mut account_data = try_from_slice_unchecked::<MovieAccountState>(&pda_account.data.borrow()).unwrap();
    msg!("review title: {}", account_data.title);

    if pda_account.owner != program_id {
      return Err(ProgramError::IllegalOwner)
    }

    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature)
    }

    let (pda, _bump_seed) = Pubkey::find_program_address(&[initializer.key.as_ref(), account_data.title.as_bytes().as_ref(),], program_id);
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into())
    }

    if !account_data.is_initialized() {
        msg!("Account is not initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    if rating > 5 || rating < 1 {
        msg!("Invalid Rating");
        return Err(ReviewError::InvalidRating.into())
    }

    let review_discriminator = "review";
    let update_len: usize = (4 + review_discriminator.len()) + 1 + 1 + (4 + description.len()) + account_data.title.len();
    if update_len > 1000 {
        msg!("Data length is larger than 1000 bytes");
        return Err(ReviewError::InvalidDataLength.into())
    }

    msg!("Review before update:");
    msg!("Title: {}", account_data.title);
    msg!("Rating: {}", account_data.rating);
    msg!("Description: {}", account_data.description);

    account_data.rating = rating;
    account_data.description = description;

    msg!("Review after update:");
    msg!("Title: {}", account_data.title);
    msg!("Rating: {}", account_data.rating);
    msg!("Description: {}", account_data.description);

    msg!("serializing account");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("state account serialized");

    Ok(())
}

pub fn add_comment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    //review: Pubkey,
    comment: String
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let commenter = next_account_info(account_info_iter)?;
    let pda_review = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;
    let pda_comment = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let user_ata = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    msg!("Adding Comment...");
    msg!("Review: {}", pda_review.key);
    msg!("Comment: {}", comment);

    let mut counter_data = try_from_slice_unchecked::<MovieCommentCounter>(&pda_counter.data.borrow()).unwrap();

    let comment_discriminator = "comment";
    let account_len: usize = (4 + comment_discriminator.len()) + 1 + 32 + (4 + comment.len());

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    let (pda, bump_seed) = Pubkey::find_program_address(&[pda_review.key.as_ref(), counter_data.counter.to_be_bytes().as_ref(),], program_id);
    if pda != *pda_comment.key {
        msg!("Invalid seeds for PDA");
        msg!("Derive PDA {} does not equal PDA passed in {}", pda, *pda_comment.key);
        return Err(ReviewError::InvalidPDA.into())
    }

    invoke_signed(
        &system_instruction::create_account(
        commenter.key,
        pda_comment.key,
        rent_lamports,
        account_len.try_into().unwrap(),
        program_id,
        ),
        &[commenter.clone(), pda_comment.clone(), system_program.clone()],
        &[&[pda_review.key.as_ref(), counter_data.counter.to_be_bytes().as_ref(), &[bump_seed]]],
    )?;

    msg!("Created Comment Account");
    
    let mut comment_data = try_from_slice_unchecked::<MovieComment>(&pda_comment.data.borrow()).unwrap();

    msg!("checking if comment account is already initialized");
    if comment_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    
    comment_data.discriminator = "comment".to_string();
    comment_data.review = *pda_review.key;
    comment_data.comment = comment;
    comment_data.is_initialized = true;
    comment_data.serialize(&mut &mut pda_comment.data.borrow_mut()[..])?;
    
    msg!("Comment Count: {}", counter_data.counter);
    counter_data.counter += 1;
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;

    // mint tokens here
    msg!("deriving mint authority");
    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[b"tokens"], program_id);

    if *mint_auth.key != mint_pda {
        msg!("Mint passed in and mint derived do not match");
        return Err(ReviewError::InvalidPDA.into())
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(ReviewError::IncorrectAccountError.into())
    }

    msg!("Minting 5 tokens to User associated token account");
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            5*LAMPORTS_PER_SOL,
        )?,
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        &[&[b"tokens", &[mint_bump]]],
    )?;
    
    Ok(())
}