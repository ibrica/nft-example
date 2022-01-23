// use solana_client::rpc_request::TokenAccountsFilter;

use {
    clap::{crate_description, crate_name, crate_version, App, Arg, ArgMatches, SubCommand},
    metaplex_token_metadata::{
        instruction::{
            create_metadata_accounts,
            update_nft_price,
            purchase_nft,
        },
        state::{
            NFTData, PREFIX,
            // MAX_SYMBOL_LENGTH, MAX_NAME_LENGTH, MAX_URI_LENGTH,
        },
    },
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_url, is_valid_pubkey, is_valid_signer},
    },
    solana_client::{
        rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    },

    solana_program::{
        account_info::AccountInfo, borsh::try_from_slice_unchecked, program_pack::Pack,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        commitment_config::{CommitmentConfig, CommitmentLevel},
        system_instruction::create_account,
        transaction::Transaction,
    },
    
    spl_token::{
        instruction::{initialize_account, initialize_mint, mint_to},
        state::{Account as TokenAccount, Mint},
    },
    std::str::FromStr,
};
use solana_account_decoder::{
    parse_account_data::{parse_account_data, AccountAdditionalData, ParsedAccount},
    UiAccountEncoding,
};

pub const DEFAULT_LAMPORTS_PER_SOL: u64 = 1_000_000_000;


fn create_metadata_account_call(
    app_matches: &ArgMatches,
    payer: Keypair,
    client: RpcClient,
) -> (NFTData, Pubkey) {

    let program_key = metaplex_token_metadata::id();
    println!("--->Program_id: {}\n", program_key);

    let accounts = client.get_program_accounts(&program_key).unwrap();
    println!("--> Saved nft accounts: {}", accounts.len());
    let id = accounts.len() as u8 + 1;
    // let id = app_matches.value_of("id").unwrap().parse::<u8>().unwrap();
    let last_price = 0 as u64;
    let listed_price = (app_matches.value_of("listed_price").unwrap().parse::<f64>().unwrap() * DEFAULT_LAMPORTS_PER_SOL as f64).round() as u64;
    let name = app_matches.value_of("name").unwrap().to_owned();
    // let symbol = app_matches.value_of("symbol").unwrap().to_owned();
    let uri = app_matches.value_of("uri").unwrap().to_owned();

    let owner_key = pubkey_of(app_matches, "owner").unwrap();
    println!("--->\n Id: {},\n Name: {},\n Uri: {},\n Last_price: {},\n Listed_price: {},\n Owner: {}\n",
        id, name, uri, last_price, listed_price, owner_key
    );

    let metadata_seeds = &[PREFIX.as_bytes(), &program_key.as_ref(),&[id]];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);
    println!("---> Generated nft Id: {}", metadata_key);

    let new_metadata_instruction = create_metadata_accounts(
        program_key,
        metadata_key,
        payer.pubkey(),
        id,
        name,
        uri,
        last_price,
        listed_price,
        owner_key,
    );

    let mut instructions = vec![];

    // if create_new_mint {
    //     instructions.append(&mut new_mint_instructions)
    // }

    instructions.push(new_metadata_instruction);

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![&payer];
    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();
    let account = client.get_account(&metadata_key).unwrap();
    let metadata: NFTData = try_from_slice_unchecked(&account.data).unwrap();
    println!("---> Retrived NFT Data: {}", metadata.name);
    (metadata, metadata_key)
}

