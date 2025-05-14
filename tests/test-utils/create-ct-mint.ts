import { AnchorProvider, toInstruction, web3 } from '@coral-xyz/anchor'
import {
  createInitializeMintInstruction,
  MINT_SIZE,
  TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token'

export const createConfidentialTransferMint = async (
  provider: AnchorProvider,
  authority: web3.PublicKey,
  decimals: number,
  payer: web3.Keypair,
): Promise<web3.PublicKey> => {
  // Generate a new keypair for the mint
  const mintKeypair = web3.Keypair.generate()

  // // xxx mint space 435, lamports 3918480

  // const MINT_EXTENDED_SIZE = 435 // from rust code

  // // Calculate lamports required for mint account (including CT extension space)
  // const lamports =
  //   await provider.connection.getMinimumBalanceForRentExemption(
  //     MINT_EXTENDED_SIZE,
  //   )

  // console.log('xxx mint lamports', lamports)

  // let ret = getInitializeConfidentialTransferMintInstruction({
  //   mint: undefined,
  //   authority: address(authority.toString()),
  //   autoApproveNewAccounts: true,
  //   auditorElgamalPubkey: null,
  // })

  // let ix = new web3.TransactionInstruction({
  //       keys: ret.accounts,
  //   programId: new web3.PublicKey(ret.programAddress.toString()),
  //   data?: ret.data
  // });

  // console.log(ret.)

  // // Create transaction to initialize the mint with CT extension
  // const transaction = new web3.Transaction().add(
  //   // Create the mint account
  //   web3.SystemProgram.createAccount({
  //     fromPubkey: payer.publicKey,
  //     newAccountPubkey: mintKeypair.publicKey,
  //     space: MINT_SIZE + 12, // Include space for CT extension
  //     lamports,
  //     programId: TOKEN_2022_PROGRAM_ID,
  //   }),
  //   // Initialize the mint
  //   createInitializeMintInstruction(
  //     mintKeypair.publicKey, // Mint address
  //     decimals, // Decimals for the token
  //     authority, // Mint authority
  //     null, // No freeze authority
  //     TOKEN_2022_PROGRAM_ID,
  //   ),
  //   // Initialize the CT extension
  //   create({
  //     mint: mintKeypair.publicKey, // Mint address
  //     authority: address(authority.toString()), // Authority for CT settings
  //     autoApproveNewAccounts: false, // Auto-approve new accounts (false for manual approval)
  //     auditorElgamalPubkey: null, // ElGamal public key (simplified)
  //   }),
  // )

  // // Send and confirm the transaction
  // const signature = await provider.sendAndConfirm(transaction, [
  //   payer,
  //   mintKeypair,
  // ])
  // console.log(
  //   `Created CT Mint: ${mintKeypair.publicKey.toBase58()}, Signature: ${signature}`,
  // )

  return mintKeypair.publicKey
}
