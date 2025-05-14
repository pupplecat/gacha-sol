import { AnchorProvider, web3 } from '@coral-xyz/anchor'
import {
  createInitializeMintInstruction,
  MINT_SIZE,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token'

export const createMint = async (
  provider: AnchorProvider,
  authority: web3.PublicKey,
  decimals: number,
  payer: web3.Keypair,
  programId = TOKEN_PROGRAM_ID,
): Promise<web3.PublicKey> => {
  // Generate a new keypair for the mint
  const mintKeypair = web3.Keypair.generate()

  // Calculate lamports required for mint account
  const lamports =
    await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE)

  // Create transaction to initialize the mint
  const transaction = new web3.Transaction().add(
    // Create the mint account
    web3.SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: mintKeypair.publicKey,
      space: MINT_SIZE,
      lamports,
      programId,
    }),
    // Initialize the mint
    createInitializeMintInstruction(
      mintKeypair.publicKey, // Mint address
      decimals, // Decimals for the token
      authority, // Mint authority
      null, // Freeze authority (null for no freeze authority)
      programId, // Token-2022 program ID
    ),
  )

  // Send and confirm the transaction
  const signature = await provider.sendAndConfirm(transaction, [
    payer,
    mintKeypair,
  ])
  console.log(
    `Created Mint: ${mintKeypair.publicKey.toBase58()}, Signature: ${signature}`,
  )

  return mintKeypair.publicKey
}

export const createMint2022 = async (
  provider: AnchorProvider,
  authority: web3.PublicKey,
  decimals: number,
  payer: web3.Keypair,
): Promise<web3.PublicKey> => {
  return createMint(provider, authority, decimals, payer, TOKEN_2022_PROGRAM_ID)
}