fn update_metadata_account_call(
    app_matches: &ArgMatches,
    payer: Keypair,
    client: RpcClient,
) -> (NFTData, Pubkey) {
    let program_key = metaplex_token_metadata::id();
    println!("--->Program_id: {}\n", program_key);

    let id = app_matches.value_of("id").unwrap().parse::<u8>().unwrap();
    let listed_price = (app_matches.value_of("listed_price").unwrap().parse::<f64>().unwrap() * DEFAULT_LAMPORTS_PER_SOL as f64).round() as u64;

    println!("--->\n Id: {},\n Price: {}",id, listed_price);
    
    let metadata_seeds = &[PREFIX.as_bytes(), &program_key.as_ref(),&[id]];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);
    println!("---> Get nft account from id: {}", metadata_key);
    
    let account = client.get_account(&metadata_key).unwrap();
    let metadata: NFTData = try_from_slice_unchecked(&account.data).unwrap();
    println!("---> Retrived NFT Data: name-{}, price-{}", metadata.name, metadata.listed_price);

    let filter1 = RpcFilterType::Memcmp(Memcmp {
        offset: 0,
        bytes: MemcmpEncodedBytes::Binary(metadata.owner_nft_address.to_string()),
        encoding: None,
    });
    let filter2 = RpcFilterType::DataSize(165);
    let account_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter1, filter2]),
        account_config,
        with_context: None,
    };

    let mut nft_owner_key: String = String::new();
    let mut nft_owner_account: Pubkey = Pubkey::new_unique();
    let holders = client.get_program_accounts_with_config(&spl_token::id(), config).unwrap();
    println!("---> Captured holder count: {}", holders.len());
    for (holder_address, holder_account) in holders {
        let data = parse_account_data(
            &metadata.owner_nft_address,
            &spl_token::id(),
            &holder_account.data,
            Some(AccountAdditionalData {
                spl_token_decimals: Some(0),
            }),
        ).unwrap();
        let amount = parse_token_amount(&data).unwrap();

        if amount == 1 {
            let owner_wallet = parse_owner(&data).unwrap();
            nft_owner_key = owner_wallet;
            nft_owner_account = holder_address;
        }
    }
    let owner = Pubkey::from_str(&*nft_owner_key).unwrap();
    println!("--> holder {} - {}", owner, nft_owner_account);

    let mut instructions = vec![];

    let new_metadata_instruction = update_nft_price(
        program_key,
        metadata_key,
        id,
        listed_price,
        payer.pubkey(),
        nft_owner_account,
    );

    instructions.push(new_metadata_instruction);

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![&payer];
    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();

    let account = client.get_account(&metadata_key).unwrap();
    let metadata: NFTData = try_from_slice_unchecked(&account.data).unwrap();
    println!("---> Updated NFT Data: name-{} new_price-{}", metadata.name, metadata.listed_price);
    (metadata, metadata_key)
}

fn get_all_nfts(
    client: &RpcClient,
) {
    let program_key = metaplex_token_metadata::id();
    let accounts = client.get_program_accounts(&program_key).unwrap();
    println!("--> Saved program accounts: {}", accounts.len());

    for (pubkey, account) in accounts {
        println!("nft_account: {:?}", pubkey);
        let metadata: NFTData = try_from_slice_unchecked(&account.data).unwrap();
        println!("data: {:?}", metadata);
    }
}

fn parse_token_amount(data: &ParsedAccount) -> Option<u64> {
    let amount = data
        .parsed
        .get("info")
        .ok_or("Invalid data account!").unwrap()
        .get("tokenAmount")
        .ok_or("Invalid token amount!").unwrap()
        .get("amount")
        .ok_or("Invalid token amount!").unwrap()
        .as_str()
        .ok_or("Invalid token amount!").unwrap()
        .parse().unwrap();
    Some(amount)
}

fn parse_owner(data: &ParsedAccount) -> Option<String> {
    let owner = data
        .parsed
        .get("info")
        .ok_or("Invalid owner account!").unwrap()
        .get("owner")
        .ok_or("Invalid owner account!").unwrap()
        .as_str()
        .ok_or("Invalid owner amount!").unwrap()
        .to_string();
    Some(owner)
}

