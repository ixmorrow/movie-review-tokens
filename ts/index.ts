import * as web3 from "@solana/web3.js"
import { createInitializeMintInstruction, AuthorityType, setAuthority, createAssociatedTokenAccountInstruction, getAssociatedTokenAddress, getMint, createMint,
  getMinimumBalanceForRentExemptMint, MINT_SIZE, TOKEN_PROGRAM_ID, getOrCreateAssociatedTokenAccount, mintTo, mintToChecked } from '@solana/spl-token'
import { Buffer } from "buffer";
import * as borsh from "@project-serum/borsh";

const RPC_ENDPOINT_URL = "https://api.devnet.solana.com";
const commitment = 'confirmed';
const connection = new web3.Connection(RPC_ENDPOINT_URL, commitment);

// MY WALLET SETTING
const id_json_path = require('os').homedir() + "/.config/solana/test-wallet.json";
const secret = Uint8Array.from(JSON.parse(require("fs").readFileSync(id_json_path)));
const wallet = web3.Keypair.fromSecretKey(secret as Uint8Array);

const program_id = new web3.PublicKey("4Q3oxdrU6TusRSJ3oSfV7cQ2Hv4X2M8PZtWixgxNaFrU")
const token_mint = new web3.PublicKey("5H9EaYK9HkrGeaMH1eRQWDti712xkwtYeLuSiLfdsCCh")
const mint_auth = new web3.PublicKey("9QQ1oYx3TLorNNh7waY872JVskNjE1wJL5jvDk9vkxdG")

async function createTokenMint(){

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
    await new Promise((resolve) => setTimeout(resolve, 1000));

    const mintInfo = await getMint(
        connection,
        tokenMint
      )
      
      console.log(mintInfo);

}

async function changeAuth(){
    let pda = (await web3.PublicKey.findProgramAddress(
      [Buffer.from("tokens")],
      program_id
    ))[0];
    console.log("PDA: " + pda);
  
    const txid = await setAuthority(connection, wallet, token_mint, wallet, AuthorityType.MintTokens, pda)

    console.log("tx signature " + txid)
    console.log(`https://explorer.solana.com/tx/${txid}?cluster=devnet`)
}