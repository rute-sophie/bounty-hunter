use std::str::FromStr;

use anchor_client::{
    anchor_lang,
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file,
    },
    Client, Cluster,
};

use anchor_lang::{AccountDeserialize, Discriminator, InstructionData, ToAccountMetas};

#[test]
fn test_initialize() {
    let program_id = "ELt3SqpiHUsHJ5fxZpH1ksug6nWjAvYBxxKqK5PHfkBa";
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = read_keypair_file(&anchor_wallet).unwrap();

    let client = Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
    let program_id = Pubkey::try_from(program_id).unwrap();
    let program = client.program(program_id).unwrap();

    let tx = program
        .request()
        .accounts(bounty_hunter::accounts::CreateBounty {
            maker: todo!(),
            bounty: todo!(),
            mint: todo!(),
            maker_token_account: todo!(),
            vault: todo!(),
            system_program: todo!(),
            token_program: todo!(),
            associated_token_program: todo!(),
        })
        .args(bounty_hunter::instruction::CreateBounty {
            seed: 0,
            description: "testeeee".to_string(),
            link: "httpQQcoisa".to_string(),
            reward: 1,
        })
        .send()
        .expect("");

    println!("Your transaction signature {}", tx);
}
