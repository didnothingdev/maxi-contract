import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MaxiFarm } from "../target/types/maxi_farm";
import { Connectivity } from "./connectivity";
import { createToken, transferToken } from "./helper";
import { NATIVE_MINT_2022 } from "@solana/spl-token";
import { sleep } from "./connectivity/utils";
import { assert } from "chai";
const log = console.log

describe("maxi_farm", () => {
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);
  const program = anchor.workspace.MaxiFarm as Program<MaxiFarm>;
  const connectivity = new Connectivity({ programId: program.programId, rpcEndPoint: provider.connection.rpcEndpoint, walletInfo: provider })
  
  const creatorAuthority = web3.Keypair.generate()
  const creatorProvider = new anchor.AnchorProvider(provider.connection, new anchor.Wallet(creatorAuthority), {})
  const creatorConnectivity = new Connectivity({ programId: program.programId, rpcEndPoint: provider.connection.rpcEndpoint, walletInfo: creatorProvider })
  const creator = creatorAuthority.publicKey
  console.log('creator:', creator.toBase58())
  
  const userAuthority = web3.Keypair.generate()
  const userProvider = new anchor.AnchorProvider(provider.connection, new anchor.Wallet(userAuthority), {})
  const userConnectivity = new Connectivity({ programId: program.programId, rpcEndPoint: provider.connection.rpcEndpoint, walletInfo: userProvider })
  const user = userAuthority.publicKey
  console.log('user:', user.toBase58())

  const withdrawerAuthority = web3.Keypair.generate()
  const withdrawerProvider = new anchor.AnchorProvider(provider.connection, new anchor.Wallet(withdrawerAuthority), {})
  const withdrawerConnectivity = new Connectivity({ programId: program.programId, rpcEndPoint: provider.connection.rpcEndpoint, walletInfo: withdrawerProvider })
  const withdrawer = withdrawerAuthority.publicKey
  console.log('withdrawer:', withdrawer.toBase58())

  const mintKeypair = web3.Keypair.generate()
  
  const connection = provider.connection;
  const commonState: { quote?: string, mint?: string, poolId?: string } = {}
  let boughtAmount = 0

  before(async () => {
    await connection.requestAirdrop(creator, 100_000_000_000) // 100 SOL
    await connection.requestAirdrop(withdrawer, 1_000_000_000) // 1 SOL
    await connection.requestAirdrop(user, 10_000_000_000) // 10 SOL
    await sleep(3_000)

    const poolState = connectivity.pdas.getPoolStateAccount({ baseMint: mintKeypair.publicKey })
    const createTokenTxInfo2 = await createToken({ mintKeypair, decimals: 6, supply: 1_000_000_000, tokenReceiver: poolState }, creatorProvider)
    commonState.mint = createTokenTxInfo2.mint.toBase58()
    console.log('Deployed Token-2022 mint address:', commonState.mint)

    // const listenerCreateEvent = program.addEventListener('CreateEvent', (event, slot) => {
    //   console.log('slot: ', slot,  'event: ', event)
    // })
  })

  it("Init main state", async () => {
    const mainState = connectivity.pdas.mainState
    const mainStateInfo = await connection.getAccountInfo(mainState)
    if (!mainStateInfo) {
      const res = await connectivity.initMainState(creator)
      if (res.Err) {
        log(`Error: ${res.Err}`)
        throw "tx failed"
      }
      if (!res.Ok) throw "tx failed"
      log(`Initialized main state!`)
    }
  });

  it("Creator invites withdrawer", async () => {
    const res = await withdrawerConnectivity.setReferrer({ referrer: creator.toBase58() })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "tx failed"
    }
    if (!res.Ok) throw "tx failed"
    log(`Withdrawer was invited by creator!`)
  });

  it("Withdrawer invites user", async () => {
    const res = await userConnectivity.setReferrer({ referrer: withdrawer.toBase58() })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "tx failed"
    }
    if (!res.Ok) throw "tx failed"
    log(`User was invited by withdrawer!`)
  });

  it("Create pool", async () => {
    const baseToken = commonState.mint
    if (!baseToken) throw "token not found"
    await sleep(3_000)
    const res = await creatorConnectivity.createPool({
      baseToken, 
      metadataUri: "https://arweave.net/X4a79JOypYxoFK5nlcKv9lhhosO_WiQIP4_p9_upR5A", 
      tax: 20_00, // 20%
      maxFeeTokens: 10_000_000_000_000, // 10M token
      realQuoteThreshold: 80_000_000_000, // 80 SOL
      privSalePeriod: null
    })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "tx failed"
    }
    if (!res.Ok) throw "tx failed"
    const poolId = res.Ok.poolId
    log(`poolId: ${poolId}`)
    commonState.poolId = poolId
  });

  it("buy", async () => {
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    await sleep(3_000)
    const amount = 0.2
    const outputAmount = (await userConnectivity.getOutputAmountOnBuy({ inputAmount: amount, poolId })).Ok
    boughtAmount = outputAmount as number
    log(`Buy Output Amount: ${outputAmount}`)
    const res = await userConnectivity.buy({ poolId, amount })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "buy fail"
    }
    if (!res.Ok) throw "sell failed"
    log(`Buy Tx Sign: ${res.Ok.txSignature}`)

    const poolStateInfo = await userConnectivity.getPoolInfo(poolId)
    log(`poolStateInfo.virtBaseReserves: ${poolStateInfo?.virtBaseReserves}`)
    log(`poolStateInfo.virtQuoteReserves: ${poolStateInfo?.virtQuoteReserves}`)
    log(`poolStateInfo.realBaseReserves: ${poolStateInfo?.realBaseReserves}`)
    log(`poolStateInfo.realQuoteReserves: ${poolStateInfo?.realQuoteReserves}`)
  })

  it("sell", async () => {
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    await sleep(3_000)
    const amount = boughtAmount // sell out bought tokens
    const outputAmount = (await userConnectivity.getOutputAmountOnSell({ inputAmount: amount, poolId })).Ok
    log(`Sell Output Amount: ${outputAmount}`)
    const res = await userConnectivity.sell({ poolId, amount })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "sell fail"
    }
    if (!res.Ok) throw "sell failed"
    log(`Sell Tx Sign: ${res.Ok.txSignature}`)

    const poolStateInfo = await userConnectivity.getPoolInfo(poolId)
    log(`poolStateInfo.virtBaseReserves: ${poolStateInfo?.virtBaseReserves}`)
    log(`poolStateInfo.virtQuoteReserves: ${poolStateInfo?.virtQuoteReserves}`)
    log(`poolStateInfo.realBaseReserves: ${poolStateInfo?.realBaseReserves}`)
    log(`poolStateInfo.realQuoteReserves: ${poolStateInfo?.realQuoteReserves}`)
  })

  it("Update main state", async () => {
    const res = await connectivity.updateMainState({ newWithdrawer: withdrawer.toString(), newTradingFee: 2 })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "updateMainState fail"
    }
    if (!res.Ok) throw "updateMainState failed"
    log(`updateMainState Tx Sign: ${res.Ok.txSignature}`)
  })

  it("buy2", async () => {
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    await sleep(3_000)
    const amount = 0.2
    const outputAmount = (await userConnectivity.getOutputAmountOnBuy({ inputAmount: amount, poolId })).Ok
    boughtAmount = outputAmount as number
    log(`Buy Output Amount: ${outputAmount}`)
    const res = await userConnectivity.buy({ poolId, amount })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "buy fail"
    }
    if (!res.Ok) throw "sell failed"
    log(`Buy Tx Sign: ${res.Ok.txSignature}`)

    const poolStateInfo = await userConnectivity.getPoolInfo(poolId)
    log(`poolStateInfo.virtBaseReserves: ${poolStateInfo?.virtBaseReserves}`)
    log(`poolStateInfo.virtQuoteReserves: ${poolStateInfo?.virtQuoteReserves}`)
    log(`poolStateInfo.realBaseReserves: ${poolStateInfo?.realBaseReserves}`)
    log(`poolStateInfo.realQuoteReserves: ${poolStateInfo?.realQuoteReserves}`)
  })

  it("sell2", async () => {
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    await sleep(3_000)
    const amount = boughtAmount
    const outputAmount = (await userConnectivity.getOutputAmountOnSell({ inputAmount: amount, poolId })).Ok
    log(`Output Amount: ${outputAmount}`)
    const res = await userConnectivity.sell({ poolId, amount })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "sell fail"
    }
    if (!res.Ok) throw "sell failed"
    log(`Sell Tx Sign: ${res.Ok.txSignature}`)

    const poolStateInfo = await userConnectivity.getPoolInfo(poolId)
    log(`poolStateInfo.virtBaseReserves: ${poolStateInfo?.virtBaseReserves}`)
    log(`poolStateInfo.virtQuoteReserves: ${poolStateInfo?.virtQuoteReserves}`)
    log(`poolStateInfo.realBaseReserves: ${poolStateInfo?.realBaseReserves}`)
    log(`poolStateInfo.realQuoteReserves: ${poolStateInfo?.realQuoteReserves}`)
  })
  
  it("Withdraw (BondingCurveIncomplete: Fail)", async () => {
    await sleep(3_000)
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    const res = await withdrawerConnectivity.withdraw({ poolId })
    if (res.Ok) assert.fail("Withdraw should be failed (BondingCurveIncomplete")
  })
  
  it("buy3_1", async () => {
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    await sleep(3_000)
    const solAmount = 54.666
    const outputAmount = (await creatorConnectivity.getOutputAmountOnBuy({ inputAmount: solAmount, poolId })).Ok
    boughtAmount = outputAmount as number
    log(`Output Amount: ${outputAmount}`)
    const res = await creatorConnectivity.buy({ poolId, amount: solAmount })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "buy fail"
    }
    if (!res.Ok) throw "sell failed"
    log(`Buy Tx Sign: ${res.Ok.txSignature}`)

    const poolStateInfo = await userConnectivity.getPoolInfo(poolId)
    log(`poolStateInfo.virtBaseReserves: ${poolStateInfo?.virtBaseReserves}`)
    log(`poolStateInfo.virtQuoteReserves: ${poolStateInfo?.virtQuoteReserves}`)
    log(`poolStateInfo.realBaseReserves: ${poolStateInfo?.realBaseReserves}`)
    log(`poolStateInfo.realQuoteReserves: ${poolStateInfo?.realQuoteReserves}`)
  })

  it("buy3_2", async () => {
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    await sleep(3_000)
    const solAmount = 54.666
    const tokenAmount = (await creatorConnectivity.getOutputAmountOnBuy({ inputAmount: solAmount, poolId })).Ok || 0
    log(`Expected Output Amount: ${tokenAmount}`)
    const inputAmount = (await creatorConnectivity.getInputAmountOnBuy({ outputAmount: tokenAmount, poolId })).Ok
    boughtAmount = inputAmount as number
    log(`Input Amount: ${inputAmount}`)
    const res = await creatorConnectivity.buy2({ poolId, amount: (tokenAmount + 1) })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "buy fail"
    }
    if (!res.Ok) throw "sell failed"
    log(`Buy Tx Sign: ${res.Ok.txSignature}`)

    const poolStateInfo = await userConnectivity.getPoolInfo(poolId)
    log(`poolStateInfo.virtBaseReserves: ${poolStateInfo?.virtBaseReserves}`)
    log(`poolStateInfo.virtQuoteReserves: ${poolStateInfo?.virtQuoteReserves}`)
    log(`poolStateInfo.realBaseReserves: ${poolStateInfo?.realBaseReserves}`)
    log(`poolStateInfo.realQuoteReserves: ${poolStateInfo?.realQuoteReserves}`)
  })

  it("Withdraw", async () => {
    await sleep(3_000)
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    const res = await withdrawerConnectivity.withdraw({ poolId })
    if (res.Err) {
      log(`Error: ${res.Err}`)
      throw "Withdraw tx Error"
    }
    if (!res.Ok) throw "withdraw failed"
    log(`Withdraw Tx Sign: ${res.Ok.txSignature}`)
  })

  it("Withdraw (Unauthorised: Fail)", async () => {
    await sleep(3_000)
    const poolId = commonState.poolId
    if (!poolId) throw "pool id not found"
    const res = await userConnectivity.withdraw({ poolId })
    if (res.Ok) assert.fail("Tx should be failed (Unauthorised access)")
  })
});
