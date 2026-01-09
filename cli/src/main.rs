use anchor_lang::{AccountDeserialize, Discriminator, InstructionData, ToAccountMetas};
use solana_client::rpc_config::{RpcProgramAccountsConfig, RpcSendTransactionConfig};
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::program_pack::Pack;
use solana_sdk::{instruction::Instruction, program_option::COption};
use {
    clap::{Arg, ArgGroup, Command, crate_description, crate_name, crate_version},
    solana_clap_v3_utils::{
        input_parsers::{
            parse_url_or_moniker,
            signer::{SignerSource, SignerSourceParserBuilder},
        },
        input_validators::normalize_to_url_if_moniker,
        keypair::signer_from_path,
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        message::Message,
        pubkey::Pubkey,
        signature::{Signature, Signer},
        transaction::Transaction,
    },
    std::{error::Error, process::exit, rc::Rc, sync::Arc},
};

struct Config {
    commitment_config: CommitmentConfig,
    payer: Arc<dyn Signer>,
    json_rpc_url: String,
    verbose: bool,
}

async fn process_get_bounty(
    rpc_client: &Arc<RpcClient>,
    bounty_address: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let data = rpc_client.get_account_data(&bounty_address).await.unwrap();

    let bounty = bounty_hunter::state::Bounty::try_deserialize(&mut data.as_ref())
        .expect("bounty does not exist");

    println!(
        "BOUNTY: \n\t maker: {} \n\t description: {} \n\t link: {} \n\t reward: {} \n\t accepted submission: {}",
        bounty.maker, bounty.description, bounty.link, bounty.reward, bounty.accepted_submission
    );

    Ok(())
}

async fn process_get_all_bounties(rpc_client: &Arc<RpcClient>) -> Result<(), Box<dyn Error>> {
    let data = rpc_client
        .get_program_accounts_with_config(
            &bounty_hunter::ID,
            RpcProgramAccountsConfig {
                filters: Some([RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Bytes([1].to_vec())))].to_vec()),
                //filters: Some([RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Base64("AQ==".to_owned())))].to_vec()),
                //filters: Some([RpcFilterType::DataSize(1214)].to_vec()),
                account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                    encoding: Some(
                        anchor_client::solana_account_decoder::UiAccountEncoding::Base64,
                    ),
                    commitment: None,
                    data_slice: None,
                    min_context_slot: None,
                },
                ..Default::default()
            },
        )
        .await
        .expect("something went wrong");

    for (pk, account) in data {
        let bounty = bounty_hunter::state::Bounty::try_deserialize(&mut account.data.as_ref())
            .expect("bounty does not exist");

        println!(
            "BOUNTY {}: \n\t maker: {} \n\t description: {} \n\t link: {} \n\t reward: {} \n\t accepted submission: {}",
            pk,
            bounty.maker,
            bounty.description,
            bounty.link,
            bounty.reward,
            bounty.accepted_submission
        );
    }

    Ok(())
}

