import { AnchorProvider, Program, web3 } from '@coral-xyz/anchor'
import { GachaSol } from '../../target/types/gacha_sol'
import { createMint, createMint2022 } from './create-mint'
import {
  createAssociatedTokenAccount,
  createAssociatedTokenAccount2022,
} from './create-ata'
import { createConfidentialTransferMint } from './create-ct-mint'

export class TestSuite {
  public payer: web3.Keypair

  constructor(
    public provider: AnchorProvider,
    public gachaProgram: Program<GachaSol>,
  ) {
    this.payer = provider.wallet.payer!
  }

  async createAssociatedTokenAccount(
    mint: web3.PublicKey,
    owner: web3.PublicKey,
  ): Promise<web3.PublicKey> {
    return createAssociatedTokenAccount(this.provider, mint, owner, this.payer)
  }

  async createAssociatedTokenAccount2022(
    mint: web3.PublicKey,
    owner: web3.PublicKey,
  ): Promise<web3.PublicKey> {
    return createAssociatedTokenAccount2022(
      this.provider,
      mint,
      owner,
      this.payer,
    )
  }

  async createMint2022(
    authority: web3.PublicKey,
    decimals = 6,
  ): Promise<web3.PublicKey> {
    return createMint2022(this.provider, authority, decimals, this.payer)
  }

  async createMint(
    authority: web3.PublicKey,
    decimals = 6,
  ): Promise<web3.PublicKey> {
    return createMint(this.provider, authority, decimals, this.payer)
  }

  get gameConfigPublicKey() {
    let gameConfigPubkey = web3.PublicKey.findProgramAddressSync(
      [Buffer.from('game_config')],
      this.gachaProgram.programId,
    )

    return gameConfigPubkey
  }

  async createConfidentialTransferMint(
    authority: web3.PublicKey,
    decimals = 6,
  ) {
    return createConfidentialTransferMint(
      this.provider,
      authority,
      decimals,
      this.payer,
    )
  }
}
