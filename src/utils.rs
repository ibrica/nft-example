use {
    crate::{
        error::MetadataError,
        state::{
            // get_reservation_list, EditionMarker, Key, MasterEditionV1, Metadata, EDITION,
            // EDITION_MARKER_BIT_SIZE, MAX_CREATOR_LIMIT, MAX_EDITION_LEN, MAX_EDITION_MARKER_SIZE,
            // MAX_MASTER_EDITION_LEN, MAX_SYMBOL_LENGTH,
            MAX_URI_LENGTH, MAX_NAME_LENGTH, MAX_DATA_SIZE, PREFIX, NFTData,
        },
    },
    arrayref::{array_ref, array_refs
        // array_mut_ref, mut_array_refs, 
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo,
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        program_option::COption,
        program_pack::{IsInitialized, Pack},
        pubkey::Pubkey,
        system_instruction,
        sysvar::{rent::Rent, Sysvar},
    },
    // metaplex_token_metadata::{
    //     instruction::{create_master_edition, create_metadata_accounts, update_metadata_accounts},
    // },
    spl_token::{
    //     instruction::{set_authority, AuthorityType},
        state::{Account},
    },
    std::convert::TryInto,
};

pub fn assert_data_valid(
    data: &NFTData,
    existing_metadata: &NFTData,
) -> ProgramResult {
    if data.name.len() > MAX_NAME_LENGTH {
        return Err(MetadataError::NameTooLong.into());
    }

    if data.uri.len() > MAX_URI_LENGTH {
        return Err(MetadataError::UriTooLong.into());
    }


    Ok(())
}

/// assert initialized account
pub fn assert_initialized<T: Pack + IsInitialized>(
    account_info: &AccountInfo,
) -> Result<T, ProgramError> {
    let account: T = T::unpack_unchecked(&account_info.data.borrow())?;
    if !account.is_initialized() {
        Err(MetadataError::Uninitialized.into())
    } else {
        Ok(account)
    }
}

/// Create account almost from scratch, lifted from
/// https://github.com/solana-labs/solana-program-library/tree/master/associated-token-account/program/src/processor.rs#L51-L98
#[inline(always)]
pub fn create_or_allocate_account_raw<'a>(
    program_id: Pubkey,
    new_account_info: &AccountInfo<'a>,
    rent_sysvar_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    size: usize,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    let rent = &Rent::from_account_info(rent_sysvar_info)?;
    let required_lamports = rent
        .minimum_balance(size)
        .max(1)
        .saturating_sub(new_account_info.lamports());

    if required_lamports > 0 {
        msg!("--> Transfer {} lamports to the new account", required_lamports);
        invoke(
            &system_instruction::transfer(&payer_info.key, new_account_info.key, required_lamports),
            &[
                payer_info.clone(),
                new_account_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    let accounts = &[new_account_info.clone(), system_program_info.clone()];

    msg!("--> Allocate space for the account");
    invoke_signed(
        &system_instruction::allocate(new_account_info.key, size.try_into().unwrap()),
        accounts,
        &[&signer_seeds],
    )?;

    msg!("--> Assign the account to the owning program");
    invoke_signed(
        &system_instruction::assign(new_account_info.key, &program_id),
        accounts,
        &[&signer_seeds],
    )?;

    Ok(())
}


/// Unpacks COption from a slice, taken from token program
fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

/// Cheap method to just grab owner Pubkey from token account, instead of deserializing entire thing
pub fn get_owner_from_token_account(
    token_account_info: &AccountInfo,
) -> Result<Pubkey, ProgramError> {
    // TokeAccount layout:   mint(32), owner(32), ...
    let data = token_account_info.try_borrow_data()?;
    let owner_data = array_ref![data, 32, 32];
    Ok(Pubkey::new_from_array(*owner_data))
}

pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        Err(MetadataError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

// / TokenMintToParams
pub struct TokenMintToParams<'a: 'b, 'b> {
    /// mint
    pub mint: AccountInfo<'a>,
    /// destination
    pub destination: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority_signer_seeds
    pub authority_signer_seeds: Option<&'b [&'b [u8]]>,
    /// token_program
    pub token_program: AccountInfo<'a>,
}


pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(MetadataError::IncorrectOwner.into())
    } else {
        Ok(())
    }
}


pub fn try_from_slice_checked<T: BorshDeserialize>(
    data: &[u8],
    data_size: usize,
) -> Result<T, ProgramError> {
    if data.len() != data_size
    {
        return Err(MetadataError::DataTypeMismatch.into());
    }

    let result: T = try_from_slice_unchecked(data)?;

    Ok(result)
}

pub struct CreateMetadataAccountsLogicArgs<'a> {
    pub metadata_account_info: &'a AccountInfo<'a>,
    // pub mint_info: &'a AccountInfo<'a>,
    // pub mint_authority_info: &'a AccountInfo<'a>,
    pub payer_account_info: &'a AccountInfo<'a>,
    // pub update_authority_info: &'a AccountInfo<'a>,
    pub system_account_info: &'a AccountInfo<'a>,
    pub rent_info: &'a AccountInfo<'a>,
}

