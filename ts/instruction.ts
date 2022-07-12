import * as web3 from "@solana/web3.js"
import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { Buffer } from "buffer";
import * as borsh from "@project-serum/borsh";

export const createReviewIx = (
    i: Buffer,
    feePayer: web3.PublicKey,
    movie: web3.PublicKey,
    comment: web3.PublicKey,
    tokenMint: web3.PublicKey,
    mintAuth: web3.PublicKey,
    userATA: web3.PublicKey,
    programId: web3.PublicKey
    ) => {
    return new web3.TransactionInstruction({
      keys: [
        {
            pubkey: feePayer,
            isSigner: true,
            isWritable: false,
        },
        {
          pubkey: movie,
          isSigner: false,
          isWritable: true,
        },
        {
            pubkey: comment,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: tokenMint,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: mintAuth,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: userATA,
            isSigner: false,
            isWritable: true,
        },
        {
          pubkey: web3.SystemProgram.programId,
          isSigner: false,
          isWritable: false
        },
        {
            pubkey: TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
      ],
      data: i,
      programId: programId,
    });
};

export const addCommentIx = (
    i: Buffer,
    commenter: web3.PublicKey,
    pdaReview: web3.PublicKey,
    pdaCounter: web3.PublicKey,
    pdaComment: web3.PublicKey,
    tokenMint: web3.PublicKey,
    mintAuth: web3.PublicKey,
    userATA: web3.PublicKey,
    programId: web3.PublicKey
    ) => {
        return new web3.TransactionInstruction({
            keys: [
                {
                    pubkey: commenter,
                    isSigner: true,
                    isWritable: false,
                },
                {
                  pubkey: pdaReview,
                  isSigner: false,
                  isWritable: true,
                },
                {
                    pubkey: pdaCounter,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: pdaComment,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: tokenMint,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: mintAuth,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: userATA,
                    isSigner: false,
                    isWritable: true,
                },
                {
                  pubkey: web3.SystemProgram.programId,
                  isSigner: false,
                  isWritable: false
                },
                {
                    pubkey: TOKEN_PROGRAM_ID,
                    isSigner: false,
                    isWritable: false,
                },
              ],
              data: i,
              programId: programId,
        })
}

export const updateReviewIx = (i: Buffer, feePayer: web3.PublicKey, movie: web3.PublicKey, programId: web3.PublicKey) => {
    return new web3.TransactionInstruction({
      keys: [
        {
            pubkey: feePayer,
            isSigner: true,
            isWritable: false,
        },
        {
          pubkey: movie,
          isSigner: false,
          isWritable: true,
        }
      ],
      data: i,
      programId: programId,
    });
  }

export const REVIEW_IX_DATA_LAYOUT = borsh.struct([
    borsh.u8("variant"),
    borsh.str("title"),
    borsh.u8("rating"),
    borsh.str("description"),
])

export const COMMENT_IX_DATA_LAYOUT = borsh.struct([
    borsh.u8("variant"),
    borsh.str("comment")
])


export const borshAccountSchema = borsh.struct([
    borsh.u8('counter'),
])

const borshSeedSchema = borsh.struct([
    borsh.u32('counter')
])

export function deserialize(buffer?: Buffer) {
    const ReviewCommentCounter = borshAccountSchema.decode(buffer)
    return ReviewCommentCounter
}

export function serializeCounter(count: number) {
    const buffer = Buffer.alloc(1000)
    borshSeedSchema.encode({ counter: count }, buffer)

    const instructionBuffer = buffer.slice(0, borshSeedSchema.getSpan(buffer))

    return instructionBuffer
}