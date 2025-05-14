import * as anchor from '@coral-xyz/anchor'
import { Program } from '@coral-xyz/anchor'
import { GachaSol } from '../target/types/gacha_sol'

describe('gacha-sol', () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env())

  let provider = anchor.getProvider()
  const program = anchor.workspace.gachaSol as Program<GachaSol>

  it('Is initialized!', async () => {
    console.log('K K K')
  })
})
