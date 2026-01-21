use anchor_litesvm::{AnchorLiteSVM, Signer};
use bounty_hunter::state::Bounty;
use litesvm_utils::{AssertionHelpers, TestHelpers};

use spl_associated_token_account_client::address::get_associated_token_address;

#[test]
fn test() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        bounty_hunter::ID,
        include_bytes!("../../target/deploy/bounty_hunter.so"),
    );

    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 3).unwrap();

    let maker_token_account = ctx
        .svm
        .create_associated_token_account(&mint.pubkey(), &user)
        .unwrap();

    ctx.svm
        .mint_to(&mint.pubkey(), &maker_token_account, &user, 10_000)
        .unwrap();

    let seed = 1u64;

    let (bounty, _) = ctx.svm.get_pda_with_bump(
        &[b"bounty", user.pubkey().as_array(), &seed.to_le_bytes()],
        &bounty_hunter::ID,
    );
    let vault = get_associated_token_address(&bounty, &mint.pubkey());

    // --- Create bounty ---
    let ix = ctx
        .program()
        .accounts(bounty_hunter::accounts::CreateBounty {
            maker: user.pubkey(),
            bounty: bounty,
            mint: mint.pubkey(),
            maker_token_account: maker_token_account,
            vault: vault,
            system_program: solana_system_interface::program::ID,
            token_program: spl_token::ID,
            associated_token_program: spl_associated_token_account_client::program::ID,
        })
        .args(bounty_hunter::instruction::CreateBounty {
            seed: seed,
            description: "testeeee".to_string(),
            link: "httpQQcoisa".to_string(),
            reward: 1,
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[&user])
        .unwrap()
        .assert_success();

    let b: Bounty = ctx.get_account(&bounty).unwrap();

    assert_eq!(b.maker, user.pubkey());
    assert_eq!(b.description, "testeeee".to_string());
    assert_eq!(b.link, "httpQQcoisa".to_string());
    assert_eq!(b.reward, 1);
    assert_eq!(b.seed, seed);

    ctx.svm.assert_token_balance(&vault, 1);
}

#[test]
fn cancel_bounty_test() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        bounty_hunter::ID,
        include_bytes!("../../target/deploy/bounty_hunter.so"),
    );

    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 3).unwrap();

    let maker_token_account = ctx
        .svm
        .create_associated_token_account(&mint.pubkey(), &user)
        .unwrap();

    ctx.svm
        .mint_to(&mint.pubkey(), &maker_token_account, &user, 10_000)
        .unwrap();

    let seed = 1u64;

    let (bounty, _) = ctx.svm.get_pda_with_bump(
        &[b"bounty", user.pubkey().as_array(), &seed.to_le_bytes()],
        &bounty_hunter::ID,
    );
    let vault = get_associated_token_address(&bounty, &mint.pubkey());

    // --- Cancel bounty ---
    let cancel_ix = ctx
        .program()
        .accounts(bounty_hunter::accounts::CancelBounty {
            maker: user.pubkey(),
            bounty,
            vault,
            mint: mint.pubkey(),
            maker_token_account,
            token_program: spl_token::ID,
            associated_token_program: spl_associated_token_account_client::program::ID,
        })
        .args(bounty_hunter::instruction::CancelBounty {})
        .instruction()
        .unwrap();

    ctx.execute_instruction(cancel_ix, &[&user])
        .unwrap()
        .assert_success();

    // --- Assertions ---

    // Vault should be emptied
    ctx.svm.assert_token_balance(&vault, 0);

    // Maker should get the reward back
    ctx.svm.assert_token_balance(&maker_token_account, 10_000);

    // Bounty account should be closed (or no longer exist)
    //let b: Bounty = ctx.get_account(&bounty).unwrap();
    //assert!(b.is_none());
}

#[test]
fn submit_solution_test() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        bounty_hunter::ID,
        include_bytes!("../../target/deploy/bounty_hunter.so"),
    );

    let user = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let mint = ctx.svm.create_token_mint(&user, 3).unwrap();

    let maker_token_account = ctx
        .svm
        .create_associated_token_account(&mint.pubkey(), &user)
        .unwrap();

    ctx.svm
        .mint_to(&mint.pubkey(), &maker_token_account, &user, 10_000)
        .unwrap();

    let seed = 1u64;

    let (bounty, _) = ctx.svm.get_pda_with_bump(
        &[b"bounty", user.pubkey().as_array(), &seed.to_le_bytes()],
        &bounty_hunter::ID,
    );
    let vault = get_associated_token_address(&bounty, &mint.pubkey());

    let create_ix = ctx
        .program()
        .accounts(bounty_hunter::accounts::CreateBounty {
            maker: user.pubkey(),
            bounty,
            mint: mint.pubkey(),
            maker_token_account,
            vault,
            system_program: solana_system_interface::program::ID,
            token_program: spl_token::ID,
            associated_token_program: spl_associated_token_account_client::program::ID,
        })
        .args(bounty_hunter::instruction::CreateBounty {
            seed,
            description: "solve me".to_string(),
            link: "https://bounty.link".to_string(),
            reward: 1,
        })
        .instruction()
        .unwrap();

    // --- Setup hunter ---
    let hunter = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let (submission, _) = ctx.svm.get_pda_with_bump(
        &[b"submission", hunter.pubkey().as_ref(), bounty.as_ref()],
        &bounty_hunter::ID,
    );

    // --- Submit solution ---
    let submit_ix = ctx
        .program()
        .accounts(bounty_hunter::accounts::SubmitSolution {
            hunter: hunter.pubkey(),
            bounty,
            submission,
            system_program: solana_system_interface::program::ID,
        })
        .args(bounty_hunter::instruction::SubmitSolution {
            link: "https://github.com/solution".to_string(),
            notes: "Here is my fix".to_string(),
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(submit_ix, &[&hunter])
        .unwrap()
        .assert_success();

    // --- Assertions ---
    let b:Bounty = ctx.get_account(&submission).unwrap();

   // assert_eq!(b, bounty);
   // assert_eq!(b.accepted_submission., hunter.pubkey());
}
