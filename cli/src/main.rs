use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_client::rpc_config::RpcProgramAccountsConfig;
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_sdk::instruction::Instruction;
use {
    clap::{Arg, Command, crate_description, crate_name, crate_version},
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
                filters: Some(
                    [RpcFilterType::Memcmp(Memcmp::new(
                        0,
                        MemcmpEncodedBytes::Bytes([1].to_vec()),
                    ))]
                    .to_vec(),
                ),
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

async fn process_get_submission(
    rpc_client: &Arc<RpcClient>,
    submission_address: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let data = rpc_client
        .get_account_data(&submission_address)
        .await
        .unwrap();

    let submission = bounty_hunter::state::Submission::try_deserialize(&mut data.as_ref())
        .expect("bounty does not exist");

    println!(
        "SUBMISSION: \n\t hunter: {} \n\t notes: {} \n\t link: {} \n\t bounty: {}",
        submission.hunter, submission.notes, submission.link, submission.bounty
    );

    Ok(())
}

async fn process_accept_submission(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    submission_address: Pubkey,
) -> Result<Signature, Box<dyn Error>> {
    let data = rpc_client
        .get_account_data(&submission_address)
        .await
        .unwrap();

    let submission = bounty_hunter::state::Submission::try_deserialize(&mut data.as_ref())
        .expect("bounty does not exist");

    let accounts = bounty_hunter::accounts::AcceptSolution {
        maker: payer.pubkey(),
        bounty: submission.bounty,
        submission: submission_address,
    }
    .to_account_metas(None);

    let data = bounty_hunter::instruction::AcceptSolution {}.data();

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

    Ok(signature)
}

async fn process_get_all_submissions(
    rpc_client: &Arc<RpcClient>,
    bounty_address: Option<Pubkey>,
) -> Result<(), Box<dyn Error>> {
    let mut filter_bytes = vec![2];

    if let Some(ba) = bounty_address {
        filter_bytes.extend_from_slice(ba.as_ref());
    }
    let data = rpc_client
        .get_program_accounts_with_config(
            &bounty_hunter::ID,
            RpcProgramAccountsConfig {
                filters: Some(
                    [RpcFilterType::Memcmp(Memcmp::new(
                        0,
                        MemcmpEncodedBytes::Bytes(filter_bytes),
                    ))]
                    .to_vec(),
                ),
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
        let submission =
            bounty_hunter::state::Submission::try_deserialize(&mut account.data.as_ref())
                .expect("submission does not exist");

        println!(
            "SUBMISSION {}: \n\t hunter: {} \n\t notes: {} \n\t link: {} \n\t bounty: {}",
            pk, submission.hunter, submission.notes, submission.link, submission.bounty
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

async fn process_submit_solution(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    bounty_address: Pubkey,
    notes: String,
    link: String,
) -> Result<Signature, Box<dyn Error>> {
    let submission = Pubkey::find_program_address(
        &[
            b"submission",
            payer.pubkey().as_ref(),
            bounty_address.as_ref(),
        ],
        &bounty_hunter::ID,
    )
    .0;

    let accounts = bounty_hunter::accounts::SubmitSolution {
        bounty: bounty_address,
        hunter: payer.pubkey(),
        submission,
        system_program: solana_system_interface::program::ID,
    }
    .to_account_metas(None);

    let data = bounty_hunter::instruction::SubmitSolution { notes, link }.data();

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

    println!("submission : {:?}", submission);

    Ok(signature)
}

async fn process_cancel_bounty(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    bounty_address: Pubkey,
) -> Result<Signature, Box<dyn Error>> {
    let accounts = bounty_hunter::accounts::CancelBounty {
        bounty: bounty_address,
        maker: payer.pubkey(),
        system_program: solana_system_interface::program::ID,
    }
    .to_account_metas(None);

    let data = bounty_hunter::instruction::CancelBounty {}.data();

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
            Command::new("submit-solution")
                .about("Submits a solution to a bounty")
                .arg(
                    Arg::new("bounty_address")
                        .value_name("bounty_address")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .help("Specify the bounty address"),
                )
                .arg(
                    Arg::new("notes")
                        .value_name("notes")
                        .takes_value(true)
                        .required(true)
                        .help("Submission notes"),
                )
                .arg(
                    Arg::new("link")
                        .value_name("link")
                        .takes_value(true)
                        .required(true)
                        .help("Submission link"),
                ),
        )
        .subcommand(
            Command::new("get-bounty").about("Gets a bounty").arg(
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
        .subcommand(
            Command::new("get-submission")
                .about("Gets a submission")
                .arg(
                    Arg::new("submission_address")
                        .value_name("submission_address")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .display_order(1)
                        .help("Specify the submission address"),
                ),
        )
        .subcommand(
            Command::new("accept-submission")
                .about("Accepts a submission")
                .arg(
                    Arg::new("submission_address")
                        .value_name("submission_address")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .display_order(1)
                        .help("Specify the submission address"),
                ),
        )
        .subcommand(
            Command::new("cancel-bounty").about("Cancels a bounty").arg(
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
        .subcommand(Command::new("get-all-bounties").about("Gets all bounties"))
        .subcommand(
            Command::new("get-all-submissions")
                .about("Gets all submission")
                .arg(
                    Arg::new("bounty_address")
                        .value_name("bounty_address")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .long("bounty-address")
                        .short('b')
                        .required(false)
                        .help("Filters submissions by bounty"),
                ),
        )
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
        ("submit-solution", arg_matches) => {
            let notes: &String = arg_matches.get_one("notes").expect("notes is missing");
            let link: &String = arg_matches.get_one("link").expect("link is missing");
            let bounty_address =
                SignerSource::try_get_pubkey(arg_matches, "bounty_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response = process_submit_solution(
                &rpc_client,
                &config.payer,
                bounty_address,
                notes.clone(),
                link.clone(),
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("error: submit-solution: {}", err);
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
        ("get-submission", arg_matches) => {
            let submission_address = SignerSource::try_get_pubkey(
                arg_matches,
                "submission_address",
                &mut wallet_manager,
            )
            .unwrap()
            .unwrap();
            process_get_submission(&rpc_client, submission_address)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: get-submission: {}", err);
                    exit(1);
                });
        }
        ("accept-submission", arg_matches) => {
            let submission_address = SignerSource::try_get_pubkey(
                arg_matches,
                "submission_address",
                &mut wallet_manager,
            )
            .unwrap()
            .unwrap();
            let response = process_accept_submission(&rpc_client, &config.payer, submission_address)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: accept-submission: {}", err);
                    exit(1);
                });
            println!("{}", response);
        }
        ("cancel-bounty", arg_matches) => {
            let bounty_address =
                SignerSource::try_get_pubkey(arg_matches, "bounty_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response = process_cancel_bounty(&rpc_client, &config.payer, bounty_address)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: cancel-bounty: {}", err);
                    exit(1);
                });
            println!("{}", response);
        }
        ("get-all-bounties", _arg_matches) => {
            process_get_all_bounties(&rpc_client)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: get-bounty: {}", err);
                    exit(1);
                });
        }
        ("get-all-submissions", arg_matches) => {
            let bounty_address = if let Ok(Some(pk)) =
                SignerSource::try_get_pubkey(arg_matches, "bounty_address", &mut wallet_manager)
            {
                Some(pk)
            } else {
                None
            };
            process_get_all_submissions(&rpc_client, bounty_address)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: get-all-submission: {}", err);
                    exit(1);
                });
        }
        _ => unreachable!(),
    };

    Ok(())
}