/// Create a new account instruction
pub fn process_create_metadata_accounts_logic(
    program_id: &Pubkey,
    accounts: CreateMetadataAccountsLogicArgs,
    data: NFTData,
    id: u8,
    // allow_direct_creator_writes: bool,
    // is_mutable: bool,
) -> ProgramResult {
    let CreateMetadataAccountsLogicArgs {
        metadata_account_info,
        // mint_info,
        // mint_authority_info,
        payer_account_info,
        // update_authority_info,
        system_account_info,
        rent_info,
    } = accounts;

    let metadata_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        // mint_info.key.as_ref(),
        &[id],
    ];
    let (metadata_key, metadata_bump_seed) =
        Pubkey::find_program_address(metadata_seeds, program_id);
    let metadata_authority_signer_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        // mint_info.key.as_ref(),
        &[id],
        &[metadata_bump_seed],
    ];

    if *metadata_account_info.key != metadata_key {
        return Err(MetadataError::InvalidMetadataKey.into());
    }
    
    create_or_allocate_account_raw(
        *program_id,
        &metadata_account_info.clone(),
        rent_info,
        system_account_info,
        payer_account_info,
        MAX_DATA_SIZE,
        metadata_authority_signer_seeds,
    )?;
    
    let mut metadata = NFTData::from_account_info(metadata_account_info)?;
    
    assert_data_valid(
        &data,
        &metadata,
    )?;
    
    metadata.name = data.name;
    metadata.uri = data.uri;
    metadata.last_price = data.last_price;
    metadata.listed_price = data.listed_price;
    metadata.owner_nft_address = data.owner_nft_address;


    puff_out_data_fields(&mut metadata);

    metadata.serialize(&mut *metadata_account_info.data.borrow_mut())?;
    msg!("--> metadata saved");
    Ok(())
}

pub fn puff_out_data_fields(metadata: &mut NFTData) {
    let mut array_of_zeroes = vec![];
    while array_of_zeroes.len() < MAX_NAME_LENGTH - metadata.name.len() {
        array_of_zeroes.push(0u8);
    }
    metadata.name =
        metadata.name.clone() + std::str::from_utf8(&array_of_zeroes).unwrap();

    let mut array_of_zeroes = vec![];
    while array_of_zeroes.len() < MAX_URI_LENGTH - metadata.uri.len() {
        array_of_zeroes.push(0u8);
    }
    metadata.uri = metadata.uri.clone() + std::str::from_utf8(&array_of_zeroes).unwrap();
}

pub struct PurchaseNFTLogicArgs<'a> {
    pub nftdata_account_info: &'a AccountInfo<'a>,
    pub payer_account_info: &'a AccountInfo<'a>,
    pub nft_owner_address_info: &'a AccountInfo<'a>,
    pub nft_account_info: &'a AccountInfo<'a>,
    pub new_token_mint_address: &'a AccountInfo<'a>,
    pub system_account_info: &'a AccountInfo<'a>,
    pub rent_info: &'a AccountInfo<'a>,
}

/// Create a new account instruction
pub fn process_purchase_nft_logic(
    program_id: &Pubkey,
    accounts: PurchaseNFTLogicArgs,
    id: u8,
    new_name: Option<String>,
    new_uri: Option<String>,
    price: Option<u64>,
) -> ProgramResult {
    let PurchaseNFTLogicArgs {
        nftdata_account_info,
        payer_account_info,
        nft_owner_address_info,
        nft_account_info,
        new_token_mint_address,
        system_account_info,
        rent_info,
    } = accounts;

    let nftdata_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &[id],
    ];

    let (nftdata_key, nftdata_bump_seed) =
        Pubkey::find_program_address(nftdata_seeds, program_id);
 
    if nftdata_account_info.key != &nftdata_key {
        return Err(MetadataError::InvalidMetadataKey.into());
    }
    
    let mut nftdata = NFTData::from_account_info(nftdata_account_info)?;
    let token_account: Account = assert_initialized(&nft_account_info)?;
    msg!("--> received: {}, generated: {}", nft_owner_address_info.key, token_account.owner);
    if nft_owner_address_info.key != &token_account.owner {
        return Err(MetadataError::OwnerMismatch.into());
    }
    
    msg!("---> NFT Owner address: {}, Retrieved: {}", token_account.mint, nftdata.owner_nft_address);
    if nftdata.owner_nft_address != token_account.mint {
        return Err(MetadataError::InvalidOwner.into());
    }

    msg!("--> Transfer {} lamports to the new account", nftdata.listed_price);
    invoke(
        &system_instruction::transfer(&payer_account_info.key, &nft_owner_address_info.key, nftdata.listed_price as u64),
        &[
            payer_account_info.clone(),
            nft_owner_address_info.clone(),
            system_account_info.clone(),
        ],
    )?;

    
    // metadata.id = data.id;
    nftdata.name = match new_name {
        Some(new_name) => {
            new_name
        }
        None => {
            nftdata.name
        }
    };
    nftdata.uri = match new_uri {
        Some(new_uri) => {
            new_uri
        }
        None => {
            nftdata.uri
        }
    };
    nftdata.last_price = nftdata.listed_price;
    nftdata.listed_price = match price {
        Some(price) => {
            price
        }
        None => {
            nftdata.listed_price
        }
    };
    nftdata.owner_nft_address = *new_token_mint_address.key;

    puff_out_data_fields(&mut nftdata);


    nftdata.serialize(&mut *nftdata_account_info.data.borrow_mut())?;
    msg!("--> metadata replaced");
    Ok(())
}
