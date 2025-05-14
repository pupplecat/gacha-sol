import { AnchorProvider, web3 } from '@coral-xyz/anchor'
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token'

export const createAssociatedTokenAccount = async (
  provider: AnchorProvider,
  mint: web3.PublicKey,
  owner: web3.PublicKey,
  payer: web3.Keypair,
  programId = TOKEN_PROGRAM_ID,
): Promise<web3.PublicKey> => {
  // Get the associated token address
  const ata = await getAssociatedTokenAddress(
    mint,
    owner,
    false, // allowOwnerOffCurve: false for standard ATA derivation
    TOKEN_PROGRAM_ID,
  )

  // Check if the ATA already exists
  const accountInfo = await provider.connection.getAccountInfo(ata)
  if (accountInfo) {
    return ata // Return existing ATA
  }

  // Create transaction to initialize the ATA
  const transaction = new web3.Transaction().add(
    createAssociatedTokenAccountInstruction(
      payer.publicKey, // Payer of the transaction
      ata, // Associated token account address
      owner, // Owner of the ATA
      mint, // Mint address
      TOKEN_PROGRAM_ID, // Token-2022 program ID
    ),
  )

  // Send and confirm the transaction
  const signature = await provider.sendAndConfirm(transaction, [payer])
  console.log(`Created ATA: ${ata.toBase58()}, Signature: ${signature}`)

  return ata
}

export const createAssociatedTokenAccount2022 = async (
  provider: AnchorProvider,
  mint: web3.PublicKey,
  owner: web3.PublicKey,
  payer: web3.Keypair,
): Promise<web3.PublicKey> => {
  return createAssociatedTokenAccount(
    provider,
    mint,
    owner,
    payer,
    TOKEN_2022_PROGRAM_ID,
  )
}