fn purchase_nft_call(
    app_matches: &ArgMatches,
    payer: Keypair,
    client: RpcClient,
) -> (NFTData, Pubkey) {
    let program_key = metaplex_token_metadata::id();
    println!("--->Program_id: {}\n", program_key);

    let id = app_matches.value_of("id").unwrap().parse::<u8>().unwrap();
    let listed_price = match app_matches.value_of("listed_price") {
        Some(_val) => Some((app_matches.value_of("listed_price").unwrap().parse::<f64>().unwrap() * DEFAULT_LAMPORTS_PER_SOL as f64).round() as u64),
        None => None,
    };
    let uri = match app_matches.value_of("uri") {
        Some(val) => Some(val.to_owned()),
        None => None,
    };

    let name = match app_matches.value_of("name") {
        Some(val) => Some(val.to_owned()),
        None => None,
    };
    
    println!("--->\n Id: {},", id);
    if listed_price != None {
        println!("   Price: {}", listed_price.unwrap());
    };
    // if name != None {
    //     println!("   Name: {}", name);
    // }
    // if uri != None {
    //     println!("   Uri: {}", uri);
    // }
    
    let metadata_seeds = &[PREFIX.as_bytes(), &program_key.as_ref(),&[id]];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &program_key);
    println!("---> Get nft account from id: {}", metadata_key);
    
    let account = client.get_account(&metadata_key).unwrap();
    let metadata: NFTData = try_from_slice_unchecked(&account.data).unwrap();
    println!("---> Retrived NFT Data: name-{}, price-{}, owner_nft_account-{}", metadata.name, metadata.listed_price, metadata.owner_nft_address);
    
    let filter1 = RpcFilterType::Memcmp(Memcmp {
        offset: 0,
        bytes: MemcmpEncodedBytes::Binary(metadata.owner_nft_address.to_string()),
        encoding: None,
    });
    let filter2 = RpcFilterType::DataSize(165);
    let account_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter1, filter2]),
        account_config,
        with_context: None,
    };

    let mut nft_owner_key: String = String::new();
    let mut nft_owner_account: Pubkey = Pubkey::new_unique();
    let holders = client.get_program_accounts_with_config(&spl_token::id(), config).unwrap();
    for (holder_address, holder_account) in holders {
        let data = parse_account_data(
            &metadata.owner_nft_address,
            &spl_token::id(),
            &holder_account.data,
            Some(AccountAdditionalData {
                spl_token_decimals: Some(0),
            }),
        ).unwrap();
        let amount = parse_token_amount(&data).unwrap();

        if amount == 1 {
            let owner_wallet = parse_owner(&data).unwrap();
            nft_owner_key = owner_wallet;
            nft_owner_account = holder_address;
        }
    }
    let owner = Pubkey::from_str(&*nft_owner_key).unwrap();
    println!("--> holder {} - {}", owner, nft_owner_account);

    let mut instructions = vec![];

    let new_metadata_instruction = purchase_nft(
        program_key,
        metadata_key,
        id,
        name,
        uri,
        listed_price,
        payer.pubkey(),
        owner,
        nft_owner_account,
        // should be new mint key
        metadata.owner_nft_address,
    );

    instructions.push(new_metadata_instruction);

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers = vec![&payer];
    transaction.sign(&signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).unwrap();

    let account = client.get_account(&metadata_key).unwrap();
    let metadata: NFTData = try_from_slice_unchecked(&account.data).unwrap();
    println!("---> Updated NFT Data: name-{} new_owner-{}", metadata.name, metadata.owner_nft_address);
    (metadata, metadata_key)
}

