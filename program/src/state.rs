use {
    crate::{error::MetadataError, utils::try_from_slice_checked},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
        pubkey::Pubkey,
    },
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "metadata";


pub const MAX_NAME_LENGTH: usize = 32;


pub const MAX_URI_LENGTH: usize = 200;


pub const MAX_DATA_SIZE: usize =
    1
    + 4
    + MAX_NAME_LENGTH
    + 4
    + MAX_URI_LENGTH
    + 8
    + 8
    + 32;


#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct NFTData {
    /// The id for the asset
    pub id: u8,
    /// The name of the asset
    pub name: String,
    /// URI pointing to On-chain or CDN token image
    pub uri: String,
    /// Historical price in last sales (0-10000)
    pub last_price: u64,
    /// Published price for new sales (0-10000)
    pub listed_price: u64,
    /// Address of owner NFT
    pub owner_nft_address: Pubkey,
}

impl NFTData {
    pub fn from_account_info(a: &AccountInfo) -> Result<NFTData, ProgramError> {
        let md: NFTData =
            try_from_slice_checked(&a.data.borrow_mut(), MAX_DATA_SIZE)?;

        Ok(md)
    }
}
