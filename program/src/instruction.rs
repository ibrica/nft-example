use {
    crate::{
        // deprecated_instruction::{MintPrintingTokensViaTokenArgs, SetReservationListArgs},
        state::{ NFTData,
            // Creator, 
            // EDITION, EDITION_MARKER_BIT_SIZE, PREFIX
        },
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for create call
pub struct CreateMetadataAccountArgs {
    /// Note that unique metadatas are disabled for now.
    pub data: NFTData,
    /// Whether you want your metadata to be updateable in the future.
    // pub is_mutable: bool,
    pub id: u8,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct UpdateNFTPriceArgs {
    /// Update price of NFT from Id for it's owner.
    pub id: u8,
    pub price: u64,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PurchaseNFTArgs {
    pub id: u8,
    pub new_name: Option<String>,
    pub new_uri: Option<String>,
    pub new_price: Option<u64>,
}

/// Instructions supported by the Metadata program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum MetadataInstruction {
    /// Create Metadata object.
    ///   0. `[writable]`  Metadata key (pda of ['metadata', program id, mint id])
    ///   1. `[]` Mint of token asset
    ///   2. `[signer]` Mint authority
    ///   3. `[signer]` payer
    ///   4. `[]` update authority info
    ///   5. `[]` System program
    ///   6. `[]` Rent info
    CreateMetadataAccount(CreateMetadataAccountArgs),
    
    /// Update price of NFT from Id.
    ///   0. `[writable]`  Metadata key (pda of ['metadata', program id, mint id])
    ///   1. `[]` Mint of token asset
    ///   2. `[signer]` Mint authority
    ///   3. `[signer]` payer
    ///   4. `[]` update authority info
    ///   5. `[]` System program
    ///   6. `[]` Rent info
    UpdateNFTPrice(UpdateNFTPriceArgs),

    /// Update price of NFT from Id.
    ///   0. `[writable]`  Metadata key (pda of ['metadata', program id, mint id])
    ///   1. `[]` Mint of token asset
    ///   2. `[signer]` Mint authority
    ///   3. `[signer]` payer
    ///   4. `[]` update authority info
    ///   5. `[]` System program
    ///   6. `[]` Rent info
    PurchaseNFT(PurchaseNFTArgs),
}

/// Creates an CreateMetadataAccounts instruction
#[allow(clippy::too_many_arguments)]
pub fn create_metadata_accounts(
    program_id: Pubkey,
    metadata_account: Pubkey,
    // mint: Pubkey,
    // mint_authority: Pubkey,
    payer: Pubkey,
    // update_authority: Pubkey,
    id: u8,
    name: String,
    // symbol: String,
    uri: String,
    last_price: u64,
    listed_price: u64,
    owner_nft_address: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            // AccountMeta::new_readonly(mint, false),
            //AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new(payer, true),
           // AccountMeta::new(owner_nft_address, false),
            // AccountMeta::new_readonly(update_authority, update_authority_is_signer),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::CreateMetadataAccount(CreateMetadataAccountArgs {
            data: NFTData {
                id,
                name,
                uri,
                last_price,
                listed_price,
                owner_nft_address,
            },
            id,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// update NFT price instruction
pub fn update_nft_price(
    program_id: Pubkey,
    metadata_account: Pubkey,
    id: u8,
    new_price: u64,
    owner: Pubkey,
    owner_nft_token_account: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new_readonly(owner_nft_token_account, false),
        ],
        data: MetadataInstruction::UpdateNFTPrice(UpdateNFTPriceArgs {
            id,
            price: new_price,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// purchase NFT instruction
pub fn purchase_nft(
    program_id: Pubkey,
    metadata_account: Pubkey,
    id: u8,
    new_name: Option<String>,
    new_uri: Option<String>,
    new_price: Option<u64>,
    payer: Pubkey,
    nft_owner_address: Pubkey,
    nft_token_account: Pubkey,
    new_token_mint_address: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(metadata_account, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(nft_owner_address, false),
            AccountMeta::new_readonly(nft_token_account, false),
            AccountMeta::new_readonly(new_token_mint_address, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: MetadataInstruction::PurchaseNFT(PurchaseNFTArgs {
            id,
            new_name,
            new_uri,
            new_price,
        })
        .try_to_vec()
        .unwrap(),
    }
}
