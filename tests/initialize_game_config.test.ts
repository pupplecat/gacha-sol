import * as anchor from '@coral-xyz/anchor'
import { Program, web3 } from '@coral-xyz/anchor'
import { GachaSol } from '../target/types/gacha_sol'
import { BN } from 'bn.js'
import { SYSTEM_PROGRAM_ID } from '@coral-xyz/anchor/dist/cjs/native/system'
import { TestSuite } from './test-utils/test-suite'

describe('gacha-sol', () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env())

  let provider = anchor.getProvider() as anchor.AnchorProvider
  const program = anchor.workspace.gachaSol as Program<GachaSol>
  const programId = program.programId

  it('Should initialize game config successfully', async () => {
    let testSuite = new TestSuite(provider, program)

    let gameConfigPubkey = testSuite.gameConfigPublicKey

    console.log('gameConfigPubkey', gameConfigPubkey.toString())

    let authority = web3.Keypair.generate()
    let payer = testSuite.payer!
    let purchaseMint = await testSuite.createMint(authority.publicKey)
    let rewardMint = await testSuite.createConfidentialTransferMint(
      authority.publicKey,
    )
    let gameVault = await testSuite.createAssociatedTokenAccount(
      purchaseMint,
      authority.publicKey,
    )

    let accounts = {
      gameConfig: gameConfigPubkey,
      authority: authority.publicKey,
      purchaseMint,
      rewardMint,
      gameVault: gameVault,
      payer: payer.publicKey,
      systemProgram: SYSTEM_PROGRAM_ID,
    }
    let tx = await program.methods
      .initializeGameConfig({
        pullPrice: new BN(1_000_000),
      })
      .accounts(accounts)
      .signers([payer])
      .rpc()
  })
})
