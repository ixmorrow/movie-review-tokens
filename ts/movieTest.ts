import * as web3 from "@solana/web3.js"
import { createAssociatedTokenAccountInstruction, getAssociatedTokenAddress, getMint,
TOKEN_PROGRAM_ID, getOrCreateAssociatedTokenAccount } from '@solana/spl-token'
import { Buffer } from "buffer";
import * as borsh from "@project-serum/borsh";
import { program_id, token_mint, mint_auth } from "./const";
import { createReviewIx, addCommentIx, deserialize, serializeCounter, COMMENT_IX_DATA_LAYOUT, REVIEW_IX_DATA_LAYOUT, updateReviewIx } from "./instruction";


const RPC_ENDPOINT_URL = "https://api.devnet.solana.com";
const commitment = 'confirmed';
const connection = new web3.Connection(RPC_ENDPOINT_URL, commitment);

// MY WALLET SETTING
const id_json_path = require('os').homedir() + "/.config/solana/test-wallet.json";
const secret = Uint8Array.from(JSON.parse(require("fs").readFileSync(id_json_path)));
const wallet = web3.Keypair.fromSecretKey(secret as Uint8Array);


async function createNewReview(title: string, rating: number, description: string, feePayer: web3.Keypair) {

    console.log("Program id: " + program_id.toBase58());
    console.log("Fee payer: " + feePayer.publicKey);

    const tx = new web3.Transaction();

    let utf8Encode = new TextEncoder();
    let buff = utf8Encode.encode(title);

    const review_pda = (await web3.PublicKey.findProgramAddress(
      [feePayer.publicKey.toBuffer(), buff],
      program_id
    ))[0]
    console.log("Review pda: ", review_pda.toBase58())

    let comment_buff = utf8Encode.encode("comment")
    const comment_count_pda = (await web3.PublicKey.findProgramAddress(
        [review_pda.toBuffer(), comment_buff],
        program_id
    ))[0]
    console.log("Count pda: ", comment_count_pda.toBase58())

    const userATA = await getOrCreateAssociatedTokenAccount(
        connection,
        wallet,
        token_mint,
        wallet.publicKey
    )

    const userAddress = await getAssociatedTokenAddress(token_mint, wallet.publicKey)

    const payload = {
      variant: 0,
      title: title,
      rating: rating,
      description: description,
    }
    const msgBuffer = Buffer.alloc(1000);
    REVIEW_IX_DATA_LAYOUT.encode(payload, msgBuffer);
    const postIxData = msgBuffer.slice(0, REVIEW_IX_DATA_LAYOUT.getSpan(msgBuffer));

    console.log("creating init instruction");

    const ix = createReviewIx(
        postIxData,
        feePayer.publicKey,
        review_pda,
        comment_count_pda,
        userAddress
    );
    tx.add(ix);

    if ((await connection.getBalance(feePayer.publicKey)) < 1.0) {
        console.log("Requesting Airdrop of 2 SOL...");
        await connection.requestAirdrop(feePayer.publicKey, 2e9);
        console.log("Airdrop received");
      }
  

    let signers = [feePayer];

    console.log("sending tx");
    let txid = await web3.sendAndConfirmTransaction(connection, tx, signers, {
      skipPreflight: true,
      preflightCommitment: "confirmed"
    });
    console.log(`https://explorer.solana.com/tx/${txid}?cluster=devnet`);

}

async function commentOnReview(comment: string, review: web3.PublicKey){
    console.log("Program id: " + program_id.toBase58())
    console.log("Fee payer: " + wallet.publicKey)
    console.log("Review PDA: ", review.toBase58())

    const tx = new web3.Transaction()

    let utf8Encode = new TextEncoder()
    //let buff = utf8Encode.encode(comment)
    let comment_buff = utf8Encode.encode("comment")

    // derive User's associated token account of Movie token Mint
    const userATA = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet,
      token_mint,
      wallet.publicKey
  )

  const userAddress = await getAssociatedTokenAddress(token_mint, wallet.publicKey)

    // derive pda of comment counter
    const comment_count_pda = (await web3.PublicKey.findProgramAddress(
        [review.toBuffer(), comment_buff],
        program_id
    ))[0]
    console.log("Count pda: ", comment_count_pda.toBase58())

    // fetch and deserialize data of comment counter account
    let account = await connection.getAccountInfo(comment_count_pda)
    if (account != null) {
        let commentData = deserialize(account.data)
        //console.log(commentData)
        let countBuff = serializeCounter(commentData.counter)
        //console.log(countBuff)

        // derive pda of address of where the comment will live
        const comment_pda = (await web3.PublicKey.findProgramAddress(
            [review.toBuffer(), countBuff],
            program_id
        ))[0]
        console.log("Comment pda: ", comment_pda.toBase58())

        const payload = {
          variant: 2,
          comment: comment
        }
        const msgBuffer = Buffer.alloc(1000);
        COMMENT_IX_DATA_LAYOUT.encode(payload, msgBuffer);
        const postIxData = msgBuffer.slice(0, COMMENT_IX_DATA_LAYOUT.getSpan(msgBuffer));

        const ix = addCommentIx(
          postIxData,
          wallet.publicKey,
          review,
          comment_count_pda,
          comment_pda,
          userAddress,
        )
        tx.add(ix)

        if ((await connection.getBalance(wallet.publicKey)) < 1.0) {
          console.log("Requesting Airdrop of 2 SOL...");
          await connection.requestAirdrop(wallet.publicKey, 2e9);
          console.log("Airdrop received");
        }
    
  
      let signers = [wallet];
  
      console.log("sending tx");
      let txid = await web3.sendAndConfirmTransaction(connection, tx, signers, {
        skipPreflight: true,
        preflightCommitment: "confirmed"
      });
      console.log(`https://explorer.solana.com/tx/${txid}?cluster=devnet`);
    }
}


  createNewReview("Thor: Love and Thunder", 5, "Haven't seen it yet, but am excited", wallet)
  //commentOnReview("bollocks", new web3.PublicKey("7BKNtkCMhNHyKJ369PTtAzBVWaZpU4R1svzrUyhH5j7z"))
  
  // test review
  // https://explorer.solana.com/tx/4XCKaAernwDsi5rWpZh2G5Mo8dW41jKwkfzE1ardYqhA97UJoEWPbP8YXQLrPJTQ9UY4oSuqpZMZfiAD2iTnm2KW?cluster=devnet