async fn process_create_bounty(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    description: String,
    link: String,
    reward: u64,
) -> Result<Signature, Box<dyn Error>> {
    let seed: u64 = rand::random();
    let bounty = Pubkey::find_program_address(
        &[
            b"bounty",
            payer.pubkey().as_ref(),
            seed.to_le_bytes().as_ref(),
        ],
        &bounty_hunter::ID,
    );

    let accounts = bounty_hunter::accounts::CreateBounty {
        bounty: bounty.0,
        maker: payer.pubkey(),
        system_program: solana_system_interface::program::ID,
    }
    .to_account_metas(None);

    let data = bounty_hunter::instruction::CreateBounty {
        description,
        link,
        reward,
        seed,
    }
    .data();

    let ix = Instruction {
        accounts,
        data,
        program_id: bounty_hunter::ID,
    };

    let mut transaction =
        Transaction::new_unsigned(Message::new(&[ix].as_slice(), Some(&payer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&[payer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    println!("bounty : {:?}", bounty.0);

    Ok(signature)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app_matches = Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg({
            let arg = Arg::new("config_file")
                .short('C')
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::new("payer")
                .long("payer")
                .short('k')
                .value_name("KEYPAIR")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair [default: client keypair]"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::new("json_rpc_url")
                .short('u')
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .value_parser(parse_url_or_moniker)
                .help("JSON RPC URL for the cluster [default: value from configuration file]"),
        )
        .subcommand(
            Command::new("create-bounty")
                .about("Creates a bounty")
                .arg(
                    Arg::new("description")
                        .value_name("description")
                        .takes_value(true)
                        .required(true)
                        .help("Bounty description"),
                )
                .arg(
                    Arg::new("link")
                        .value_name("link")
                        .takes_value(true)
                        .required(true)
                        .help("Bounty link"),
                )
                .arg(
                    Arg::new("reward")
                        .value_name("reward")
                        .takes_value(true)
                        .required(true)
                        .help("Bounty reward"),
                ),
        )
        .subcommand(
            Command::new("get-bounty").about("gets a bounty").arg(
                Arg::new("bounty_address")
                    .value_name("bounty_address")
                    .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .display_order(1)
                    .help("Specify the bounty address"),
            ),
        )
        .subcommand(Command::new("get-all-bounties").about("gets all bounties"))
        /*.subcommand(
            Command::new("delete-config")
                .about("Deletes a list")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .display_order(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("receiver_address")
                        .short('r')
                        .long("receiver")
                        .value_name("RECEIVER_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(false)
                        .help("Specify the receiver address"),
        ))
        .subcommand(
            Command::new("set-authority")
                .about("Sets the authority of a mint config")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .display_order(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("new_authority")
                        .value_name("NEW_AUTHORITY")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .short('a')
                        .long("new-authority")
                        .help("Specify the new authority address"),
        ))
        .subcommand(
            Command::new("set-gating-program")
                .about("Sets the gating program of a mint config")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .display_order(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("new_gating_program")
                        .value_name("NEW_GATING_PROGRAM")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .display_order(2)
                        .short('g')
                        .long("new-gating-program")
                        .help("Specify the new gating program address"),
        ))
        .subcommand(
            Command::new("set-instructions")
                .about("Sets the gating program of a mint config")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .display_order(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("enable_thaw")
                        .value_name("ENABLE_THAW")
                        .takes_value(false)
                        .long("enable-thaw")
                        .required(false)
                        .help("Enable thaw instructions"),
                )
                .arg(
                    Arg::new("disable_thaw")
                        .value_name("DISABLE_THAW")
                        .takes_value(false)
                        .long("disable-thaw")
                        .required(false)
                        .help("Disable thaw instructions"),
                )
                .arg(
                    Arg::new("enable_freeze")
                        .value_name("ENABLE_FREEZE")
                        .takes_value(false)
                        .long("enable-freeze")
                        .required(false)
                        .help("Enable freeze instructions"),
                )
                .arg(
                    Arg::new("disable_freeze")
                        .value_name("DISABLE_FREEZE")
                        .takes_value(false)
                        .long("disable-freeze")
                        .required(false)
                        .help("Disable freeze instructions"),
                )
                .group(ArgGroup::new("thaw")
                    .required(true)
                    .args(&["enable_thaw", "disable_thaw"])
                )
                .group(ArgGroup::new("freeze")
                    .required(true)
                    .args(&["enable_freeze", "disable_freeze"])
                )
        )
        .subcommand(
            Command::new("thaw-permissionless")
                .about("Thaws a token account")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .long("mint")
                        .required_unless_present("token_account")
                        .display_order(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("token_account")
                        .value_name("TOKEN_ACCOUNT")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .long("token-account")
                        .required_unless_present("mint_address")
                        .required_unless_present("token_account_owner")
                        .conflicts_with("mint_address")
                        .conflicts_with("token_account_owner")
                        .help("Specify the token account address"),
                )
                .arg(
                    Arg::new("token_account_owner")
                        .value_name("TOKEN_ACCOUNT_OWNER")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .long("owner")
                        .required_unless_present("token_account")
                        .conflicts_with("token_account")
                        .help("Specify the token account owner address"),
                )
        )
        .subcommand(
            Command::new("create-ata-and-thaw-permissionless")
                .about("Creates an associated token account and thaws it")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .long("mint")
                        .required(true)
                        .display_order(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("token_account_owner")
                        .value_name("TOKEN_ACCOUNT_OWNER")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .long("owner")
                        .required(true)
                        .help("Specify the token account owner address"),
                )
        )
        .subcommand(
            Command::new("freeze-permissionless")
            .about("Freezes a token account")
            .arg(
                Arg::new("mint_address")
                    .value_name("MINT_ADDRESS")
                    .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                    .takes_value(true)
                    .long("mint")
                    .required_unless_present("token_account")
                    .display_order(1)
                    .help("Specify the mint address. Requires the owner to be specified."),
            )
            .arg(
                Arg::new("token_account")
                    .value_name("TOKEN_ACCOUNT")
                    .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                    .takes_value(true)
                    .long("token-account")
                    .required_unless_present("mint_address")
                    .required_unless_present("token_account_owner")
                    .conflicts_with("mint_address")
                    .conflicts_with("token_account_owner")
                    .help("Specify the token account address"),
            )
            .arg(
                Arg::new("token_account_owner")
                    .value_name("TOKEN_ACCOUNT_OWNER")
                    .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                    .takes_value(true)
                    .long("owner")
                    .required_unless_present("token_account")
                    .conflicts_with("token_account")
                    .help("Specify the token account owner address. Requires the mint address to be specified."),
            )
        )
        .subcommand(
            Command::new("freeze")
            .about("Freezes a token account using the defined freeze authority.")
            .arg(
                Arg::new("token_account")
                    .value_name("TOKEN_ACCOUNT")
                    .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                    .takes_value(true)
                    .help("Specify the token account address"),
            )
        )
        .subcommand(
            Command::new("thaw")
            .about("Thaws a token account using the defined freeze authority.")
            .arg(
                Arg::new("token_account")
                    .value_name("TOKEN_ACCOUNT")
                    .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                    .takes_value(true)
                    .help("Specify the token account address"),
            )
        )*/
        .get_matches();

    let (command, matches) = app_matches.subcommand().unwrap();
    let mut wallet_manager: Option<Rc<RemoteWalletManager>> = None;

    let config = {
        let cli_config = if let Some(config_file) = matches.try_get_one::<String>("config_file")? {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let payer = if let Ok(Some((signer, _))) =
            SignerSource::try_get_signer(matches, "payer", &mut wallet_manager)
        {
            Box::new(signer)
        } else {
            signer_from_path(
                matches,
                &cli_config.keypair_path,
                "payer",
                &mut wallet_manager,
            )?
        };

        let json_rpc_url = normalize_to_url_if_moniker(
            matches
                .get_one::<String>("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url),
        );

        Config {
            commitment_config: CommitmentConfig::confirmed(),
            payer: Arc::from(payer),
            json_rpc_url,
            verbose: matches.try_contains_id("verbose")?,
        }
    };
    solana_logger::setup_with_default("solana=info");

    if config.verbose {
        println!("JSON RPC URL: {}", config.json_rpc_url);
    }
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        config.json_rpc_url.clone(),
        config.commitment_config,
    ));

    match (command, matches) {
        ("create-bounty", arg_matches) => {
            let description: &String = arg_matches
                .get_one("description")
                .expect("description is missing");
            let link: &String = arg_matches.get_one("link").expect("description is missing");
            let reward: u64 = arg_matches
                .get_one::<String>("reward")
                .expect("description is missing")
                .parse()
                .expect("unable to parse to u64");
            let response = process_create_bounty(
                &rpc_client,
                &config.payer,
                description.clone(),
                link.clone(),
                reward,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: create-bounty: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("get-bounty", arg_matches) => {
            let bounty_address =
                SignerSource::try_get_pubkey(arg_matches, "bounty_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            process_get_bounty(&rpc_client, bounty_address)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: get-bounty: {}", err);
                    exit(1);
                });
        }
        ("get-all-bounties", _arg_matches) => {
            process_get_all_bounties(&rpc_client)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: get-bounty: {}", err);
                    exit(1);
                });
        }

        /*("delete-list", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let receiver_address =
                SignerSource::try_get_pubkey(arg_matches, "receiver_address", &mut wallet_manager)
                    .unwrap();
            let response = process_delete_config(
                &rpc_client,
                &config.payer,
                &mint_address,
                receiver_address.as_ref(),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: delete-list: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("set-authority", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let new_authority =
                SignerSource::try_get_pubkey(arg_matches, "new_authority", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response =
                process_set_authority(&rpc_client, &config.payer, &mint_address, &new_authority)
                    .await
                    .unwrap_or_else(|err| {
                        eprintln!("error: set-authority: {}", err);
                        exit(1);
                    });
            println!("{}", response);
        }
        ("set-gating-program", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let new_gating_program = SignerSource::try_get_pubkey(
                arg_matches,
                "new_gating_program",
                &mut wallet_manager,
            )
            .unwrap()
            .unwrap();
            let response = process_set_gating_program(
                &rpc_client,
                &config.payer,
                &mint_address,
                &new_gating_program,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: set-gating-program: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("set-instructions", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();

            // clap enforces either enable or disable flags are present
            // just need to get the enable to know what to do
            let enable_thaw = arg_matches.contains_id("enable_thaw");
            let enable_freeze = arg_matches.contains_id("enable_freeze");

            let response = process_set_instructions(
                &rpc_client,
                &config.payer,
                &mint_address,
                enable_thaw,
                enable_freeze,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: set-instructions: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("thaw-permissionless", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap();
            let token_account =
                SignerSource::try_get_pubkey(arg_matches, "token_account", &mut wallet_manager)
                    .unwrap();
            let token_account_owner = SignerSource::try_get_pubkey(
                arg_matches,
                "token_account_owner",
                &mut wallet_manager,
            )
            .unwrap();
            let response = process_thaw_permissionless(
                &rpc_client,
                &config.payer,
                mint_address,
                token_account,
                token_account_owner,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: thaw-permissionless: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("create-ata-and-thaw-permissionless", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let token_account_owner = SignerSource::try_get_pubkey(
                arg_matches,
                "token_account_owner",
                &mut wallet_manager,
            )
            .unwrap()
            .unwrap();
            let response = process_create_ata_and_thaw_permissionless(
                &rpc_client,
                &config.payer,
                mint_address,
                token_account_owner,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: thaw-permissionless: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("freeze-permissionless", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap();
            let token_account =
                SignerSource::try_get_pubkey(arg_matches, "token_account", &mut wallet_manager)
                    .unwrap();
            let token_account_owner = SignerSource::try_get_pubkey(
                arg_matches,
                "token_account_owner",
                &mut wallet_manager,
            )
            .unwrap();
            let response = process_freeze_permissionless(
                &rpc_client,
                &config.payer,
                mint_address,
                token_account,
                token_account_owner,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: freeze-permissionless: {}", err);
                exit(1);
            });
            println!("{}", response);
        }
        ("freeze", arg_matches) => {
            let token_account =
                SignerSource::try_get_pubkey(arg_matches, "token_account", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response = process_freeze(&rpc_client, &config.payer, token_account)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: freeze: {}", err);
                    exit(1);
                });
            println!("{}", response);
        }
        ("thaw", arg_matches) => {
            let token_account =
                SignerSource::try_get_pubkey(arg_matches, "token_account", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response = process_thaw(&rpc_client, &config.payer, token_account)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: thaw: {}", err);
                    exit(1);
                });
            println!("{}", response);
        }*/
        _ => unreachable!(),
    };

    Ok(())
}
