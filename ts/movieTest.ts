import * as web3 from "@solana/web3.js"
import { commentOnReview, createNewReview } from "./index";


const RPC_ENDPOINT_URL = "https://api.devnet.solana.com";
const commitment = 'confirmed';
const connection = new web3.Connection(RPC_ENDPOINT_URL, commitment);

// MY WALLET SETTING
const id_json_path = require('os').homedir() + "/.config/solana/test-wallet.json";
const secret = Uint8Array.from(JSON.parse(require("fs").readFileSync(id_json_path)));
const wallet = web3.Keypair.fromSecretKey(secret as Uint8Array);

// YOUR PROGRAM ID
const program_id = new web3.PublicKey("4Q3oxdrU6TusRSJ3oSfV7cQ2Hv4X2M8PZtWixgxNaFrU")

async function testMovieReviewProgram(title: string, rating: number, description: string, comment: string, feePayer: web3.Keypair, programId: web3.PublicKey){
  // create new review
  const review = await createNewReview(title, rating, description, feePayer, programId)

  // sleep to allow time to update
  await new Promise((resolve) => setTimeout(resolve, 1000))

  // leave comment on review that was just created
  commentOnReview(comment, review.review, feePayer, review.mint, programId)
}



/* 
  This script creates a new token mint everytime it's run, feel free to play around with it. The majority of the logic is in the index.ts file.
  You will need to paste the program id of your program in the 'program_id' variable above.
  You need to either change the path to your wallet or generate a new keypair.
  The script will fund whatever keypair you pass in it it doesn't have enough SOL.
*/
testMovieReviewProgram("test movie 4", 5, "this happened to my buddy eric.", "still trash", wallet, program_id)