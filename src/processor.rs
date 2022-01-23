use {
    crate::{
        error::MetadataError,
        instruction::MetadataInstruction,
        state::{
            NFTData,
            PREFIX,
        },
        utils::{
            assert_owned_by, assert_initialized,
            process_create_metadata_accounts_logic,
            process_purchase_nft_logic,
            CreateMetadataAccountsLogicArgs,
            PurchaseNFTLogicArgs,
        },
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
    spl_token::state::{Account, Mint},
};

pub fn process_instruction<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    input: &[u8],
) -> ProgramResult {
    let instruction = MetadataInstruction::try_from_slice(input)?;
    match instruction {
        MetadataInstruction::CreateMetadataAccount(args) => {
            msg!("Instruction: Create Metadata Accounts");
            process_create_metadata_accounts(
                program_id,
                accounts,
                args.data,
                args.id,
            )
        }
        MetadataInstruction::UpdateNFTPrice(args) => {
            msg!("Instruction: Update NFT Price from Id");
            process_update_NFT_price(
                program_id,
                accounts,
                args.id,
                args.price,
            )
        }
        MetadataInstruction::PurchaseNFT(args) => {
            msg!("Instruction: Purchase NFT from Id");
            process_purchase_nft(
                program_id,
                accounts,
                args.id,
                args.new_name,
                args.new_uri,
                args.new_price,
            )
        }
    }
}

pub fn process_create_metadata_accounts<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: NFTData,
    id: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_account_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    process_create_metadata_accounts_logic(
        &program_id,
        CreateMetadataAccountsLogicArgs {
            metadata_account_info,
            payer_account_info,
            system_account_info,
            rent_info,
        },
        data,
        id,
    )
}

/// Purchase NFT from id instruction
pub fn process_purchase_nft<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    id: u8,
    new_name: Option<String>,
    new_uri: Option<String>,
    price: Option<u64>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let nftdata_account_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let nft_owner_address_info = next_account_info(account_info_iter)?;
    let nft_account_info = next_account_info(account_info_iter)?;
    let new_token_mint_address = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    process_purchase_nft_logic(
        &program_id,
        PurchaseNFTLogicArgs {
            nftdata_account_info,
            payer_account_info,
            nft_owner_address_info,
            nft_account_info,
            new_token_mint_address,
            system_account_info,
            rent_info,
        },
        id,
        new_name,
        new_uri,
        price,
    )
}

/// Update existing NFT price instruction
pub fn process_update_NFT_price(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    NFT_id: u8,
    new_price: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let metadata_account_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let owner_nft_token_account_info = next_account_info(account_info_iter)?;
    let metadata_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &[NFT_id],
    ];
    let (metadata_key, _) =
        Pubkey::find_program_address(metadata_seeds, program_id);
    if *metadata_account_info.key != metadata_key {
        msg!("----> Error: mismatch with NFT id and parsed NFT account");
        return Err(MetadataError::InvalidMetadataKey.into());
    }

    assert_owned_by(metadata_account_info, program_id)?;

    let mut metadata = NFTData::from_account_info(metadata_account_info)?;
    let token_account: Account = assert_initialized(&owner_nft_token_account_info)?;
    msg!("--> retrived: {}, generated: {}", metadata.owner_nft_address, token_account.mint);

    assert_owned_by(owner_nft_token_account_info, &spl_token::id())?;
    if metadata.owner_nft_address !=  token_account.mint {
        return Err(MetadataError::OwnerMismatch.into());
    }
    msg!("---> NFT Onwer address: {}, Retrieved: {}", owner_account_info.key, token_account.owner);
    let token_account: Account = assert_initialized(&owner_nft_token_account_info)?;
    if token_account.owner != *owner_account_info.key {
        return Err(MetadataError::InvalidOwner.into());
    }

    metadata.listed_price = new_price;

    metadata.serialize(&mut *metadata_account_info.data.borrow_mut())?;
    Ok(())
}