fn main() {
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::with_name("keypair")
                .long("keypair")
                .value_name("KEYPAIR")
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster [default: devnet]"),
        )
        .arg(
            Arg::with_name("update_authority")
                .long("update_authority")
                .value_name("UPDATE_AUTHORITY")
                .takes_value(true)
                .global(true)
                .help("Update authority filepath or url to keypair besides yourself, defaults to normal keypair"),
        ).subcommand(
            SubCommand::with_name("create_metadata_accounts")
                .about("Create Metadata Accounts")
                .arg(
                    Arg::with_name("name")
                        .long("name")
                        .required(true)
                        .value_name("NAME")
                        .takes_value(true)
                        .help("Name for the NFT"),
                ).arg(
                    Arg::with_name("listed_price")
                        .long("price")
                        .value_name("PRICE")
                        .required(true)
                        .takes_value(true)
                        .help("Published price for new sales (0-10000)"),
                )
                .arg(
                    Arg::with_name("uri")
                        .long("uri")
                        .value_name("URI")
                        .takes_value(true)
                        .required(true)
                        .help("URI for the NFT"),
                )
                .arg(
                    Arg::with_name("owner")
                        .long("owner")
                        .value_name("OWNER")
                        .takes_value(true)
                        .required(true)
                        .help("Pubkey for an owner NFT"),
                )
        ).subcommand(
            SubCommand::with_name("update_metadata_accounts")
                .about("Update Metadata Accounts")
                .arg(
                    Arg::with_name("id")
                        .long("id")
                        .value_name("ID")
                        .required(true)
                        .takes_value(true)
                        .help("NFT Id for update"),
                )
                .arg(
                    Arg::with_name("listed_price")
                        .long("price")
                        .value_name("PRICE")
                        .takes_value(true)
                        .required(true)
                        .help("Published price for new sales (0-10000)"),
                )
        ).subcommand(
            SubCommand::with_name("show")
                .about("Show")
        ).subcommand(
            SubCommand::with_name("buy_nft")
                .about("Buy nft and mint NFT to your account")
                .arg(
                    Arg::with_name("id")
                        .long("id")
                        .value_name("ID")
                        .required(true)
                        .takes_value(true)
                        .help("NFT Id for update"),
                )
                .arg(
                    Arg::with_name("name")
                        .long("new_name")
                        .value_name("NAME")
                        .takes_value(true)
                        .help("Name for the NFT"),
                ).arg(
                    Arg::with_name("listed_price")
                        .long("new_price")
                        .value_name("PRICE")
                        .takes_value(true)
                        .help("Published price for new sales (0-10000)"),
                )
                .arg(
                    Arg::with_name("uri")
                        .long("new_uri")
                        .value_name("URI")
                        .takes_value(true)
                        .help("URI for the NFT"),
                )
        //     SubCommand::with_name("puff_unpuffed_metadata")
                    // .about("Take metadata that still have variable length name, symbol, and uri fields and stretch them out with null symbols so they can be searched more easily by RPC.")
        ).get_matches();

    let client = RpcClient::new(
        app_matches
            .value_of("json_rpc_url")
            .unwrap_or(&"https://api.devnet.solana.com".to_owned())
            .to_owned(),
    );

    let payer = read_keypair_file(app_matches.value_of("keypair").unwrap()).unwrap();

    let (sub_command, sub_matches) = app_matches.subcommand();
    match (sub_command, sub_matches) {
        ("create_metadata_accounts", Some(arg_matches)) => {
            let (metadata, metadata_key) = create_metadata_account_call(arg_matches, payer, client);
            println!(
                "Create metadata account with owner {:?} and key {:?} and name of {:?} and id of {}",
                metadata.owner_nft_address, metadata_key, metadata.name, metadata.id
            );
        }
        ("update_metadata_accounts", Some(arg_matches)) => {
            let (metadata, metadata_key) = update_metadata_account_call(arg_matches, payer, client);
            println!(
                "Update metadata account with owner {:?} and key {:?} and name of {:?} and id of {}",
                metadata.owner_nft_address, metadata_key, metadata.name, metadata.id
            );
        }
        ("show", Some(arg_matches)) => {
            get_all_nfts(&client);
        }
        ("buy_nft", Some(arg_matches)) => {
            let (metadata, metadata_key) = purchase_nft_call(arg_matches, payer, client);
            println!(
                "Minted Token account with owner {:?} and key {:?} and name of {:?} and id of {}",
                metadata.owner_nft_address, metadata_key, metadata.name, metadata.id
            );
        }
        _ => unreachable!(),
    }
}
