import * as web3 from "@solana/web3.js"
import { AuthorityType, setAuthority,
  getAssociatedTokenAddress, createMint, getOrCreateAssociatedTokenAccount, } from '@solana/spl-token'
import { Buffer } from "buffer";
import { createReviewIx, addCommentIx, deserialize, COMMENT_IX_DATA_LAYOUT, REVIEW_IX_DATA_LAYOUT } from "./instruction";

const RPC_ENDPOINT_URL = "https://api.devnet.solana.com";
const commitment = 'confirmed';
const connection = new web3.Connection(RPC_ENDPOINT_URL, commitment);

type review = {
  mint: web3.PublicKey,
  auth: web3.PublicKey,
  review: web3.PublicKey,
  commentCounter: web3.PublicKey,
}

export async function createTokenMint(wallet: web3.Keypair){

    // create mint address w pda as authority
    const tokenMint = await createMint(
        connection,
        wallet,
        wallet.publicKey,
        null,
        9 // We are using 9 to match the CLI decimal default exactly
      )
    console.log("movie review mint pubkey: ", tokenMint.toBase58())

    // sleep to allow time to update
    // await new Promise((resolve) => setTimeout(resolve, 1000));

    // const mintInfo = await getMint(
    //     connection,
    //     tokenMint
    //   )
      
    // console.log(mintInfo);

    return tokenMint
}

export async function changeAuth(wallet: web3.Keypair, tokenMint: web3.PublicKey, programId: web3.PublicKey){
  console.log("Changing token Mint authority to PDA")
  let pda = (await web3.PublicKey.findProgramAddress(
    [Buffer.from("tokens")],
    programId
  ))[0];
  console.log("Mint Authority: " + pda)

  const txid = await setAuthority(connection, wallet, tokenMint, wallet, AuthorityType.MintTokens, pda)

  return pda
}

export async function createNewReview(title: string, rating: number, description: string, feePayer: web3.Keypair, programId: web3.PublicKey) {

  const review = {} as review

  console.log("Program id: " + programId.toBase58());
  console.log("Fee payer: " + feePayer.publicKey);

  // create token mint
  const tokenMint = await createTokenMint(feePayer)
  review.mint = tokenMint

  // setting mint authority
  const mintAuth = await changeAuth(feePayer, tokenMint, programId)
  review.auth = mintAuth

  // deriving review PDA
  let utf8Encode = new TextEncoder();
  let buff = utf8Encode.encode(title);
  const review_pda = (await web3.PublicKey.findProgramAddress(
    [feePayer.publicKey.toBuffer(), buff],
    programId
  ))[0]
  console.log("Review pda: ", review_pda.toBase58())
  review.review = review_pda

  // deriving counter PDA
  let comment_buff = utf8Encode.encode("comment")
  const comment_count_pda = (await web3.PublicKey.findProgramAddress(
      [review_pda.toBuffer(), comment_buff],
      programId
  ))[0]
  console.log("Comment counter pda: ", comment_count_pda.toBase58())
  review.commentCounter = comment_count_pda

  // creating user ATA of token mint
  const userATA = await getOrCreateAssociatedTokenAccount(
    connection,
    feePayer,
    tokenMint,
    feePayer.publicKey
  )
  const userAddress = await getAssociatedTokenAddress(tokenMint, feePayer.publicKey)
  console.log("Users associate token account: ", userAddress.toBase58())

  // creating instruction
  const payload = {
    variant: 0,
    title: title,
    rating: rating,
    description: description,
  }
  const msgBuffer = Buffer.alloc(1000);
  REVIEW_IX_DATA_LAYOUT.encode(payload, msgBuffer);
  const postIxData = msgBuffer.slice(0, REVIEW_IX_DATA_LAYOUT.getSpan(msgBuffer));

  const tx = new web3.Transaction();
  console.log("creating init instruction");
  const ix = createReviewIx(
      postIxData,
      feePayer.publicKey,
      review_pda,
      comment_count_pda,
      tokenMint,
      mintAuth,
      userAddress,
      programId
  );
  tx.add(ix);

  if ((await connection.getBalance(feePayer.publicKey)) < 1.0) {
    console.log("Requesting Airdrop of 2 SOL...");
    await connection.requestAirdrop(feePayer.publicKey, 2e9);
    console.log("Airdrop received");
  }

  console.log("sending tx to create review");
  let txid = await web3.sendAndConfirmTransaction(connection, tx, [feePayer], {
    skipPreflight: true,
    preflightCommitment: "confirmed"
  });
  console.log(`https://explorer.solana.com/tx/${txid}?cluster=devnet`);

  return review
}

export async function commentOnReview(comment: string, review: web3.PublicKey, feePayer: web3.Keypair, tokenMint: web3.PublicKey, programId: web3.PublicKey){
  console.log("Program id: " + programId.toBase58())
  console.log("Fee payer: " + feePayer.publicKey)
  console.log("Review PDA: ", review.toBase58())

  const tx = new web3.Transaction()

  let utf8Encode = new TextEncoder()
  //let buff = utf8Encode.encode(comment)
  let comment_buff = utf8Encode.encode("comment")

  // derive User's associated token account of Movie token Mint
  const userATA = await getOrCreateAssociatedTokenAccount(
    connection,
    feePayer,
    tokenMint,
    feePayer.publicKey
)

const userAddress = await getAssociatedTokenAddress(tokenMint, feePayer.publicKey)

  // derive pda of comment counter
  const comment_count_pda = (await web3.PublicKey.findProgramAddress(
      [review.toBuffer(), comment_buff],
      programId
  ))[0]
  console.log("Count pda: ", comment_count_pda.toBase58())

  // fetch and deserialize data of comment counter account
  let account = await connection.getAccountInfo(comment_count_pda)
  
  let commentData = deserialize(account?.data)
  console.log(commentData)

  // derive pda of address of where the comment will live
  const comment_pda = (await web3.PublicKey.findProgramAddress(
      [review.toBuffer(), Buffer.from([commentData.counter])],
      programId
  ))[0]
  console.log("Comment pda: ", comment_pda.toBase58())

  let mintAuth = (await web3.PublicKey.findProgramAddress(
    [Buffer.from("tokens")],
    programId
  ))[0];
  console.log("Mint Authority: ", mintAuth.toBase58())

  const payload = {
    variant: 2,
    comment: comment
  }
  const msgBuffer = Buffer.alloc(1000);
  COMMENT_IX_DATA_LAYOUT.encode(payload, msgBuffer);
  const postIxData = msgBuffer.slice(0, COMMENT_IX_DATA_LAYOUT.getSpan(msgBuffer));

  const ix = addCommentIx(
    postIxData,
    feePayer.publicKey,
    review,
    comment_count_pda,
    comment_pda,
    tokenMint,
    mintAuth,
    userAddress,
    programId
  )
  tx.add(ix)

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