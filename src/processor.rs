use crate::error::ReviewError;
use crate::instruction::MovieInstruction;
use crate::state::{MovieAccountState, MovieComment, MovieCommentCounter};
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::convert::TryInto;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // unpack instruction data
    let instruction = MovieInstruction::unpack(instruction_data)?;
    match instruction {
        MovieInstruction::AddMovieReview {
            title,
            rating,
            description,
        } => add_movie_review(program_id, accounts, title, rating, description),
        // add UpdateMovieReview to match against our new data structure
        MovieInstruction::UpdateMovieReview {
            title,
            rating,
            description,
        } => {
            // make call to update function that we'll define next
            update_movie_review(program_id, accounts, title, rating, description)
        }

        MovieInstruction::AddComment { comment } => add_comment(program_id, accounts, comment),
    }
}

pub fn add_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String,
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

    let (pda, bump_seed) = Pubkey::find_program_address(
        &[initializer.key.as_ref(), title.as_bytes().as_ref()],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    if rating > 5 || rating < 1 {
        msg!("Rating cannot be higher than 5");
        return Err(ReviewError::InvalidRating.into());
    }

    let total_len: usize = 1 + 1 + (4 + title.len()) + (4 + description.len());
    if total_len > 1000 {
        msg!("Data length is larger than 1000 bytes");
        return Err(ReviewError::InvalidDataLength.into());
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
        &[
            initializer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        &[&[
            initializer.key.as_ref(),
            title.as_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;

    msg!("PDA created: {}", pda);

    msg!("unpacking state account");
    let mut account_data =
        try_from_slice_unchecked::<MovieAccountState>(&pda_account.data.borrow()).unwrap();
    msg!("borrowed account data");

    msg!("checking if movie account is already initialized");
    if account_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    account_data.title = title;
    account_data.rating = rating;
    account_data.description = description;
    account_data.is_initialized = true;

    msg!("serializing account");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("state account serialized");

    // counter account
    msg!("create comment counter");
    let counter_len: usize = 1;

    let rent = Rent::get()?;
    let counter_rent_lamports = rent.minimum_balance(counter_len);

    let (counter, counter_bump) =
        Pubkey::find_program_address(&[pda.as_ref(), "comment".as_ref()], program_id);
    if counter != *pda_counter.key {
        msg!("Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            pda_counter.key,
            counter_rent_lamports,
            counter_len.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            pda_counter.clone(),
            system_program.clone(),
        ],
        &[&[pda.as_ref(), "comment".as_ref(), &[counter_bump]]],
    )?;
    msg!("comment counter created");

    let mut counter_data =
        try_from_slice_unchecked::<MovieCommentCounter>(&pda_counter.data.borrow()).unwrap();

    counter_data.counter = 0;
    msg!("comment count: {}", counter_data.counter);
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;

    // mint tokens here
    msg!("deriving mint authority");
    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[b"tokens"], program_id);

    if *mint_auth.key != mint_pda {
        msg!("Mint passed in and mint derived do not match");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(ReviewError::IncorrectAccountError.into());
    }

    msg!("Minting 10 tokens to User associated token account");
    invoke_signed(
        // instruction
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            10 * LAMPORTS_PER_SOL,
        )?,
        // account_infos
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        // seeds
        &[&[b"tokens", &[mint_bump]]],
    )?;

    Ok(())
}

pub fn update_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String,
) -> ProgramResult {
    msg!("Updating movie review...");

    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;

    msg!("unpacking state account");
    let mut account_data =
        try_from_slice_unchecked::<MovieAccountState>(&pda_account.data.borrow()).unwrap();
    msg!("review title: {}", account_data.title);

    if pda_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, _bump_seed) = Pubkey::find_program_address(
        &[
            initializer.key.as_ref(),
            account_data.title.as_bytes().as_ref(),
        ],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    if !account_data.is_initialized() {
        msg!("Account is not initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    if rating > 5 || rating < 1 {
        msg!("Invalid Rating");
        return Err(ReviewError::InvalidRating.into());
    }

    let update_len: usize = 1 + 1 + (4 + description.len()) + account_data.title.len();
    if update_len > 1000 {
        msg!("Data length is larger than 1000 bytes");
        return Err(ReviewError::InvalidDataLength.into());
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
    comment: String,
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

    let mut counter_data =
        try_from_slice_unchecked::<MovieCommentCounter>(&pda_counter.data.borrow()).unwrap();

    let account_len: usize = 32 + (4 + comment.len());

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    let (pda, bump_seed) = Pubkey::find_program_address(
        &[
            pda_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
        ],
        program_id,
    );
    if pda != *pda_comment.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    invoke_signed(
        &system_instruction::create_account(
            commenter.key,
            pda_comment.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[
            commenter.clone(),
            pda_comment.clone(),
            system_program.clone(),
        ],
        &[&[
            pda_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;

    let mut comment_data =
        try_from_slice_unchecked::<MovieComment>(&pda_comment.data.borrow()).unwrap();
    comment_data.review = *pda_review.key;
    comment_data.comment = comment;
    comment_data.serialize(&mut &mut pda_comment.data.borrow_mut()[..])?;

    msg!("Comment Count: {}", counter_data.counter);
    counter_data.counter += 1;
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;

    // mint tokens here
    msg!("deriving mint authority");
    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[b"tokens"], program_id);

    if *mint_auth.key != mint_pda {
        msg!("Mint passed in and mint derived do not match");
        return Err(ReviewError::InvalidPDA.into());
    }

    if *token_program.key != TOKEN_PROGRAM_ID {
        msg!("Incorrect token program");
        return Err(ReviewError::IncorrectAccountError.into());
    }

    msg!("Minting 5 tokens to User associated token account");
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            token_mint.key,
            user_ata.key,
            mint_auth.key,
            &[],
            5 * LAMPORTS_PER_SOL,
        )?,
        &[token_mint.clone(), user_ata.clone(), mint_auth.clone()],
        &[&[b"tokens", &[mint_bump]]],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        assert_matches::*,
        solana_program::{
            instruction::{AccountMeta, Instruction},
            program_pack::Pack,
            system_program::ID as SYSTEM_PROGRAM_ID,
        },
        solana_program_test::*,
        solana_sdk::{
            client::SyncClient, signature::Signer, signer::keypair::Keypair,
            system_instruction::create_account, transaction::Transaction,
        },
        spl_associated_token_account::{
            get_associated_token_address,
            instruction::{create_associated_token_account, AssociatedTokenAccountInstruction},
            ID as ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
        },
        spl_token::{instruction::initialize_mint, state::Mint, ID as TOKEN_PROGRAM_ID},
    };
    // first unit test
    #[tokio::test]
    async fn it_works() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "bpf_program_template",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // derive pda for token mint authority
        let (mint_auth, _bump_seed) = Pubkey::find_program_address(&[b"tokens"], &program_id);

        // create mint account
        let mint_keypair = Keypair::new();
        let rent = banks_client.get_rent().await.unwrap();
        let mint_rent = rent.minimum_balance(Mint::LEN);
        let create_mint_acct_ix = create_account(
            &payer.pubkey(),
            &mint_keypair.pubkey(),
            mint_rent,
            Mint::LEN.try_into().unwrap(),
            &TOKEN_PROGRAM_ID,
        );
        // create initialize mint instruction
        let init_mint_ix = initialize_mint(
            &TOKEN_PROGRAM_ID,
            &mint_keypair.pubkey(),
            &mint_auth,
            Some(&mint_auth),
            9,
        )
        .unwrap();

        // create review pda
        let title: String = "Captain America".to_owned();
        const rating: u8 = 3;
        let review: String = "Liked the movie".to_owned();
        let (review_pda, _bump_seed) =
            Pubkey::find_program_address(&[payer.pubkey().as_ref(), title.as_bytes()], &program_id);

        // create comment pda
        let (comment_pda, _bump_seed) =
            Pubkey::find_program_address(&[review_pda.as_ref(), b"comment"], &program_id);

        // create user associate token account of token mint
        let init_ata_ix: Instruction = create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint_keypair.pubkey(),
        );

        let user_ata: Pubkey = get_associated_token_address(&wallet_address, &mint_keypair.pubkey());
        let init_ata_ix = Instruction {
            program_id: ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(user_ata, false),
                AccountMeta::new_readonly(wallet_address, false),
                AccountMeta::new_readonly(mint_keypair.pubkey(), true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID, false)
            ],
            data: vec![0]
        };

        // concat data to single buffer
        let mut data_vec = vec![0];
        data_vec.append(
            &mut (TryInto::<u32>::try_into(title.len()).unwrap().to_le_bytes())
                .try_into()
                .unwrap(),
        );
        data_vec.append(&mut title.into_bytes());
        data_vec.push(rating);
        data_vec.append(
            &mut (TryInto::<u32>::try_into(review.len())
                .unwrap()
                .to_le_bytes())
            .try_into()
            .unwrap(),
        );
        data_vec.append(&mut review.into_bytes());

        // create transaction object with instructions, accounts, and input data
        let mut transaction = Transaction::new_with_payer(
            &[
                create_mint_acct_ix,
                init_mint_ix,
                init_ata_ix,
                Instruction {
                    program_id: program_id,
                    accounts: vec![
                        AccountMeta::new_readonly(payer.pubkey(), true),
                        AccountMeta::new(review_pda, false),
                        AccountMeta::new(comment_pda, false),
                        AccountMeta::new(mint_keypair.pubkey(), false),
                        AccountMeta::new_readonly(mint_auth, false),
                        AccountMeta::new(user_ata, false),
                        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                    ],
                    //data: vec![0, title, rating, review],
                    data: data_vec,
                },
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &mint_keypair], recent_blockhash);

        // process transaction and compare the result
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(_));
    }
}
